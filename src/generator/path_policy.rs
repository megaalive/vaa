//! Explicit patch path policy checks (allow/deny), reusable by guard + patch evidence.

use serde::{Deserialize, Serialize};

use crate::generator::error::GeneratorError;
use crate::generator::repo_guard::path_policy_violations;
use crate::generator::spec::{load_generator_spec, PatchPolicy};

/// Result of checking a changed-path list against a patch policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathPolicyReport {
    pub changed_files: Vec<String>,
    pub violations: Vec<String>,
    pub ok: bool,
}

/// Check changed paths against an explicit policy.
#[must_use]
pub fn check_path_policy(changed_files: &[String], policy: &PatchPolicy) -> PathPolicyReport {
    let violations = path_policy_violations(changed_files, policy);
    PathPolicyReport {
        changed_files: changed_files.to_vec(),
        ok: violations.is_empty(),
        violations,
    }
}

/// Load policy from a generator spec and check paths.
pub fn check_paths_against_spec(
    spec_path: impl AsRef<std::path::Path>,
    changed_files: &[String],
) -> Result<PathPolicyReport, GeneratorError> {
    let spec = load_generator_spec(spec_path)?;
    let report = check_path_policy(changed_files, &spec.patch_policy);
    if !report.ok {
        return Err(GeneratorError::from_diagnostics(
            &report
                .violations
                .iter()
                .map(|v| format!("path policy: {v}"))
                .collect::<Vec<_>>(),
        ));
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_backend_paths() {
        let policy = PatchPolicy {
            allowed_paths: vec!["src/backend/**".into()],
            forbidden_paths: vec!["**/stack.lock.toml".into()],
        };
        let report = check_path_policy(&["src/backend/x.rs".into()], &policy);
        assert!(report.ok);
    }

    #[test]
    fn rejects_forbidden_and_disallowed() {
        let policy = PatchPolicy {
            allowed_paths: vec!["src/backend/**".into()],
            forbidden_paths: vec!["**/stack.lock.toml".into()],
        };
        let report = check_path_policy(
            &[
                "integrations/hlax64/stack.lock.toml".into(),
                "README.md".into(),
            ],
            &policy,
        );
        assert!(!report.ok);
        assert!(report.violations.len() >= 2);
    }
}
