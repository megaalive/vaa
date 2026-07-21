//! End-to-end `vaa run` controller: fixture generate → verify → repair → evidence.

use std::path::{Path, PathBuf};

use crate::candidate::CandidateProtocol;
use crate::evidence::{sha256_digest_prefixed, EvidenceAggregator, EvidenceExpect, EvidenceReport};
use crate::model::{FixtureModelAdapter, ModelAdapter};
use crate::orchestrate::{MachineState, Orchestrator};
use crate::run::{RunDir, RunId};
use crate::semasm::{
    match_task_requirements, SemasmDoctor, SemasmVerify, TargetCapabilities, VerifyError,
};
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
}

/// Options for [`run_fixture_loop`].
pub struct RunConfig<'a> {
    pub task_path: &'a Path,
    pub contract_path: &'a Path,
    pub run_base: &'a Path,
    /// Queued sources in order (wrong, then repair, …).
    pub fixture_sources: Vec<String>,
    pub max_attempts: u32,
}

/// Drive orchestrator + fixture model + SemASM verify until finished or exhausted.
pub fn run_fixture_loop(config: &RunConfig<'_>) -> Result<RunOutcome, RunError> {
    let locked = load_locked_task(config.task_path)?;
    let target = locked.task().target.clone();
    let task_id = locked.task().task_id.clone();

    let run_id = RunId::generate();
    let run_dir =
        RunDir::create(config.run_base, &run_id).map_err(|e| RunError::RunDir(e.to_string()))?;

    let contract_bytes = std::fs::read(config.contract_path)
        .map_err(|e| RunError::Io(format!("read contract: {e}")))?;
    let contract_digest = sha256_digest_prefixed(&contract_bytes);

    let mut adapter = FixtureModelAdapter::new("fixture");
    let key = format!("{task_id}::{target}");
    for source in &config.fixture_sources {
        adapter.add_response(&key, source);
    }

    let mut orch = Orchestrator::new();
    transit(&mut orch, MachineState::TaskLoaded, "task locked")?;
    transit(&mut orch, MachineState::TargetIdentified, &target)?;

    let caps = TargetCapabilities::for_target(&target);
    let cm = match_task_requirements(locked.task(), &caps);
    let doctor = SemasmDoctor::run();
    let binary = doctor
        .binary_path
        .clone()
        .ok_or(RunError::SemasmUnavailable)?;

    let mut protocol = CandidateProtocol::with_max(&target, config.max_attempts);
    let mut last_verify = None;
    let mut last_expect = None;
    let mut accepted = 0u32;
    let mut need_candidate = true;

    while need_candidate && !protocol.is_exhausted() && adapter.pending_count(&key) > 0 {
        transit(&mut orch, MachineState::CandidateSubmitted, "generate")?;

        let response = adapter
            .generate("", &task_id, &target)
            .map_err(|e| RunError::Model(e.to_string()))?;

        let cand_dir = run_dir
            .candidate_dir(accepted)
            .map_err(|e| RunError::RunDir(e.to_string()))?;
        std::fs::create_dir_all(&cand_dir).map_err(|e| RunError::Io(e.to_string()))?;
        let source_path = cand_dir.join("candidate.asm");
        std::fs::write(&source_path, &response.source).map_err(|e| RunError::Io(e.to_string()))?;

        let outcome = protocol.submit(&response.source, &source_path, &target);
        if !outcome.accepted {
            transit(&mut orch, MachineState::CandidateRejected, "rejected")?;
            return Err(RunError::Candidate(format!("{:?}", outcome.rejection)));
        }

        accepted += 1;
        transit(&mut orch, MachineState::CandidateAccepted, &outcome.digest)?;
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

        let source_digest = sha256_digest_prefixed(response.source.as_bytes());
        let expect = EvidenceExpect::new(target.clone(), source_digest, contract_digest.clone());

        let verify = match SemasmVerify::run(&source_path, config.contract_path, &binary, &target) {
            Ok(report) => report,
            Err(VerifyError::BinaryNotFound) => return Err(RunError::SemasmUnavailable),
            Err(e) => {
                transit(
                    &mut orch,
                    MachineState::VerificationCompleted,
                    &format!("verify error: {e}"),
                )?;
                last_expect = Some(expect);
                last_verify = None;
                break;
            }
        };

        transit(
            &mut orch,
            MachineState::VerificationCompleted,
            verify.raw_status.as_str(),
        )?;

        let outcome_status = verify.outcome;
        last_verify = Some(verify);
        last_expect = Some(expect);

        match outcome_status {
            EvidenceStatus::Incomplete | EvidenceStatus::Violated | EvidenceStatus::Failed
                if adapter.pending_count(&key) > 0 && !protocol.is_exhausted() =>
            {
                // Stay at VerificationCompleted; next loop does CandidateSubmitted.
                need_candidate = true;
            }
            _ => {
                need_candidate = false;
            }
        }
    }

    let expect = last_expect.ok_or_else(|| RunError::Io("no candidate verified".into()))?;
    let evidence = EvidenceAggregator::build(
        &locked,
        Some(run_id.to_string()),
        last_verify,
        Some(doctor),
        Some(cm),
        &expect,
    );

    transit(
        &mut orch,
        MachineState::RunFinished,
        &format!("{:?}", evidence.final_status),
    )?;

    let evidence_path = run_dir.paths().evidence_dir.join("evidence.json");
    if let Ok(body) = serde_json::to_string_pretty(&evidence) {
        let _ = std::fs::write(evidence_path, body);
    }

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
