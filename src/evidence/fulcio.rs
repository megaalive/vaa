//! Fulcio keyless client (feature `fulcio`) — P8-I.
//!
//! Requests a short-lived signing certificate via OIDC identity, then signs
//! DSSE. Manual / release-adjacent only. Gate CI stays offline.
//! Fulcio identity attest ≠ SemASM Verified.

use std::sync::{Arc, Mutex};

use base64::{
    engine::general_purpose::{STANDARD as B64, URL_SAFE_NO_PAD},
    Engine,
};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::signer::{dsse_pae, DsseEnvelope, DsseSignature, DSSE_PAYLOAD_TYPE_TRANSPARENCY};

#[derive(Debug, thiserror::Error)]
pub enum FulcioError {
    #[error("fulcio transport: {0}")]
    Transport(String),
    #[error("fulcio protocol: {0}")]
    Protocol(String),
    #[error("fulcio oidc: {0}")]
    Oidc(String),
    #[error("json: {0}")]
    Json(String),
    #[error("crypto: {0}")]
    Crypto(String),
}

pub trait FulcioTransport: Send + Sync {
    fn post_json(&self, path: &str, body: &str) -> Result<(u16, String), FulcioError>;
}

/// In-memory mock Fulcio for unit tests (no network).
#[derive(Default, Clone)]
pub struct MockFulcioTransport {
    inner: Arc<Mutex<MockState>>,
}

#[derive(Default)]
struct MockState {
    last_body: Option<String>,
    cert_pem: String,
}

impl MockFulcioTransport {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MockState {
                last_body: None,
                cert_pem: "-----BEGIN CERTIFICATE-----\nMOCK\n-----END CERTIFICATE-----\n".into(),
            })),
        }
    }

    #[must_use]
    pub fn last_request_body(&self) -> Option<String> {
        self.inner.lock().ok()?.last_body.clone()
    }
}

impl FulcioTransport for MockFulcioTransport {
    fn post_json(&self, path: &str, body: &str) -> Result<(u16, String), FulcioError> {
        if path != "/api/v2/signingCert" {
            return Err(FulcioError::Protocol(format!("unexpected POST {path}")));
        }
        let mut st = self
            .inner
            .lock()
            .map_err(|e| FulcioError::Transport(e.to_string()))?;
        st.last_body = Some(body.to_owned());
        let resp = json!({
            "signedCertificateEmbeddedSct": {
                "chain": {
                    "certificates": [st.cert_pem]
                }
            }
        });
        Ok((200, resp.to_string()))
    }
}

#[cfg(feature = "fulcio")]
pub struct UreqFulcioTransport {
    pub base_url: String,
}

#[cfg(feature = "fulcio")]
impl FulcioTransport for UreqFulcioTransport {
    fn post_json(&self, path: &str, body: &str) -> Result<(u16, String), FulcioError> {
        let url = format!("{}{path}", self.base_url.trim_end_matches('/'));
        let resp = ureq::post(&url)
            .set("Content-Type", "application/json")
            .set("Accept", "application/json")
            .send_string(body)
            .map_err(|e| FulcioError::Transport(e.to_string()))?;
        let status = resp.status();
        let text = resp
            .into_string()
            .map_err(|e| FulcioError::Transport(e.to_string()))?;
        Ok((status, text))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FulcioSigningResult {
    pub certificate_chain_pem: Vec<String>,
    pub dsse: DsseEnvelope,
    pub public_key_b64: String,
}

/// Extract `sub` claim from a compact JWT (no signature verification — Fulcio verifies).
pub fn oidc_subject(token: &str) -> Result<String, FulcioError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return Err(FulcioError::Oidc("OIDC token is not a JWT".into()));
    }
    let payload = URL_SAFE_NO_PAD
        .decode(parts[1].as_bytes())
        .or_else(|_| B64.decode(parts[1].as_bytes()))
        .map_err(|e| FulcioError::Oidc(format!("jwt payload b64: {e}")))?;
    let v: serde_json::Value = serde_json::from_slice(&payload)
        .map_err(|e| FulcioError::Oidc(format!("jwt json: {e}")))?;
    v.get("sub")
        .and_then(|s| s.as_str())
        .map(str::to_owned)
        .ok_or_else(|| FulcioError::Oidc("jwt missing sub".into()))
}

fn ed25519_public_pem(vk: &VerifyingKey) -> String {
    let raw = vk.to_bytes();
    // SPKI for Ed25519: 302a300506032b6570032100 || 32-byte key
    let mut spki = vec![
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
    ];
    spki.extend_from_slice(&raw);
    let spki_b64 = B64.encode(&spki);
    let mut lines = String::from("-----BEGIN PUBLIC KEY-----\n");
    for chunk in spki_b64.as_bytes().chunks(64) {
        lines.push_str(std::str::from_utf8(chunk).unwrap_or(""));
        lines.push('\n');
    }
    lines.push_str("-----END PUBLIC KEY-----\n");
    lines
}

/// Request Fulcio cert and sign a DSSE payload with an ephemeral Ed25519 key.
pub fn keyless_sign_dsse(
    transport: &dyn FulcioTransport,
    oidc_token: &str,
    payload_type: &str,
    payload: &[u8],
    signing_key: &SigningKey,
) -> Result<FulcioSigningResult, FulcioError> {
    let sub = oidc_subject(oidc_token)?;
    let proof = signing_key.sign(sub.as_bytes());
    let vk = signing_key.verifying_key();
    let pem = ed25519_public_pem(&vk);

    let body = json!({
        "credentials": {
            "oidcIdentityToken": oidc_token
        },
        "publicKeyRequest": {
            "publicKey": {
                "algorithm": "ED25519",
                "content": pem
            },
            "proofOfPossession": B64.encode(proof.to_bytes())
        }
    });
    let body_s = serde_json::to_string(&body).map_err(|e| FulcioError::Json(e.to_string()))?;
    let (status, resp) = transport.post_json("/api/v2/signingCert", &body_s)?;
    if status != 200 && status != 201 {
        return Err(FulcioError::Protocol(format!(
            "signingCert HTTP {status}: {resp}"
        )));
    }
    let chain = parse_cert_chain(&resp)?;

    let pae = dsse_pae(payload_type, payload);
    let sig = signing_key.sign(&pae);
    let dsse = DsseEnvelope {
        payload_type: payload_type.to_owned(),
        payload_b64: B64.encode(payload),
        signatures: vec![DsseSignature {
            keyid: "fulcio-keyless".into(),
            sig_b64: B64.encode(sig.to_bytes()),
            public_key_b64: B64.encode(vk.to_bytes()),
        }],
    };

    // Local Ed25519 verify of DSSE (cert chain is identity metadata, not SemASM proof).
    verify_dsse_ed25519(&dsse)?;

    Ok(FulcioSigningResult {
        certificate_chain_pem: chain,
        dsse,
        public_key_b64: B64.encode(vk.to_bytes()),
    })
}

fn parse_cert_chain(resp: &str) -> Result<Vec<String>, FulcioError> {
    let v: serde_json::Value =
        serde_json::from_str(resp).map_err(|e| FulcioError::Json(e.to_string()))?;
    let certs = v
        .pointer("/signedCertificateEmbeddedSct/chain/certificates")
        .or_else(|| v.pointer("/signedCertificateDetachedSct/chain/certificates"))
        .and_then(|c| c.as_array())
        .ok_or_else(|| FulcioError::Protocol("response missing certificate chain".into()))?;
    let out: Vec<String> = certs
        .iter()
        .filter_map(|c| c.as_str().map(str::to_owned))
        .collect();
    if out.is_empty() {
        return Err(FulcioError::Protocol("empty certificate chain".into()));
    }
    Ok(out)
}

fn verify_dsse_ed25519(env: &DsseEnvelope) -> Result<(), FulcioError> {
    let payload = B64
        .decode(env.payload_b64.as_bytes())
        .map_err(|e| FulcioError::Crypto(format!("payload b64: {e}")))?;
    let pae = dsse_pae(&env.payload_type, &payload);
    let sig0 = env
        .signatures
        .first()
        .ok_or_else(|| FulcioError::Crypto("dsse missing signatures".into()))?;
    let pk = B64
        .decode(sig0.public_key_b64.as_bytes())
        .map_err(|e| FulcioError::Crypto(format!("pk b64: {e}")))?;
    let pk: [u8; 32] = pk
        .as_slice()
        .try_into()
        .map_err(|_| FulcioError::Crypto("pk length".into()))?;
    let vk = VerifyingKey::from_bytes(&pk).map_err(|e| FulcioError::Crypto(e.to_string()))?;
    let sig_bytes = B64
        .decode(sig0.sig_b64.as_bytes())
        .map_err(|e| FulcioError::Crypto(format!("sig b64: {e}")))?;
    let sig = Signature::from_slice(&sig_bytes).map_err(|e| FulcioError::Crypto(e.to_string()))?;
    vk.verify(&pae, &sig)
        .map_err(|e| FulcioError::Crypto(format!("dsse verify: {e}")))
}

/// Offline JWT used with [`MockFulcioTransport`] (`--dry-run`).
#[must_use]
pub fn dry_run_oidc_token() -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
    let payload = URL_SAFE_NO_PAD.encode(br#"{"sub":"vaa-fulcio-dry-run"}"#);
    format!("{header}.{payload}.sig")
}

/// Convenience: keyless sign of transparency JSON bytes.
pub fn keyless_sign_transparency(
    transport: &dyn FulcioTransport,
    oidc_token: &str,
    transparency_json: &[u8],
    seed: [u8; 32],
) -> Result<FulcioSigningResult, FulcioError> {
    let key = SigningKey::from_bytes(&seed);
    keyless_sign_dsse(
        transport,
        oidc_token,
        DSSE_PAYLOAD_TYPE_TRANSPARENCY,
        transparency_json,
        &key,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_jwt(sub: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
        let payload = URL_SAFE_NO_PAD.encode(format!(r#"{{"sub":"{sub}"}}"#).as_bytes());
        format!("{header}.{payload}.sig")
    }

    #[test]
    fn oidc_subject_parses() {
        let t = fake_jwt("repo:megaalive/vaa:ref:refs/heads/main");
        assert_eq!(
            oidc_subject(&t).unwrap(),
            "repo:megaalive/vaa:ref:refs/heads/main"
        );
    }

    #[test]
    fn mock_keyless_round_trip() {
        let mock = MockFulcioTransport::new();
        let token = fake_jwt("unit-test-sub");
        let r = keyless_sign_transparency(
            &mock,
            &token,
            br#"{"schema":"vaa-transparency-v1"}"#,
            [3u8; 32],
        )
        .expect("sign");
        assert!(!r.certificate_chain_pem.is_empty());
        assert_eq!(r.dsse.payload_type, DSSE_PAYLOAD_TYPE_TRANSPARENCY);
        let body = mock.last_request_body().expect("body");
        assert!(body.contains("oidcIdentityToken"));
        assert!(body.contains("ED25519"));
        assert!(body.contains("proofOfPossession"));
    }

    #[test]
    fn bad_token_fail_closed() {
        let mock = MockFulcioTransport::new();
        let err = keyless_sign_transparency(&mock, "not-a-jwt", b"{}", [1u8; 32]).unwrap_err();
        assert!(matches!(err, FulcioError::Oidc(_)));
    }
}
