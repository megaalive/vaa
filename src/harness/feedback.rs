//! Strict outcome / retry classification for harness loops.

use serde::{Deserialize, Serialize};

use crate::evidence::EvidenceStatus;
use crate::exit_code::ExitCode;

/// Single classification vocabulary for agent harnesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessOutcomeClass {
    /// Sealed / verified success (`verified` only, or under_preconditions when allowed).
    Accepted,
    /// Semantic/behavior violation — agent may repair candidate/generator.
    ViolatedRepairable,
    /// Static OK without `--allow-execution`, or coverage Incomplete.
    IncompleteCoverage,
    /// Missing toolchain / timeout / I/O — retry after tooling fix only.
    ToolchainRetryable,
    /// Path policy / security block — never auto-retry.
    PolicyBlocked,
    /// Parse / internal / unknown failure.
    Failed,
}

impl HarnessOutcomeClass {
    /// Stable label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::ViolatedRepairable => "violated_repairable",
            Self::IncompleteCoverage => "incomplete_coverage",
            Self::ToolchainRetryable => "toolchain_retryable",
            Self::PolicyBlocked => "policy_blocked",
            Self::Failed => "failed",
        }
    }

    /// Whether a harness may safely retry without changing agent inputs.
    #[must_use]
    pub const fn may_auto_retry(self) -> bool {
        matches!(self, Self::ToolchainRetryable)
    }
}

/// Suggested next action for the agent controller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessNextAction {
    /// Stop — acceptance reached.
    Done,
    /// Re-edit candidate `.asm` (direct mode).
    EditCandidate,
    /// Edit generator source then regenerate (generator mode).
    EditGenerator,
    /// Re-run with `--allow-execution`.
    OptInExecution,
    /// Fix host toolchain / doctor.
    FixToolchain,
    /// Human / policy intervention required.
    StopPolicy,
    /// Treat as hard failure.
    Abort,
}

/// Compact submit result returned to harnesses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessSubmitResult {
    pub schema_version: String,
    pub class: HarnessOutcomeClass,
    pub next_action: HarnessNextAction,
    pub evidence_status: String,
    pub raw_status: Option<String>,
    pub exit_code: u8,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seal_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch_evidence_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assembler: Option<String>,
    pub may_auto_retry: bool,
}

/// Schema for [`HarnessSubmitResult`].
pub const HARNESS_SUBMIT_SCHEMA_VERSION: &str = "0.1";

/// Map SemASM/VAA evidence + optional agent-failure code into harness class.
#[must_use]
pub fn classify_outcome(
    evidence: EvidenceStatus,
    raw_status: Option<&str>,
    failure_code: Option<&str>,
    allow_under_preconditions: bool,
) -> (HarnessOutcomeClass, HarnessNextAction, ExitCode) {
    if let Some(code) = failure_code {
        return classify_failure_code(code);
    }

    match evidence {
        EvidenceStatus::Verified => (
            HarnessOutcomeClass::Accepted,
            HarnessNextAction::Done,
            ExitCode::Success,
        ),
        EvidenceStatus::VerifiedUnderPreconditions => {
            if allow_under_preconditions {
                (
                    HarnessOutcomeClass::Accepted,
                    HarnessNextAction::Done,
                    ExitCode::Success,
                )
            } else {
                (
                    HarnessOutcomeClass::IncompleteCoverage,
                    HarnessNextAction::Abort,
                    ExitCode::Incomplete,
                )
            }
        }
        EvidenceStatus::Violated => (
            HarnessOutcomeClass::ViolatedRepairable,
            HarnessNextAction::EditCandidate,
            ExitCode::Violated,
        ),
        EvidenceStatus::Incomplete => {
            if raw_status == Some("execution_denied") {
                (
                    HarnessOutcomeClass::IncompleteCoverage,
                    HarnessNextAction::OptInExecution,
                    ExitCode::Incomplete,
                )
            } else {
                (
                    HarnessOutcomeClass::IncompleteCoverage,
                    HarnessNextAction::Abort,
                    ExitCode::Incomplete,
                )
            }
        }
        EvidenceStatus::Failed => (
            HarnessOutcomeClass::Failed,
            HarnessNextAction::Abort,
            ExitCode::ToolFailure,
        ),
    }
}

// The fatal-code arm is spelled out even though it matches the fallback: it
// pins the stable SemASM codes in docs/CONTROLLER_PROTOCOL.md to a class.
#[allow(clippy::match_same_arms)]
fn classify_failure_code(code: &str) -> (HarnessOutcomeClass, HarnessNextAction, ExitCode) {
    match code {
        "TOOLCHAIN_INCOMPLETE"
        | "SCRATCH_IO"
        | "CONTRACT_IO"
        | "SOURCE_IO"
        | "HARNESS_IO"
        | "ASSEMBLE_ERROR"
        | "ASSEMBLE_HARNESS_ERROR"
        | "LINK_ERROR"
        | "TIMEOUT" => (
            HarnessOutcomeClass::ToolchainRetryable,
            HarnessNextAction::FixToolchain,
            ExitCode::ToolFailure,
        ),
        "UNSUPPORTED_SHAPE"
        | "HARNESS_MISMATCH"
        | "CONTRACT_INVALID"
        | "CONTRACT_ENCODING"
        | "ASSEMBLE_FAILED"
        | "ASSEMBLE_HARNESS_FAILED"
        | "LINK_FAILED"
        | "INVALID_TARGET"
        | "SEAL_FAILED"
        | "ALREADY_SEALED"
        | "SUITE_REQUIRED" => (
            HarnessOutcomeClass::Failed,
            HarnessNextAction::Abort,
            ExitCode::ToolFailure,
        ),
        "BUDGET_EXHAUSTED" => (
            HarnessOutcomeClass::Failed,
            HarnessNextAction::Abort,
            ExitCode::BudgetExhausted,
        ),
        "POLICY_BLOCK" | "FORBIDDEN_PATH" => (
            HarnessOutcomeClass::PolicyBlocked,
            HarnessNextAction::StopPolicy,
            ExitCode::SecurityBlock,
        ),
        _ => (
            HarnessOutcomeClass::Failed,
            HarnessNextAction::Abort,
            ExitCode::ToolFailure,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verified_is_accepted() {
        let (c, n, e) = classify_outcome(EvidenceStatus::Verified, None, None, false);
        assert_eq!(c, HarnessOutcomeClass::Accepted);
        assert_eq!(n, HarnessNextAction::Done);
        assert_eq!(e, ExitCode::Success);
        assert!(!c.may_auto_retry());
    }

    #[test]
    fn execution_denied_suggests_opt_in() {
        let (c, n, _) = classify_outcome(
            EvidenceStatus::Incomplete,
            Some("execution_denied"),
            None,
            false,
        );
        assert_eq!(c, HarnessOutcomeClass::IncompleteCoverage);
        assert_eq!(n, HarnessNextAction::OptInExecution);
    }

    #[test]
    fn toolchain_code_is_retryable() {
        let (c, n, _) = classify_outcome(
            EvidenceStatus::Failed,
            None,
            Some("TOOLCHAIN_INCOMPLETE"),
            false,
        );
        assert_eq!(c, HarnessOutcomeClass::ToolchainRetryable);
        assert_eq!(n, HarnessNextAction::FixToolchain);
        assert!(c.may_auto_retry());
    }

    #[test]
    fn under_preconditions_not_promoted_by_default() {
        let (c, _, e) = classify_outcome(
            EvidenceStatus::VerifiedUnderPreconditions,
            Some("verified_under_preconditions"),
            None,
            false,
        );
        assert_eq!(c, HarnessOutcomeClass::IncompleteCoverage);
        assert_eq!(e, ExitCode::Incomplete);
    }
}
