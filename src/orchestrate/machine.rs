use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MachineState {
    Idle,
    TaskLoaded,
    TargetIdentified,
    CandidateSubmitted,
    CandidateAccepted,
    CandidateRejected,
    BuildInProgress,
    BuildCompleted,
    BuildFailed,
    VerificationInProgress,
    VerificationCompleted,
    RunFinished,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: MachineState,
    pub to: MachineState,
    pub reason: String,
}

#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("invalid transition: {from:?} -> {to:?}: {reason}")]
    InvalidTransition {
        from: MachineState,
        to: MachineState,
        reason: String,
    },
    #[error("orchestrator not started")]
    NotStarted,
    #[error("candidate limit exhausted")]
    CandidateExhausted,
}

pub struct Orchestrator {
    state: MachineState,
    transitions: Vec<StateTransition>,
    allowed_edges: Vec<(MachineState, MachineState)>,
}

impl Orchestrator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: MachineState::Idle,
            transitions: Vec::new(),
            allowed_edges: Self::build_transition_table(),
        }
    }

    #[must_use]
    pub fn state(&self) -> MachineState {
        self.state
    }

    #[must_use]
    pub fn transition_count(&self) -> usize {
        self.transitions.len()
    }

    #[must_use]
    pub fn transitions(&self) -> &[StateTransition] {
        &self.transitions
    }

    pub fn transit(&mut self, to: MachineState, reason: &str) -> Result<(), OrchestratorError> {
        let from = self.state;

        if !self.allowed_edges.contains(&(from, to)) {
            return Err(OrchestratorError::InvalidTransition {
                from,
                to,
                reason: reason.to_owned(),
            });
        }

        self.state = to;
        self.transitions.push(StateTransition {
            from,
            to,
            reason: reason.to_owned(),
        });
        Ok(())
    }

    fn build_transition_table() -> Vec<(MachineState, MachineState)> {
        vec![
            (MachineState::Idle, MachineState::TaskLoaded),
            (MachineState::TaskLoaded, MachineState::TargetIdentified),
            (MachineState::TaskLoaded, MachineState::Error),
            (
                MachineState::TargetIdentified,
                MachineState::CandidateSubmitted,
            ),
            (MachineState::TargetIdentified, MachineState::Error),
            (
                MachineState::CandidateSubmitted,
                MachineState::CandidateAccepted,
            ),
            (
                MachineState::CandidateSubmitted,
                MachineState::CandidateRejected,
            ),
            (MachineState::CandidateSubmitted, MachineState::Error),
            (
                MachineState::CandidateAccepted,
                MachineState::BuildInProgress,
            ),
            (MachineState::CandidateAccepted, MachineState::Error),
            (
                MachineState::CandidateRejected,
                MachineState::CandidateSubmitted,
            ),
            (MachineState::CandidateRejected, MachineState::RunFinished),
            (MachineState::BuildInProgress, MachineState::BuildCompleted),
            (MachineState::BuildInProgress, MachineState::BuildFailed),
            (MachineState::BuildInProgress, MachineState::Error),
            (
                MachineState::BuildCompleted,
                MachineState::VerificationInProgress,
            ),
            (MachineState::BuildCompleted, MachineState::Error),
            (MachineState::BuildFailed, MachineState::CandidateSubmitted),
            (MachineState::BuildFailed, MachineState::RunFinished),
            (
                MachineState::VerificationInProgress,
                MachineState::VerificationCompleted,
            ),
            (MachineState::VerificationInProgress, MachineState::Error),
            (
                MachineState::VerificationCompleted,
                MachineState::RunFinished,
            ),
            (
                MachineState::VerificationCompleted,
                MachineState::CandidateSubmitted,
            ),
            (MachineState::RunFinished, MachineState::Idle),
            (MachineState::Error, MachineState::Idle),
            (MachineState::Error, MachineState::TaskLoaded),
        ]
    }

    pub fn reset(&mut self) {
        self.state = MachineState::Idle;
        self.transitions.clear();
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_idle() {
        let o = Orchestrator::new();
        assert_eq!(o.state(), MachineState::Idle);
    }

    #[test]
    fn valid_transition_succeeds() {
        let mut o = Orchestrator::new();
        o.transit(MachineState::TaskLoaded, "loaded task")
            .expect("transit");
        assert_eq!(o.state(), MachineState::TaskLoaded);
    }

    #[test]
    fn invalid_transition_fails() {
        let mut o = Orchestrator::new();
        let result = o.transit(MachineState::VerificationCompleted, "skip ahead");
        assert!(matches!(
            result,
            Err(OrchestratorError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn full_path_succeeds() {
        let mut o = Orchestrator::new();
        let path = vec![
            MachineState::TaskLoaded,
            MachineState::TargetIdentified,
            MachineState::CandidateSubmitted,
            MachineState::CandidateAccepted,
            MachineState::BuildInProgress,
            MachineState::BuildCompleted,
            MachineState::VerificationInProgress,
            MachineState::VerificationCompleted,
            MachineState::RunFinished,
        ];
        for s in path {
            o.transit(s, "step").expect("transit");
        }
        assert_eq!(o.state(), MachineState::RunFinished);
    }

    #[test]
    fn reset_clears_state() {
        let mut o = Orchestrator::new();
        o.transit(MachineState::TaskLoaded, "load")
            .expect("transit");
        o.reset();
        assert_eq!(o.state(), MachineState::Idle);
        assert_eq!(o.transition_count(), 0);
    }
}
