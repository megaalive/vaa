//! SealSigner backends (P7-A / P8-K): practice Ed25519, Sigstore-shaped DSSE,
//! SoftHSM PKCS#11 (feature `pkcs11`).

use std::fs;
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use super::seal::SealError;
use super::seal_sign::{
    load_signing_key_from_env, SealSignature, SIGNATURE_ALG, SIGNED_OVER_ACCEPTANCE,
    SIGNER_KIND_HSM_PKCS11, SIGNER_KIND_PRACTICE_ED25519, SIGNER_KIND_SIGSTORE_DSSE,
};

/// Backend identifier recorded in provenance / DSSE.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SignerKind {
    PracticeEd25519,
    SigstoreDsse,
    HsmPkcs11,
}

impl SignerKind {
    /// Stable kebab-case label persisted on seal signatures (G5).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PracticeEd25519 => SIGNER_KIND_PRACTICE_ED25519,
            Self::SigstoreDsse => SIGNER_KIND_SIGSTORE_DSSE,
            Self::HsmPkcs11 => SIGNER_KIND_HSM_PKCS11,
        }
    }
}

pub trait SealSigner: Send + Sync {
    fn kind(&self) -> SignerKind;
    fn sign_acceptance_digest(&self, acceptance_digest: &str) -> Result<SealSignature, SealError>;
}

/// Alpha practice key from `VAA_SEAL_SIGNING_KEY` hex seed file.
pub struct PracticeEd25519Signer {
    key: SigningKey,
}

impl PracticeEd25519Signer {
    pub fn from_env() -> Result<Option<Self>, SealError> {
        Ok(load_signing_key_from_env()?.map(|key| Self { key }))
    }

    #[must_use]
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            key: SigningKey::from_bytes(&seed),
        }
    }
}

impl SealSigner for PracticeEd25519Signer {
    fn kind(&self) -> SignerKind {
        SignerKind::PracticeEd25519
    }

    fn sign_acceptance_digest(&self, acceptance_digest: &str) -> Result<SealSignature, SealError> {
        let sig = self.key.sign(acceptance_digest.as_bytes());
        let vk = self.key.verifying_key();
        Ok(SealSignature {
            alg: SIGNATURE_ALG.to_owned(),
            public_key_b64: B64.encode(vk.to_bytes()),
            sig_b64: B64.encode(sig.to_bytes()),
            signed_over: SIGNED_OVER_ACCEPTANCE.to_owned(),
            signer_kind: Some(SIGNER_KIND_PRACTICE_ED25519.to_owned()),
        })
    }
}

/// Sigstore-*shaped* DSSE over a payload (typically `vaa-transparency-v1` JSON).
/// Uses the same practice Ed25519 key material; not Fulcio keyless OIDC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DsseEnvelope {
    pub payload_type: String,
    pub payload_b64: String,
    pub signatures: Vec<DsseSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DsseSignature {
    pub keyid: String,
    pub sig_b64: String,
    pub public_key_b64: String,
}

pub const DSSE_PAYLOAD_TYPE_TRANSPARENCY: &str = "application/vnd.vaa.transparency.v1+json";

/// PAE encoding per DSSE: `DSSEv1` + lengths + type + payload.
#[must_use]
pub fn dsse_pae(payload_type: &str, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"DSSEv1 ");
    out.extend_from_slice(payload_type.len().to_string().as_bytes());
    out.push(b' ');
    out.extend_from_slice(payload_type.as_bytes());
    out.push(b' ');
    out.extend_from_slice(payload.len().to_string().as_bytes());
    out.push(b' ');
    out.extend_from_slice(payload);
    out
}

pub struct SigstoreDsseSigner {
    inner: PracticeEd25519Signer,
}

impl SigstoreDsseSigner {
    pub fn from_env() -> Result<Option<Self>, SealError> {
        Ok(PracticeEd25519Signer::from_env()?.map(|inner| Self { inner }))
    }

    #[must_use]
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            inner: PracticeEd25519Signer::from_seed(seed),
        }
    }

    pub fn sign_payload(
        &self,
        payload_type: &str,
        payload: &[u8],
    ) -> Result<DsseEnvelope, SealError> {
        let pae = dsse_pae(payload_type, payload);
        let sig = self.inner.key.sign(&pae);
        let vk = self.inner.key.verifying_key();
        Ok(DsseEnvelope {
            payload_type: payload_type.to_owned(),
            payload_b64: B64.encode(payload),
            signatures: vec![DsseSignature {
                keyid: "vaa-practice".into(),
                sig_b64: B64.encode(sig.to_bytes()),
                public_key_b64: B64.encode(vk.to_bytes()),
            }],
        })
    }
}

impl SealSigner for SigstoreDsseSigner {
    fn kind(&self) -> SignerKind {
        SignerKind::SigstoreDsse
    }

    fn sign_acceptance_digest(&self, acceptance_digest: &str) -> Result<SealSignature, SealError> {
        // Still produce a seal signature block for acceptance_digest; DSSE is
        // the preferred carrier for transparency documents.
        let mut sig = self.inner.sign_acceptance_digest(acceptance_digest)?;
        sig.signer_kind = Some(SIGNER_KIND_SIGSTORE_DSSE.to_owned());
        Ok(sig)
    }
}

pub fn verify_dsse_envelope(env: &DsseEnvelope) -> Result<(), SealError> {
    let payload = B64
        .decode(env.payload_b64.as_bytes())
        .map_err(|e| SealError::Signature(format!("dsse payload b64: {e}")))?;
    let pae = dsse_pae(&env.payload_type, &payload);
    let Some(sig0) = env.signatures.first() else {
        return Err(SealError::Signature("dsse missing signatures".into()));
    };
    let pk = B64
        .decode(sig0.public_key_b64.as_bytes())
        .map_err(|e| SealError::Signature(format!("dsse pk b64: {e}")))?;
    let pk: [u8; 32] = pk
        .as_slice()
        .try_into()
        .map_err(|_| SealError::Signature("dsse pk length".into()))?;
    let vk =
        VerifyingKey::from_bytes(&pk).map_err(|e| SealError::Signature(format!("dsse pk: {e}")))?;
    let sig_bytes = B64
        .decode(sig0.sig_b64.as_bytes())
        .map_err(|e| SealError::Signature(format!("dsse sig b64: {e}")))?;
    let sig = Signature::from_slice(&sig_bytes)
        .map_err(|e| SealError::Signature(format!("dsse sig: {e}")))?;
    vk.verify(&pae, &sig)
        .map_err(|e| SealError::Signature(format!("dsse verify: {e}")))
}

/// SoftHSM / PKCS#11 signer. Without `--features pkcs11`, signing fail-closes as scaffold.
pub struct HsmPkcs11Signer {
    pub module_path: PathBuf,
    pub key_label: String,
    pub pin: String,
    pub slot_index: usize,
}

impl HsmPkcs11Signer {
    #[must_use]
    pub fn scaffold(module_path: impl Into<PathBuf>, key_label: impl Into<String>) -> Self {
        Self {
            module_path: module_path.into(),
            key_label: key_label.into(),
            pin: std::env::var("VAA_HSM_PIN").unwrap_or_else(|_| "1234".into()),
            slot_index: std::env::var("VAA_HSM_SLOT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
        }
    }

    #[must_use]
    pub fn from_env_paths() -> Self {
        let module = std::env::var("VAA_HSM_MODULE").unwrap_or_else(|_| {
            if cfg!(windows) {
                "softhsm2.dll".into()
            } else {
                "/usr/lib/softhsm/libsofthsm2.so".into()
            }
        });
        let label = std::env::var("VAA_HSM_KEY_LABEL").unwrap_or_else(|_| "vaa".into());
        Self::scaffold(module, label)
    }
}

impl SealSigner for HsmPkcs11Signer {
    fn kind(&self) -> SignerKind {
        SignerKind::HsmPkcs11
    }

    fn sign_acceptance_digest(&self, acceptance_digest: &str) -> Result<SealSignature, SealError> {
        #[cfg(feature = "pkcs11")]
        {
            let mut sig = super::pkcs11_softhsm::sign_acceptance_digest_rsa(
                &self.module_path,
                &self.pin,
                &self.key_label,
                self.slot_index,
                acceptance_digest,
            )?;
            sig.signer_kind = Some(SIGNER_KIND_HSM_PKCS11.to_owned());
            Ok(sig)
        }
        #[cfg(not(feature = "pkcs11"))]
        {
            let _ = acceptance_digest;
            Err(SealError::Signature(format!(
                "HSM PKCS#11 backend requires --features pkcs11 (module={}, label={}); SoftHSM ≠ hardware HSM ≠ trust root",
                self.module_path.display(),
                self.key_label
            )))
        }
    }
}

/// Select signer from env: `VAA_SEAL_SIGNER=practice|dsse|hsm` (default practice).
pub fn signer_from_env() -> Result<Option<Box<dyn SealSigner>>, SealError> {
    let kind = std::env::var("VAA_SEAL_SIGNER").unwrap_or_else(|_| "practice".into());
    match kind.trim().to_ascii_lowercase().as_str() {
        "practice" | "ed25519" | "" => {
            Ok(PracticeEd25519Signer::from_env()?.map(|s| Box::new(s) as Box<dyn SealSigner>))
        }
        "dsse" | "sigstore" => {
            Ok(SigstoreDsseSigner::from_env()?.map(|s| Box::new(s) as Box<dyn SealSigner>))
        }
        "hsm" | "pkcs11" => Ok(Some(Box::new(HsmPkcs11Signer::from_env_paths()))),
        other => Err(SealError::Signature(format!(
            "unknown VAA_SEAL_SIGNER={other}"
        ))),
    }
}

/// Write DSSE JSON next to a transparency document.
pub fn write_dsse_file(path: &Path, env: &DsseEnvelope) -> Result<(), SealError> {
    let bytes = serde_json::to_vec_pretty(env).map_err(|e| SealError::Json(e.to_string()))?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| SealError::Io(e.to_string()))?;
        }
    }
    fs::write(path, bytes).map_err(|e| SealError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn practice_signer_round_trip_block() {
        let signer = PracticeEd25519Signer::from_seed([7u8; 32]);
        let sig = signer.sign_acceptance_digest("sha256:abc").expect("sign");
        assert_eq!(sig.alg, SIGNATURE_ALG);
        assert_eq!(sig.signed_over, SIGNED_OVER_ACCEPTANCE);
        assert_eq!(
            sig.signer_kind.as_deref(),
            Some(SIGNER_KIND_PRACTICE_ED25519)
        );
        assert_eq!(signer.kind().as_str(), SIGNER_KIND_PRACTICE_ED25519);
    }

    #[test]
    fn dsse_acceptance_block_labels_sigstore_kind() {
        let signer = SigstoreDsseSigner::from_seed([9u8; 32]);
        let sig = signer.sign_acceptance_digest("sha256:abc").expect("sign");
        assert_eq!(sig.signer_kind.as_deref(), Some(SIGNER_KIND_SIGSTORE_DSSE));
    }

    #[test]
    fn dsse_keyid_is_practice_not_trust_root() {
        let signer = SigstoreDsseSigner::from_seed([9u8; 32]);
        let env = signer
            .sign_payload(DSSE_PAYLOAD_TYPE_TRANSPARENCY, br#"{"schema":"vaa-transparency-v1"}"#)
            .expect("dsse");
        assert_eq!(env.signatures[0].keyid, "vaa-practice");
    }

    #[cfg(not(feature = "pkcs11"))]
    #[test]
    fn hsm_without_pkcs11_feature_fail_closes() {
        let signer = HsmPkcs11Signer::scaffold("softhsm2.so", "vaa");
        let err = signer
            .sign_acceptance_digest("sha256:abc")
            .expect_err("scaffold without pkcs11");
        let msg = err.to_string();
        assert!(msg.contains("pkcs11"), "{msg}");
        assert!(msg.contains("trust root") || msg.contains("hardware"), "{msg}");
    }

    #[test]
    fn hsm_signer_kind_label_is_hsm_pkcs11() {
        assert_eq!(
            HsmPkcs11Signer::scaffold("softhsm2.so", "vaa").kind().as_str(),
            SIGNER_KIND_HSM_PKCS11
        );
    }

    #[test]
    fn dsse_sign_verify() {
        let signer = SigstoreDsseSigner::from_seed([9u8; 32]);
        let payload = br#"{"schema":"vaa-transparency-v1"}"#;
        let env = signer
            .sign_payload(DSSE_PAYLOAD_TYPE_TRANSPARENCY, payload)
            .expect("dsse");
        verify_dsse_envelope(&env).expect("verify");
    }

    #[test]
    fn hsm_without_live_module_fail_closed() {
        let s = HsmPkcs11Signer::scaffold("/nonexistent/libsofthsm2.so", "k");
        let err = s.sign_acceptance_digest("sha256:x").unwrap_err();
        let msg = err.to_string();
        #[cfg(feature = "pkcs11")]
        assert!(msg.contains("not found"), "{msg}");
        #[cfg(not(feature = "pkcs11"))]
        assert!(msg.contains("pkcs11"), "{msg}");
    }

    #[test]
    fn env_key_documented() {
        assert_eq!(
            super::super::seal_sign::ENV_SEAL_SIGNING_KEY,
            "VAA_SEAL_SIGNING_KEY"
        );
    }
}
