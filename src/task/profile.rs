//! Built-in verification profile expansion (Sei P1b).
//!
//! Profiles are deterministic expansions into
//! [`crate::task::SemanticEvidenceRequirements`]. Locked tasks store the
//! expanded requirements so later profile-definition changes cannot alter
//! already-sealed digests.

use crate::task::model::{
    SemanticEvidenceRequirements, SemanticEvidenceSliceReq, Task, VerificationProfile,
};

/// SemASM alias model id (`VerificationReport.alias_analysis`).
pub const ALIAS_MODEL_REGION_AFFINE_V1: &str = "region-affine-v1";
/// SemASM region-access model id.
pub const REGION_ACCESS_MODEL_AFFINE_V1: &str = "region-access-affine-v1";
/// SemASM contract-expression model id.
pub const CONTRACT_EXPR_MODEL_V1: &str = "contract-expr-v1";

/// Built-in profile for simple / observational leaves.
pub const PROFILE_LEAF_PURE_V1: &str = "leaf-pure-v1";
/// Built-in profile for affine memory leaves (memcpy-class).
pub const PROFILE_MEMORY_LEAF_AFFINE_V1: &str = "memory-leaf-affine-v1";

/// Expand `verification.profile` into frozen `semantic_evidence` requirements.
///
/// Returns validation diagnostics (empty = ok). Idempotent when evidence already
/// matches the named profile expansion.
pub fn expand_verification_profile(task: &mut Task) -> Vec<String> {
    let Some(VerificationProfile { name }) = task.verification.profile.clone() else {
        return Vec::new();
    };

    let Some(expanded) = builtin_semantic_evidence(&name) else {
        return vec![format!(
            "unknown verification.profile.name `{name}` (known: {PROFILE_LEAF_PURE_V1}, {PROFILE_MEMORY_LEAF_AFFINE_V1})"
        )];
    };

    if !task.verification.semantic_evidence.is_unset() {
        if task.verification.semantic_evidence == expanded {
            return Vec::new();
        }
        return vec![
            "verification.profile cannot be combined with a conflicting verification.semantic_evidence block (omit one, or match the profile expansion)"
                .to_owned(),
        ];
    }

    task.verification.semantic_evidence = expanded;
    Vec::new()
}

/// Look up a built-in profile by name.
#[must_use]
pub fn builtin_semantic_evidence(name: &str) -> Option<SemanticEvidenceRequirements> {
    match name {
        PROFILE_LEAF_PURE_V1 => Some(leaf_pure_v1()),
        PROFILE_MEMORY_LEAF_AFFINE_V1 => Some(memory_leaf_affine_v1()),
        _ => None,
    }
}

fn leaf_pure_v1() -> SemanticEvidenceRequirements {
    // Soft profile for single-buffer observational leaves: require living
    // alias evidence when `[function.memory]` is present; do not require
    // contract-expr (many leaf ensures are outside the v1 subset and SemASM
    // omits the slice when every expression is not_evaluated).
    SemanticEvidenceRequirements {
        alias: SemanticEvidenceSliceReq {
            required: true,
            model: Some(ALIAS_MODEL_REGION_AFFINE_V1.to_owned()),
            allow_incomplete: true,
            allow_caller_obligations: true,
            allow_unknown_accesses: false,
            allow_not_evaluated: false,
        },
        region_access: SemanticEvidenceSliceReq::default(),
        contract_expressions: SemanticEvidenceSliceReq::default(),
    }
}

fn memory_leaf_affine_v1() -> SemanticEvidenceRequirements {
    SemanticEvidenceRequirements {
        alias: SemanticEvidenceSliceReq {
            required: true,
            model: Some(ALIAS_MODEL_REGION_AFFINE_V1.to_owned()),
            allow_incomplete: false,
            allow_caller_obligations: true,
            allow_unknown_accesses: false,
            allow_not_evaluated: false,
        },
        region_access: SemanticEvidenceSliceReq {
            required: true,
            model: Some(REGION_ACCESS_MODEL_AFFINE_V1.to_owned()),
            allow_incomplete: false,
            allow_caller_obligations: false,
            allow_unknown_accesses: false,
            allow_not_evaluated: false,
        },
        contract_expressions: SemanticEvidenceSliceReq {
            required: true,
            model: Some(CONTRACT_EXPR_MODEL_V1.to_owned()),
            allow_incomplete: false,
            allow_caller_obligations: false,
            allow_unknown_accesses: false,
            allow_not_evaluated: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::model::{
        ArtifactKind, Behavior, Budgets, Capabilities, Delivery, Entry, InstructionPolicy,
        MemoryPolicy, ValueKind, VerificationRequirements,
    };
    use std::collections::BTreeMap;

    fn minimal() -> Task {
        Task {
            schema_version: "0.1".to_owned(),
            task_id: "profile-demo".to_owned(),
            artifact_kind: ArtifactKind::CallableFunction,
            target: "x86_64-unknown-linux-gnu".to_owned(),
            entry: Entry {
                symbol: "f".to_owned(),
                abi: "sysv64".to_owned(),
            },
            inputs: BTreeMap::new(),
            output: ValueKind {
                kind: "i64".to_owned(),
            },
            behavior: Behavior {
                summary: "demo".to_owned(),
                integer_overflow: None,
                empty_input_result: None,
            },
            capabilities: Capabilities {
                syscalls: vec![],
                imports: vec![],
                heap: false,
                filesystem: false,
                network: false,
                environment: false,
                clock: false,
                random: false,
            },
            memory: MemoryPolicy {
                max_stack_bytes: 64,
                allow_global_writable: false,
                allow_self_modifying_code: false,
            },
            instructions: InstructionPolicy {
                required_features: vec![],
                forbidden_mnemonics: vec![],
                allow_unknown_semantics: false,
            },
            verification: VerificationRequirements {
                require_complete_lowering: false,
                require_abi_check: false,
                require_object_inspection: false,
                require_behavioral_tests: false,
                require_reproducible_build: false,
                semantic_evidence: SemanticEvidenceRequirements::default(),
                profile: None,
            },
            budgets: Budgets {
                max_candidates: 1,
                max_repairs_per_candidate: 0,
                max_wall_time_seconds: 10,
                max_model_tokens: 0,
                max_no_progress_iterations: 1,
            },
            delivery: Delivery {
                include_source: true,
                include_object: false,
                include_binary: false,
                include_evidence: true,
            },
            tests: vec![],
        }
    }

    #[test]
    fn expands_leaf_pure_when_unset() {
        let mut task = minimal();
        task.verification.profile = Some(VerificationProfile {
            name: PROFILE_LEAF_PURE_V1.to_owned(),
        });
        assert!(expand_verification_profile(&mut task).is_empty());
        assert!(task.verification.semantic_evidence.alias.required);
        assert_eq!(
            task.verification.semantic_evidence.alias.model.as_deref(),
            Some(ALIAS_MODEL_REGION_AFFINE_V1)
        );
        assert!(task.verification.semantic_evidence.alias.allow_incomplete);
        assert!(!task.verification.semantic_evidence.region_access.required);
        assert!(
            !task
                .verification
                .semantic_evidence
                .contract_expressions
                .required
        );
    }

    #[test]
    fn expands_memory_leaf_strict() {
        let mut task = minimal();
        task.verification.profile = Some(VerificationProfile {
            name: PROFILE_MEMORY_LEAF_AFFINE_V1.to_owned(),
        });
        assert!(expand_verification_profile(&mut task).is_empty());
        let se = &task.verification.semantic_evidence;
        assert!(se.region_access.required);
        assert_eq!(
            se.region_access.model.as_deref(),
            Some(REGION_ACCESS_MODEL_AFFINE_V1)
        );
        assert!(!se.alias.allow_incomplete);
        assert!(!se.contract_expressions.allow_not_evaluated);
    }

    #[test]
    fn unknown_profile_is_diagnostic() {
        let mut task = minimal();
        task.verification.profile = Some(VerificationProfile {
            name: "not-a-real-profile".to_owned(),
        });
        let diags = expand_verification_profile(&mut task);
        assert!(diags
            .iter()
            .any(|d| d.contains("unknown verification.profile")));
    }

    #[test]
    fn conflicting_explicit_evidence_rejected() {
        let mut task = minimal();
        task.verification.profile = Some(VerificationProfile {
            name: PROFILE_LEAF_PURE_V1.to_owned(),
        });
        task.verification.semantic_evidence.alias.required = true;
        task.verification.semantic_evidence.alias.model = Some("wrong-model".to_owned());
        let diags = expand_verification_profile(&mut task);
        assert!(diags.iter().any(|d| d.contains("conflicting")));
    }

    #[test]
    fn matching_explicit_evidence_is_idempotent() {
        let mut task = minimal();
        task.verification.profile = Some(VerificationProfile {
            name: PROFILE_LEAF_PURE_V1.to_owned(),
        });
        task.verification.semantic_evidence = leaf_pure_v1();
        assert!(expand_verification_profile(&mut task).is_empty());
        assert_eq!(task.verification.semantic_evidence, leaf_pure_v1());
    }
}
