//! Full-run seal chain verification (`candidates/0000` … final).

use std::fs;
use std::path::{Path, PathBuf};

use super::bundle::{verify_bundle, BUNDLE_SEAL};
use super::seal::{verify_seal, SealEnvelope, SealError};

/// Chain-wide identity taken from candidate `0000` (must hold for every link).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainIdentity {
    pub task_id: String,
    pub task_digest: String,
    pub run_id: Option<String>,
    pub target: String,
    pub contract_digest: String,
}

impl ChainIdentity {
    #[must_use]
    pub fn from_envelope(envelope: &SealEnvelope) -> Self {
        Self {
            task_id: envelope.provenance.task_id.clone(),
            task_digest: envelope.acceptance.task_digest.clone(),
            run_id: envelope.provenance.run_id.clone(),
            target: envelope.acceptance.target.clone(),
            contract_digest: envelope.acceptance.contract_digest.clone(),
        }
    }

    /// Fields that must stay constant across the repair / ingest chain.
    fn check_against(&self, index: u32, envelope: &SealEnvelope) -> Result<(), SealError> {
        let tag = format!("candidate {index:04}");
        if envelope.provenance.task_id != self.task_id {
            return Err(SealError::Chain(format!(
                "{tag}: task_id differs from chain identity"
            )));
        }
        if envelope.provenance.run_id != self.run_id {
            return Err(SealError::Chain(format!(
                "{tag}: run_id differs from chain identity"
            )));
        }
        if envelope.acceptance.task_digest != self.task_digest {
            return Err(SealError::Chain(format!(
                "{tag}: task_digest differs from chain identity"
            )));
        }
        if envelope.acceptance.target != self.target {
            return Err(SealError::Chain(format!(
                "{tag}: target differs from chain identity"
            )));
        }
        if envelope.acceptance.contract_digest != self.contract_digest {
            return Err(SealError::Chain(format!(
                "{tag}: contract_digest differs from chain identity"
            )));
        }
        Ok(())
    }
}

/// Summary of a successful [`verify_chain`] pass.
#[derive(Debug, Clone)]
pub struct VerifyChainReport {
    pub candidate_count: u32,
    pub last_envelope_digest: String,
    pub last_acceptance_digest: String,
    pub identity: ChainIdentity,
}

/// Verify every candidate bundle in a run directory as a contiguous hash chain,
/// then confirm `evidence/final.seal.json` matches the last candidate.
///
/// Invariants:
/// - `candidates/0000` … `candidates/NNNN` exist contiguously (no gaps)
/// - candidate 0 has `previous_seal_digest == None`
/// - candidate `i` has `previous_seal_digest == envelope_digest` of `i-1`
/// - `provenance.candidate_index` matches the directory index
/// - all candidates share the same chain identity (task/run/target/contract)
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
    let mut identity: Option<ChainIdentity> = None;
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
            identity = Some(ChainIdentity::from_envelope(&envelope));
        } else {
            let baseline = identity.as_ref().expect("identity set at candidate 0");
            baseline.check_against(*index, &envelope)?;

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
    let identity = identity.expect("identity set");

    // Final must agree with last candidate and the same chain identity.
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
    identity.check_against(last.provenance.candidate_index, &final_env)?;

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
        identity,
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
    use crate::evidence::bundle::{materialize_bundle_files, BUNDLE_SEAL, BUNDLE_SOURCE};
    use crate::evidence::report::sha256_digest_prefixed;
    use crate::evidence::seal::{
        acceptance_digest_of, envelope_digest_of, write_final_sealed_evidence,
        write_sealed_evidence, GeneratorMeta, SealBuildInput, SealEnvelope,
    };
    use crate::evidence::{EvidenceAggregator, EvidenceExpect, EvidenceStatus};
    use crate::semasm::capabilities::CapabilityMatch;
    use crate::semasm::doctor::{DoctorReport, DoctorStatus, SemasmVersion};
    use crate::semasm::verify::VerifyReport;
    use crate::task::load_locked_task;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_run() -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "vaa_chain_{}_{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("candidates")).unwrap();
        fs::create_dir_all(dir.join("evidence")).unwrap();
        dir
    }

    struct CandOpts<'a> {
        run_id: &'a str,
        task_path: PathBuf,
        source: &'a [u8],
        contract: &'a [u8],
        report_json: &'a str,
        candidate_index: u32,
        previous_seal_digest: Option<String>,
    }

    fn make_report(
        opts: &CandOpts<'_>,
    ) -> (crate::evidence::EvidenceReport, EvidenceExpect, Vec<u8>) {
        let task = load_locked_task(&opts.task_path).unwrap();
        let task_bytes = fs::read(&opts.task_path).unwrap();
        let expect = EvidenceExpect::new(
            task.task().target.clone(),
            sha256_digest_prefixed(opts.source),
            sha256_digest_prefixed(opts.contract),
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
            raw_json: opts.report_json.to_owned(),
        };
        let report = EvidenceAggregator::build(
            &task,
            Some(opts.run_id.to_owned()),
            Some(verify),
            Some(DoctorReport {
                status: DoctorStatus::Available,
                binary_path: Some(PathBuf::from("semasm")),
                version: Some(SemasmVersion {
                    version: "0.1.0".to_owned(),
                    schema_version: "0.1".to_owned(),
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
        (report, expect, task_bytes)
    }

    fn default_task() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/tasks/sum_i64.vaa.toml")
    }

    /// Same `task_id` + `target` as default, but different locked content → different digest.
    fn digest_mut_task() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/tasks/sum_i64_budget_mut.vaa.toml")
    }

    /// Same `target` as default, different `task_id` (and therefore digest).
    fn alt_id_task() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/tasks/sum_i64_alt_id.vaa.toml")
    }

    fn write_candidate(run_root: &Path, dir_index: u32, opts: CandOpts<'_>) -> SealEnvelope {
        let (report, expect, task_bytes) = make_report(&opts);
        let cand = candidate_path(run_root, dir_index);
        fs::create_dir_all(&cand).unwrap();
        materialize_bundle_files(&cand, &task_bytes, opts.contract, Some(opts.report_json))
            .unwrap();
        fs::write(cand.join(BUNDLE_SOURCE), opts.source).unwrap();
        write_sealed_evidence(
            &cand,
            &report,
            &expect,
            SealBuildInput {
                candidate_index: opts.candidate_index,
                previous_seal_digest: opts.previous_seal_digest,
                generator: GeneratorMeta::ingest("unit"),
            },
        )
        .unwrap()
    }

    fn linked_pair(run_root: &Path) -> (SealEnvelope, SealEnvelope) {
        let contract = b"contract";
        let report_json = r#"{"status":"execution_denied"}"#;
        let e0 = write_candidate(
            run_root,
            0,
            CandOpts {
                run_id: "run-chain",
                task_path: default_task(),
                source: b"cand0",
                contract,
                report_json,
                candidate_index: 0,
                previous_seal_digest: None,
            },
        );
        let e1 = write_candidate(
            run_root,
            1,
            CandOpts {
                run_id: "run-chain",
                task_path: default_task(),
                source: b"cand1",
                contract,
                report_json,
                candidate_index: 1,
                previous_seal_digest: Some(e0.envelope_digest.clone()),
            },
        );
        let (report, _, _) = make_report(&CandOpts {
            run_id: "run-chain",
            task_path: default_task(),
            source: b"cand1",
            contract,
            report_json,
            candidate_index: 1,
            previous_seal_digest: None,
        });
        write_final_sealed_evidence(&run_root.join("evidence"), &report, &e1).unwrap();
        (e0, e1)
    }

    fn assert_chain_err(err: SealError, needle: &str) {
        match err {
            SealError::Chain(msg) => assert!(
                msg.contains(needle),
                "expected chain error containing `{needle}`, got `{msg}`"
            ),
            other => panic!("expected Chain error, got {other:?}"),
        }
    }

    #[test]
    fn verify_chain_accepts_linked_candidates() {
        let run_root = temp_run();
        let (_e0, e1) = linked_pair(&run_root);
        let summary = verify_chain(&run_root).expect("chain ok");
        assert_eq!(summary.candidate_count, 2);
        assert_eq!(summary.last_envelope_digest, e1.envelope_digest);
        assert_eq!(summary.identity.run_id.as_deref(), Some("run-chain"));
        let _ = fs::remove_dir_all(&run_root);
    }

    #[test]
    fn verify_chain_rejects_deleted_predecessor() {
        let run_root = temp_run();
        let _ = linked_pair(&run_root);
        fs::remove_dir_all(candidate_path(&run_root, 0)).unwrap();
        let err = verify_chain(&run_root).expect_err("gap");
        assert!(matches!(err, SealError::Chain(_)));
        let _ = fs::remove_dir_all(&run_root);
    }

    #[test]
    fn rejects_wrong_previous_digest() {
        let run_root = temp_run();
        let contract = b"contract";
        let report_json = r#"{"status":"execution_denied"}"#;
        let e0 = write_candidate(
            &run_root,
            0,
            CandOpts {
                run_id: "run-chain",
                task_path: default_task(),
                source: b"cand0",
                contract,
                report_json,
                candidate_index: 0,
                previous_seal_digest: None,
            },
        );
        let e1 = write_candidate(
            &run_root,
            1,
            CandOpts {
                run_id: "run-chain",
                task_path: default_task(),
                source: b"cand1",
                contract,
                report_json,
                candidate_index: 1,
                previous_seal_digest: Some(
                    "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                        .into(),
                ),
            },
        );
        let (report, _, _) = make_report(&CandOpts {
            run_id: "run-chain",
            task_path: default_task(),
            source: b"cand1",
            contract,
            report_json,
            candidate_index: 1,
            previous_seal_digest: None,
        });
        write_final_sealed_evidence(&run_root.join("evidence"), &report, &e1).unwrap();
        let _ = e0;
        let err = verify_chain(&run_root).expect_err("bad prev");
        assert_chain_err(err, "previous_seal_digest mismatch");
        let _ = fs::remove_dir_all(&run_root);
    }

    #[test]
    fn rejects_non_null_genesis_link() {
        let run_root = temp_run();
        let contract = b"contract";
        let report_json = r#"{"status":"execution_denied"}"#;
        let e0 = write_candidate(
            &run_root,
            0,
            CandOpts {
                run_id: "run-chain",
                task_path: default_task(),
                source: b"cand0",
                contract,
                report_json,
                candidate_index: 0,
                previous_seal_digest: Some(
                    "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                        .into(),
                ),
            },
        );
        let (report, _, _) = make_report(&CandOpts {
            run_id: "run-chain",
            task_path: default_task(),
            source: b"cand0",
            contract,
            report_json,
            candidate_index: 0,
            previous_seal_digest: None,
        });
        write_final_sealed_evidence(&run_root.join("evidence"), &report, &e0).unwrap();
        let err = verify_chain(&run_root).expect_err("genesis");
        assert_chain_err(err, "previous_seal_digest = null");
        let _ = fs::remove_dir_all(&run_root);
    }

    #[test]
    fn rejects_candidate_index_mismatch() {
        let run_root = temp_run();
        let contract = b"contract";
        let report_json = r#"{"status":"execution_denied"}"#;
        let e0 = write_candidate(
            &run_root,
            0,
            CandOpts {
                run_id: "run-chain",
                task_path: default_task(),
                source: b"cand0",
                contract,
                report_json,
                candidate_index: 0,
                previous_seal_digest: None,
            },
        );
        let e1 = write_candidate(
            &run_root,
            1,
            CandOpts {
                run_id: "run-chain",
                task_path: default_task(),
                source: b"cand1",
                contract,
                report_json,
                candidate_index: 99, // dir is 0001
                previous_seal_digest: Some(e0.envelope_digest.clone()),
            },
        );
        let (report, _, _) = make_report(&CandOpts {
            run_id: "run-chain",
            task_path: default_task(),
            source: b"cand1",
            contract,
            report_json,
            candidate_index: 99,
            previous_seal_digest: None,
        });
        write_final_sealed_evidence(&run_root.join("evidence"), &report, &e1).unwrap();
        let err = verify_chain(&run_root).expect_err("index");
        assert_chain_err(err, "candidate_index mismatch");
        let _ = fs::remove_dir_all(&run_root);
    }

    #[test]
    fn rejects_final_seal_mismatch() {
        let run_root = temp_run();
        let (e0, e1) = linked_pair(&run_root);
        let _ = e1;
        // Overwrite final with candidate 0's seal (linked_pair wrote e1).
        let (report, _, _) = make_report(&CandOpts {
            run_id: "run-chain",
            task_path: default_task(),
            source: b"cand0",
            contract: b"contract",
            report_json: r#"{"status":"execution_denied"}"#,
            candidate_index: 0,
            previous_seal_digest: None,
        });
        write_final_sealed_evidence(&run_root.join("evidence"), &report, &e0).unwrap();
        let err = verify_chain(&run_root).expect_err("final");
        assert_chain_err(err, "final.seal envelope_digest");
        let _ = fs::remove_dir_all(&run_root);
    }

    #[test]
    fn rejects_run_id_change() {
        let run_root = temp_run();
        let contract = b"contract";
        let report_json = r#"{"status":"execution_denied"}"#;
        let e0 = write_candidate(
            &run_root,
            0,
            CandOpts {
                run_id: "run-a",
                task_path: default_task(),
                source: b"cand0",
                contract,
                report_json,
                candidate_index: 0,
                previous_seal_digest: None,
            },
        );
        let e1 = write_candidate(
            &run_root,
            1,
            CandOpts {
                run_id: "run-b",
                task_path: default_task(),
                source: b"cand1",
                contract,
                report_json,
                candidate_index: 1,
                previous_seal_digest: Some(e0.envelope_digest.clone()),
            },
        );
        let (report, _, _) = make_report(&CandOpts {
            run_id: "run-b",
            task_path: default_task(),
            source: b"cand1",
            contract,
            report_json,
            candidate_index: 1,
            previous_seal_digest: None,
        });
        write_final_sealed_evidence(&run_root.join("evidence"), &report, &e1).unwrap();
        let err = verify_chain(&run_root).expect_err("run_id");
        assert_chain_err(err, "run_id differs from chain identity");
        let _ = fs::remove_dir_all(&run_root);
    }

    #[test]
    fn rejects_task_digest_change() {
        let run_root = temp_run();
        let contract = b"contract";
        let report_json = r#"{"status":"execution_denied"}"#;
        let e0 = write_candidate(
            &run_root,
            0,
            CandOpts {
                run_id: "run-chain",
                task_path: default_task(),
                source: b"cand0",
                contract,
                report_json,
                candidate_index: 0,
                previous_seal_digest: None,
            },
        );
        // Same task_id + target; only locked content (budget) differs → digest only.
        let e1 = write_candidate(
            &run_root,
            1,
            CandOpts {
                run_id: "run-chain",
                task_path: digest_mut_task(),
                source: b"cand1",
                contract,
                report_json,
                candidate_index: 1,
                previous_seal_digest: Some(e0.envelope_digest.clone()),
            },
        );
        assert_eq!(e0.provenance.task_id, e1.provenance.task_id);
        assert_eq!(e0.acceptance.target, e1.acceptance.target);
        assert_ne!(e0.acceptance.task_digest, e1.acceptance.task_digest);
        let (report, _, _) = make_report(&CandOpts {
            run_id: "run-chain",
            task_path: digest_mut_task(),
            source: b"cand1",
            contract,
            report_json,
            candidate_index: 1,
            previous_seal_digest: None,
        });
        write_final_sealed_evidence(&run_root.join("evidence"), &report, &e1).unwrap();
        let err = verify_chain(&run_root).expect_err("task_digest");
        assert_chain_err(err, "task_digest differs from chain identity");
        let _ = fs::remove_dir_all(&run_root);
    }

    #[test]
    fn rejects_task_id_change() {
        let run_root = temp_run();
        let contract = b"contract";
        let report_json = r#"{"status":"execution_denied"}"#;
        let e0 = write_candidate(
            &run_root,
            0,
            CandOpts {
                run_id: "run-chain",
                task_path: default_task(),
                source: b"cand0",
                contract,
                report_json,
                candidate_index: 0,
                previous_seal_digest: None,
            },
        );
        let e1 = write_candidate(
            &run_root,
            1,
            CandOpts {
                run_id: "run-chain",
                task_path: alt_id_task(),
                source: b"cand1",
                contract,
                report_json,
                candidate_index: 1,
                previous_seal_digest: Some(e0.envelope_digest.clone()),
            },
        );
        assert_eq!(e0.acceptance.target, e1.acceptance.target);
        assert_ne!(e0.provenance.task_id, e1.provenance.task_id);
        let (report, _, _) = make_report(&CandOpts {
            run_id: "run-chain",
            task_path: alt_id_task(),
            source: b"cand1",
            contract,
            report_json,
            candidate_index: 1,
            previous_seal_digest: None,
        });
        write_final_sealed_evidence(&run_root.join("evidence"), &report, &e1).unwrap();
        let err = verify_chain(&run_root).expect_err("task_id");
        assert_chain_err(err, "task_id differs from chain identity");
        let _ = fs::remove_dir_all(&run_root);
    }

    #[test]
    fn rejects_contract_change() {
        let run_root = temp_run();
        let report_json = r#"{"status":"execution_denied"}"#;
        let e0 = write_candidate(
            &run_root,
            0,
            CandOpts {
                run_id: "run-chain",
                task_path: default_task(),
                source: b"cand0",
                contract: b"contract-a",
                report_json,
                candidate_index: 0,
                previous_seal_digest: None,
            },
        );
        let e1 = write_candidate(
            &run_root,
            1,
            CandOpts {
                run_id: "run-chain",
                task_path: default_task(),
                source: b"cand1",
                contract: b"contract-b",
                report_json,
                candidate_index: 1,
                previous_seal_digest: Some(e0.envelope_digest.clone()),
            },
        );
        let (report, _, _) = make_report(&CandOpts {
            run_id: "run-chain",
            task_path: default_task(),
            source: b"cand1",
            contract: b"contract-b",
            report_json,
            candidate_index: 1,
            previous_seal_digest: None,
        });
        write_final_sealed_evidence(&run_root.join("evidence"), &report, &e1).unwrap();
        let err = verify_chain(&run_root).expect_err("contract");
        assert_chain_err(err, "contract_digest differs from chain identity");
        let _ = fs::remove_dir_all(&run_root);
    }

    #[test]
    fn rejects_target_change() {
        // Target is bound by verify_bundle to the on-disk task file. Tampering the seal
        // target while leaving task.vaa.toml unchanged fails at bundle verify (not chain
        // identity). Isolated target drift across two *valid* bundles would also change
        // task_digest; that path is covered by rejects_task_digest_change.
        let run_root = temp_run();
        let (_e0, mut e1) = linked_pair(&run_root);

        e1.acceptance.target = "x86_64-pc-windows-msvc".into();
        e1.acceptance_digest = acceptance_digest_of(&e1.acceptance);
        e1.envelope_digest = envelope_digest_of(&e1.acceptance, &e1.provenance);

        let cand1 = candidate_path(&run_root, 1);
        let evidence_raw = fs::read_to_string(cand1.join("evidence.json")).unwrap();
        let mut report: crate::evidence::EvidenceReport =
            serde_json::from_str(&evidence_raw).unwrap();
        report.target = "x86_64-pc-windows-msvc".into();
        fs::write(
            cand1.join("evidence.json"),
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .unwrap();
        fs::write(
            cand1.join(BUNDLE_SEAL),
            serde_json::to_string_pretty(&e1).unwrap(),
        )
        .unwrap();
        write_final_sealed_evidence(&run_root.join("evidence"), &report, &e1).unwrap();

        let err = verify_chain(&run_root).expect_err("target");
        match err {
            SealError::Bundle(msg) => assert!(
                msg.contains("task.target != acceptance.target"),
                "unexpected: {msg}"
            ),
            other => panic!("expected Bundle target mismatch, got {other:?}"),
        }
        let _ = fs::remove_dir_all(&run_root);
    }
}
