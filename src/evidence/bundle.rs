//! Bundle verification: re-hash on-disk artifacts against sealed digests.

use std::fs;
use std::path::Path;

use super::report::sha256_digest_prefixed;
use super::seal::{verify_seal, SealEnvelope, SealError};
use crate::task::load_locked_task;

/// Required filenames inside a sealable evidence bundle directory.
pub const BUNDLE_TASK: &str = "task.vaa.toml";
pub const BUNDLE_CONTRACT: &str = "contract.sem.toml";
pub const BUNDLE_SOURCE: &str = "candidate.asm";
pub const BUNDLE_REPORT: &str = "semasm-report.json";
pub const BUNDLE_EVIDENCE: &str = "evidence.json";
pub const BUNDLE_SEAL: &str = "evidence.seal.json";

/// Verify a candidate/final bundle directory against sealed digests.
///
/// Recomputes digests of task/contract/source/report artifacts — unlike
/// [`verify_seal`], which only checks evidence↔seal JSON drift.
pub fn verify_bundle(bundle_dir: &Path) -> Result<SealEnvelope, SealError> {
    let evidence_path = bundle_dir.join(BUNDLE_EVIDENCE);
    let seal_path = bundle_dir.join(BUNDLE_SEAL);
    verify_seal(&evidence_path, &seal_path)?;

    let seal_raw = fs::read_to_string(&seal_path).map_err(|e| SealError::Io(e.to_string()))?;
    let envelope: SealEnvelope =
        serde_json::from_str(&seal_raw).map_err(|e| SealError::Json(e.to_string()))?;

    let task_path = bundle_dir.join(BUNDLE_TASK);
    let locked = load_locked_task(&task_path).map_err(|e| SealError::Bundle(e.to_string()))?;
    if locked.digest().prefixed() != envelope.acceptance.task_digest {
        return Err(SealError::Bundle(
            "task.vaa.toml digest != acceptance.task_digest".into(),
        ));
    }

    let contract_bytes = fs::read(bundle_dir.join(BUNDLE_CONTRACT))
        .map_err(|e| SealError::Bundle(format!("read contract: {e}")))?;
    let contract_digest = sha256_digest_prefixed(&contract_bytes);
    if contract_digest != envelope.acceptance.contract_digest {
        return Err(SealError::Bundle(
            "contract.sem.toml digest != acceptance.contract_digest".into(),
        ));
    }

    let source_bytes = fs::read(bundle_dir.join(BUNDLE_SOURCE))
        .map_err(|e| SealError::Bundle(format!("read candidate: {e}")))?;
    let source_digest = sha256_digest_prefixed(&source_bytes);
    if source_digest != envelope.acceptance.source_digest {
        return Err(SealError::Bundle(
            "candidate.asm digest != acceptance.source_digest".into(),
        ));
    }

    let report_path = bundle_dir.join(BUNDLE_REPORT);
    if envelope.acceptance.semasm_report_digest == "none" {
        if report_path.exists() {
            return Err(SealError::Bundle(
                "semasm-report.json present but seal says none".into(),
            ));
        }
    } else {
        let report_bytes = fs::read(&report_path)
            .map_err(|e| SealError::Bundle(format!("read semasm-report: {e}")))?;
        let report_digest = sha256_digest_prefixed(&report_bytes);
        if report_digest != envelope.acceptance.semasm_report_digest {
            return Err(SealError::Bundle(
                "semasm-report.json digest != acceptance.semasm_report_digest".into(),
            ));
        }
    }

    Ok(envelope)
}

/// Materialize bundle companion files next to candidate evidence.
pub fn materialize_bundle_files(
    bundle_dir: &Path,
    task_bytes: &[u8],
    contract_bytes: &[u8],
    semasm_report_json: Option<&str>,
) -> Result<(), SealError> {
    fs::create_dir_all(bundle_dir).map_err(|e| SealError::Io(e.to_string()))?;
    fs::write(bundle_dir.join(BUNDLE_TASK), task_bytes)
        .map_err(|e| SealError::Io(e.to_string()))?;
    fs::write(bundle_dir.join(BUNDLE_CONTRACT), contract_bytes)
        .map_err(|e| SealError::Io(e.to_string()))?;
    if let Some(raw) = semasm_report_json {
        fs::write(bundle_dir.join(BUNDLE_REPORT), raw.as_bytes())
            .map_err(|e| SealError::Io(e.to_string()))?;
    } else {
        let path = bundle_dir.join(BUNDLE_REPORT);
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::seal::{write_sealed_evidence, GeneratorMeta, SealBuildInput};
    use crate::evidence::{EvidenceAggregator, EvidenceExpect, EvidenceStatus};
    use crate::semasm::capabilities::CapabilityMatch;
    use crate::semasm::doctor::{DoctorReport, DoctorStatus, SemasmVersion};
    use crate::semasm::verify::VerifyReport;
    use crate::task::load_locked_task;
    use std::path::PathBuf;

    #[test]
    fn verify_bundle_rejects_source_swap() {
        let task_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/tasks/sum_i64.vaa.toml");
        let task = load_locked_task(&task_path).unwrap();
        let task_bytes = fs::read(&task_path).unwrap();
        let contract_bytes = b"contract-bytes";
        let source_bytes = b"mov rax, 1";
        let report_json = r#"{"status":"execution_denied"}"#;

        let expect = EvidenceExpect::new(
            task.task().target.clone(),
            sha256_digest_prefixed(source_bytes),
            sha256_digest_prefixed(contract_bytes),
        );
        let verify = VerifyReport {
            outcome: EvidenceStatus::Incomplete,
            raw_status: "execution_denied".to_owned(),
            schema_version: Some("0.4".to_owned()),
            diagnostics: vec![],
            target: Some(task.task().target.clone()),
            source_digest: Some(expect.expected_source_digest.clone()),
            contract_digest: Some(expect.expected_contract_digest.clone()),
            tool_version: Some("semasm 0.1.0".to_owned()),
            raw_json: report_json.to_owned(),
        };
        let report = EvidenceAggregator::build(
            &task,
            Some("run-1".to_owned()),
            Some(verify),
            Some(DoctorReport {
                status: DoctorStatus::Available,
                binary_path: Some(PathBuf::from("semasm")),
                version: Some(SemasmVersion {
                    version: "0.1.0".to_owned(),
                    schema_version: "0.1".to_owned(),
                }),
                details: vec![],
            }),
            Some(CapabilityMatch {
                compatible: true,
                missing: vec![],
                insufficient: vec![],
            }),
            &expect,
        );

        let dir = std::env::temp_dir().join(format!(
            "vaa_bundle_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        materialize_bundle_files(&dir, &task_bytes, contract_bytes, Some(report_json)).unwrap();
        fs::write(dir.join(BUNDLE_SOURCE), source_bytes).unwrap();
        write_sealed_evidence(
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

        verify_bundle(&dir).expect("bundle ok");

        fs::write(dir.join(BUNDLE_SOURCE), b"mov rax, 2").unwrap();
        let err = verify_bundle(&dir).expect_err("source swap");
        assert!(matches!(err, SealError::Bundle(_)));

        let _ = fs::remove_dir_all(&dir);
    }
}
