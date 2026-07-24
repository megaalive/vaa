//! Repository guard: exact revision, clean worktree, path policy.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::generator::error::GeneratorError;
use crate::generator::spec::{GeneratorSpec, PatchPolicy};

/// Outcome of a repository guard check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoGuardReport {
    /// Absolute or normalized repository path checked.
    pub repository_path: PathBuf,
    /// HEAD commit as reported by git (40-hex when available).
    pub head_revision: String,
    /// Expected revision from the generator spec (`git:<hex>`).
    pub expected_revision: String,
    /// Whether HEAD matches the expected pin.
    pub revision_ok: bool,
    /// Whether the worktree is considered clean under the configured policy.
    pub worktree_clean: bool,
    /// Porcelain status lines (empty when clean).
    pub dirty_entries: Vec<String>,
    /// Paths extracted from porcelain that violate patch policy (if checked).
    pub policy_violations: Vec<String>,
}

/// Guard options derived from [`GeneratorSpec`] or CLI overrides.
#[derive(Debug, Clone)]
pub struct RepoGuardConfig {
    pub repository_path: PathBuf,
    pub expected_revision: String,
    pub require_clean_worktree: bool,
    pub allow_untracked_files: bool,
    pub patch_policy: PatchPolicy,
    /// When true, also evaluate dirty paths against allow/deny lists.
    pub check_path_policy: bool,
}

impl RepoGuardConfig {
    /// Build config from a validated generator spec.
    ///
    /// `spec_path` is used to resolve relative `repository.path`.
    pub fn from_spec(spec: &GeneratorSpec, spec_path: &Path) -> Result<Self, GeneratorError> {
        let repo = resolve_repository_path(spec_path, &spec.repository.path)?;
        Ok(Self {
            repository_path: repo,
            expected_revision: spec.repository.expected_revision.clone(),
            require_clean_worktree: spec.repository.require_clean_worktree,
            allow_untracked_files: spec.repository.allow_untracked_files,
            patch_policy: spec.patch_policy.clone(),
            check_path_policy: true,
        })
    }
}

/// Resolve `repository.path` relative to the generator spec file directory.
pub fn resolve_repository_path(
    spec_path: &Path,
    repository_path: &str,
) -> Result<PathBuf, GeneratorError> {
    let raw = PathBuf::from(repository_path);
    let joined = if raw.is_absolute() {
        raw
    } else {
        let parent = spec_path.parent().unwrap_or_else(|| Path::new("."));
        parent.join(raw)
    };
    std::fs::canonicalize(&joined).map_err(|source| GeneratorError::Io {
        path: joined,
        source,
    })
}

/// Run revision + worktree (+ optional path policy) checks.
pub fn check_repository(config: &RepoGuardConfig) -> Result<RepoGuardReport, GeneratorError> {
    if !config.repository_path.is_dir() {
        return Err(GeneratorError::Validation(format!(
            "repository path is not a directory: {}",
            config.repository_path.display()
        )));
    }

    let head = git_rev_parse_head(&config.repository_path)?;
    let expected = strip_git_prefix(&config.expected_revision);
    let revision_ok = revisions_match(&head, expected);

    let porcelain = git_status_porcelain(&config.repository_path)?;
    let dirty_entries = porcelain;
    let changed_paths = dirty_paths_from_porcelain(&dirty_entries);
    let meaningful_dirty: Vec<String> = if config.allow_untracked_files {
        dirty_entries
            .iter()
            .filter(|line| !line.starts_with("?? ") && !line.starts_with("!! "))
            .cloned()
            .collect()
    } else {
        dirty_entries.clone()
    };
    let worktree_clean = meaningful_dirty.is_empty();

    let mut policy_violations = Vec::new();
    if config.check_path_policy && !changed_paths.is_empty() {
        policy_violations = path_policy_violations(&changed_paths, &config.patch_policy);
    }

    let report = RepoGuardReport {
        repository_path: config.repository_path.clone(),
        head_revision: head,
        expected_revision: config.expected_revision.clone(),
        revision_ok,
        worktree_clean,
        dirty_entries,
        policy_violations,
    };

    let mut diagnostics = Vec::new();
    if !report.revision_ok {
        diagnostics.push(format!(
            "revision mismatch: HEAD `git:{}` != expected `{}`",
            report.head_revision, report.expected_revision
        ));
    }
    if config.require_clean_worktree && !report.worktree_clean {
        diagnostics.push(format!(
            "worktree is dirty ({} entry/entries); require_clean_worktree=true",
            meaningful_dirty.len()
        ));
    }
    if !report.policy_violations.is_empty() {
        diagnostics.push(format!(
            "path policy violated: {}",
            report.policy_violations.join(", ")
        ));
    }
    if !diagnostics.is_empty() {
        return Err(GeneratorError::from_diagnostics(&diagnostics));
    }

    Ok(report)
}

/// Evaluate changed paths against allow/deny globs (deny wins).
#[must_use]
pub fn path_policy_violations(changed_paths: &[String], policy: &PatchPolicy) -> Vec<String> {
    let mut violations = Vec::new();
    for path in changed_paths {
        let norm = normalize_path(path);
        if policy
            .forbidden_paths
            .iter()
            .any(|pat| glob_match(pat, &norm))
        {
            violations.push(format!("{norm} (forbidden)"));
            continue;
        }
        if !policy.allowed_paths.is_empty()
            && !policy
                .allowed_paths
                .iter()
                .any(|pat| glob_match(pat, &norm))
        {
            violations.push(format!("{norm} (not allowed)"));
        }
    }
    violations
}

fn git_rev_parse_head(repo: &Path) -> Result<String, GeneratorError> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .map_err(|source| GeneratorError::Io {
            path: repo.to_path_buf(),
            source,
        })?;
    if !output.status.success() {
        return Err(GeneratorError::Validation(format!(
            "git rev-parse HEAD failed in `{}`: {}",
            repo.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let head = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if head.is_empty() || !head.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(GeneratorError::Validation(format!(
            "unexpected git HEAD `{head}`"
        )));
    }
    Ok(head.to_ascii_lowercase())
}

fn git_status_porcelain(repo: &Path) -> Result<Vec<String>, GeneratorError> {
    let output = Command::new("git")
        .args(["status", "--porcelain", "-uall"])
        .current_dir(repo)
        .output()
        .map_err(|source| GeneratorError::Io {
            path: repo.to_path_buf(),
            source,
        })?;
    if !output.status.success() {
        return Err(GeneratorError::Validation(format!(
            "git status failed in `{}`: {}",
            repo.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn dirty_paths_from_porcelain(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|line| {
            // XY PATH or XY ORIG -> PATH (rename)
            if line.len() < 4 {
                return None;
            }
            let rest = line[3..].trim();
            if let Some((_, to)) = rest.split_once(" -> ") {
                Some(to.to_owned())
            } else {
                Some(rest.trim_matches('"').to_owned())
            }
        })
        .collect()
}

fn strip_git_prefix(revision: &str) -> &str {
    revision.strip_prefix("git:").unwrap_or(revision).trim()
}

fn revisions_match(head: &str, expected: &str) -> bool {
    let head = head.to_ascii_lowercase();
    let expected = expected.to_ascii_lowercase();
    if head == expected {
        return true;
    }
    // Allow abbreviated expected pins when they uniquely prefix HEAD.
    if expected.len() >= 7 && head.starts_with(&expected) {
        return true;
    }
    if head.len() >= 7 && expected.starts_with(&head) {
        return true;
    }
    false
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

/// Minimal glob: `*` (segment), `**` (any depth), exact otherwise.
#[must_use]
pub fn glob_match(pattern: &str, path: &str) -> bool {
    let pattern = normalize_path(pattern);
    let path = normalize_path(path);
    glob_match_parts(
        &pattern.split('/').collect::<Vec<_>>(),
        &path.split('/').collect::<Vec<_>>(),
    )
}

fn glob_match_parts(pattern: &[&str], path: &[&str]) -> bool {
    let mut pi = 0;
    let mut pj = 0;
    let mut star_pat = None;
    let mut star_path = 0;

    while pj < path.len() {
        if pi < pattern.len() && (pattern[pi] == "*" || pattern[pi] == path[pj]) {
            pi += 1;
            pj += 1;
            continue;
        }
        if pi < pattern.len() && pattern[pi] == "**" {
            star_pat = Some(pi);
            star_path = pj;
            pi += 1;
            continue;
        }
        if let Some(spi) = star_pat {
            pi = spi + 1;
            star_path += 1;
            pj = star_path;
            continue;
        }
        return false;
    }

    while pi < pattern.len() && pattern[pi] == "**" {
        pi += 1;
    }
    pi == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    fn init_temp_repo() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vaa-repo-guard-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("mkdir");
        run_git(&dir, &["init"]);
        run_git(&dir, &["config", "user.email", "vaa@test"]);
        run_git(&dir, &["config", "user.name", "vaa"]);
        fs::write(dir.join("README"), "ok").expect("write");
        run_git(&dir, &["add", "README"]);
        run_git(&dir, &["commit", "-m", "init"]);
        dir
    }

    fn run_git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git");
        assert!(status.success(), "git {args:?} failed");
    }

    #[test]
    fn accepts_clean_matching_revision() {
        let repo = init_temp_repo();
        let head = git_rev_parse_head(&repo).expect("head");
        let report = check_repository(&RepoGuardConfig {
            repository_path: repo.clone(),
            expected_revision: format!("git:{head}"),
            require_clean_worktree: true,
            allow_untracked_files: false,
            patch_policy: PatchPolicy::default(),
            check_path_policy: false,
        })
        .expect("clean repo ok");
        assert!(report.revision_ok);
        assert!(report.worktree_clean);
        let _ = fs::remove_dir_all(&repo);
    }

    #[test]
    fn rejects_dirty_worktree() {
        let repo = init_temp_repo();
        let head = git_rev_parse_head(&repo).expect("head");
        fs::write(repo.join("dirty.txt"), "x").expect("dirty");
        let err = check_repository(&RepoGuardConfig {
            repository_path: repo.clone(),
            expected_revision: format!("git:{head}"),
            require_clean_worktree: true,
            allow_untracked_files: false,
            patch_policy: PatchPolicy::default(),
            check_path_policy: false,
        })
        .expect_err("dirty");
        assert!(err.to_string().contains("dirty"), "{err}");
        let _ = fs::remove_dir_all(&repo);
    }

    #[test]
    fn allow_untracked_skips_question_mark_entries() {
        let repo = init_temp_repo();
        let head = git_rev_parse_head(&repo).expect("head");
        fs::write(repo.join("scratch.tmp"), "x").expect("untracked");
        check_repository(&RepoGuardConfig {
            repository_path: repo.clone(),
            expected_revision: format!("git:{head}"),
            require_clean_worktree: true,
            allow_untracked_files: true,
            patch_policy: PatchPolicy::default(),
            check_path_policy: false,
        })
        .expect("untracked allowed");
        let _ = fs::remove_dir_all(&repo);
    }

    #[test]
    fn forbidden_path_glob_matches() {
        let policy = PatchPolicy {
            allowed_paths: vec!["src/**".to_owned()],
            forbidden_paths: vec!["**/stack.lock.toml".to_owned()],
        };
        let v =
            path_policy_violations(&["integrations/hlax64/stack.lock.toml".to_owned()], &policy);
        assert!(!v.is_empty());
        let ok = path_policy_violations(&["src/backend/foo.rs".to_owned()], &policy);
        assert!(ok.is_empty());
    }
}
