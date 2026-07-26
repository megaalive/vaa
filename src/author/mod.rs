//! Fluent Agent Surface Release C — `vaa author` case lifecycle.
//!
//! Agents may propose draft task/contract edits. Humans lock via CLI.
//! Admission is checked before lock; experimental locks stay
//! `authoring_only` and never claim `sealed_acceptance`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::evidence::sha256_digest_prefixed;
use crate::semasm::{admit_leaf, snapshot_digest, AdmissionTier};
use crate::task::{load_locked_task, TaskError};

/// Relative path of the checked-in template catalog (from repo / install root).
pub const AUTHOR_TEMPLATES_DIR: &str = "schemas/author-templates";
/// Case state file written by `init` / updated by `lock`.
pub const AUTHOR_STATE_FILE: &str = "AUTHOR_STATE.toml";
/// Immutable marker written by a successful `lock`.
pub const LOCKED_MARKER_FILE: &str = "LOCKED";
/// Default assembler for admission lookups on the author path.
pub const DEFAULT_AUTHOR_ASSEMBLER: &str = "nasm";

const CATALOG_TOML: &str = include_str!("../../schemas/author-templates/catalog.toml");

const TEMPLATE_TASK: &[(&str, &str)] = &[
    (
        "pure-int-unary",
        include_str!("../../schemas/author-templates/pure-int-unary/task.vaa.toml"),
    ),
    (
        "pure-int-binary",
        include_str!("../../schemas/author-templates/pure-int-binary/task.vaa.toml"),
    ),
    (
        "pure-int-ternary",
        include_str!("../../schemas/author-templates/pure-int-ternary/task.vaa.toml"),
    ),
    (
        "buffer-read",
        include_str!("../../schemas/author-templates/buffer-read/task.vaa.toml"),
    ),
    (
        "buffer-write",
        include_str!("../../schemas/author-templates/buffer-write/task.vaa.toml"),
    ),
    (
        "dual-buffer",
        include_str!("../../schemas/author-templates/dual-buffer/task.vaa.toml"),
    ),
];

const TEMPLATE_CONTRACT: &[(&str, &str)] = &[
    (
        "pure-int-unary",
        include_str!("../../schemas/author-templates/pure-int-unary/contract.sem.toml"),
    ),
    (
        "pure-int-binary",
        include_str!("../../schemas/author-templates/pure-int-binary/contract.sem.toml"),
    ),
    (
        "pure-int-ternary",
        include_str!("../../schemas/author-templates/pure-int-ternary/contract.sem.toml"),
    ),
    (
        "buffer-read",
        include_str!("../../schemas/author-templates/buffer-read/contract.sem.toml"),
    ),
    (
        "buffer-write",
        include_str!("../../schemas/author-templates/buffer-write/contract.sem.toml"),
    ),
    (
        "dual-buffer",
        include_str!("../../schemas/author-templates/dual-buffer/contract.sem.toml"),
    ),
];

/// Errors from the authoring surface.
#[derive(Debug, Error)]
pub enum AuthorError {
    #[error("{0}")]
    Message(String),
    #[error("I/O on `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Task(#[from] TaskError),
}

impl AuthorError {
    fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

/// One catalog row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateMeta {
    pub name: String,
    pub known_ci_shape: bool,
    #[serde(default)]
    pub summary: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CatalogFile {
    #[serde(default)]
    templates: Vec<TemplateMeta>,
}

/// Author case lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorCaseState {
    Draft,
    Locked,
}

impl AuthorCaseState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Locked => "locked",
        }
    }
}

/// `AUTHOR_STATE.toml` contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorState {
    pub schema_version: String,
    pub state: AuthorCaseState,
    pub template: String,
    pub name: String,
    pub target: String,
    /// True unless the template is a known-CI shape.
    pub experimental: bool,
    /// Set on lock: admission tier string or `authoring_only`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_snapshot_digest: Option<String>,
}

/// Result of `vaa author init`.
#[derive(Debug, Clone, Serialize)]
pub struct InitResult {
    pub ok: bool,
    pub case_dir: PathBuf,
    pub state: AuthorState,
    pub task: PathBuf,
    pub contract: PathBuf,
}

/// Result of `vaa author review`.
#[derive(Debug, Clone, Serialize)]
pub struct ReviewResult {
    pub ok: bool,
    pub case_dir: PathBuf,
    pub state: Option<AuthorState>,
    pub task_digest: Option<String>,
    pub contract_digest: Option<String>,
    pub capability_snapshot_digest: String,
    pub admission: Option<AdmissionSummary>,
    pub issues: Vec<String>,
}

/// Compact admission row for review / lock JSON.
#[derive(Debug, Clone, Serialize)]
pub struct AdmissionSummary {
    pub leaf: String,
    pub target: String,
    pub assembler: String,
    pub admitted: bool,
    pub tier: Option<String>,
    pub acceptance_level: Option<String>,
}

/// Result of `vaa author lock`.
#[derive(Debug, Clone, Serialize)]
pub struct LockResult {
    pub ok: bool,
    pub case_dir: PathBuf,
    pub state: AuthorState,
    pub locked_marker: PathBuf,
    pub acceptance: String,
}

/// Load embedded catalog metadata.
#[must_use]
pub fn load_catalog() -> Vec<TemplateMeta> {
    let file: CatalogFile =
        toml::from_str(CATALOG_TOML).expect("schemas/author-templates/catalog.toml must parse");
    file.templates
}

/// Look up template metadata by name.
#[must_use]
pub fn template_meta(name: &str) -> Option<TemplateMeta> {
    load_catalog().into_iter().find(|t| t.name == name)
}

/// Whether `name` is a known catalog template.
#[must_use]
pub fn is_known_template(name: &str) -> bool {
    TEMPLATE_TASK.iter().any(|(n, _)| *n == name)
}

/// Derive ABI label from a target triple (Win64 vs SysV).
#[must_use]
pub fn abi_for_target(target: &str) -> &'static str {
    if target.contains("windows") {
        "win64"
    } else {
        "sysv64"
    }
}

/// Build a safe `task_id` from routine name + target.
#[must_use]
pub fn task_id_for(name: &str, target: &str) -> String {
    let target_slug: String = target
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("author-{name}-{target_slug}-v1")
}

fn fill_placeholders(text: &str, name: &str, target: &str, abi: &str, task_id: &str) -> String {
    text.replace("__NAME__", name)
        .replace("__TARGET__", target)
        .replace("__ABI__", abi)
        .replace("__TASK_ID__", task_id)
}

fn template_task_src(template: &str) -> Option<&'static str> {
    TEMPLATE_TASK
        .iter()
        .find(|(n, _)| *n == template)
        .map(|(_, s)| *s)
}

fn template_contract_src(template: &str) -> Option<&'static str> {
    TEMPLATE_CONTRACT
        .iter()
        .find(|(n, _)| *n == template)
        .map(|(_, s)| *s)
}

fn write_text(path: &Path, contents: &str) -> Result<(), AuthorError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| AuthorError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(path, contents).map_err(|source| AuthorError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn read_text(path: &Path) -> Result<String, AuthorError> {
    fs::read_to_string(path).map_err(|source| AuthorError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Load `AUTHOR_STATE.toml` from a case directory.
pub fn load_author_state(case_dir: &Path) -> Result<AuthorState, AuthorError> {
    let path = case_dir.join(AUTHOR_STATE_FILE);
    let text = read_text(&path)?;
    toml::from_str(&text).map_err(|e| AuthorError::msg(format!("invalid {AUTHOR_STATE_FILE}: {e}")))
}

fn write_author_state(case_dir: &Path, state: &AuthorState) -> Result<(), AuthorError> {
    let path = case_dir.join(AUTHOR_STATE_FILE);
    let mut text = format!(
        "schema_version = \"{}\"\n\
state = \"{}\"\n\
template = \"{}\"\n\
name = \"{}\"\n\
target = \"{}\"\n\
experimental = {}\n",
        state.schema_version,
        state.state.as_str(),
        state.template,
        state.name,
        state.target,
        state.experimental
    );
    if let Some(v) = &state.acceptance {
        text.push_str(&format!("acceptance = \"{v}\"\n"));
    }
    if let Some(v) = &state.task_digest {
        text.push_str(&format!("task_digest = \"{v}\"\n"));
    }
    if let Some(v) = &state.contract_digest {
        text.push_str(&format!("contract_digest = \"{v}\"\n"));
    }
    if let Some(v) = &state.capability_snapshot_digest {
        text.push_str(&format!("capability_snapshot_digest = \"{v}\"\n"));
    }
    write_text(&path, &text)
}

fn validate_routine_name(name: &str) -> Result<(), AuthorError> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(AuthorError::msg("routine name must not be empty"));
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(AuthorError::msg(
            "routine name must start with ASCII letter or underscore",
        ));
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(AuthorError::msg(
            "routine name must match [A-Za-z_][A-Za-z0-9_]*",
        ));
    }
    Ok(())
}

/// Initialize a draft authoring case from a template.
pub fn author_init(
    template: &str,
    name: &str,
    target: &str,
    out_dir: Option<&Path>,
) -> Result<InitResult, AuthorError> {
    validate_routine_name(name)?;
    if target.trim().is_empty() || target.chars().any(char::is_whitespace) {
        return Err(AuthorError::msg("target must be a non-empty triple"));
    }
    let meta = template_meta(template).ok_or_else(|| {
        AuthorError::msg(format!(
            "unknown template `{template}` (see schemas/author-templates/README.md)"
        ))
    })?;
    let task_src = template_task_src(template)
        .ok_or_else(|| AuthorError::msg(format!("missing embedded task for `{template}`")))?;
    let contract_src = template_contract_src(template)
        .ok_or_else(|| AuthorError::msg(format!("missing embedded contract for `{template}`")))?;

    let base = out_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".vaa/author"));
    let case_dir = base.join(name);

    if case_dir.join(LOCKED_MARKER_FILE).is_file() {
        return Err(AuthorError::msg(format!(
            "case `{}` is locked (LOCKED marker present); refuse init mutation",
            case_dir.display()
        )));
    }
    if case_dir.join(AUTHOR_STATE_FILE).is_file() {
        let existing = load_author_state(&case_dir)?;
        if existing.state == AuthorCaseState::Locked {
            return Err(AuthorError::msg(format!(
                "case `{}` is already locked; refuse init mutation",
                case_dir.display()
            )));
        }
        return Err(AuthorError::msg(format!(
            "case `{}` already exists (state={}); remove it or choose another --name / --out",
            case_dir.display(),
            existing.state.as_str()
        )));
    }

    let abi = abi_for_target(target);
    let task_id = task_id_for(name, target);
    let task_text = fill_placeholders(task_src, name, target, abi, &task_id);
    let contract_text = fill_placeholders(contract_src, name, target, abi, &task_id);

    let task_path = case_dir.join("task.vaa.toml");
    let contract_path = case_dir.join("contract.sem.toml");
    write_text(&task_path, &task_text)?;
    write_text(&contract_path, &contract_text)?;

    // Fail closed: filled stubs must validate as tasks.
    let _locked = load_locked_task(&task_path)?;

    let experimental = !meta.known_ci_shape;
    let state = AuthorState {
        schema_version: "0.1".into(),
        state: AuthorCaseState::Draft,
        template: template.to_owned(),
        name: name.to_owned(),
        target: target.to_owned(),
        experimental,
        acceptance: None,
        task_digest: None,
        contract_digest: None,
        capability_snapshot_digest: None,
    };
    write_author_state(&case_dir, &state)?;

    Ok(InitResult {
        ok: true,
        case_dir,
        state,
        task: task_path,
        contract: contract_path,
    })
}

fn contract_digest(case_dir: &Path) -> Result<String, AuthorError> {
    let path = case_dir.join("contract.sem.toml");
    let bytes = fs::read(&path).map_err(|source| AuthorError::Io {
        path: path.clone(),
        source,
    })?;
    Ok(sha256_digest_prefixed(&bytes))
}

fn admission_summary(name: &str, target: &str) -> AdmissionSummary {
    match admit_leaf(name, target, DEFAULT_AUTHOR_ASSEMBLER) {
        Some(entry) => AdmissionSummary {
            leaf: name.to_owned(),
            target: target.to_owned(),
            assembler: DEFAULT_AUTHOR_ASSEMBLER.to_owned(),
            admitted: true,
            tier: Some(entry.tier.as_str().to_owned()),
            acceptance_level: Some(entry.snapshot.acceptance_level),
        },
        None => AdmissionSummary {
            leaf: name.to_owned(),
            target: target.to_owned(),
            assembler: DEFAULT_AUTHOR_ASSEMBLER.to_owned(),
            admitted: false,
            tier: None,
            acceptance_level: None,
        },
    }
}

/// Review a case directory: validate task, digests, admission, issues.
pub fn author_review(case_dir: &Path) -> Result<ReviewResult, AuthorError> {
    let mut issues = Vec::new();
    let state = match load_author_state(case_dir) {
        Ok(s) => Some(s),
        Err(e) => {
            issues.push(format!("AUTHOR_STATE: {e}"));
            None
        }
    };

    let task_path = case_dir.join("task.vaa.toml");
    let (task_digest, task_name, task_target) = match load_locked_task(&task_path) {
        Ok(locked) => {
            let dig = locked.digest().prefixed();
            let name = locked.task().entry.symbol.clone();
            let target = locked.task().target.clone();
            if let Some(s) = &state {
                if s.name != name {
                    issues.push(format!(
                        "AUTHOR_STATE.name `{}` ≠ task entry.symbol `{name}`",
                        s.name
                    ));
                }
                if s.target != target {
                    issues.push(format!(
                        "AUTHOR_STATE.target `{}` ≠ task target `{target}`",
                        s.target
                    ));
                }
            }
            (Some(dig), Some(name), Some(target))
        }
        Err(e) => {
            issues.push(format!("task validation: {e}"));
            (None, None, None)
        }
    };

    let contract_digest = match contract_digest(case_dir) {
        Ok(d) => Some(d),
        Err(e) => {
            issues.push(format!("contract: {e}"));
            None
        }
    };

    if case_dir.join(LOCKED_MARKER_FILE).is_file() {
        if state.as_ref().is_none_or(|s| s.state != AuthorCaseState::Locked) {
            issues.push("LOCKED marker present but AUTHOR_STATE.state is not locked".into());
        }
    }

    let leaf = task_name
        .or_else(|| state.as_ref().map(|s| s.name.clone()))
        .unwrap_or_default();
    let target = task_target
        .or_else(|| state.as_ref().map(|s| s.target.clone()))
        .unwrap_or_default();
    let admission = if leaf.is_empty() || target.is_empty() {
        None
    } else {
        let summary = admission_summary(&leaf, &target);
        if !summary.admitted {
            if state.as_ref().is_some_and(|s| s.experimental) {
                issues.push(format!(
                    "leaf `{leaf}` not admitted for `{target}` (experimental case — lock requires --experimental)"
                ));
            } else {
                issues.push(format!(
                    "leaf `{leaf}` not admitted for `{target}` with assembler `{DEFAULT_AUTHOR_ASSEMBLER}`"
                ));
            }
        }
        Some(summary)
    };

    let snap = snapshot_digest().to_owned();

    let ok = issues.is_empty()
        && task_digest.is_some()
        && contract_digest.is_some()
        && state.is_some();

    Ok(ReviewResult {
        ok,
        case_dir: case_dir.to_path_buf(),
        state,
        task_digest,
        contract_digest,
        capability_snapshot_digest: snap,
        admission,
        issues,
    })
}

/// Lock a case: fail-closed on review issues; require admission or `--experimental`.
pub fn author_lock(case_dir: &Path, experimental: bool) -> Result<LockResult, AuthorError> {
    if case_dir.join(LOCKED_MARKER_FILE).is_file() {
        return Err(AuthorError::msg(format!(
            "case `{}` is already locked",
            case_dir.display()
        )));
    }

    let mut review = author_review(case_dir)?;
    let mut state = review
        .state
        .clone()
        .ok_or_else(|| AuthorError::msg("missing AUTHOR_STATE.toml"))?;

    if state.state == AuthorCaseState::Locked {
        return Err(AuthorError::msg("AUTHOR_STATE already reports locked"));
    }

    // Filter informational experimental-admission notices when locking with --experimental.
    if experimental {
        review.issues.retain(|issue| {
            !(issue.contains("not admitted") && issue.contains("experimental case"))
        });
        // Also drop the generic not-admitted issue for experimental locks.
        review.issues.retain(|issue| !issue.contains("not admitted"));
    }

    if !review.issues.is_empty() {
        return Err(AuthorError::msg(format!(
            "review issues remain (fail-closed):\n  - {}",
            review.issues.join("\n  - ")
        )));
    }

    let task_digest = review
        .task_digest
        .ok_or_else(|| AuthorError::msg("missing task digest"))?;
    let contract_digest = review
        .contract_digest
        .ok_or_else(|| AuthorError::msg("missing contract digest"))?;

    let admission = review
        .admission
        .ok_or_else(|| AuthorError::msg("admission lookup unavailable"))?;

    let acceptance = if admission.admitted && !experimental {
        // Prefer mapped tier; never claim sealed_acceptance from author lock alone.
        let tier = admission
            .tier
            .as_deref()
            .unwrap_or(AdmissionTier::BehavioralAcceptance.as_str());
        if tier == AdmissionTier::SealedAcceptance.as_str() {
            AdmissionTier::BehavioralAcceptance.as_str()
        } else {
            tier
        }
        .to_owned()
    } else if experimental {
        // Explicit experimental: authoring_only; refuse sealed_acceptance.
        if state.experimental || !admission.admitted {
            AdmissionTier::AuthoringOnly.as_str().to_owned()
        } else {
            // Admitted known-CI leaf locked with --experimental still stays authoring_only.
            AdmissionTier::AuthoringOnly.as_str().to_owned()
        }
    } else {
        return Err(AuthorError::msg(format!(
            "leaf `{}` is not admitted for target `{}` (assembler `{DEFAULT_AUTHOR_ASSEMBLER}`). \
Pass --experimental to lock as authoring_only (never sealed_acceptance).",
            state.name, state.target
        )));
    };

    if acceptance == AdmissionTier::SealedAcceptance.as_str() {
        return Err(AuthorError::msg(
            "author lock refuses sealed_acceptance (honesty: lock does not grant seal authority)",
        ));
    }

    state.state = AuthorCaseState::Locked;
    state.acceptance = Some(acceptance.clone());
    state.task_digest = Some(task_digest.clone());
    state.contract_digest = Some(contract_digest.clone());
    state.capability_snapshot_digest = Some(review.capability_snapshot_digest.clone());
    if experimental {
        state.experimental = true;
    }
    write_author_state(case_dir, &state)?;

    let marker_path = case_dir.join(LOCKED_MARKER_FILE);
    let marker = format!(
        "state = \"locked\"\n\
template = \"{}\"\n\
name = \"{}\"\n\
target = \"{}\"\n\
experimental = {}\n\
acceptance = \"{acceptance}\"\n\
task_digest = \"{task_digest}\"\n\
contract_digest = \"{contract_digest}\"\n\
capability_snapshot_digest = \"{}\"\n\
note = \"Human CLI lock only — agents must not lock acceptance authority.\"\n",
        state.template,
        state.name,
        state.target,
        state.experimental,
        review.capability_snapshot_digest
    );
    write_text(&marker_path, &marker)?;

    Ok(LockResult {
        ok: true,
        case_dir: case_dir.to_path_buf(),
        state,
        locked_marker: marker_path,
        acceptance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(prefix: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "{prefix}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn catalog_lists_six_templates() {
        let cat = load_catalog();
        assert_eq!(cat.len(), 6);
        assert!(template_meta("pure-int-binary").unwrap().known_ci_shape);
        assert!(!template_meta("pure-int-ternary").unwrap().known_ci_shape);
    }

    #[test]
    fn init_creates_draft_valid_task() {
        let out = tmp("vaa-author-init");
        let res = author_init(
            "pure-int-binary",
            "max_i64",
            "x86_64-pc-windows-msvc",
            Some(&out),
        )
        .expect("init");
        assert!(res.ok);
        assert_eq!(res.state.state, AuthorCaseState::Draft);
        assert!(!res.state.experimental);
        assert!(res.case_dir.join(AUTHOR_STATE_FILE).is_file());
        load_locked_task(&res.task).expect("validate filled task");
    }

    #[test]
    fn lock_without_admission_fails() {
        let out = tmp("vaa-author-noadm");
        let init = author_init(
            "pure-int-binary",
            "not_a_real_leaf",
            "x86_64-pc-windows-msvc",
            Some(&out),
        )
        .expect("init");
        let err = author_lock(&init.case_dir, false).expect_err("must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("not admitted") || msg.contains("review issues"),
            "unexpected: {msg}"
        );
    }

    #[test]
    fn lock_admitted_max_i64_writes_locked_marker() {
        let out = tmp("vaa-author-lock");
        let init = author_init(
            "pure-int-binary",
            "max_i64",
            "x86_64-pc-windows-msvc",
            Some(&out),
        )
        .expect("init");
        let locked = author_lock(&init.case_dir, false).expect("lock");
        assert!(locked.ok);
        assert_eq!(locked.state.state, AuthorCaseState::Locked);
        assert!(locked.locked_marker.is_file());
        assert_ne!(
            locked.acceptance,
            AdmissionTier::SealedAcceptance.as_str()
        );
        let again = author_init(
            "pure-int-binary",
            "max_i64",
            "x86_64-pc-windows-msvc",
            Some(&out),
        );
        assert!(again.is_err(), "init must refuse locked case");
    }

    #[test]
    fn experimental_lock_sets_authoring_only() {
        let out = tmp("vaa-author-exp");
        let init = author_init(
            "pure-int-ternary",
            "clamp_i64",
            "x86_64-pc-windows-msvc",
            Some(&out),
        )
        .expect("init");
        assert!(init.state.experimental);
        let locked = author_lock(&init.case_dir, true).expect("experimental lock");
        assert_eq!(
            locked.acceptance,
            AdmissionTier::AuthoringOnly.as_str()
        );
    }
}
