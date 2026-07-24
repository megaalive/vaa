//! Generator-versus-verifier triage for repair routing.

use serde::{Deserialize, Serialize};

/// Where a failure should be routed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriageClass {
    /// Likely a defect in the external generator / backend under test.
    GeneratorDefect,
    /// SemASM/VAA incomplete — do not treat as a generator bug.
    VerifierIncomplete,
    /// Toolchain, schema, identity, or guard failure.
    ToolchainOrIdentity,
    /// Contract/behavior violation after complete verification.
    SemanticRejected,
    /// Success / accepted — no repair.
    Accepted,
    /// Unclassified; fail closed toward incomplete.
    Unknown,
}

/// One triage decision with a short rationale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriageDecision {
    pub class: TriageClass,
    pub rationale: String,
    /// Whether a coding agent should edit generator source.
    pub suggest_generator_repair: bool,
}

/// Classify a case/suite/patch status string for repair routing.
#[must_use]
pub fn triage_status(status: &str) -> TriageDecision {
    let normalized = status
        .trim()
        .trim_matches('"')
        .replace(['_', ' '], "")
        .to_ascii_lowercase();

    if normalized.contains("verifiedunderpreconditions") {
        return TriageDecision {
            class: TriageClass::VerifierIncomplete,
            rationale: "verified_under_preconditions is not unconditional verified; do not treat as generator defect by default".into(),
            suggest_generator_repair: false,
        };
    }
    if normalized.contains("verified") || normalized == "accepted" || normalized.contains("pass") {
        return TriageDecision {
            class: TriageClass::Accepted,
            rationale: "status indicates acceptance".into(),
            suggest_generator_repair: false,
        };
    }
    if normalized.contains("incomplete")
        || normalized.contains("missing")
        || normalized.contains("skipped")
        || normalized == "generated"
    {
        return TriageDecision {
            class: TriageClass::VerifierIncomplete,
            rationale: "incomplete/missing evidence is a verifier or harness gap, not a proven generator defect".into(),
            suggest_generator_repair: false,
        };
    }
    if normalized.contains("toolchain")
        || normalized.contains("identity")
        || normalized.contains("schema")
        || normalized == "failed"
        || normalized.contains("forbidden")
    {
        return TriageDecision {
            class: TriageClass::ToolchainOrIdentity,
            rationale: "toolchain/schema/identity/guard failure — fix environment or authority policy first".into(),
            suggest_generator_repair: false,
        };
    }
    if normalized.contains("violat")
        || normalized.contains("behaviorfailed")
        || normalized == "rejected"
    {
        return TriageDecision {
            class: TriageClass::SemanticRejected,
            rationale:
                "complete verification rejected the candidate — generator repair is in scope".into(),
            suggest_generator_repair: true,
        };
    }
    if normalized.contains("generator") || normalized.contains("build") {
        return TriageDecision {
            class: TriageClass::GeneratorDefect,
            rationale: "status points at generator build/generation failure".into(),
            suggest_generator_repair: true,
        };
    }

    TriageDecision {
        class: TriageClass::Unknown,
        rationale: format!("unrecognized status `{status}` — fail closed as incomplete"),
        suggest_generator_repair: false,
    }
}

/// Map [`TriageClass::SemanticRejected`] / generator defects to generator repair.
#[must_use]
pub fn should_export_repair_packet(decision: &TriageDecision) -> bool {
    decision.suggest_generator_repair
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_not_generator_repair() {
        let d = triage_status("Incomplete");
        assert_eq!(d.class, TriageClass::VerifierIncomplete);
        assert!(!d.suggest_generator_repair);
    }

    #[test]
    fn under_preconditions_not_unconditional() {
        let d = triage_status("VerifiedUnderPreconditions");
        assert_eq!(d.class, TriageClass::VerifierIncomplete);
        assert!(!d.suggest_generator_repair);
    }

    #[test]
    fn behavior_failed_suggests_repair() {
        let d = triage_status("BehaviorFailed");
        assert_eq!(d.class, TriageClass::SemanticRejected);
        assert!(d.suggest_generator_repair);
        assert!(should_export_repair_packet(&d));
    }

    #[test]
    fn accepted_no_repair() {
        let d = triage_status("Verified");
        assert_eq!(d.class, TriageClass::Accepted);
        assert!(!d.suggest_generator_repair);
    }
}
