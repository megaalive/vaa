//! End-to-end `vaa run` controller: fixture generate → verify → repair → seal.

use std::path::{Path, PathBuf};

use crate::candidate::CandidateProtocol;
use crate::evidence::{EvidenceReport, GeneratorMeta};
use crate::model::{FixtureModelAdapter, ModelAdapter};
use crate::orchestrate::{MachineState, Orchestrator};
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

    let run_id = RunId::generate();
    let run_dir =
        RunDir::create(config.run_base, &run_id).map_err(|e| RunError::RunDir(e.to_string()))?;

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
        return Err(RunError::SemasmUnavailable);
    }

    let mut protocol = CandidateProtocol::with_max(&target, config.max_attempts);
    let mut last_evidence: Option<EvidenceReport> = None;
    let mut previous_seal_digest: Option<String> = None;
    let mut accepted = 0u32;
    let mut need_candidate = true;

    while need_candidate && !protocol.is_exhausted() && adapter.pending_count(&key) > 0 {
        transit(&mut orch, MachineState::CandidateSubmitted, "generate")?;

        let response = adapter
            .generate("", &task_id, &target)
            .map_err(|e| RunError::Model(e.to_string()))?;

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
                RunError::SemasmUnavailable
            }
            crate::run::verify_seal::VerifySealError::Candidate(msg) => {
                let _ = orch.transit(MachineState::CandidateRejected, "rejected");
                RunError::Candidate(msg)
            }
            other => RunError::VerifySeal(other.to_string()),
        })?;

        previous_seal_digest = Some(outcome.seal.envelope_digest.clone());

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
