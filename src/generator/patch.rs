//! Patch-level evidence (distinct from per-candidate seals).

use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::canonical_json::canonical_json_bytes;
use crate::evidence::sha256_digest_prefixed;
use crate::generator::error::GeneratorError;
use crate::generator::repo_guard::path_policy_violations;
use crate::generator::spec::PatchPolicy;
use crate::generator::suite::SuiteStatus;

/// Accepted patch evidence schema version.
pub const PATCH_EVIDENCE_SCHEMA_VERSION: &str = "0.1";

/// Patch acceptance status (mirrors suite status vocabulary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchStatus {
    Accepted,
    Rejected,
    Incomplete,
    Failed,
}

/// Patch-level evidence artifact (plan §10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchEvidence {
    pub schema_version: String,
    pub base_revision: String,
    pub patched_revision: String,
    pub patch_digest: String,
    pub changed_files: Vec<String>,
    pub forbidden_paths_changed: Vec<String>,
    pub generator_binary_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator_spec_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack_lock_digest: Option<String>,
    pub suite_id: String,
    pub suite_digest: String,
    pub suite_status: SuiteStatus,
    pub status: PatchStatus,
}

/// Inputs to build patch evidence.
#[derive(Debug, Clone)]
pub struct PatchEvidenceInput {
    pub base_revision: String,
    pub patched_revision: String,
    pub changed_files: Vec<String>,
    pub patch_policy: PatchPolicy,
    pub generator_binary_digest: String,
    pub generator_spec_digest: Option<String>,
    pub stack_lock_digest: Option<String>,
    pub suite_id: String,
    pub suite_digest: String,
    pub suite_status: SuiteStatus,
    /// Optional unified diff / patch bytes used for `patch_digest`.
    pub patch_bytes: Option<Vec<u8>>,
}

/// Build patch evidence; fail closed on forbidden path changes.
pub fn build_patch_evidence(input: &PatchEvidenceInput) -> Result<PatchEvidence, GeneratorError> {
    if input.base_revision.trim().is_empty() || input.patched_revision.trim().is_empty() {
        return Err(GeneratorError::Validation(
            "base_revision and patched_revision must not be empty".to_owned(),
        ));
    }
    if input.generator_binary_digest.trim().is_empty() {
        return Err(GeneratorError::Validation(
            "generator_binary_digest must not be empty".to_owned(),
        ));
    }
    if input.suite_id.trim().is_empty() || input.suite_digest.trim().is_empty() {
        return Err(GeneratorError::Validation(
            "suite_id and suite_digest must not be empty".to_owned(),
        ));
    }

    let forbidden = path_policy_violations(&input.changed_files, &input.patch_policy)
        .into_iter()
        .filter(|v| v.contains("(forbidden)"))
        .map(|v| v.replace(" (forbidden)", ""))
        .collect::<Vec<_>>();

    let patch_digest = match &input.patch_bytes {
        Some(bytes) => sha256_digest_prefixed(bytes),
        None => digest_changed_files(&input.changed_files),
    };

    let status = derive_patch_status(input.suite_status, &forbidden);

    Ok(PatchEvidence {
        schema_version: PATCH_EVIDENCE_SCHEMA_VERSION.to_owned(),
        base_revision: input.base_revision.clone(),
        patched_revision: input.patched_revision.clone(),
        patch_digest,
        changed_files: input.changed_files.clone(),
        forbidden_paths_changed: forbidden,
        generator_binary_digest: input.generator_binary_digest.clone(),
        generator_spec_digest: input.generator_spec_digest.clone(),
        stack_lock_digest: input.stack_lock_digest.clone(),
        suite_id: input.suite_id.clone(),
        suite_digest: input.suite_digest.clone(),
        suite_status: input.suite_status,
        status,
    })
}

fn derive_patch_status(suite: SuiteStatus, forbidden: &[String]) -> PatchStatus {
    if !forbidden.is_empty() {
        return PatchStatus::Failed;
    }
    match suite {
        SuiteStatus::Accepted => PatchStatus::Accepted,
        SuiteStatus::Rejected => PatchStatus::Rejected,
        SuiteStatus::Incomplete => PatchStatus::Incomplete,
        SuiteStatus::Failed => PatchStatus::Failed,
    }
}

fn digest_changed_files(files: &[String]) -> String {
    let mut sorted = files.to_vec();
    sorted.sort();
    let hash = Sha256::digest(canonical_json_bytes(&sorted));
    format!("sha256:{}", hex_encode(&hash))
}

/// Canonical digest of a patch evidence document.
#[must_use]
pub fn patch_evidence_digest(evidence: &PatchEvidence) -> String {
    let hash = Sha256::digest(canonical_json_bytes(evidence));
    format!("sha256:{}", hex_encode(&hash))
}

/// Load patch evidence JSON from disk.
pub fn load_patch_evidence(path: impl AsRef<Path>) -> Result<PatchEvidence, GeneratorError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|source| GeneratorError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let evidence: PatchEvidence =
        serde_json::from_slice(&bytes).map_err(|error| GeneratorError::Parse {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    validate_patch_evidence(&evidence)?;
    Ok(evidence)
}

/// Write patch evidence JSON (pretty).
pub fn write_patch_evidence(
    path: impl AsRef<Path>,
    evidence: &PatchEvidence,
) -> Result<(), GeneratorError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| GeneratorError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let body = serde_json::to_vec_pretty(evidence).map_err(|error| {
        GeneratorError::Validation(format!("serialize patch evidence: {error}"))
    })?;
    std::fs::write(path, body).map_err(|source| GeneratorError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Structural validation for patch evidence.
pub fn validate_patch_evidence(evidence: &PatchEvidence) -> Result<(), GeneratorError> {
    let mut diagnostics = Vec::new();
    if evidence.schema_version != PATCH_EVIDENCE_SCHEMA_VERSION {
        diagnostics.push(format!(
            "unsupported schema_version `{}`",
            evidence.schema_version
        ));
    }
    if evidence.base_revision.trim().is_empty() {
        diagnostics.push("base_revision must not be empty".to_owned());
    }
    if evidence.patched_revision.trim().is_empty() {
        diagnostics.push("patched_revision must not be empty".to_owned());
    }
    if !evidence.patch_digest.starts_with("sha256:") {
        diagnostics.push("patch_digest must be sha256-prefixed".to_owned());
    }
    if !evidence.generator_binary_digest.starts_with("sha256:") {
        diagnostics.push("generator_binary_digest must be sha256-prefixed".to_owned());
    }
    if evidence.suite_id.trim().is_empty() {
        diagnostics.push("suite_id must not be empty".to_owned());
    }
    if !evidence.forbidden_paths_changed.is_empty() && evidence.status == PatchStatus::Accepted {
        diagnostics
            .push("status cannot be accepted when forbidden_paths_changed is non-empty".to_owned());
    }
    if evidence.status == PatchStatus::Accepted && evidence.suite_status != SuiteStatus::Accepted {
        diagnostics.push("status cannot be accepted unless suite_status is accepted".to_owned());
    }
    if !diagnostics.is_empty() {
        return Err(GeneratorError::from_diagnostics(&diagnostics));
    }
    Ok(())
}

/// Verify an on-disk patch evidence file (structure + honesty invariants).
pub fn verify_patch_evidence_file(path: impl AsRef<Path>) -> Result<PatchEvidence, GeneratorError> {
    let evidence = load_patch_evidence(path)?;
    Ok(evidence)
}

/// List changed paths from `git diff --name-only <base>..<patched>` (optional helper).
pub fn git_changed_files(
    repo: &Path,
    base: &str,
    patched: &str,
) -> Result<Vec<String>, GeneratorError> {
    let base = base.strip_prefix("git:").unwrap_or(base);
    let patched = patched.strip_prefix("git:").unwrap_or(patched);
    let range = format!("{base}..{patched}");
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", &range])
        .current_dir(repo)
        .output()
        .map_err(|source| GeneratorError::Io {
            path: repo.to_path_buf(),
            source,
        })?;
    if !output.status.success() {
        return Err(GeneratorError::Validation(format!(
            "git diff failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::spec::PatchPolicy;

    #[test]
    fn accepts_clean_patch_with_accepted_suite() {
        let evidence = build_patch_evidence(&PatchEvidenceInput {
            base_revision: "git:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            patched_revision: "git:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            changed_files: vec!["src/backend/foo.rs".into()],
            patch_policy: PatchPolicy {
                allowed_paths: vec!["src/backend/**".into()],
                forbidden_paths: vec!["**/stack.lock.toml".into()],
            },
            generator_binary_digest:
                "sha256:1111111111111111111111111111111111111111111111111111111111111111".into(),
            generator_spec_digest: None,
            stack_lock_digest: None,
            suite_id: "s".into(),
            suite_digest: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                .into(),
            suite_status: SuiteStatus::Accepted,
            patch_bytes: Some(b"diff".to_vec()),
        })
        .unwrap();
        assert_eq!(evidence.status, PatchStatus::Accepted);
        assert!(evidence.forbidden_paths_changed.is_empty());
        validate_patch_evidence(&evidence).unwrap();
    }

    #[test]
    fn forbidden_path_forces_failed() {
        let evidence = build_patch_evidence(&PatchEvidenceInput {
            base_revision: "git:a".into(),
            patched_revision: "git:b".into(),
            changed_files: vec!["integrations/hlax64/stack.lock.toml".into()],
            patch_policy: PatchPolicy {
                allowed_paths: vec![],
                forbidden_paths: vec!["**/stack.lock.toml".into()],
            },
            generator_binary_digest:
                "sha256:1111111111111111111111111111111111111111111111111111111111111111".into(),
            generator_spec_digest: None,
            stack_lock_digest: None,
            suite_id: "s".into(),
            suite_digest: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                .into(),
            suite_status: SuiteStatus::Accepted,
            patch_bytes: None,
        })
        .unwrap();
        assert_eq!(evidence.status, PatchStatus::Failed);
        assert!(!evidence.forbidden_paths_changed.is_empty());
    }

    #[test]
    fn roundtrip_write_load() {
        let dir = std::env::temp_dir().join(format!(
            "vaa-patch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let evidence = build_patch_evidence(&PatchEvidenceInput {
            base_revision: "git:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            patched_revision: "git:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            changed_files: vec!["src/a.rs".into()],
            patch_policy: PatchPolicy::default(),
            generator_binary_digest:
                "sha256:1111111111111111111111111111111111111111111111111111111111111111".into(),
            generator_spec_digest: None,
            stack_lock_digest: None,
            suite_id: "s".into(),
            suite_digest: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                .into(),
            suite_status: SuiteStatus::Accepted,
            patch_bytes: Some(b"x".to_vec()),
        })
        .unwrap();
        let path = dir.join("patch.json");
        write_patch_evidence(&path, &evidence).unwrap();
        let loaded = verify_patch_evidence_file(&path).unwrap();
        assert_eq!(loaded, evidence);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
