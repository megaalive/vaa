//! Portable transparency export (`vaa-transparency-v1`).
//!
//! Exports digests from a run directory so they can be stored **outside** the
//! run tree (CI artifact, Git note, etc.). This is **not** a remote immutable
//! log service and **not** authenticity.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::chain::verify_chain;
use super::seal::SealError;
use super::seal_log::{read_seal_log, SealLogEntry};

/// Document type id / schema version string.
pub const TRANSPARENCY_SCHEMA_VERSION: &str = "vaa-transparency-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransparencyEntry {
    pub candidate_index: u32,
    pub acceptance_digest: String,
    pub envelope_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_seal_digest: Option<String>,
    pub final_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransparencyDocument {
    pub schema_version: String,
    /// UTC unix epoch seconds as a decimal string (no calendar crate).
    pub exported_at: String,
    pub run_id: String,
    pub task_id: String,
    pub target: String,
    pub final_envelope_digest: String,
    pub final_acceptance_digest: String,
    pub entries: Vec<TransparencyEntry>,
}

fn exported_at_now() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or_else(|_| "0".into(), |d| d.as_secs().to_string())
}

/// Build a transparency document from a verified run directory.
pub fn export_transparency(run_dir: &Path) -> Result<TransparencyDocument, SealError> {
    let report = verify_chain(run_dir)?;
    let evidence_dir = run_dir.join("evidence");
    let log = read_seal_log(&evidence_dir)?;

    let (run_id, task_id, entries) = if log.is_empty() {
        (
            report
                .identity
                .run_id
                .clone()
                .unwrap_or_else(|| "unknown".into()),
            report.identity.task_id.clone(),
            vec![TransparencyEntry {
                candidate_index: report.candidate_count.saturating_sub(1),
                acceptance_digest: report.last_acceptance_digest.clone(),
                envelope_digest: report.last_envelope_digest.clone(),
                previous_seal_digest: None,
                final_status: "unknown".into(),
            }],
        )
    } else {
        let first = &log[0];
        (
            first.run_id.clone(),
            first.task_id.clone(),
            log.iter().map(TransparencyEntry::from).collect(),
        )
    };

    Ok(TransparencyDocument {
        schema_version: TRANSPARENCY_SCHEMA_VERSION.to_owned(),
        exported_at: exported_at_now(),
        run_id,
        task_id,
        target: report.identity.target,
        final_envelope_digest: report.last_envelope_digest,
        final_acceptance_digest: report.last_acceptance_digest,
        entries,
    })
}

impl From<&SealLogEntry> for TransparencyEntry {
    fn from(e: &SealLogEntry) -> Self {
        Self {
            candidate_index: e.candidate_index,
            acceptance_digest: e.acceptance_digest.clone(),
            envelope_digest: e.envelope_digest.clone(),
            previous_seal_digest: e.previous_seal_digest.clone(),
            final_status: e.final_status.clone(),
        }
    }
}

/// Write pretty JSON transparency document.
pub fn write_transparency_file(
    run_dir: &Path,
    output: &Path,
) -> Result<TransparencyDocument, SealError> {
    let doc = export_transparency(run_dir)?;
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| SealError::Io(e.to_string()))?;
        }
    }
    let body = serde_json::to_string_pretty(&doc).map_err(|e| SealError::Json(e.to_string()))?;
    fs::write(output, body).map_err(|e| SealError::Io(e.to_string()))?;
    Ok(doc)
}

/// Load and validate schema of a transparency file.
pub fn read_transparency_file(path: &Path) -> Result<TransparencyDocument, SealError> {
    let raw = fs::read_to_string(path).map_err(|e| SealError::Io(e.to_string()))?;
    let doc: TransparencyDocument =
        serde_json::from_str(&raw).map_err(|e| SealError::Json(e.to_string()))?;
    if doc.schema_version != TRANSPARENCY_SCHEMA_VERSION {
        return Err(SealError::UnsupportedSchema(doc.schema_version));
    }
    Ok(doc)
}

/// Fail-closed: export file digests must match the live run chain.
pub fn verify_transparency_against_run(
    transparency_path: &Path,
    run_dir: &Path,
) -> Result<(), SealError> {
    let exported = read_transparency_file(transparency_path)?;
    let live = export_transparency(run_dir)?;

    if exported.run_id != live.run_id {
        return Err(SealError::EvidenceMismatch(format!(
            "transparency run_id drift: export={} live={}",
            exported.run_id, live.run_id
        )));
    }
    if exported.task_id != live.task_id {
        return Err(SealError::EvidenceMismatch(format!(
            "transparency task_id drift: export={} live={}",
            exported.task_id, live.task_id
        )));
    }
    if exported.target != live.target {
        return Err(SealError::EvidenceMismatch(format!(
            "transparency target drift: export={} live={}",
            exported.target, live.target
        )));
    }
    if exported.final_envelope_digest != live.final_envelope_digest {
        return Err(SealError::EvidenceMismatch(
            "transparency final_envelope_digest drift".into(),
        ));
    }
    if exported.final_acceptance_digest != live.final_acceptance_digest {
        return Err(SealError::EvidenceMismatch(
            "transparency final_acceptance_digest drift".into(),
        ));
    }
    if exported.entries.len() != live.entries.len() {
        return Err(SealError::EvidenceMismatch(format!(
            "transparency entry count drift: export={} live={}",
            exported.entries.len(),
            live.entries.len()
        )));
    }
    for (i, (a, b)) in exported.entries.iter().zip(live.entries.iter()).enumerate() {
        if a.envelope_digest != b.envelope_digest || a.acceptance_digest != b.acceptance_digest {
            return Err(SealError::EvidenceMismatch(format!(
                "transparency entry {i} digest drift"
            )));
        }
        if a.candidate_index != b.candidate_index {
            return Err(SealError::EvidenceMismatch(format!(
                "transparency entry {i} candidate_index drift"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::bundle::{materialize_bundle_files, BUNDLE_SOURCE};
    use crate::evidence::report::sha256_digest_prefixed;
    use crate::evidence::seal::{
        write_final_sealed_evidence, write_sealed_evidence, GeneratorMeta, SealBuildInput,
    };
    use crate::evidence::seal_log::{append_seal_log, SealLogEntry};
    use crate::evidence::{EvidenceAggregator, EvidenceExpect, EvidenceStatus};
    use crate::semasm::capabilities::CapabilityMatch;
    use crate::semasm::doctor::{DoctorReport, DoctorStatus, SemasmVersion};
    use crate::semasm::verify::VerifyReport;
    use crate::task::load_locked_task;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn tempfile_dir(prefix: &str) -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn one_candidate_run() -> PathBuf {
        let task_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/tasks/sum_i64.vaa.toml");
        let task = load_locked_task(&task_path).unwrap();
        let task_bytes = fs::read(&task_path).unwrap();
        let source = b"xor eax, eax";
        let contract = b"[contract]\n";
        let report_json = r#"{"status":"execution_denied"}"#;
        let expect = EvidenceExpect::new(
            task.task().target.clone(),
            sha256_digest_prefixed(source),
            sha256_digest_prefixed(contract),
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
            raw_json: report_json.into(),
        };
        let report = EvidenceAggregator::build(
            &task,
            Some("run-t0".into()),
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

        let run = tempfile_dir("vaa_transparency_run");
        fs::create_dir_all(run.join("candidates")).unwrap();
        fs::create_dir_all(run.join("evidence")).unwrap();
        let cand = run.join("candidates").join("0000");
        fs::create_dir_all(&cand).unwrap();
        materialize_bundle_files(&cand, &task_bytes, contract, Some(report_json)).unwrap();
        fs::write(cand.join(BUNDLE_SOURCE), source).unwrap();
        let env = write_sealed_evidence(
            &cand,
            &report,
            &expect,
            SealBuildInput {
                candidate_index: 0,
                previous_seal_digest: None,
                generator: GeneratorMeta::ingest("unit"),
            },
        )
        .unwrap();
        append_seal_log(
            &run.join("evidence"),
            &SealLogEntry::from_seal("run-t0", &task.task().task_id, report.final_status, &env),
        )
        .unwrap();
        write_final_sealed_evidence(&run.join("evidence"), &report, &env).unwrap();
        run
    }

    #[test]
    fn export_and_verify_transparency_round_trip() {
        let run = one_candidate_run();
        let out = run.join("transparency.json");
        write_transparency_file(&run, &out).unwrap();
        verify_transparency_against_run(&out, &run).unwrap();
        let doc = read_transparency_file(&out).unwrap();
        assert_eq!(doc.schema_version, TRANSPARENCY_SCHEMA_VERSION);
        assert_eq!(doc.run_id, "run-t0");
        assert_eq!(doc.entries.len(), 1);
        let _ = fs::remove_dir_all(&run);
    }

    #[test]
    fn verify_transparency_detects_tamper() {
        let run = one_candidate_run();
        let out = run.join("transparency.json");
        let mut doc = write_transparency_file(&run, &out).unwrap();
        doc.final_envelope_digest =
            "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into();
        fs::write(&out, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
        let err = verify_transparency_against_run(&out, &run).unwrap_err();
        assert!(matches!(err, SealError::EvidenceMismatch(_)));
        let _ = fs::remove_dir_all(&run);
    }

    #[test]
    fn verify_transparency_rejects_malformed_json() {
        let dir = std::env::temp_dir().join(format!(
            "vaa_transparency_malformed_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.json");
        fs::write(&path, b"{not-json").unwrap();
        let err = read_transparency_file(&path).unwrap_err();
        assert!(
            matches!(err, SealError::Json(_)),
            "expected SealError::Json for malformed transparency: {err}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
