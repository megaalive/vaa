//! Task specification loading, validation, and immutability.

mod digest;
mod error;
mod locked;
mod model;
mod validate;

pub use digest::{canonical_task_bytes, task_digest, TaskDigest};
pub use error::TaskError;
pub use locked::LockedTask;
pub use model::{
    ArtifactKind, Behavior, Budgets, Capabilities, Delivery, Entry, InputSpec, InstructionPolicy,
    MemoryPolicy, Task, TaskTest, TomlValue, ValueKind, VerificationRequirements,
};
pub use validate::validate_task;

use std::path::Path;

/// Load a task document from TOML without semantic validation.
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
    toml::from_str::<Task>(text).map_err(|error| TaskError::Parse {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

/// Load, validate, and lock a task file.
pub fn load_locked_task(path: impl AsRef<Path>) -> Result<LockedTask, TaskError> {
    let path = path.as_ref();
    let task = load_task_file(path)?;
    let diagnostics = validate_task(&task);
    if !diagnostics.is_empty() {
        return Err(TaskError::from_diagnostics(&diagnostics));
    }
    Ok(LockedTask::lock(task))
}

/// Validate an in-memory task and lock it.
pub fn lock_task(task: Task) -> Result<LockedTask, TaskError> {
    let diagnostics = validate_task(&task);
    if !diagnostics.is_empty() {
        return Err(TaskError::from_diagnostics(&diagnostics));
    }
    Ok(LockedTask::lock(task))
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
