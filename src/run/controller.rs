//! End-to-end `vaa run` controller: fixture generate → verify → repair → seal.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::candidate::CandidateProtocol;
use crate::evidence::{EvidenceReport, GeneratorMeta};
use crate::model::{FixtureModelAdapter, ModelAdapter};
use crate::orchestrate::{MachineState, Orchestrator};
use crate::run::event::{EventKind, EventLog};
use crate::run::verify_seal::{
    doctor_and_capabilities, verify_candidate_and_seal, VerifySealInput,
};
use crate::run::{RunDir, RunId};
use crate::task::load_locked_task;
use crate::EvidenceStatus;

/// Result of a fixture-driven controller run.
#[derive(Debug)]
pub struct RunOutcome {
    pub evidence: EvidenceReport,
    pub transitions: usize,
    pub candidates_accepted: u32,
    pub run_root: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error(transparent)]
    Task(#[from] crate::task::TaskError),
    #[error("run directory: {0}")]
    RunDir(String),
    #[error("model: {0}")]
    Model(String),
    #[error("candidate rejected: {0}")]
    Candidate(String),
    #[error("orchestrator: {0}")]
    Orchestrator(String),
    #[error("io: {0}")]
    Io(String),
    #[error("semasm unavailable")]
    SemasmUnavailable,
    #[error("verify/seal: {0}")]
    VerifySeal(String),
    #[error("budget exhausted: {0}")]
    BudgetExhausted(String),
    #[error("event log: {0}")]
    EventLog(String),
}

/// Options for [`run_fixture_loop`].
pub struct RunConfig<'a> {
    pub task_path: &'a Path,
    pub contract_path: &'a Path,
    pub run_base: &'a Path,
    /// Queued sources in order (wrong, then repair, …).
    pub fixture_sources: Vec<String>,
    pub max_attempts: u32,
    /// Forward SemASM `--allow-execution` (default false).
    pub allow_execution: bool,
}

/// Drive orchestrator + fixture model + SemASM verify until finished or exhausted.
pub fn run_fixture_loop(config: &RunConfig<'_>) -> Result<RunOutcome, RunError> {
    let locked = load_locked_task(config.task_path)?;
    let target = locked.task().target.clone();
    let task_id = locked.task().task_id.clone();
    let budgets = locked.task().budgets.clone();
    let wall_deadline = Instant::now() + std::time::Duration::from_secs(budgets.max_wall_time_seconds);

    let run_id = RunId::generate();
    let run_dir =
        RunDir::create(config.run_base, &run_id).map_err(|e| RunError::RunDir(e.to_string()))?;

    let mut events = EventLog::new(run_dir.event_log_path().to_path_buf());
    events
        .record(EventKind::RunStarted {
            task_id: task_id.clone(),
            task_digest: locked.digest().prefixed(),
        })
        .map_err(|e| RunError::EventLog(e.to_string()))?;

    let mut adapter = FixtureModelAdapter::new("fixture");
    let key = format!("{task_id}::{target}");
    for source in &config.fixture_sources {
        adapter.add_response(&key, source);
    }

    let mut orch = Orchestrator::new();
    transit(&mut orch, MachineState::TaskLoaded, "task locked")?;
    transit(&mut orch, MachineState::TargetIdentified, &target)?;

    let (doctor, cm) = doctor_and_capabilities(&locked);
    if doctor.binary_path.is_none() {
        let _ = events.record(EventKind::Error {
            message: "semasm unavailable".into(),
        });
        return Err(RunError::SemasmUnavailable);
    }
    let _ = events.record(EventKind::SemasmCheck {
        version: doctor
            .version
            .as_ref()
            .map_or_else(|| "unknown".into(), |v| v.version.clone()),
        compatible: cm.compatible,
    });

    let max_attempts = config.max_attempts.min(budgets.max_candidates);
    let mut protocol = CandidateProtocol::with_max(&target, max_attempts);
    let mut last_evidence: Option<EvidenceReport> = None;
    let mut previous_seal_digest: Option<String> = None;
    let mut accepted = 0u32;
    let mut need_candidate = true;
    let mut no_progress = 0u32;
    let mut last_progress_status: Option<EvidenceStatus> = None;

    while need_candidate && !protocol.is_exhausted() && adapter.pending_count(&key) > 0 {
        if let Some(reason) = budget_exhausted(
            Instant::now(),
            wall_deadline,
            accepted,
            budgets.max_candidates,
            0,
            u32::MAX,
        ) {
            let _ = events.record(EventKind::Error {
                message: reason.to_owned(),
            });
            return Err(RunError::BudgetExhausted(reason.to_owned()));
        }

        transit(&mut orch, MachineState::CandidateSubmitted, "generate")?;

        let response = adapter
            .generate("", &task_id, &target)
            .map_err(|e| RunError::Model(e.to_string()))?;

        let _ = events.record(EventKind::CandidateSubmitted {
            index: accepted,
            source_path: format!("fixture:{accepted}"),
        });

        let _ = events.record(EventKind::VerificationStarted);

        let outcome = verify_candidate_and_seal(VerifySealInput {
            locked: &locked,
            task_path: config.task_path,
            contract_path: config.contract_path,
            source_bytes: response.source.as_bytes(),
            run_dir: &run_dir,
            run_id: run_id.to_string(),
            protocol: &mut protocol,
            candidate_index: accepted,
            previous_seal_digest: previous_seal_digest.clone(),
            generator: GeneratorMeta::fixture("fixture", Some(response.generation_id.clone())),
            doctor: doctor.clone(),
            capability_match: cm.clone(),
            allow_execution: config.allow_execution,
        })
        .map_err(|e| match e {
            crate::run::verify_seal::VerifySealError::SemasmUnavailable => {
                let _ = events.record(EventKind::Error {
                    message: "semasm unavailable".into(),
                });
                RunError::SemasmUnavailable
            }
            crate::run::verify_seal::VerifySealError::Candidate(msg) => {
                let _ = orch.transit(MachineState::CandidateRejected, "rejected");
                let _ = events.record(EventKind::CandidateRejected {
                    index: accepted,
                    reason: msg.clone(),
                });
                RunError::Candidate(msg)
            }
            other => {
                let _ = events.record(EventKind::Error {
                    message: other.to_string(),
                });
                RunError::VerifySeal(other.to_string())
            }
        })?;

        previous_seal_digest = Some(outcome.seal.envelope_digest.clone());

        let _ = events.record(EventKind::CandidateAccepted {
            index: outcome.candidate_index,
        });
        let _ = events.record(EventKind::VerificationCompleted {
            outcome: format!("{:?}", outcome.evidence.final_status),
        });

        transit(
            &mut orch,
            MachineState::CandidateAccepted,
            &outcome.source_digest,
        )?;
        transit(
            &mut orch,
            MachineState::BuildInProgress,
            "delegated_to_semasm",
        )?;
        transit(
            &mut orch,
            MachineState::BuildCompleted,
            "delegated_to_semasm",
        )?;
        transit(
            &mut orch,
            MachineState::VerificationInProgress,
            "semasm agent verify",
        )?;

        accepted += 1;
        transit(
            &mut orch,
            MachineState::VerificationCompleted,
            &format!("{:?}", outcome.evidence.final_status),
        )?;

        let status = outcome.evidence.final_status;
        // Progress = status improved toward Verified, or first candidate.
        let progressed = match (last_progress_status, status) {
            (None, _) => true,
            (Some(prev), cur) if prev != EvidenceStatus::Verified && cur == EvidenceStatus::Verified => {
                true
            }
            (Some(prev), cur) if prev != cur => true,
            _ => false,
        };
        if progressed {
            no_progress = 0;
        } else {
            no_progress = no_progress.saturating_add(1);
        }
        last_progress_status = Some(status);
        if no_progress > budgets.max_no_progress_iterations {
            let _ = events.record(EventKind::Error {
                message: "max_no_progress_iterations exceeded".into(),
            });
            return Err(RunError::BudgetExhausted(
                "max_no_progress_iterations exceeded".into(),
            ));
        }

        last_evidence = Some(outcome.evidence);

        match status {
            EvidenceStatus::Incomplete | EvidenceStatus::Violated | EvidenceStatus::Failed
                if adapter.pending_count(&key) > 0 && !protocol.is_exhausted() =>
            {
                need_candidate = true;
            }
            _ => {
                need_candidate = false;
            }
        }
    }

    let evidence = last_evidence.ok_or_else(|| RunError::Io("no candidate verified".into()))?;

    transit(
        &mut orch,
        MachineState::RunFinished,
        &format!("{:?}", evidence.final_status),
    )?;

    let _ = events.record(EventKind::RunFinished {
        outcome: format!("{:?}", evidence.final_status),
        candidate_count: accepted,
    });

    Ok(RunOutcome {
        evidence,
        transitions: orch.transition_count(),
        candidates_accepted: accepted,
        run_root: run_dir.root().to_path_buf(),
    })
}

fn transit(orch: &mut Orchestrator, to: MachineState, reason: &str) -> Result<(), RunError> {
    orch.transit(to, reason)
        .map_err(|e| RunError::Orchestrator(e.to_string()))
}

/// Fail-closed budget gate (B2). Returns a stable reason string when exhausted.
fn budget_exhausted(
    now: Instant,
    wall_deadline: Instant,
    accepted: u32,
    max_candidates: u32,
    no_progress: u32,
    max_no_progress: u32,
) -> Option<&'static str> {
    if now > wall_deadline {
        return Some("max_wall_time_seconds exceeded");
    }
    if accepted >= max_candidates {
        return Some("max_candidates exceeded");
    }
    if no_progress > max_no_progress {
        return Some("max_no_progress_iterations exceeded");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn repair_loop_state_edges_are_legal() {
        let mut orch = Orchestrator::new();
        transit(&mut orch, MachineState::TaskLoaded, "t").unwrap();
        transit(&mut orch, MachineState::TargetIdentified, "x").unwrap();
        for i in 0..2 {
            transit(
                &mut orch,
                MachineState::CandidateSubmitted,
                &format!("c{i}"),
            )
            .unwrap();
            transit(&mut orch, MachineState::CandidateAccepted, "a").unwrap();
            transit(&mut orch, MachineState::BuildInProgress, "b").unwrap();
            transit(&mut orch, MachineState::BuildCompleted, "b").unwrap();
            transit(&mut orch, MachineState::VerificationInProgress, "v").unwrap();
            transit(&mut orch, MachineState::VerificationCompleted, "v").unwrap();
        }
        transit(&mut orch, MachineState::RunFinished, "done").unwrap();
        assert!(orch.transition_count() >= 14);
    }

    #[test]
    fn budget_exhausted_on_wall_candidates_and_no_progress() {
        let start = Instant::now();
        assert_eq!(
            budget_exhausted(start + Duration::from_secs(2), start + Duration::from_secs(1), 0, 4, 0, 2),
            Some("max_wall_time_seconds exceeded")
        );
        assert_eq!(
            budget_exhausted(start, start + Duration::from_secs(60), 2, 2, 0, 2),
            Some("max_candidates exceeded")
        );
        assert_eq!(
            budget_exhausted(start, start + Duration::from_secs(60), 0, 4, 3, 2),
            Some("max_no_progress_iterations exceeded")
        );
        assert!(budget_exhausted(start, start + Duration::from_secs(60), 1, 4, 2, 2).is_none());
    }

    #[test]
    fn run_records_events_jsonl() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let task = manifest.join("fixtures/run/count_byte/count_byte.vaa.toml");
        let contract = manifest.join("fixtures/run/count_byte/count_byte.sem.toml");
        let wrong = manifest.join("fixtures/run/count_byte/01_wrong.asm");
        let repaired = manifest.join("fixtures/run/count_byte/02_repaired.asm");
        let tmp = std::env::temp_dir().join(format!(
            "vaa_ctrl_events_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let config = RunConfig {
            task_path: &task,
            contract_path: &contract,
            run_base: &tmp,
            fixture_sources: vec![
                std::fs::read_to_string(&wrong).unwrap(),
                std::fs::read_to_string(&repaired).unwrap(),
            ],
            max_attempts: 1,
            allow_execution: false,
        };
        let result = run_fixture_loop(&config);
        let run_root = std::fs::read_dir(&tmp)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let events_path = run_root.join("events.jsonl");
        let body = std::fs::read_to_string(&events_path).unwrap_or_default();
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(
            !body.is_empty(),
            "events.jsonl must be non-empty after run attempt; result={result:?}"
        );
        assert!(body.contains("run_started") || body.contains("task_id"));
    }
}
