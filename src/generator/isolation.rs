//! Stronger isolation for generator build/generate (plan §20).
//!
//! Shared / agent-adjacent execution must not inherit credentials.
//! ProcessRunner already `env_clear`s; this module supplies the **generator
//! profile allowlist** and a credential deny registry used for audits and
//! to refuse forbidden `extra_env` injections.

use serde::{Deserialize, Serialize};

use crate::generator::error::GeneratorError;

/// Exact environment variable names that must never reach a generator
/// subprocess (case-insensitive match).
pub const CREDENTIAL_ENV_EXACT: &[&str] = &[
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "GEMINI_API_KEY",
    "GOOGLE_API_KEY",
    "HF_TOKEN",
    "HUGGING_FACE_HUB_TOKEN",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "AZURE_CLIENT_SECRET",
    "GITHUB_TOKEN",
    "GH_TOKEN",
    "NPM_TOKEN",
    "CARGO_REGISTRY_TOKEN",
    "SSH_AUTH_SOCK",
    "SSH_AGENT_PID",
    "DOCKER_AUTH_CONFIG",
    "KUBECONFIG",
    "PGPASSWORD",
    "MYSQL_PWD",
];

/// Prefixes (case-insensitive) treated as credential-bearing.
pub const CREDENTIAL_ENV_PREFIXES: &[&str] = &[
    "OPENAI_",
    "ANTHROPIC_",
    "AWS_",
    "AZURE_",
    "GEMINI_",
    "GOOGLE_APPLICATION_",
    "HF_",
    "HUGGINGFACE_",
    "VAA_LIVE_",
    "SEMASM_LIVE_",
];

/// Suffixes (case-insensitive) that usually mark secrets.
pub const CREDENTIAL_ENV_SUFFIXES: &[&str] = &[
    "_API_KEY",
    "_ACCESS_KEY",
    "_SECRET",
    "_SECRET_KEY",
    "_TOKEN",
    "_PASSWORD",
    "_PASSWD",
    "_CREDENTIALS",
];

/// Whether an environment variable name looks credential-bearing.
#[must_use]
pub fn is_credential_env_name(name: &str) -> bool {
    let upper = name.trim().to_ascii_uppercase();
    if upper.is_empty() {
        return false;
    }
    if CREDENTIAL_ENV_EXACT
        .iter()
        .any(|exact| exact.eq_ignore_ascii_case(&upper))
    {
        return true;
    }
    if CREDENTIAL_ENV_PREFIXES
        .iter()
        .any(|prefix| upper.starts_with(prefix))
    {
        return true;
    }
    CREDENTIAL_ENV_SUFFIXES
        .iter()
        .any(|suffix| upper.ends_with(suffix))
}

/// Allowlist for generator build/generate subprocesses (plan §20).
///
/// Broader than the default SemASM allowlist so `cmd` / toolchains can run,
/// but still closed: credentials never pass unless explicitly (and
/// incorrectly) added here — audits catch that via [`is_credential_env_name`].
#[must_use]
pub fn generator_subprocess_allowed_env() -> Vec<String> {
    let mut names = vec![
        "PATH".to_owned(),
        "PATHEXT".to_owned(),
        "HOME".to_owned(),
        "USER".to_owned(),
        "USERNAME".to_owned(),
        "USERPROFILE".to_owned(),
        "HOMEDRIVE".to_owned(),
        "HOMEPATH".to_owned(),
        "TMP".to_owned(),
        "TEMP".to_owned(),
        "TMPDIR".to_owned(),
        "SystemRoot".to_owned(),
        "SYSTEMROOT".to_owned(),
        "windir".to_owned(),
        "COMSPEC".to_owned(),
        "OS".to_owned(),
        "NUMBER_OF_PROCESSORS".to_owned(),
        "PROCESSOR_ARCHITECTURE".to_owned(),
        "PROCESSOR_IDENTIFIER".to_owned(),
        "ProgramFiles".to_owned(),
        "ProgramFiles(x86)".to_owned(),
        "ProgramData".to_owned(),
        "LOCALAPPDATA".to_owned(),
        // Needed for some toolchains; still scrubbed if credential-shaped.
        "APPDATA".to_owned(),
        "LANG".to_owned(),
        "LC_ALL".to_owned(),
        "TERM".to_owned(),
        "RUSTUP_HOME".to_owned(),
        "CARGO_HOME".to_owned(),
        "DOTNET_ROOT".to_owned(),
        "DOTNET_CLI_HOME".to_owned(),
        "DOTNET_NOLOGO".to_owned(),
        "DOTNET_SKIP_FIRST_TIME_EXPERIENCE".to_owned(),
        "DOTNET_CLI_TELEMETRY_OPTOUT".to_owned(),
    ];
    names.retain(|name| !is_credential_env_name(name));
    names.sort();
    names.dedup();
    names
}

/// Refuse credential-shaped keys in an `extra_env` injection list.
pub fn reject_credential_extra_env(extra_env: &[(String, String)]) -> Result<(), GeneratorError> {
    let bad: Vec<&str> = extra_env
        .iter()
        .filter_map(|(k, _)| {
            if is_credential_env_name(k) {
                Some(k.as_str())
            } else {
                None
            }
        })
        .collect();
    if bad.is_empty() {
        return Ok(());
    }
    Err(GeneratorError::Validation(format!(
        "refusing to inject credential env into generator subprocess: {}",
        bad.join(", ")
    )))
}

/// Audit view of the current process environment under isolation rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsolationAudit {
    pub allowed_for_generator: Vec<String>,
    pub present_allowed: Vec<String>,
    pub present_credential_denied: Vec<String>,
    pub present_other_stripped: Vec<String>,
}

/// Classify the current process environment for generator isolation.
#[must_use]
pub fn audit_current_environment() -> IsolationAudit {
    let allowed = generator_subprocess_allowed_env();
    let allowed_set: std::collections::BTreeSet<String> =
        allowed.iter().map(|s| s.to_ascii_uppercase()).collect();

    let mut present_allowed = Vec::new();
    let mut present_credential_denied = Vec::new();
    let mut present_other_stripped = Vec::new();

    for (key, _) in std::env::vars() {
        let upper = key.to_ascii_uppercase();
        if is_credential_env_name(&key) {
            present_credential_denied.push(key);
        } else if allowed_set.contains(&upper) {
            present_allowed.push(key);
        } else {
            present_other_stripped.push(key);
        }
    }
    present_allowed.sort();
    present_credential_denied.sort();
    present_other_stripped.sort();

    IsolationAudit {
        allowed_for_generator: allowed,
        present_allowed,
        present_credential_denied,
        present_other_stripped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_exact_and_suffix_credentials() {
        assert!(is_credential_env_name("OPENAI_API_KEY"));
        assert!(is_credential_env_name("openai_api_key"));
        assert!(is_credential_env_name("MY_SERVICE_TOKEN"));
        assert!(is_credential_env_name("AWS_SECRET_ACCESS_KEY"));
        assert!(is_credential_env_name("VAA_LIVE_KEY"));
        assert!(!is_credential_env_name("PATH"));
        assert!(!is_credential_env_name("CARGO_HOME"));
        assert!(!is_credential_env_name("SystemRoot"));
    }

    #[test]
    fn allowlist_excludes_credential_shaped_names() {
        let allowed = generator_subprocess_allowed_env();
        assert!(allowed.iter().any(|n| n == "PATH"));
        assert!(allowed.iter().any(|n| n == "SystemRoot"));
        assert!(allowed.iter().all(|n| !is_credential_env_name(n)));
    }

    #[test]
    fn rejects_credential_extra_env() {
        let err =
            reject_credential_extra_env(&[("OPENAI_API_KEY".into(), "x".into())]).unwrap_err();
        assert!(err.to_string().contains("OPENAI_API_KEY"));
        assert!(reject_credential_extra_env(&[("PATH".into(), "/bin".into())]).is_ok());
    }

    #[test]
    fn audit_runs_without_panic() {
        let audit = audit_current_environment();
        assert!(!audit.allowed_for_generator.is_empty());
    }
}
