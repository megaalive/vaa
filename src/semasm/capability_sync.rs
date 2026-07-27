//! Sync frozen SemASM capabilities into VAA fixtures (`vaa semasm capability-sync`).
//!
//! Does **not** auto-commit. With `--apply`, writes the snapshot JSON and patches
//! `CAPABILITY_SNAPSHOT_DIGEST` in `src/semasm/admission.rs`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use crate::semasm::admission::{
    load_snapshot, CapabilitiesSnapshot, SnapshotAdmission, ADMISSION_SOURCE,
    CAPABILITY_SNAPSHOT_DIGEST,
};
use crate::semasm::doctor::ENV_SEMASM_BIN;

/// Result of comparing live SemASM capabilities to the frozen snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitySyncDiff {
    pub added: Vec<String>,
    pub changed: Vec<String>,
    pub removed: Vec<String>,
    pub old_digest: String,
    pub new_digest: String,
}

impl CapabilitySyncDiff {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.changed.is_empty()
            && self.removed.is_empty()
            && self.old_digest == self.new_digest
    }
}

/// Errors from capability-sync.
#[derive(Debug, thiserror::Error)]
pub enum CapabilitySyncError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Resolve SemASM binary from `--semasm` or `SEMASM_BIN` / PATH.
#[must_use]
pub fn resolve_semasm_bin(explicit: Option<&Path>) -> PathBuf {
    if let Some(p) = explicit {
        return p.to_path_buf();
    }
    if let Ok(v) = std::env::var(ENV_SEMASM_BIN) {
        if !v.trim().is_empty() {
            return PathBuf::from(v);
        }
    }
    PathBuf::from("semasm")
}

/// Run `semasm capabilities --format json` and parse the snapshot document.
pub fn fetch_live_capabilities(semasm: &Path) -> Result<CapabilitiesSnapshot, CapabilitySyncError> {
    let output = Command::new(semasm)
        .args(["capabilities", "--format", "json"])
        .output()
        .map_err(|e| {
            CapabilitySyncError::Message(format!(
                "failed to spawn `{} capabilities --format json`: {e}",
                semasm.display()
            ))
        })?;
    if !output.status.success() {
        return Err(CapabilitySyncError::Message(format!(
            "`{} capabilities` failed (status {}): {}",
            semasm.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let snap: CapabilitiesSnapshot = serde_json::from_slice(&output.stdout)?;
    if snap.digest.is_empty() || !snap.digest.starts_with("sha256:") {
        return Err(CapabilitySyncError::Message(
            "live capabilities JSON missing sha256 digest field".into(),
        ));
    }
    Ok(snap)
}

fn admission_key(row: &SnapshotAdmission) -> String {
    let leaves = row.leaf_names.join(",");
    let targets = row.targets.join(",");
    let asms = row.assemblers.join(",");
    format!("{}|{}|{}|{}", row.capability_id, leaves, targets, asms)
}

fn index_admissions(rows: &[SnapshotAdmission]) -> BTreeMap<String, &SnapshotAdmission> {
    rows.iter().map(|r| (admission_key(r), r)).collect()
}

/// Semantic diff between frozen and live admission rows.
#[must_use]
pub fn diff_snapshots(
    old: &CapabilitiesSnapshot,
    new: &CapabilitiesSnapshot,
) -> CapabilitySyncDiff {
    let old_idx = index_admissions(&old.admission);
    let new_idx = index_admissions(&new.admission);
    let old_keys: BTreeSet<_> = old_idx.keys().cloned().collect();
    let new_keys: BTreeSet<_> = new_idx.keys().cloned().collect();

    let mut added = Vec::new();
    for k in new_keys.difference(&old_keys) {
        let row = new_idx[k];
        added.push(format!(
            "{} / {:?} / {}",
            row.leaf_names.join(","),
            row.targets,
            row.acceptance_level
        ));
    }
    let mut removed = Vec::new();
    for k in old_keys.difference(&new_keys) {
        let row = old_idx[k];
        removed.push(format!(
            "{} / {:?} / {}",
            row.leaf_names.join(","),
            row.targets,
            row.acceptance_level
        ));
    }
    let mut changed = Vec::new();
    for k in old_keys.intersection(&new_keys) {
        let a = old_idx[k];
        let b = new_idx[k];
        if a.acceptance_level != b.acceptance_level {
            changed.push(format!(
                "{} {:?}: {} → {}",
                a.leaf_names.join(","),
                a.targets,
                a.acceptance_level,
                b.acceptance_level
            ));
        }
    }
    CapabilitySyncDiff {
        added,
        changed,
        removed,
        old_digest: old.digest.clone(),
        new_digest: new.digest.clone(),
    }
}

/// Pretty-print a sync diff to a string.
#[must_use]
pub fn format_diff(diff: &CapabilitySyncDiff) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "old_digest: {}", diff.old_digest);
    let _ = writeln!(out, "new_digest: {}", diff.new_digest);
    out.push_str("\nadded:\n");
    if diff.added.is_empty() {
        out.push_str("  none\n");
    } else {
        for line in &diff.added {
            let _ = writeln!(out, "  {line}");
        }
    }
    out.push_str("\nchanged:\n");
    if diff.changed.is_empty() {
        out.push_str("  none\n");
    } else {
        for line in &diff.changed {
            let _ = writeln!(out, "  {line}");
        }
    }
    out.push_str("\nremoved:\n");
    if diff.removed.is_empty() {
        out.push_str("  none\n");
    } else {
        for line in &diff.removed {
            let _ = writeln!(out, "  {line}");
        }
    }
    out
}

/// Write snapshot JSON (pretty) to `output`.
pub fn write_snapshot_file(
    snap: &CapabilitiesSnapshot,
    output: &Path,
) -> Result<(), CapabilitySyncError> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(snap)?;
    fs::write(output, format!("{body}\n"))?;
    Ok(())
}

/// Patch `CAPABILITY_SNAPSHOT_DIGEST` in `admission.rs` (no git commit).
pub fn patch_digest_constant(
    admission_rs: &Path,
    new_digest: &str,
) -> Result<(), CapabilitySyncError> {
    let raw = fs::read_to_string(admission_rs)?;
    let needle = "pub const CAPABILITY_SNAPSHOT_DIGEST: &str =\n    \"";
    let Some(start) = raw.find(needle) else {
        return Err(CapabilitySyncError::Message(
            "CAPABILITY_SNAPSHOT_DIGEST declaration not found in admission.rs".into(),
        ));
    };
    let value_start = start + needle.len();
    let rest = &raw[value_start..];
    let Some(end_rel) = rest.find('"') else {
        return Err(CapabilitySyncError::Message(
            "CAPABILITY_SNAPSHOT_DIGEST string terminator missing".into(),
        ));
    };
    let mut updated = String::new();
    updated.push_str(&raw[..value_start]);
    updated.push_str(new_digest);
    updated.push_str(&rest[end_rel..]);
    fs::write(admission_rs, updated)?;
    Ok(())
}

/// Full sync workflow: fetch → diff → optional write.
pub fn capability_sync(
    semasm: &Path,
    output: &Path,
    apply: bool,
    admission_rs: Option<&Path>,
) -> Result<(CapabilitiesSnapshot, CapabilitySyncDiff), CapabilitySyncError> {
    let live = fetch_live_capabilities(semasm)?;
    let frozen = load_snapshot();
    if frozen.digest != CAPABILITY_SNAPSHOT_DIGEST {
        return Err(CapabilitySyncError::Message(format!(
            "frozen snapshot digest {} != CAPABILITY_SNAPSHOT_DIGEST {CAPABILITY_SNAPSHOT_DIGEST}",
            frozen.digest
        )));
    }
    let diff = diff_snapshots(&frozen, &live);
    if apply {
        write_snapshot_file(&live, output)?;
        if let Some(rs) = admission_rs {
            patch_digest_constant(rs, &live.digest)?;
        }
    }
    let _ = ADMISSION_SOURCE; // keep path documented for callers
    Ok((live, diff))
}

/// Parse a capabilities JSON value (schema check for tests / offline).
pub fn parse_capabilities_json(raw: &str) -> Result<CapabilitiesSnapshot, CapabilitySyncError> {
    let v: Value = serde_json::from_str(raw)?;
    if v.get("digest").and_then(|d| d.as_str()).is_none() {
        return Err(CapabilitySyncError::Message(
            "capabilities JSON missing digest".into(),
        ));
    }
    if v.get("admission").and_then(|a| a.as_array()).is_none() {
        return Err(CapabilitySyncError::Message(
            "capabilities JSON missing admission array".into(),
        ));
    }
    Ok(serde_json::from_value(v)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_diff_emptyish() {
        let d = CapabilitySyncDiff {
            added: vec![],
            changed: vec!["count_byte Linux: verified_under_preconditions → verified".into()],
            removed: vec![],
            old_digest: "sha256:a".into(),
            new_digest: "sha256:b".into(),
        };
        let s = format_diff(&d);
        assert!(s.contains("changed:"));
        assert!(s.contains("count_byte"));
        assert!(s.contains("added:\n  none"));
    }

    #[test]
    fn parse_frozen_via_parse_helper() {
        let raw = include_str!("../../fixtures/semasm/capabilities-snapshot.json");
        let snap = parse_capabilities_json(raw).expect("fixture");
        assert_eq!(snap.digest, CAPABILITY_SNAPSHOT_DIGEST);
    }
}
