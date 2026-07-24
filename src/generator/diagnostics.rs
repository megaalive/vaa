//! Stable diagnostic codes for automation (plan §12).
//!
//! Codes are stable within a schema major version. Messages may evolve;
//! automation must key on the code, never on the message text.

use serde::{Deserialize, Serialize};

use crate::generator::error::GeneratorError;
use crate::generator::triage::TriageClass;

/// One registered diagnostic code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DiagnosticEntry {
    /// Stable code (`CATEGORY_NAME_NNN`).
    pub code: &'static str,
    /// Category prefix (`GEN`, `ABI`, `CFG`, `DECODE`, `LOWER`, `MEM`, `BEHAVIOR`, `POLICY`).
    pub category: &'static str,
    /// Default triage class the code routes to.
    pub triage: TriageClass,
    /// Short human description (may evolve; not stable).
    pub description: &'static str,
}

/// Initial stable registry (plan §12). Append-only within schema 0.x.
pub const DIAGNOSTIC_REGISTRY: &[DiagnosticEntry] = &[
    DiagnosticEntry {
        code: "GEN_BUILD_FAILED_001",
        category: "GEN",
        triage: TriageClass::GeneratorDefect,
        description: "generator build command failed",
    },
    DiagnosticEntry {
        code: "GEN_OUTPUT_MISSING_001",
        category: "GEN",
        triage: TriageClass::GeneratorDefect,
        description: "generation produced no output artifact",
    },
    DiagnosticEntry {
        code: "GEN_OUTPUT_AMBIGUOUS_001",
        category: "GEN",
        triage: TriageClass::GeneratorDefect,
        description: "generation produced multiple/ambiguous output artifacts",
    },
    DiagnosticEntry {
        code: "GEN_NONDETERMINISTIC_001",
        category: "GEN",
        triage: TriageClass::GeneratorDefect,
        description: "twin-run generation digests differ",
    },
    DiagnosticEntry {
        code: "ABI_CALLEE_SAVED_001",
        category: "ABI",
        triage: TriageClass::SemanticRejected,
        description: "callee-saved register modified but not restored",
    },
    DiagnosticEntry {
        code: "ABI_STACK_BALANCE_001",
        category: "ABI",
        triage: TriageClass::SemanticRejected,
        description: "stack pointer not balanced at return",
    },
    DiagnosticEntry {
        code: "ABI_RETURN_REGISTER_001",
        category: "ABI",
        triage: TriageClass::SemanticRejected,
        description: "return value register violates the ABI contract",
    },
    DiagnosticEntry {
        code: "CFG_INDIRECT_BRANCH_001",
        category: "CFG",
        triage: TriageClass::VerifierIncomplete,
        description: "indirect branch prevents complete CFG recovery",
    },
    DiagnosticEntry {
        code: "CFG_INCOMPLETE_001",
        category: "CFG",
        triage: TriageClass::VerifierIncomplete,
        description: "control-flow graph could not be completed",
    },
    DiagnosticEntry {
        code: "DECODE_UNKNOWN_INSN_001",
        category: "DECODE",
        triage: TriageClass::VerifierIncomplete,
        description: "instruction not decodable by the verifier",
    },
    DiagnosticEntry {
        code: "LOWER_UNKNOWN_EFFECT_001",
        category: "LOWER",
        triage: TriageClass::VerifierIncomplete,
        description: "instruction effect not modeled by the verifier",
    },
    DiagnosticEntry {
        code: "MEM_REGION_ESCAPE_001",
        category: "MEM",
        triage: TriageClass::SemanticRejected,
        description: "memory access escapes the declared region",
    },
    DiagnosticEntry {
        code: "MEM_PERMISSION_DENIED_001",
        category: "MEM",
        triage: TriageClass::SemanticRejected,
        description: "memory access violates region permissions",
    },
    DiagnosticEntry {
        code: "MEM_ALIAS_UNRESOLVED_001",
        category: "MEM",
        triage: TriageClass::VerifierIncomplete,
        description: "aliasing between regions could not be resolved",
    },
    DiagnosticEntry {
        code: "BEHAVIOR_VECTOR_MISMATCH_001",
        category: "BEHAVIOR",
        triage: TriageClass::SemanticRejected,
        description: "behavior vector output mismatch",
    },
    DiagnosticEntry {
        code: "POLICY_FORBIDDEN_PATH_CHANGED_001",
        category: "POLICY",
        triage: TriageClass::ToolchainOrIdentity,
        description: "patch touched a forbidden path",
    },
    DiagnosticEntry {
        code: "POLICY_STACK_LOCK_MISMATCH_001",
        category: "POLICY",
        triage: TriageClass::ToolchainOrIdentity,
        description: "stack lock digest or pin mismatch",
    },
    DiagnosticEntry {
        code: "POLICY_COMPILER_DIGEST_MISMATCH_001",
        category: "POLICY",
        triage: TriageClass::ToolchainOrIdentity,
        description: "generator binary digest mismatch",
    },
];

/// Look up a registered code.
#[must_use]
pub fn lookup_diagnostic(code: &str) -> Option<&'static DiagnosticEntry> {
    DIAGNOSTIC_REGISTRY.iter().find(|entry| entry.code == code)
}

/// Structural check: `CATEGORY_..._NNN` in upper snake case with a
/// three-digit numeric suffix.
#[must_use]
pub fn is_well_formed_diagnostic_code(code: &str) -> bool {
    let Some((prefix, suffix)) = code.rsplit_once('_') else {
        return false;
    };
    if suffix.len() != 3 || !suffix.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    !prefix.is_empty()
        && prefix
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
        && prefix
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_uppercase())
}

/// Validate a code for use in evidence / repair packets.
///
/// Registered codes always pass. Unregistered codes pass only when well
/// formed (forward compatibility for newer registries), otherwise fail.
pub fn validate_diagnostic_code(code: &str) -> Result<(), GeneratorError> {
    if lookup_diagnostic(code).is_some() {
        return Ok(());
    }
    if is_well_formed_diagnostic_code(code) {
        return Ok(());
    }
    Err(GeneratorError::Validation(format!(
        "diagnostic code `{code}` is malformed; expected `CATEGORY_NAME_NNN` (e.g. `ABI_CALLEE_SAVED_001`)"
    )))
}

/// Registry entries filtered by category prefix (e.g. `ABI`).
#[must_use]
pub fn diagnostics_by_category(category: &str) -> Vec<&'static DiagnosticEntry> {
    DIAGNOSTIC_REGISTRY
        .iter()
        .filter(|entry| entry.category.eq_ignore_ascii_case(category))
        .collect()
}

/// Serializable view of a registry entry (for `--format json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticView {
    pub code: String,
    pub category: String,
    pub triage: TriageClass,
    pub description: String,
}

impl From<&DiagnosticEntry> for DiagnosticView {
    fn from(entry: &DiagnosticEntry) -> Self {
        Self {
            code: entry.code.to_owned(),
            category: entry.category.to_owned(),
            triage: entry.triage,
            description: entry.description.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_codes_unique_and_well_formed() {
        let mut seen = std::collections::HashSet::new();
        for entry in DIAGNOSTIC_REGISTRY {
            assert!(seen.insert(entry.code), "duplicate code {}", entry.code);
            assert!(
                is_well_formed_diagnostic_code(entry.code),
                "malformed registry code {}",
                entry.code
            );
            assert!(
                entry.code.starts_with(entry.category),
                "code {} does not start with category {}",
                entry.code,
                entry.category
            );
        }
    }

    #[test]
    fn lookup_finds_known_code() {
        let entry = lookup_diagnostic("ABI_CALLEE_SAVED_001").expect("known code");
        assert_eq!(entry.category, "ABI");
        assert_eq!(entry.triage, TriageClass::SemanticRejected);
    }

    #[test]
    fn validate_accepts_wellformed_unregistered() {
        assert!(validate_diagnostic_code("MEM_FUTURE_THING_002").is_ok());
    }

    #[test]
    fn validate_rejects_malformed() {
        assert!(validate_diagnostic_code("abi_callee_saved_001").is_err());
        assert!(validate_diagnostic_code("ABI_CALLEE_SAVED").is_err());
        assert!(validate_diagnostic_code("ABI_CALLEE_SAVED_1").is_err());
        assert!(validate_diagnostic_code("").is_err());
        assert!(validate_diagnostic_code("_001").is_err());
    }

    #[test]
    fn category_filter_works() {
        let abi = diagnostics_by_category("abi");
        assert_eq!(abi.len(), 3);
        assert!(abi.iter().all(|e| e.category == "ABI"));
    }

    #[test]
    fn verifier_gap_codes_do_not_route_to_generator_repair() {
        for code in [
            "CFG_INDIRECT_BRANCH_001",
            "CFG_INCOMPLETE_001",
            "DECODE_UNKNOWN_INSN_001",
            "LOWER_UNKNOWN_EFFECT_001",
            "MEM_ALIAS_UNRESOLVED_001",
        ] {
            let entry = lookup_diagnostic(code).expect("registered");
            assert_eq!(
                entry.triage,
                TriageClass::VerifierIncomplete,
                "{code} must route to verifier, not generator repair"
            );
        }
    }
}
