//! Full-run seal chain verification (`candidates/0000` … final).

use std::fs;
use std::path::{Path, PathBuf};

use super::bundle::{verify_bundle, BUNDLE_SEAL};
use super::seal::{verify_seal, SealEnvelope, SealError};

/// Summary of a successful [`verify_chain`] pass.
#[derive(Debug, Clone)]
pub struct VerifyChainReport {
    pub candidate_count: u32,
    pub last_envelope_digest: String,
    pub last_acceptance_digest: String,
}

/// Verify every candidate bundle in a run directory as a contiguous hash chain,
/// then confirm `evidence/final.seal.json` matches the last candidate.
///
/// Invariants:
/// - `candidates/0000` … `candidates/NNNN` exist contiguously (no gaps)
/// - candidate 0 has `previous_seal_digest == None`
/// - candidate `i` has `previous_seal_digest == envelope_digest` of `i-1`
/// - `provenance.candidate_index` matches the directory index
/// - final seal's `envelope_digest` equals the last candidate's
pub fn verify_chain(run_dir: &Path) -> Result<VerifyChainReport, SealError> {
    let candidates_root = run_dir.join("candidates");
    if !candidates_root.is_dir() {
        return Err(SealError::Chain("missing candidates/ directory".into()));
    }

    let indices = list_candidate_indices(&candidates_root)?;
    if indices.is_empty() {
        return Err(SealError::Chain("no candidate directories".into()));
    }

    // Contiguous from 0.
    for (expected, actual) in indices.iter().enumerate() {
        if *actual != expected as u32 {
            return Err(SealError::Chain(format!(
                "candidate index gap: expected {expected:04}, found {actual:04}"
            )));
        }
    }

    let mut previous_digest: Option<String> = None;
    let mut last_envelope: Option<SealEnvelope> = None;

    for index in &indices {
        let cand_dir = candidates_root.join(format!("{index:04}"));
        let envelope = verify_bundle(&cand_dir)?;

        if envelope.provenance.candidate_index != *index {
            return Err(SealError::Chain(format!(
                "candidate {index:04}: provenance.candidate_index mismatch"
            )));
        }

        if *index == 0 {
            if envelope.provenance.previous_seal_digest.is_some() {
                return Err(SealError::Chain(
                    "candidate 0000 must have previous_seal_digest = null".into(),
                ));
            }
        } else {
            match (&envelope.provenance.previous_seal_digest, &previous_digest) {
                (Some(prev), Some(expected)) if prev == expected => {}
                (Some(prev), Some(expected)) => {
                    return Err(SealError::Chain(format!(
                        "candidate {index:04}: previous_seal_digest mismatch \
                         (seal={prev}, predecessor={expected})"
                    )));
                }
                (None, _) => {
                    return Err(SealError::Chain(format!(
                        "candidate {index:04}: missing previous_seal_digest"
                    )));
                }
                (Some(_), None) => {
                    return Err(SealError::Chain(format!(
                        "candidate {index:04}: previous_seal_digest without predecessor digest"
                    )));
                }
            }
        }

        previous_digest = Some(envelope.envelope_digest.clone());
        last_envelope = Some(envelope);
    }

    let last = last_envelope.expect("non-empty indices");
    let final_json = run_dir.join("evidence").join("final.json");
    let final_seal = run_dir.join("evidence").join("final.seal.json");
    if !final_json.exists() || !final_seal.exists() {
        return Err(SealError::Chain(
            "missing evidence/final.json or evidence/final.seal.json".into(),
        ));
    }
    verify_seal(&final_json, &final_seal)?;

    let final_raw = fs::read_to_string(&final_seal).map_err(|e| SealError::Io(e.to_string()))?;
    let final_env: SealEnvelope =
        serde_json::from_str(&final_raw).map_err(|e| SealError::Json(e.to_string()))?;

    if final_env.envelope_digest != last.envelope_digest {
        return Err(SealError::Chain(format!(
            "final.seal envelope_digest does not match last candidate \
             (final={}, last={})",
            final_env.envelope_digest, last.envelope_digest
        )));
    }

    // Ensure the last candidate's seal file still matches final (path check).
    let last_dir = candidates_root.join(format!("{:04}", indices[indices.len() - 1]));
    let last_seal_path = last_dir.join(BUNDLE_SEAL);
    if !last_seal_path.exists() {
        return Err(SealError::Chain(
            "last candidate seal missing after verify".into(),
        ));
    }

    Ok(VerifyChainReport {
        candidate_count: indices.len() as u32,
        last_envelope_digest: last.envelope_digest,
        last_acceptance_digest: last.acceptance_digest,
    })
}

fn list_candidate_indices(candidates_root: &Path) -> Result<Vec<u32>, SealError> {
    let mut indices = Vec::new();
    let entries = fs::read_dir(candidates_root).map_err(|e| SealError::Io(e.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|e| SealError::Io(e.to_string()))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.len() != 4 || !name.chars().all(|c| c.is_ascii_digit()) {
            return Err(SealError::Chain(format!(
                "unexpected entry under candidates/: {name}"
            )));
        }
        let index: u32 = name
            .parse()
            .map_err(|_| SealError::Chain(format!("bad candidate index: {name}")))?;
        indices.push(index);
    }
    indices.sort_unstable();
    Ok(indices)
}

/// Resolve a run directory path for tests / helpers.
#[must_use]
pub fn candidate_path(run_dir: &Path, index: u32) -> PathBuf {
    run_dir.join("candidates").join(format!("{index:04}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::bundle::{materialize_bundle_files, BUNDLE_SOURCE};
    use crate::evidence::report::sha256_digest_prefixed;
    use crate::evidence::seal::{
        write_final_sealed_evidence, write_sealed_evidence, GeneratorMeta, SealBuildInput,
    };
    use crate::evidence::{EvidenceAggregator, EvidenceExpect, EvidenceStatus};
    use crate::semasm::capabilities::CapabilityMatch;
    use crate::semasm::doctor::{DoctorReport, DoctorStatus, SemasmVersion};
    use crate::semasm::verify::VerifyReport;
    use crate::task::load_locked_task;

    fn make_report(
        run_id: &str,
        source: &[u8],
        contract: &[u8],
        report_json: &str,
    ) -> (
        crate::evidence::EvidenceReport,
        EvidenceExpect,
        Vec<u8>,
        PathBuf,
    ) {
        let task_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/tasks/sum_i64.vaa.toml");
        let task = load_locked_task(&task_path).unwrap();
        let task_bytes = fs::read(&task_path).unwrap();
        let expect = EvidenceExpect::new(
            task.task().target.clone(),
            sha256_digest_prefixed(source),
            sha256_digest_prefixed(contract),
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
            Some(run_id.to_owned()),
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
        (report, expect, task_bytes, task_path)
    }

    fn write_candidate(
        run_root: &Path,
        index: u32,
        prev: Option<String>,
        source: &[u8],
        contract: &[u8],
        report_json: &str,
    ) -> SealEnvelope {
        let (report, expect, task_bytes, _) =
            make_report("run-chain", source, contract, report_json);
        let cand = candidate_path(run_root, index);
        fs::create_dir_all(&cand).unwrap();
        materialize_bundle_files(&cand, &task_bytes, contract, Some(report_json)).unwrap();
        fs::write(cand.join(BUNDLE_SOURCE), source).unwrap();
        write_sealed_evidence(
            &cand,
            &report,
            &expect,
            SealBuildInput {
                candidate_index: index,
                previous_seal_digest: prev,
                generator: GeneratorMeta::ingest("unit"),
            },
        )
        .unwrap()
    }

    #[test]
    fn verify_chain_accepts_linked_candidates() {
        let run_root = std::env::temp_dir().join(format!(
            "vaa_chain_ok_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(run_root.join("candidates")).unwrap();
        fs::create_dir_all(run_root.join("evidence")).unwrap();

        let contract = b"contract";
        let report_json = r#"{"status":"execution_denied"}"#;
        let e0 = write_candidate(&run_root, 0, None, b"cand0", contract, report_json);
        let e1 = write_candidate(
            &run_root,
            1,
            Some(e0.envelope_digest.clone()),
            b"cand1",
            contract,
            report_json,
        );

        let (report, _, _, _) = make_report("run-chain", b"cand1", contract, report_json);
        write_final_sealed_evidence(&run_root.join("evidence"), &report, &e1).unwrap();

        let summary = verify_chain(&run_root).expect("chain ok");
        assert_eq!(summary.candidate_count, 2);
        assert_eq!(summary.last_envelope_digest, e1.envelope_digest);

        let _ = fs::remove_dir_all(&run_root);
    }

    #[test]
    fn verify_chain_rejects_deleted_predecessor() {
        let run_root = std::env::temp_dir().join(format!(
            "vaa_chain_gap_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(run_root.join("candidates")).unwrap();
        fs::create_dir_all(run_root.join("evidence")).unwrap();

        let contract = b"contract";
        let report_json = r#"{"status":"execution_denied"}"#;
        let e0 = write_candidate(&run_root, 0, None, b"cand0", contract, report_json);
        let e1 = write_candidate(
            &run_root,
            1,
            Some(e0.envelope_digest.clone()),
            b"cand1",
            contract,
            report_json,
        );
        let (report, _, _, _) = make_report("run-chain", b"cand1", contract, report_json);
        write_final_sealed_evidence(&run_root.join("evidence"), &report, &e1).unwrap();

        fs::remove_dir_all(candidate_path(&run_root, 0)).unwrap();

        let err = verify_chain(&run_root).expect_err("gap");
        assert!(matches!(err, SealError::Chain(_)));

        let _ = fs::remove_dir_all(&run_root);
    }
}
