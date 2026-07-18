//! Semantic validation for loaded tasks.

use crate::task::model::{ArtifactKind, Task};
use crate::TASK_SCHEMA_VERSION;

/// Validate a deserialized task and return diagnostic messages (empty = ok).
#[must_use]
pub fn validate_task(task: &Task) -> Vec<String> {
    let mut diagnostics = Vec::new();

    if task.schema_version != TASK_SCHEMA_VERSION {
        diagnostics.push(format!(
            "unsupported schema_version `{}` (this VAA build accepts only `{TASK_SCHEMA_VERSION}`)",
            task.schema_version
        ));
    }

    if task.task_id.trim().is_empty() {
        diagnostics.push("task_id must not be empty".to_owned());
    } else if !is_safe_id(&task.task_id) {
        diagnostics.push(
            "task_id must match [A-Za-z][A-Za-z0-9._-]{0,127} (no path separators or spaces)"
                .to_owned(),
        );
    }

    if task.target.trim().is_empty() {
        diagnostics.push("target must not be empty".to_owned());
    } else if task.target.chars().any(|c| c.is_whitespace()) {
        diagnostics.push("target must not contain whitespace".to_owned());
    }

    if task.entry.symbol.trim().is_empty() {
        diagnostics.push("entry.symbol must not be empty".to_owned());
    } else if !is_safe_symbol(&task.entry.symbol) {
        diagnostics.push(
            "entry.symbol must be a simple assembler symbol ([A-Za-z_][A-Za-z0-9_]*)".to_owned(),
        );
    }

    if task.entry.abi.trim().is_empty() {
        diagnostics.push("entry.abi must not be empty".to_owned());
    }

    if task.output.kind.trim().is_empty() {
        diagnostics.push("output.kind must not be empty".to_owned());
    }

    if task.behavior.summary.trim().is_empty() {
        diagnostics.push("behavior.summary must not be empty".to_owned());
    }

    if task.memory.max_stack_bytes == 0 {
        diagnostics.push("memory.max_stack_bytes must be greater than zero".to_owned());
    }

    if task.memory.allow_self_modifying_code {
        diagnostics.push(
            "memory.allow_self_modifying_code=true is rejected in schema 0.1 (fail-closed)"
                .to_owned(),
        );
    }

    if task.capabilities.network {
        diagnostics.push(
            "capabilities.network=true is rejected in schema 0.1 (fail-closed default)"
                .to_owned(),
        );
    }

    if task.budgets.max_candidates == 0 {
        diagnostics.push("budgets.max_candidates must be at least 1".to_owned());
    }

    if task.budgets.max_wall_time_seconds == 0 {
        diagnostics.push("budgets.max_wall_time_seconds must be at least 1".to_owned());
    }

    if task.verification.require_behavioral_tests && task.tests.is_empty() {
        diagnostics.push(
            "verification.require_behavioral_tests=true requires a non-empty [[tests]] list"
                .to_owned(),
        );
    }

    let mut seen_test_names = std::collections::BTreeSet::new();
    for (index, test) in task.tests.iter().enumerate() {
        if test.name.trim().is_empty() {
            diagnostics.push(format!("tests[{index}].name must not be empty"));
        } else if !seen_test_names.insert(test.name.clone()) {
            diagnostics.push(format!(
                "duplicate test name `{}` (tests must be unique)",
                test.name
            ));
        }

        for (key, value) in &test.input {
            if key.trim().is_empty() {
                diagnostics.push(format!("tests[{index}] has an empty input key"));
            }
            if !value.is_supported_test_value() {
                diagnostics.push(format!(
                    "tests[{index}].input.{key} uses an unsupported value kind"
                ));
            }
        }

        if !test.expected.is_supported_test_value() {
            diagnostics.push(format!(
                "tests[{index}].expected uses an unsupported value kind"
            ));
        }
    }

    for (name, input) in &task.inputs {
        if name.trim().is_empty() {
            diagnostics.push("inputs contains an empty key".to_owned());
        }
        if input.kind.trim().is_empty() {
            diagnostics.push(format!("inputs.{name}.kind must not be empty"));
        }
        if input.kind == "pointer" {
            if input.element.as_ref().is_none_or(|s| s.trim().is_empty()) {
                diagnostics.push(format!(
                    "inputs.{name}.element is required when kind is `pointer`"
                ));
            }
            if let Some(length_from) = &input.length_from {
                if !task.inputs.contains_key(length_from) {
                    diagnostics.push(format!(
                        "inputs.{name}.length_from=`{length_from}` does not name an existing input"
                    ));
                }
            }
        }
    }

    // Schema 0.1 product focus is callable functions; other kinds parse but warn via diagnostic
    // only when verification demands behavioral tests without a harness design.
    if matches!(
        task.artifact_kind,
        ArtifactKind::HostedProgram | ArtifactKind::FreestandingImage
    ) && task.verification.require_behavioral_tests
    {
        diagnostics.push(format!(
            "artifact_kind `{}` with require_behavioral_tests is not supported in schema 0.1 (use callable-function or disable behavioral tests until a harness lands)",
            artifact_kind_name(task.artifact_kind)
        ));
    }

    diagnostics
}

fn artifact_kind_name(kind: ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::CallableFunction => "callable-function",
        ArtifactKind::HostedProgram => "hosted-program",
        ArtifactKind::FreestandingImage => "freestanding-image",
    }
}

fn is_safe_id(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    if value.len() > 128 {
        return false;
    }
    value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

fn is_safe_symbol(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::model::{
        Behavior, Budgets, Capabilities, Delivery, Entry, InstructionPolicy, MemoryPolicy,
        ValueKind, VerificationRequirements,
    };

    fn minimal_task() -> Task {
        Task {
            schema_version: TASK_SCHEMA_VERSION.to_owned(),
            task_id: "demo-v1".to_owned(),
            artifact_kind: ArtifactKind::CallableFunction,
            target: "x86_64-unknown-linux-gnu".to_owned(),
            entry: Entry {
                symbol: "demo".to_owned(),
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
                max_stack_bytes: 128,
                allow_global_writable: false,
                allow_self_modifying_code: false,
            },
            instructions: InstructionPolicy {
                required_features: vec![],
                forbidden_mnemonics: vec![],
                allow_unknown_semantics: false,
            },
            verification: VerificationRequirements {
                require_complete_lowering: true,
                require_abi_check: true,
                require_object_inspection: true,
                require_behavioral_tests: false,
                require_reproducible_build: false,
            },
            budgets: Budgets {
                max_candidates: 1,
                max_repairs_per_candidate: 0,
                max_wall_time_seconds: 60,
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

    use std::collections::BTreeMap;

    #[test]
    fn accepts_minimal_valid_task() {
        assert!(validate_task(&minimal_task()).is_empty());
    }

    #[test]
    fn rejects_bad_schema_version() {
        let mut task = minimal_task();
        task.schema_version = "9.9".to_owned();
        let diags = validate_task(&task);
        assert!(diags.iter().any(|d| d.contains("schema_version")));
    }

    #[test]
    fn rejects_behavioral_tests_required_but_missing() {
        let mut task = minimal_task();
        task.verification.require_behavioral_tests = true;
        let diags = validate_task(&task);
        assert!(diags.iter().any(|d| d.contains("[[tests]]")));
    }
}
