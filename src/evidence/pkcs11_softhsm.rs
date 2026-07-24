//! SoftHSM2 PKCS#11 live signer (feature `pkcs11`).
//!
//! Software token only — not a hardware HSM / production trust root claim.
//! Default Gate builds stay on practice Ed25519.

use std::path::Path;
use std::sync::Mutex;

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
use cryptoki::mechanism::Mechanism;
use cryptoki::object::{Attribute, AttributeType, KeyType, ObjectClass};
use cryptoki::session::UserType;
use cryptoki::types::AuthPin;
use rsa::pkcs8::EncodePublicKey;
use rsa::{BigUint, Pkcs1v15Sign, RsaPublicKey};
use sha2::{Digest, Sha256};

use super::seal::SealError;
use super::seal_sign::{SealSignature, SIGNATURE_ALG_RSA_PKCS1_SHA256, SIGNED_OVER_ACCEPTANCE};

/// Process-wide SoftHSM/PKCS#11 init lock (C_Initialize once per module load).
static PKCS11_INIT: Mutex<()> = Mutex::new(());

fn map_err(e: impl std::fmt::Display) -> SealError {
    SealError::Signature(format!("pkcs11: {e}"))
}

fn ensure_initialized(pkcs11: &Pkcs11) -> Result<(), SealError> {
    match pkcs11.initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK)) {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("ALREADY_INITIALIZED") || msg.contains("already initialized") {
                Ok(())
            } else {
                Err(map_err(e))
            }
        }
    }
}

/// Sign `acceptance_digest` UTF-8 bytes via SoftHSM RSA (CKM_SHA256_RSA_PKCS).
pub fn sign_acceptance_digest_rsa(
    module_path: &Path,
    pin: &str,
    key_label: &str,
    slot_index: usize,
    acceptance_digest: &str,
) -> Result<SealSignature, SealError> {
    if !module_path.exists() {
        return Err(SealError::Signature(format!(
            "PKCS#11 module not found: {}",
            module_path.display()
        )));
    }

    let _guard = PKCS11_INIT
        .lock()
        .map_err(|e| SealError::Signature(format!("pkcs11 lock: {e}")))?;
    let pkcs11 = Pkcs11::new(module_path).map_err(map_err)?;
    ensure_initialized(&pkcs11)?;

    let slots = pkcs11.get_slots_with_token().map_err(map_err)?;
    let slot = *slots.get(slot_index).ok_or_else(|| {
        SealError::Signature(format!(
            "PKCS#11 slot index {slot_index} out of range ({} tokens)",
            slots.len()
        ))
    })?;

    let session = pkcs11.open_rw_session(slot).map_err(map_err)?;
    let auth = AuthPin::new(pin.into());
    session
        .login(UserType::User, Some(&auth))
        .map_err(map_err)?;

    let label = key_label.as_bytes().to_vec();
    let priv_template = [
        Attribute::Class(ObjectClass::PRIVATE_KEY),
        Attribute::KeyType(KeyType::RSA),
        Attribute::Label(label.clone()),
        Attribute::Sign(true),
    ];
    let priv_keys = session.find_objects(&priv_template).map_err(map_err)?;
    let private = *priv_keys.first().ok_or_else(|| {
        SealError::Signature(format!("PKCS#11 private key label={key_label} not found"))
    })?;

    let pub_template = [
        Attribute::Class(ObjectClass::PUBLIC_KEY),
        Attribute::KeyType(KeyType::RSA),
        Attribute::Label(label),
    ];
    let pub_keys = session.find_objects(&pub_template).map_err(map_err)?;
    let public = *pub_keys.first().ok_or_else(|| {
        SealError::Signature(format!("PKCS#11 public key label={key_label} not found"))
    })?;

    let attrs = session
        .get_attributes(
            public,
            &[AttributeType::Modulus, AttributeType::PublicExponent],
        )
        .map_err(map_err)?;
    let (modulus, exponent) = match attrs.as_slice() {
        [Attribute::Modulus(m), Attribute::PublicExponent(e)] => (m.clone(), e.clone()),
        other => {
            return Err(SealError::Signature(format!(
                "unexpected PKCS#11 public attributes: {other:?}"
            )));
        }
    };

    let rsa_pub = RsaPublicKey::new(
        BigUint::from_bytes_be(&modulus),
        BigUint::from_bytes_be(&exponent),
    )
    .map_err(map_err)?;
    let spki = rsa_pub
        .to_public_key_der()
        .map_err(|e| SealError::Signature(format!("spki encode: {e}")))?;

    let msg = acceptance_digest.as_bytes();
    let signature = session
        .sign(&Mechanism::Sha256RsaPkcs, private, msg)
        .map_err(map_err)?;

    verify_rsa_pkcs1_sha256(&rsa_pub, msg, &signature)?;

    Ok(SealSignature {
        alg: SIGNATURE_ALG_RSA_PKCS1_SHA256.to_owned(),
        public_key_b64: B64.encode(spki.as_bytes()),
        sig_b64: B64.encode(signature),
        signed_over: SIGNED_OVER_ACCEPTANCE.to_owned(),
        signer_kind: Some(super::seal_sign::SIGNER_KIND_HSM_PKCS11.to_owned()),
    })
}

/// Verify RSA PKCS#1 v1.5 + SHA-256 over message bytes (SPKI DER public key).
pub fn verify_rsa_pkcs1_sha256_spki(
    public_key_der: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(), SealError> {
    use rsa::pkcs8::DecodePublicKey;
    let rsa_pub = RsaPublicKey::from_public_key_der(public_key_der)
        .map_err(|e| SealError::Signature(format!("rsa public key: {e}")))?;
    verify_rsa_pkcs1_sha256(&rsa_pub, message, signature)
}

fn verify_rsa_pkcs1_sha256(
    rsa_pub: &RsaPublicKey,
    message: &[u8],
    signature: &[u8],
) -> Result<(), SealError> {
    let hashed = Sha256::digest(message);
    let padding = Pkcs1v15Sign::new::<Sha256>();
    rsa_pub
        .verify(padding, &hashed, signature)
        .map_err(|_| SealError::Signature("rsa-pkcs1-sha256 verify failed".into()))
}

/// Provision SoftHSM token + RSA keypair for CI / local smoke (destructive init).
pub fn provision_rsa_keypair(
    module_path: &Path,
    so_pin: &str,
    user_pin: &str,
    token_label: &str,
    key_label: &str,
    slot_index: usize,
) -> Result<(), SealError> {
    if !module_path.exists() {
        return Err(SealError::Signature(format!(
            "PKCS#11 module not found: {}",
            module_path.display()
        )));
    }

    let _guard = PKCS11_INIT
        .lock()
        .map_err(|e| SealError::Signature(format!("pkcs11 lock: {e}")))?;
    let pkcs11 = Pkcs11::new(module_path).map_err(map_err)?;
    ensure_initialized(&pkcs11)?;

    let slots = pkcs11.get_slots_with_token().map_err(map_err)?;
    let slot = *slots.get(slot_index).ok_or_else(|| {
        SealError::Signature(format!(
            "PKCS#11 slot index {slot_index} out of range ({} tokens)",
            slots.len()
        ))
    })?;

    let so = AuthPin::new(so_pin.into());
    pkcs11.init_token(slot, &so, token_label).map_err(map_err)?;

    {
        let session = pkcs11.open_rw_session(slot).map_err(map_err)?;
        session.login(UserType::So, Some(&so)).map_err(map_err)?;
        let user = AuthPin::new(user_pin.into());
        session.init_pin(&user).map_err(map_err)?;
    }

    let session = pkcs11.open_rw_session(slot).map_err(map_err)?;
    let user = AuthPin::new(user_pin.into());
    session
        .login(UserType::User, Some(&user))
        .map_err(map_err)?;

    let label = key_label.as_bytes().to_vec();
    let pub_template = vec![
        Attribute::Token(true),
        Attribute::Private(false),
        Attribute::Label(label.clone()),
        Attribute::Verify(true),
        Attribute::PublicExponent(vec![0x01, 0x00, 0x01]),
        Attribute::ModulusBits(2048.into()),
    ];
    let priv_template = vec![
        Attribute::Token(true),
        Attribute::Private(true),
        Attribute::Label(label),
        Attribute::Sign(true),
        Attribute::Sensitive(true),
    ];
    let (_public, _private) = session
        .generate_key_pair(&Mechanism::RsaPkcsKeyPairGen, &pub_template, &priv_template)
        .map_err(map_err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn missing_module_fail_closed() {
        let err = sign_acceptance_digest_rsa(
            Path::new("/nonexistent/libsofthsm2.so"),
            "1234",
            "vaa",
            0,
            "sha256:abc",
        )
        .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn softhsm_live_round_trip() {
        if std::env::var("VAA_PKCS11_LIVE").ok().as_deref() != Some("1") {
            return;
        }
        let module = std::env::var("VAA_HSM_MODULE")
            .unwrap_or_else(|_| "/usr/lib/softhsm/libsofthsm2.so".into());
        let module = PathBuf::from(module);
        assert!(
            module.exists(),
            "VAA_PKCS11_LIVE=1 but module missing: {}",
            module.display()
        );
        let pin = std::env::var("VAA_HSM_PIN").unwrap_or_else(|_| "1234".into());
        let so_pin = std::env::var("VAA_HSM_SO_PIN").unwrap_or_else(|_| "5678".into());
        let label = std::env::var("VAA_HSM_KEY_LABEL").unwrap_or_else(|_| "vaa".into());

        provision_rsa_keypair(&module, &so_pin, &pin, "vaa-ci", &label, 0).expect("provision");
        let sig =
            sign_acceptance_digest_rsa(&module, &pin, &label, 0, "sha256:deadbeef").expect("sign");
        assert_eq!(sig.alg, SIGNATURE_ALG_RSA_PKCS1_SHA256);
        let pk = B64.decode(sig.public_key_b64.as_bytes()).expect("pk b64");
        let s = B64.decode(sig.sig_b64.as_bytes()).expect("sig b64");
        verify_rsa_pkcs1_sha256_spki(&pk, b"sha256:deadbeef", &s).expect("verify");
    }
}
