//! Append-only per-run seal digest log (`evidence/seal-log.jsonl`).
//!
//! This is a **local transparency artifact** inside the run directory: it records
//! `envelope_digest` / `acceptance_digest` in order. It is **not** an external
//! transparency log and **not** authenticity (no signatures).

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::seal::{SealEnvelope, SealError};
use super::status::EvidenceStatus;

/// Filename under `evidence/`.
pub const SEAL_LOG_NAME: &str = "seal-log.jsonl";

/// Schema for one JSONL line.
pub const SEAL_LOG_SCHEMA_VERSION: &str = "0.1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SealLogEntry {
    pub schema_version: String,
    pub run_id: String,
    pub task_id: String,
    pub candidate_index: u32,
    pub acceptance_digest: String,
    pub envelope_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_seal_digest: Option<String>,
    pub final_status: String,
}

impl SealLogEntry {
    #[must_use]
    pub fn from_seal(
        run_id: &str,
        task_id: &str,
        final_status: EvidenceStatus,
        envelope: &SealEnvelope,
    ) -> Self {
        Self {
            schema_version: SEAL_LOG_SCHEMA_VERSION.to_owned(),
            run_id: run_id.to_owned(),
            task_id: task_id.to_owned(),
            candidate_index: envelope.provenance.candidate_index,
            acceptance_digest: envelope.acceptance_digest.clone(),
            envelope_digest: envelope.envelope_digest.clone(),
            previous_seal_digest: envelope.provenance.previous_seal_digest.clone(),
            final_status: format!("{final_status:?}"),
        }
    }
}

/// Append one seal line to `evidence/seal-log.jsonl` (create if missing).
pub fn append_seal_log(evidence_dir: &Path, entry: &SealLogEntry) -> Result<PathBuf, SealError> {
    std::fs::create_dir_all(evidence_dir).map_err(|e| SealError::Io(e.to_string()))?;
    let path = evidence_dir.join(SEAL_LOG_NAME);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| SealError::Io(e.to_string()))?;
    let line = serde_json::to_string(entry).map_err(|e| SealError::Json(e.to_string()))?;
    writeln!(file, "{line}").map_err(|e| SealError::Io(e.to_string()))?;
    file.flush().map_err(|e| SealError::Io(e.to_string()))?;
    let _ = file.sync_all();
    Ok(path)
}

/// Read all seal-log entries (empty vec if file missing).
pub fn read_seal_log(evidence_dir: &Path) -> Result<Vec<SealLogEntry>, SealError> {
    let path = evidence_dir.join(SEAL_LOG_NAME);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(&path).map_err(|e| SealError::Io(e.to_string()))?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| SealError::Io(e.to_string()))?;
        if line.trim().is_empty() {
            continue;
        }
        let entry: SealLogEntry = serde_json::from_str(&line)
            .map_err(|e| SealError::Json(format!("seal-log.jsonl line {}: {e}", i + 1)))?;
        entries.push(entry);
    }
    Ok(entries)
}

/// When a log exists, require it to match sealed candidate envelope digests in order.
pub fn verify_seal_log_against_digests(
    evidence_dir: &Path,
    envelope_digests: &[String],
) -> Result<(), SealError> {
    let entries = read_seal_log(evidence_dir)?;
    if entries.is_empty() {
        // Backward compatible: older runs have no log.
        return Ok(());
    }
    if entries.len() != envelope_digests.len() {
        return Err(SealError::EvidenceMismatch(format!(
            "seal-log.jsonl has {} entries but chain has {} candidates",
            entries.len(),
            envelope_digests.len()
        )));
    }
    for (i, (entry, expected)) in entries.iter().zip(envelope_digests.iter()).enumerate() {
        if entry.candidate_index != i as u32 {
            return Err(SealError::EvidenceMismatch(format!(
                "seal-log.jsonl entry {i}: candidate_index={}, expected {i}",
                entry.candidate_index
            )));
        }
        if &entry.envelope_digest != expected {
            return Err(SealError::EvidenceMismatch(format!(
                "seal-log.jsonl entry {i}: envelope_digest mismatch"
            )));
        }
        if i == 0 {
            if entry.previous_seal_digest.is_some() {
                return Err(SealError::EvidenceMismatch(
                    "seal-log.jsonl entry 0: previous_seal_digest must be null".into(),
                ));
            }
        } else if entry.previous_seal_digest.as_ref() != Some(&envelope_digests[i - 1]) {
            return Err(SealError::EvidenceMismatch(format!(
                "seal-log.jsonl entry {i}: previous_seal_digest mismatch"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::seal::{write_sealed_evidence, GeneratorMeta, SealBuildInput};
    use crate::evidence::EvidenceAggregator;
    use crate::semasm::capabilities::CapabilityMatch;
    use crate::semasm::doctor::{DoctorReport, DoctorStatus, SemasmVersion};
    use crate::semasm::verify::VerifyReport;
    use crate::task::load_locked_task;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tempfile_dir(prefix: &str) -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("{prefix}_{nanos}_{n}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_envelope() -> (PathBuf, SealEnvelope) {
        let task = load_locked_task(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/tasks/sum_i64.vaa.toml"),
        )
        .unwrap();
        let expect = crate::evidence::EvidenceExpect::new(
            task.task().target.clone(),
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        let verify = VerifyReport {
            outcome: EvidenceStatus::Incomplete,
            raw_status: "execution_denied".into(),
            schema_version: Some("0.4".into()),
            diagnostics: vec![],
            target: Some(task.task().target.clone()),
            source_digest: Some(expect.expected_source_digest.clone()),
            contract_digest: Some(expect.expected_contract_digest.clone()),
            tool_version: Some("semasm 0.1.0".into()),
            raw_json: r#"{"status":"execution_denied"}"#.into(),
        };
        let report = EvidenceAggregator::build(
            &task,
            Some("run-log-1".into()),
            Some(verify),
            Some(DoctorReport {
                status: DoctorStatus::Available,
                binary_path: Some(PathBuf::from("semasm")),
                version: Some(SemasmVersion {
                    version: "0.1.0".into(),
                    schema_version: "0.1".into(),
                }),
                details: vec![],
                live_probe: None,
            }),
            Some(CapabilityMatch {
                compatible: true,
                missing: vec![],
                insufficient: vec![],
            }),
            &expect,
        );
        let dir = tempfile_dir("vaa_seal_log");
        let env = write_sealed_evidence(
            &dir,
            &report,
            &expect,
            SealBuildInput {
                candidate_index: 0,
                previous_seal_digest: None,
                generator: GeneratorMeta::ingest("unit"),
            },
        )
        .unwrap();
        (dir, env)
    }

    #[test]
    fn append_and_verify_log_round_trip() {
        let (dir, env) = sample_envelope();
        let evidence_dir = tempfile_dir("vaa_seal_log_ev");
        let entry =
            SealLogEntry::from_seal("run-log-1", "sum_i64", EvidenceStatus::Incomplete, &env);
        append_seal_log(&evidence_dir, &entry).unwrap();
        let digests = vec![env.envelope_digest.clone()];
        verify_seal_log_against_digests(&evidence_dir, &digests).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&evidence_dir);
    }

    #[test]
    fn missing_log_is_ok_for_backward_compat() {
        let evidence_dir = tempfile_dir("vaa_seal_log_missing");
        verify_seal_log_against_digests(&evidence_dir, &["sha256:dead".into()]).unwrap();
        let _ = std::fs::remove_dir_all(&evidence_dir);
    }

    #[test]
    fn log_digest_mismatch_fails() {
        let (dir, env) = sample_envelope();
        let evidence_dir = tempfile_dir("vaa_seal_log_bad");
        let entry =
            SealLogEntry::from_seal("run-log-1", "sum_i64", EvidenceStatus::Incomplete, &env);
        append_seal_log(&evidence_dir, &entry).unwrap();
        let err = verify_seal_log_against_digests(
            &evidence_dir,
            &["sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into()],
        )
        .unwrap_err();
        assert!(matches!(err, SealError::EvidenceMismatch(_)));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&evidence_dir);
    }
}
