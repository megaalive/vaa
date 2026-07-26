//! Task specification loading, validation, and immutability.

mod digest;
mod error;
mod locked;
mod model;
mod profile;
mod validate;

pub use digest::{canonical_task_bytes, task_digest, TaskDigest};
pub use error::TaskError;
pub use locked::LockedTask;
pub use model::{
    ArtifactKind, Behavior, Budgets, Capabilities, Delivery, Entry, InputSpec, InstructionPolicy,
    MemoryPolicy, SemanticEvidenceRequirements, SemanticEvidenceSliceReq, Task, TaskTest,
    TomlValue, ValueKind, VerificationProfile, VerificationRequirements,
};
pub use profile::{
    builtin_semantic_evidence, expand_verification_profile, ALIAS_MODEL_REGION_AFFINE_V1,
    CONTRACT_EXPR_MODEL_V1, PROFILE_LEAF_PURE_V1, PROFILE_MEMORY_LEAF_AFFINE_V1,
    REGION_ACCESS_MODEL_AFFINE_V1,
};
pub use validate::validate_task;

use std::path::Path;

/// Load a task document from TOML without semantic validation or profile expansion.
pub fn load_task_file(path: impl AsRef<Path>) -> Result<Task, TaskError> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path).map_err(|source| TaskError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    parse_task_toml(path, &text)
}

/// Parse task TOML from a string (path used only for diagnostics).
pub fn parse_task_toml(path: &Path, text: &str) -> Result<Task, TaskError> {
    toml::from_str::<Task>(text).map_err(|error| {
        let raw = error.to_string();
        TaskError::Parse {
            path: path.to_path_buf(),
            message: enrich_task_parse_message(&raw),
        }
    })
}

/// Append beginner-oriented hints for common task TOML mistakes.
fn enrich_task_parse_message(message: &str) -> String {
    let mut hints: Vec<&str> = Vec::new();
    let lower = message.to_ascii_lowercase();
    if lower.contains("unknown field") {
        hints.push(
            "hint: task schema 0.1 rejects unknown keys (no free-form `description`); \
             see schemas/task.vaa.schema.json and docs/task-schema.md",
        );
    }
    if lower.contains("artifact_kind")
        || message.contains("standalone-executable")
        || message.contains("standalone_executable")
    {
        hints.push(
            "hint: artifact_kind must be one of: callable-function, hosted-program, \
             freestanding-image (not standalone-executable)",
        );
    }
    if lower.contains("max_length") || (lower.contains("inputs") && lower.contains("string")) {
        hints.push(
            "hint: inputs.* use ValueKind/InputSpec shapes from schema 0.1 — \
             there is no kind=\"string\" + max_length field",
        );
    }
    if hints.is_empty() {
        message.to_owned()
    } else {
        format!("{message}\n{}", hints.join("\n"))
    }
}

/// Expand profiles, validate, and lock a task.
pub fn lock_task(mut task: Task) -> Result<LockedTask, TaskError> {
    let mut diagnostics = expand_verification_profile(&mut task);
    diagnostics.extend(validate_task(&task));
    if !diagnostics.is_empty() {
        return Err(TaskError::from_diagnostics(&diagnostics));
    }
    Ok(LockedTask::lock(task))
}

/// Load, expand profiles, validate, and lock a task file.
pub fn load_locked_task(path: impl AsRef<Path>) -> Result<LockedTask, TaskError> {
    let task = load_task_file(path)?;
    lock_task(task)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("tasks")
            .join(name)
    }

    #[test]
    fn loads_sum_i64_fixture() {
        let locked = load_locked_task(fixture("sum_i64.vaa.toml")).expect("valid fixture");
        assert_eq!(locked.task().task_id, "sum-i64-v1");
        assert_eq!(locked.task().tests.len(), 3);
        assert!(locked.digest_matches());
        assert_eq!(locked.digest().hex.len(), 64);
        assert!(locked.task().verification.semantic_evidence.is_unset());
        assert!(locked.task().verification.profile.is_none());
    }

    #[test]
    fn expands_leaf_pure_profile_fixture() {
        let locked =
            load_locked_task(fixture("sum_i64_profile_leaf_pure.vaa.toml")).expect("valid");
        assert_eq!(locked.task().task_id, "sum-i64-leaf-pure-profile-v1");
        assert_eq!(
            locked
                .task()
                .verification
                .profile
                .as_ref()
                .map(|p| p.name.as_str()),
            Some(PROFILE_LEAF_PURE_V1)
        );
        let se = &locked.task().verification.semantic_evidence;
        assert!(se.alias.required);
        assert_eq!(
            se.alias.model.as_deref(),
            Some(ALIAS_MODEL_REGION_AFFINE_V1)
        );
        assert!(se.alias.allow_incomplete);
        assert!(!se.contract_expressions.required);
        assert!(!se.region_access.required);
        assert!(locked.digest_matches());

        // Expansion is frozen into the digest: legacy sum_i64 differs.
        let legacy = load_locked_task(fixture("sum_i64.vaa.toml")).expect("legacy");
        assert_ne!(locked.digest(), legacy.digest());
    }

    #[test]
    fn expands_memory_leaf_profile_fixture() {
        let locked =
            load_locked_task(fixture("memcpy_profile_memory_leaf.vaa.toml")).expect("valid");
        assert_eq!(
            locked
                .task()
                .verification
                .profile
                .as_ref()
                .map(|p| p.name.as_str()),
            Some(PROFILE_MEMORY_LEAF_AFFINE_V1)
        );
        let se = &locked.task().verification.semantic_evidence;
        assert!(se.region_access.required);
        assert_eq!(
            se.region_access.model.as_deref(),
            Some(REGION_ACCESS_MODEL_AFFINE_V1)
        );
        assert!(!se.alias.allow_incomplete);
        assert!(!se.contract_expressions.allow_not_evaluated);
        assert!(locked.digest_matches());
    }

    #[test]
    fn rejects_unknown_profile_fixture() {
        let error = load_locked_task(fixture("invalid_unknown_profile.vaa.toml"))
            .expect_err("unknown profile");
        assert!(
            error.to_string().contains("unknown verification.profile"),
            "unexpected: {error}"
        );
    }

    #[test]
    fn rejects_conflicting_profile_and_explicit_fixture() {
        let error = load_locked_task(fixture("invalid_profile_and_explicit.vaa.toml"))
            .expect_err("conflict");
        assert!(
            error.to_string().contains("conflicting"),
            "unexpected: {error}"
        );
    }

    #[test]
    fn rejects_unknown_field_fixture() {
        let error = load_locked_task(fixture("invalid_unknown_field.vaa.toml"))
            .expect_err("unknown field must fail");
        let message = error.to_string();
        assert!(
            message.contains("unknown field") || message.contains("invalid task TOML"),
            "unexpected message: {message}"
        );
        assert!(
            message.contains("hint:") || message.contains("schema 0.1"),
            "expected beginner hint, got: {message}"
        );
    }

    #[test]
    fn parse_hint_mentions_artifact_kind_enum() {
        let msg = enrich_task_parse_message(
            "TOML parse error: unknown variant `standalone-executable`, expected one of `callable-function`, `hosted-program`, `freestanding-image` for key `artifact_kind`",
        );
        assert!(msg.contains("hosted-program"));
        assert!(msg.contains("hint:"));
    }

    #[test]
    fn rejects_bad_schema_fixture() {
        let error = load_locked_task(fixture("invalid_schema_version.vaa.toml"))
            .expect_err("schema must fail");
        assert!(error.to_string().contains("schema_version"));
    }

    #[test]
    fn rejects_missing_tests_fixture() {
        let error = load_locked_task(fixture("invalid_missing_tests.vaa.toml"))
            .expect_err("missing tests must fail");
        assert!(error.to_string().contains("[[tests]]"));
    }

    #[test]
    fn rejects_zero_budget_negative_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/negative/task_zero_budget.vaa.toml");
        let error = load_locked_task(&path).expect_err("zero budget must fail");
        assert!(
            error.to_string().contains("max_candidates"),
            "unexpected: {error}"
        );
    }

    #[test]
    fn repair_cannot_silently_reuse_old_digest_after_policy_edit() {
        let locked = load_locked_task(fixture("sum_i64.vaa.toml")).expect("valid");
        let original_digest = locked.digest().clone();

        // Simulate a hostile or buggy repair path that clones and mutates policy.
        let mut tampered = locked.task().clone();
        tampered.capabilities.network = true;
        // Direct lock bypasses validation on purpose here to show digest divergence.
        let relocked = LockedTask::lock(tampered);
        assert_ne!(original_digest, *relocked.digest());
    }
}
