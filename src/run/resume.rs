//! Restart resume from an existing sealed run directory (E1).

use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::run::event::{EventKind, EventLog, EventLogError};
use crate::run::run_dir::{RunDir, RunDirError};

/// Where to continue after a crash / restart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeCursor {
    /// Next exclusive candidate index (`create_candidate_dir` target).
    pub next_candidate_index: u32,
    /// `envelope_digest` of the last sealed candidate, if any.
    pub previous_seal_digest: Option<String>,
    /// Approximate event count already on disk (for append-only EventLog).
    pub event_count: usize,
}

/// Scan sealed candidates under `run_dir` for the append cursor.
pub fn scan_resume_cursor(run_dir: &RunDir) -> Result<ResumeCursor, RunDirError> {
    let mut next = 0u32;
    let mut previous_seal_digest = None;
    while next <= 9999 {
        let dir = run_dir.candidate_dir(next)?;
        let seal_path = dir.join("evidence.seal.json");
        if !seal_path.is_file() {
            if dir.exists() {
                return Err(RunDirError::Open {
                    path: dir,
                    reason: format!(
                        "candidate {next:04} exists without evidence.seal.json (incomplete)"
                    ),
                });
            }
            break;
        }
        let raw = fs::read_to_string(&seal_path).map_err(|source| RunDirError::Write {
            path: seal_path.clone(),
            source,
        })?;
        let value: Value = serde_json::from_str(&raw).map_err(|e| RunDirError::Open {
            path: seal_path.clone(),
            reason: format!("invalid seal JSON: {e}"),
        })?;
        let digest = value
            .get("envelope_digest")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RunDirError::Open {
                path: seal_path,
                reason: "missing envelope_digest".into(),
            })?;
        previous_seal_digest = Some(digest.to_owned());
        next = next.saturating_add(1);
    }
    let event_count = count_event_lines(run_dir.event_log_path());
    Ok(ResumeCursor {
        next_candidate_index: next,
        previous_seal_digest,
        event_count,
    })
}

fn count_event_lines(path: &Path) -> usize {
    fs::read_to_string(path).map_or(0, |body| {
        body.lines().filter(|l| !l.trim().is_empty()).count()
    })
}

/// Append-aware event log helpers (E1).
impl EventLog {
    /// Open an existing log and seed counters from on-disk lines.
    #[must_use]
    pub fn open_existing(path: std::path::PathBuf) -> Self {
        let count = count_event_lines(&path);
        let bytes = fs::metadata(&path).map_or(0, |m| m.len());
        Self::new(path).with_seed(count, bytes)
    }

    /// Record a resume marker for operators / auditors.
    pub fn record_resume(&mut self, next_index: u32) -> Result<(), EventLogError> {
        self.record(EventKind::Info {
            message: format!("resume from candidate index {next_index:04}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::bundle::materialize_bundle_files;
    use crate::evidence::report::sha256_digest_prefixed;
    use crate::evidence::seal::{
        write_final_sealed_evidence, write_sealed_evidence, GeneratorMeta, SealBuildInput,
    };
    use crate::evidence::seal_log::{append_seal_log, SealLogEntry};
    use crate::evidence::{EvidenceAggregator, EvidenceExpect, EvidenceStatus};
    use crate::run::RunId;
    use crate::semasm::capabilities::CapabilityMatch;
    use crate::semasm::doctor::{DoctorReport, DoctorStatus, SemasmVersion};
    use crate::semasm::verify::VerifyReport;
    use crate::task::load_locked_task;
    use std::path::PathBuf;

    fn temp_base() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vaa_resume_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn seal_one(
        run_dir: &RunDir,
        run_id: &str,
        index: u32,
        previous: Option<String>,
        source: &[u8],
    ) -> String {
        let task_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/tasks/sum_i64.vaa.toml");
        let contract = b"[contract]\nname=\"t\"\n";
        let report_json = r#"{"schema_version":"0.4","status":"execution_denied","target":"x86_64-pc-windows-msvc"}"#;
        let locked = load_locked_task(&task_path).unwrap();
        let task_bytes = fs::read(&task_path).unwrap();
        let mut expect = EvidenceExpect::new(
            locked.task().target.clone(),
            sha256_digest_prefixed(source),
            sha256_digest_prefixed(contract),
        );
        if locked.task().verification.require_object_inspection {
            expect.object_inspection = Some(crate::evidence::ObjectInspectionOutcome {
                error: None,
                has_wxorx: false,
                has_executable_stack: false,
                format: "test".into(),
            });
        }
        let verify = VerifyReport {
            outcome: EvidenceStatus::Incomplete,
            raw_status: "execution_denied".into(),
            schema_version: Some("0.4".into()),
            diagnostics: vec![],
            target: Some(locked.task().target.clone()),
            source_digest: Some(expect.expected_source_digest.clone()),
            contract_digest: Some(expect.expected_contract_digest.clone()),
            tool_version: Some("semasm 0.1.0".into()),
            raw_json: report_json.into(),
        };
        let evidence = EvidenceAggregator::build(
            &locked,
            Some(run_id.into()),
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
        let cand = run_dir.create_candidate_dir(index).unwrap();
        materialize_bundle_files(&cand, &task_bytes, contract, Some(report_json)).unwrap();
        fs::write(cand.join("candidate.asm"), source).unwrap();
        let seal = write_sealed_evidence(
            &cand,
            &evidence,
            &expect,
            SealBuildInput {
                candidate_index: index,
                previous_seal_digest: previous,
                generator: GeneratorMeta::fixture("resume-test", None),
            },
        )
        .unwrap();
        let entry =
            SealLogEntry::from_seal(run_id, &locked.task().task_id, evidence.final_status, &seal);
        append_seal_log(&run_dir.paths().evidence_dir, &entry).unwrap();
        write_final_sealed_evidence(&run_dir.paths().evidence_dir, &evidence, &seal).unwrap();
        seal.envelope_digest
    }

    #[test]
    fn resume_rejects_sealed_rewrite_and_chains_next() {
        let base = temp_base();
        let id = RunId::generate();
        let run_dir = RunDir::create(&base, &id).unwrap();
        let run_id = id.to_string();

        let mut events = EventLog::new(run_dir.event_log_path().to_path_buf());
        events
            .record(EventKind::RunStarted {
                task_id: "sum-i64-win64-v1".into(),
                task_digest: "sha256:dead".into(),
            })
            .unwrap();

        let d0 = seal_one(&run_dir, &run_id, 0, None, b"xor eax, eax\nret\n");
        let root = run_dir.root().to_path_buf();
        drop(run_dir);

        let reopened = RunDir::open(&root).unwrap();
        let cursor = reopened.resume_cursor().unwrap();
        assert_eq!(cursor.next_candidate_index, 1);
        assert_eq!(cursor.previous_seal_digest.as_deref(), Some(d0.as_str()));
        assert!(cursor.event_count >= 1);

        let err = reopened.create_candidate_dir(0).unwrap_err();
        assert!(matches!(
            err,
            RunDirError::CandidateAlreadySealed { index: 0, .. }
        ));

        let mut events = EventLog::open_existing(reopened.event_log_path().to_path_buf());
        events.record_resume(cursor.next_candidate_index).unwrap();

        let d1 = seal_one(
            &reopened,
            &run_id,
            cursor.next_candidate_index,
            cursor.previous_seal_digest.clone(),
            b"xor eax, eax\ninc eax\nret\n",
        );
        assert_ne!(d0, d1);

        let chain = crate::evidence::verify_chain(&root).unwrap();
        assert_eq!(chain.candidate_count, 2);

        let seal1: Value = serde_json::from_str(
            &fs::read_to_string(root.join("candidates/0001/evidence.seal.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            seal1["provenance"]["previous_seal_digest"].as_str(),
            Some(d0.as_str())
        );

        let _ = fs::remove_dir_all(&base);
    }
}
