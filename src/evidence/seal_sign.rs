//! Optional Ed25519 authenticity over `acceptance_digest` (A0).
//!
//! Sign/verify are always compiled. Signing activates only when
//! `VAA_SEAL_SIGNING_KEY` points at a 32-byte hex seed file. Verification runs
//! whenever a seal carries a `signature` object. Set
//! `VAA_REQUIRE_SEAL_SIGNATURE=1` to reject unsigned seals.
//!
//! Not part of `acceptance_digest` / `envelope_digest` hash bodies (no chicken-egg).

use std::fs;
use std::path::Path;

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

use super::seal::{SealEnvelope, SealError};

/// Env var: path to a file containing a 32-byte Ed25519 seed as 64 hex digits.
pub const ENV_SEAL_SIGNING_KEY: &str = "VAA_SEAL_SIGNING_KEY";

/// Env var: when `1`/`true`/`yes`, unsigned seals fail verification.
pub const ENV_REQUIRE_SEAL_SIGNATURE: &str = "VAA_REQUIRE_SEAL_SIGNATURE";

pub const SIGNATURE_ALG: &str = "ed25519";
pub const SIGNED_OVER_ACCEPTANCE: &str = "acceptance_digest";

/// Optional authenticity block on [`SealEnvelope`] (schema 0.2 additive).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SealSignature {
    pub alg: String,
    pub public_key_b64: String,
    pub sig_b64: String,
    pub signed_over: String,
}

fn require_signature_env() -> bool {
    match std::env::var(ENV_REQUIRE_SEAL_SIGNATURE) {
        Ok(v) => {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
        }
        Err(_) => false,
    }
}

fn parse_hex_seed(raw: &str) -> Result<[u8; 32], SealError> {
    let hex: String = raw
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    if hex.len() != 64 {
        return Err(SealError::Signature(format!(
            "signing seed must be 64 hex chars (32 bytes), got {}",
            hex.len()
        )));
    }
    let mut out = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk).map_err(|e| SealError::Signature(e.to_string()))?;
        out[i] = u8::from_str_radix(s, 16)
            .map_err(|e| SealError::Signature(format!("invalid hex seed: {e}")))?;
    }
    Ok(out)
}

/// Load signing key from `VAA_SEAL_SIGNING_KEY` when set.
pub fn load_signing_key_from_env() -> Result<Option<SigningKey>, SealError> {
    let Ok(path) = std::env::var(ENV_SEAL_SIGNING_KEY) else {
        return Ok(None);
    };
    let path = path.trim();
    if path.is_empty() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path)
        .map_err(|e| SealError::Signature(format!("read {ENV_SEAL_SIGNING_KEY}={path}: {e}")))?;
    let seed = parse_hex_seed(&raw)?;
    Ok(Some(SigningKey::from_bytes(&seed)))
}

/// Generate a new seed file and return (public_key_hex, public_key_b64).
pub fn keygen_seal(out: &Path) -> Result<(String, String), SealError> {
    let signing_key = SigningKey::generate(&mut OsRng);
    let seed = signing_key.to_bytes();
    let hex = bytes_to_hex(&seed);
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| SealError::Io(e.to_string()))?;
        }
    }
    fs::write(out, format!("{hex}\n")).map_err(|e| SealError::Io(e.to_string()))?;
    let vk = signing_key.verifying_key();
    let pk_bytes = vk.to_bytes();
    let pk_hex = bytes_to_hex(&pk_bytes);
    let pk_b64 = B64.encode(pk_bytes);
    Ok((pk_hex, pk_b64))
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut hex = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

/// Sign `acceptance_digest` UTF-8 bytes when a signing key is configured.
pub fn maybe_sign_envelope(envelope: &mut SealEnvelope) -> Result<(), SealError> {
    let Some(signing_key) = load_signing_key_from_env()? else {
        return Ok(());
    };
    let msg = envelope.acceptance_digest.as_bytes();
    let sig = signing_key.sign(msg);
    let vk = signing_key.verifying_key();
    envelope.signature = Some(SealSignature {
        alg: SIGNATURE_ALG.to_owned(),
        public_key_b64: B64.encode(vk.to_bytes()),
        sig_b64: B64.encode(sig.to_bytes()),
        signed_over: SIGNED_OVER_ACCEPTANCE.to_owned(),
    });
    Ok(())
}

/// Verify optional / required seal signature after digest checks.
pub fn verify_envelope_signature(envelope: &SealEnvelope) -> Result<(), SealError> {
    match &envelope.signature {
        None => {
            if require_signature_env() {
                return Err(SealError::Signature(
                    "signature required (VAA_REQUIRE_SEAL_SIGNATURE) but missing".into(),
                ));
            }
            Ok(())
        }
        Some(sig) => {
            if sig.alg != SIGNATURE_ALG {
                return Err(SealError::Signature(format!(
                    "unsupported signature alg: {}",
                    sig.alg
                )));
            }
            if sig.signed_over != SIGNED_OVER_ACCEPTANCE {
                return Err(SealError::Signature(format!(
                    "unsupported signed_over: {}",
                    sig.signed_over
                )));
            }
            let pk_bytes = B64
                .decode(sig.public_key_b64.as_bytes())
                .map_err(|e| SealError::Signature(format!("public_key_b64: {e}")))?;
            let pk_arr: [u8; 32] = pk_bytes
                .try_into()
                .map_err(|_| SealError::Signature("public_key_b64 must be 32 bytes".into()))?;
            let verifying_key = VerifyingKey::from_bytes(&pk_arr)
                .map_err(|e| SealError::Signature(format!("public key: {e}")))?;

            let sig_bytes = B64
                .decode(sig.sig_b64.as_bytes())
                .map_err(|e| SealError::Signature(format!("sig_b64: {e}")))?;
            let sig_arr: [u8; 64] = sig_bytes
                .try_into()
                .map_err(|_| SealError::Signature("sig_b64 must be 64 bytes".into()))?;
            let signature = Signature::from_bytes(&sig_arr);

            verifying_key
                .verify(envelope.acceptance_digest.as_bytes(), &signature)
                .map_err(|_| SealError::Signature("ed25519 verify failed".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::seal::{seal_envelope, AcceptanceBody, GeneratorMeta, ProvenanceBody};
    use crate::evidence::status::EvidenceStatus;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn tempfile_path(prefix: &str) -> std::path::PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("{prefix}_{}_{}.seed", std::process::id(), n))
    }

    fn sample_envelope() -> SealEnvelope {
        seal_envelope(
            AcceptanceBody {
                task_digest:
                    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
                target: "win64".into(),
                contract_digest:
                    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
                source_digest:
                    "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into(),
                semasm_report_digest: "none".into(),
                final_status: EvidenceStatus::Incomplete,
                checks: vec![],
            },
            ProvenanceBody {
                task_id: "t".into(),
                run_id: Some("r".into()),
                generator: GeneratorMeta::ingest("unit"),
                candidate_index: 0,
                previous_seal_digest: None,
            },
        )
    }

    #[test]
    fn sign_then_verify_round_trip() {
        let _guard = env_lock().lock().unwrap();
        let path = tempfile_path("vaa_seal_key");
        let (pk_hex, _) = keygen_seal(&path).unwrap();
        assert_eq!(pk_hex.len(), 64);

        std::env::set_var(ENV_SEAL_SIGNING_KEY, &path);
        std::env::remove_var(ENV_REQUIRE_SEAL_SIGNATURE);

        let mut env = sample_envelope();
        assert!(env.signature.is_none());
        maybe_sign_envelope(&mut env).unwrap();
        assert!(env.signature.is_some());
        verify_envelope_signature(&env).unwrap();

        std::env::remove_var(ENV_SEAL_SIGNING_KEY);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn tampered_acceptance_digest_fails_signature() {
        let _guard = env_lock().lock().unwrap();
        let path = tempfile_path("vaa_seal_key_tamper");
        keygen_seal(&path).unwrap();
        std::env::set_var(ENV_SEAL_SIGNING_KEY, &path);
        std::env::remove_var(ENV_REQUIRE_SEAL_SIGNATURE);

        let mut env = sample_envelope();
        maybe_sign_envelope(&mut env).unwrap();
        env.acceptance_digest =
            "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".into();
        let err = verify_envelope_signature(&env).unwrap_err();
        assert!(matches!(err, SealError::Signature(_)));

        std::env::remove_var(ENV_SEAL_SIGNING_KEY);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn unsigned_ok_without_require_env() {
        let _guard = env_lock().lock().unwrap();
        std::env::remove_var(ENV_SEAL_SIGNING_KEY);
        std::env::remove_var(ENV_REQUIRE_SEAL_SIGNATURE);
        let env = sample_envelope();
        verify_envelope_signature(&env).unwrap();
    }

    #[test]
    fn unsigned_fails_when_required() {
        let _guard = env_lock().lock().unwrap();
        std::env::remove_var(ENV_SEAL_SIGNING_KEY);
        std::env::set_var(ENV_REQUIRE_SEAL_SIGNATURE, "1");
        let env = sample_envelope();
        let err = verify_envelope_signature(&env).unwrap_err();
        assert!(matches!(err, SealError::Signature(_)));
        std::env::remove_var(ENV_REQUIRE_SEAL_SIGNATURE);
    }
}
