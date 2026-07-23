//! Filesystem content-addressed store under `.vaa/cache/`.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::evidence::sha256_digest_prefixed;

use super::keys::{
    build_cache_key, verification_cache_key, BuildKeyMaterials, VerificationKeyMaterials,
};
use super::policy::{may_reuse_build, may_reuse_verification, CacheReuseDecision};

/// Cache on-disk schema (bump when record shape changes incompatibly).
pub const CACHE_SCHEMA_VERSION: &str = "0.1";

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("io: {0}")]
    Io(String),
    #[error("integrity: {0}")]
    Integrity(String),
    #[error("policy: {0}")]
    Policy(String),
    #[error("not found")]
    NotFound,
    #[error("path abuse: {0}")]
    PathAbuse(String),
}

/// Counts for `vaa cache status`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub root: String,
    pub blobs: u64,
    pub verification_entries: u64,
    pub build_entries: u64,
}

/// Persisted verification cache record (JSON under `verification/<key>.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationCacheRecord {
    pub schema_version: String,
    pub key_digest: String,
    pub final_status: String,
    pub report_blob_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_status: Option<String>,
}

/// Persisted build cache record (JSON under `builds/<key>.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildCacheRecord {
    pub schema_version: String,
    pub key_digest: String,
    pub object_blob_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary_blob_digest: Option<String>,
    /// Canonical-ish BuildManifest JSON (paths may be relative placeholders).
    pub manifest_json: String,
}

/// Object (+ optional binary) bytes recovered from a build cache hit.
pub struct BuildCacheArtifacts {
    pub record: BuildCacheRecord,
    pub object: Vec<u8>,
    pub binary: Option<Vec<u8>>,
}

/// Local filesystem cache root.
#[derive(Debug, Clone)]
pub struct CacheStore {
    root: PathBuf,
}

impl CacheStore {
    #[must_use]
    pub fn open(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Ensure layout dirs exist.
    pub fn ensure_layout(&self) -> Result<(), CacheError> {
        for sub in ["blobs/sha256", "verification", "builds", "index"] {
            fs::create_dir_all(self.root.join(sub))
                .map_err(|e| CacheError::Io(format!("mkdir {sub}: {e}")))?;
        }
        Ok(())
    }

    #[must_use]
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            root: self.root.display().to_string(),
            blobs: count_files(&self.root.join("blobs/sha256")),
            verification_entries: count_files(&self.root.join("verification")),
            build_entries: count_files(&self.root.join("builds")),
        }
    }

    /// Write bytes to content-addressed blob store; returns `sha256:…`.
    pub fn put_blob(&self, bytes: &[u8]) -> Result<String, CacheError> {
        self.ensure_layout()?;
        let digest = sha256_digest_prefixed(bytes);
        let hex = digest_hex(&digest)?;
        let path = self.blob_path(&hex)?;
        if path.is_file() {
            return Ok(digest);
        }
        atomic_write(&path, bytes)?;
        Ok(digest)
    }

    /// Read blob by `sha256:…` digest.
    pub fn get_blob(&self, digest: &str) -> Result<Vec<u8>, CacheError> {
        let hex = digest_hex(digest)?;
        let path = self.blob_path(&hex)?;
        fs::read(&path).map_err(|_| CacheError::NotFound)
    }

    pub fn put_verification(
        &self,
        materials: &VerificationKeyMaterials,
        final_status: &str,
        report_raw_json: &str,
        raw_status: Option<&str>,
    ) -> Result<VerificationCacheRecord, CacheError> {
        self.ensure_layout()?;
        let key_digest = verification_cache_key(materials);
        let report_blob_digest = self.put_blob(report_raw_json.as_bytes())?;
        let record = VerificationCacheRecord {
            schema_version: CACHE_SCHEMA_VERSION.to_owned(),
            key_digest: key_digest.clone(),
            final_status: final_status.to_owned(),
            report_blob_digest,
            raw_status: raw_status.map(str::to_owned),
        };
        let path = self.verification_path(&key_digest)?;
        let body = serde_json::to_vec_pretty(&record)
            .map_err(|e| CacheError::Io(format!("serialize verification: {e}")))?;
        atomic_write(&path, &body)?;
        Ok(record)
    }

    pub fn get_verification(
        &self,
        materials: &VerificationKeyMaterials,
        require_verified: bool,
    ) -> Result<(VerificationCacheRecord, String), CacheError> {
        let key_digest = verification_cache_key(materials);
        let path = self.verification_path(&key_digest)?;
        let raw = fs::read_to_string(&path).map_err(|_| CacheError::NotFound)?;
        let record: VerificationCacheRecord = serde_json::from_str(&raw)
            .map_err(|e| CacheError::Integrity(format!("parse verification: {e}")))?;
        if record.key_digest != key_digest {
            return Err(CacheError::Integrity("key digest mismatch".into()));
        }
        match may_reuse_verification(&record, require_verified) {
            CacheReuseDecision::Allow => {}
            CacheReuseDecision::Deny(reason) => {
                return Err(CacheError::Policy(reason.to_owned()));
            }
        }
        let report = self.get_blob(&record.report_blob_digest)?;
        let report_str = String::from_utf8(report)
            .map_err(|e| CacheError::Integrity(format!("report utf-8: {e}")))?;
        // Integrity: blob digest must match stored pointer (get_blob path is by digest).
        Ok((record, report_str))
    }

    pub fn put_build(
        &self,
        materials: &BuildKeyMaterials,
        object_bytes: &[u8],
        binary_bytes: Option<&[u8]>,
        manifest_json: &str,
    ) -> Result<BuildCacheRecord, CacheError> {
        self.ensure_layout()?;
        let key_digest = build_cache_key(materials);
        let object_blob_digest = self.put_blob(object_bytes)?;
        let binary_blob_digest = match binary_bytes {
            Some(b) => Some(self.put_blob(b)?),
            None => None,
        };
        let record = BuildCacheRecord {
            schema_version: CACHE_SCHEMA_VERSION.to_owned(),
            key_digest: key_digest.clone(),
            object_blob_digest,
            binary_blob_digest,
            manifest_json: manifest_json.to_owned(),
        };
        let path = self.build_path(&key_digest)?;
        let body = serde_json::to_vec_pretty(&record)
            .map_err(|e| CacheError::Io(format!("serialize build: {e}")))?;
        atomic_write(&path, &body)?;
        Ok(record)
    }

    pub fn get_build(
        &self,
        materials: &BuildKeyMaterials,
    ) -> Result<BuildCacheArtifacts, CacheError> {
        let key_digest = build_cache_key(materials);
        let path = self.build_path(&key_digest)?;
        let raw = fs::read_to_string(&path).map_err(|_| CacheError::NotFound)?;
        let record: BuildCacheRecord = serde_json::from_str(&raw)
            .map_err(|e| CacheError::Integrity(format!("parse build: {e}")))?;
        if record.key_digest != key_digest {
            return Err(CacheError::Integrity("key digest mismatch".into()));
        }
        match may_reuse_build(&record) {
            CacheReuseDecision::Allow => {}
            CacheReuseDecision::Deny(reason) => {
                return Err(CacheError::Policy(reason.to_owned()));
            }
        }
        let object = self.get_blob(&record.object_blob_digest)?;
        let binary = match &record.binary_blob_digest {
            Some(d) => Some(self.get_blob(d)?),
            None => None,
        };
        Ok(BuildCacheArtifacts {
            record,
            object,
            binary,
        })
    }

    fn blob_path(&self, hex: &str) -> Result<PathBuf, CacheError> {
        reject_path_abuse(hex)?;
        Ok(self.root.join("blobs/sha256").join(hex))
    }

    fn verification_path(&self, key_digest: &str) -> Result<PathBuf, CacheError> {
        let hex = digest_hex(key_digest)?;
        reject_path_abuse(&hex)?;
        Ok(self.root.join("verification").join(format!("{hex}.json")))
    }

    fn build_path(&self, key_digest: &str) -> Result<PathBuf, CacheError> {
        let hex = digest_hex(key_digest)?;
        reject_path_abuse(&hex)?;
        Ok(self.root.join("builds").join(format!("{hex}.json")))
    }
}

/// Default cache root: `$VAA_CACHE_DIR` or `./.vaa/cache`.
#[must_use]
pub fn resolve_cache_root() -> PathBuf {
    std::env::var_os("VAA_CACHE_DIR").map_or_else(default_cache_root, PathBuf::from)
}

#[must_use]
pub fn default_cache_root() -> PathBuf {
    PathBuf::from(".vaa").join("cache")
}

fn digest_hex(prefixed: &str) -> Result<String, CacheError> {
    let hex = prefixed
        .strip_prefix("sha256:")
        .unwrap_or(prefixed)
        .to_ascii_lowercase();
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(CacheError::PathAbuse(format!("bad digest: {prefixed}")));
    }
    Ok(hex)
}

fn reject_path_abuse(component: &str) -> Result<(), CacheError> {
    if component.contains("..")
        || component.contains('/')
        || component.contains('\\')
        || component.is_empty()
    {
        return Err(CacheError::PathAbuse(component.to_owned()));
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), CacheError> {
    let parent = path
        .parent()
        .ok_or_else(|| CacheError::Io("missing parent".into()))?;
    fs::create_dir_all(parent).map_err(|e| CacheError::Io(e.to_string()))?;
    let tmp = parent.join(format!(
        ".{}.tmp",
        path.file_name().and_then(|s| s.to_str()).unwrap_or("cache")
    ));
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)
            .map_err(|e| CacheError::Io(format!("open tmp: {e}")))?;
        file.write_all(bytes)
            .map_err(|e| CacheError::Io(format!("write tmp: {e}")))?;
        file.sync_all()
            .map_err(|e| CacheError::Io(format!("sync tmp: {e}")))?;
    }
    fs::rename(&tmp, path).map_err(|e| CacheError::Io(format!("rename: {e}")))?;
    if let Ok(f) = File::open(path) {
        let _ = f.sync_all();
    }
    Ok(())
}

fn count_files(dir: &Path) -> u64 {
    let Ok(rd) = fs::read_dir(dir) else {
        return 0;
    };
    rd.filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .count() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::keys::{args_fingerprint, BuildKeyMaterials, VerificationKeyMaterials};
    use crate::cache::policy::CacheReuseDecision;
    use crate::cache::{may_reuse_verification, verification_cache_key};

    fn temp_store() -> CacheStore {
        let dir = std::env::temp_dir().join(format!(
            "vaa_cache_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        let _ = fs::remove_dir_all(&dir);
        CacheStore::open(dir)
    }

    fn ver_mat() -> VerificationKeyMaterials {
        VerificationKeyMaterials {
            source_digest:
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            contract_digest:
                "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            task_digest: "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                .into(),
            target: "x86_64-pc-windows-msvc".into(),
            semasm_version: "0.1.0".into(),
            allow_execution: false,
            capability_source: "vaa_embedded_agent_verify_snapshot".into(),
        }
    }

    #[test]
    fn verification_round_trip() {
        let store = temp_store();
        let mat = ver_mat();
        let rec = store
            .put_verification(
                &mat,
                "Incomplete",
                r#"{"status":"execution_denied"}"#,
                Some("execution_denied"),
            )
            .expect("put");
        let (got, report) = store.get_verification(&mat, false).expect("get");
        assert_eq!(got.key_digest, rec.key_digest);
        assert_eq!(report, r#"{"status":"execution_denied"}"#);
        assert!(matches!(
            store.get_verification(&mat, true),
            Err(CacheError::Policy(_))
        ));
        let _ = fs::remove_dir_all(store.root());
    }

    #[test]
    fn missing_blob_fails_get() {
        let store = temp_store();
        store.ensure_layout().unwrap();
        let mat = ver_mat();
        let key = verification_cache_key(&mat);
        let hex = digest_hex(&key).unwrap();
        let path = store
            .root()
            .join("verification")
            .join(format!("{hex}.json"));
        let bogus = VerificationCacheRecord {
            schema_version: CACHE_SCHEMA_VERSION.to_owned(),
            key_digest: key,
            final_status: "Verified".into(),
            report_blob_digest:
                "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".into(),
            raw_status: Some("verified".into()),
        };
        fs::write(&path, serde_json::to_vec(&bogus).unwrap()).unwrap();
        assert!(matches!(
            store.get_verification(&mat, true),
            Err(CacheError::NotFound)
        ));
        let _ = fs::remove_dir_all(store.root());
    }

    #[test]
    fn build_round_trip() {
        let store = temp_store();
        let mat = BuildKeyMaterials {
            source_digest:
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            target: "elf64".into(),
            assembler_digest:
                "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            linker_digest:
                "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into(),
            assembler_args_fingerprint: args_fingerprint(&["-f".into(), "elf64".into()]),
            linker_args_fingerprint: args_fingerprint(&["-o".into(), "x".into()]),
            container_image_digest: String::new(),
        };
        store
            .put_build(&mat, b"obj", Some(b"bin"), r#"{"ok":true}"#)
            .expect("put");
        let arts = store.get_build(&mat).expect("get");
        assert_eq!(arts.object, b"obj");
        assert_eq!(arts.binary.as_deref(), Some(&b"bin"[..]));
        let _ = fs::remove_dir_all(store.root());
    }

    #[test]
    fn path_abuse_rejected() {
        let store = temp_store();
        store.ensure_layout().unwrap();
        assert!(matches!(
            store.blob_path("../evil"),
            Err(CacheError::PathAbuse(_))
        ));
        let _ = fs::remove_dir_all(store.root());
    }

    #[test]
    fn policy_helper_still_rejects_failed() {
        let rec = VerificationCacheRecord {
            schema_version: CACHE_SCHEMA_VERSION.to_owned(),
            key_digest: "sha256:k".into(),
            final_status: "Failed".into(),
            report_blob_digest: "sha256:r".into(),
            raw_status: None,
        };
        assert!(matches!(
            may_reuse_verification(&rec, false),
            CacheReuseDecision::Deny(_)
        ));
    }
}
