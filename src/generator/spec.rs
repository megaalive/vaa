//! External generator specification (`ExternalGeneratorSpec`).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::generator::error::GeneratorError;
use crate::generator::stack_lock::{
    is_safe_generator_id, validate_exact_revision, validate_sha256_digest,
};

/// Accepted schema version for generator specs.
pub const GENERATOR_SPEC_SCHEMA_VERSION: &str = "0.1";

/// Typed external generator pack specification (generator-agnostic).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratorSpec {
    /// Document schema version.
    pub schema_version: String,
    /// Stable id (matches `[generators.<id>]` in stack.lock when used as key).
    pub generator_id: String,
    /// Optional taxonomy for docs/triage — not a hard VAA enum gate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Generator repository under test.
    pub repository: GeneratorRepository,
    /// Build recipe.
    pub build: BuildSpec,
    /// Deterministic generation recipe.
    pub generation: GenerationSpec,
    /// Digest / toolchain identity requirements.
    #[serde(default)]
    pub identity: IdentityPolicy,
    /// Path allow/deny for repair patches (relative to generator repo).
    #[serde(default)]
    pub patch_policy: PatchPolicy,
}

/// Source repository for the generator under test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratorRepository {
    /// Clone URL or relative path to the generator checkout.
    pub path: String,
    /// Exact revision expected (`git:<commit>`).
    pub expected_revision: String,
    /// Fail if worktree is dirty (enforced in a later chip).
    #[serde(default = "default_true")]
    pub require_clean_worktree: bool,
    /// Allow untracked files when clean-worktree is required.
    #[serde(default)]
    pub allow_untracked_files: bool,
}

fn default_true() -> bool {
    true
}

/// How to build the generator binary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildSpec {
    /// Locked argv (`["cargo", "build", "--release"]`).
    pub command: Vec<String>,
    /// Working directory relative to the pack or absolute label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    /// Relative path to the generator binary after build.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator_binary: Option<String>,
    /// Optional expected digest after build (`sha256:<hex>`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_binary_sha256: Option<String>,
    /// Build timeout in seconds.
    #[serde(default = "default_build_timeout")]
    pub timeout_seconds: u64,
}

fn default_build_timeout() -> u64 {
    600
}

/// How to run generation into a staging directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationSpec {
    /// Locked argv with placeholders `{generator}`, `{input}`, `{target}`, `{output}`.
    pub command: Vec<String>,
    /// Working directory for generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    /// Relative output path under staging (default `candidate.asm`).
    #[serde(default = "default_output_relative")]
    pub output_relative: String,
    /// Remove pre-existing output before generation.
    #[serde(default = "default_true")]
    pub clean_output_directory: bool,
    /// Generation timeout in seconds.
    #[serde(default = "default_gen_timeout")]
    pub timeout_seconds: u64,
}

fn default_output_relative() -> String {
    "candidate.asm".to_owned()
}

fn default_gen_timeout() -> u64 {
    120
}

/// Identity requirements recorded in evidence (enforced later).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityPolicy {
    #[serde(default = "default_true")]
    pub require_generator_digest: bool,
    #[serde(default = "default_true")]
    pub require_build_log_digest: bool,
    #[serde(default = "default_true")]
    pub require_toolchain_identity: bool,
}

impl Default for IdentityPolicy {
    fn default() -> Self {
        Self {
            require_generator_digest: true,
            require_build_log_digest: true,
            require_toolchain_identity: true,
        }
    }
}

/// Patch / repair path policy for this generator.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchPolicy {
    /// Glob allowlist relative to the generator repository.
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    /// Glob denylist (authority / pack files).
    #[serde(default)]
    pub forbidden_paths: Vec<String>,
}

/// Load a generator spec from disk and validate.
pub fn load_generator_spec(path: impl AsRef<Path>) -> Result<GeneratorSpec, GeneratorError> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path).map_err(|source| GeneratorError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let spec = parse_generator_spec(path, &text)?;
    let diagnostics = validate_generator_spec(&spec);
    if !diagnostics.is_empty() {
        return Err(GeneratorError::from_diagnostics(&diagnostics));
    }
    Ok(spec)
}

/// Parse generator spec TOML (path used for diagnostics only).
pub fn parse_generator_spec(path: &Path, text: &str) -> Result<GeneratorSpec, GeneratorError> {
    toml::from_str::<GeneratorSpec>(text).map_err(|error| GeneratorError::Parse {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

/// Validate schema and semantic constraints (load-time only).
#[must_use]
pub fn validate_generator_spec(spec: &GeneratorSpec) -> Vec<String> {
    let mut diagnostics = Vec::new();

    if spec.schema_version != GENERATOR_SPEC_SCHEMA_VERSION {
        diagnostics.push(format!(
            "unsupported schema_version `{}` (accepts only `{GENERATOR_SPEC_SCHEMA_VERSION}`)",
            spec.schema_version
        ));
    }

    let id = &spec.generator_id;
    if id.trim().is_empty() {
        diagnostics.push("generator_id must not be empty".to_owned());
    } else if !is_safe_generator_id(id) {
        diagnostics.push(format!(
            "generator_id `{id}` must match [A-Za-z][A-Za-z0-9._-]{{0,127}}"
        ));
    }

    if spec.repository.path.trim().is_empty() {
        diagnostics.push("repository.path must not be empty".to_owned());
    }
    if let Err(msg) = validate_exact_revision(&spec.repository.expected_revision) {
        diagnostics.push(format!("repository.expected_revision: {msg}"));
    }

    if spec.build.command.is_empty() || spec.build.command.iter().any(|part| part.trim().is_empty())
    {
        diagnostics.push("build.command must be a non-empty argv list".to_owned());
    }
    if let Some(digest) = &spec.build.expected_binary_sha256 {
        if let Err(msg) = validate_sha256_digest(digest) {
            diagnostics.push(format!("build.expected_binary_sha256: {msg}"));
        }
    }
    if spec.build.timeout_seconds == 0 {
        diagnostics.push("build.timeout_seconds must be > 0".to_owned());
    }

    if spec.generation.command.is_empty()
        || spec
            .generation
            .command
            .iter()
            .any(|part| part.trim().is_empty())
    {
        diagnostics.push("generation.command must be a non-empty argv list".to_owned());
    }
    if !spec
        .generation
        .command
        .iter()
        .any(|part| part.contains("{output}"))
    {
        diagnostics.push("generation.command must include an `{output}` placeholder".to_owned());
    }
    if spec.generation.timeout_seconds == 0 {
        diagnostics.push("generation.timeout_seconds must be > 0".to_owned());
    }
    if spec.generation.output_relative.trim().is_empty() {
        diagnostics.push("generation.output_relative must not be empty".to_owned());
    } else if is_forbidden_output_relative(&spec.generation.output_relative) {
        diagnostics
            .push("generation.output_relative must be a relative path without `..`".to_owned());
    }

    for (label, paths) in [
        (
            "patch_policy.allowed_paths",
            &spec.patch_policy.allowed_paths,
        ),
        (
            "patch_policy.forbidden_paths",
            &spec.patch_policy.forbidden_paths,
        ),
    ] {
        for path in paths {
            if path.trim().is_empty() {
                diagnostics.push(format!("{label} contains an empty entry"));
            }
        }
    }

    diagnostics
}

/// Reject absolute paths and `..` segments (portable across Windows/Unix).
fn is_forbidden_output_relative(path: &str) -> bool {
    let trimmed = path.trim();
    if trimmed.starts_with('/') || trimmed.starts_with('\\') {
        return true;
    }
    // Windows drive-absolute (`C:\…` / `C:/…`).
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return true;
    }
    Path::new(trimmed).is_absolute() || trimmed.split(['/', '\\']).any(|p| p == "..")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("integrations")
            .join("hlax64")
            .join(name)
    }

    #[test]
    fn loads_hlax64_generator_spec_fixture() {
        let spec = load_generator_spec(fixture("generator.spec.toml")).expect("valid spec");
        assert_eq!(spec.generator_id, "hlax64");
        assert_eq!(spec.kind.as_deref(), Some("compiler"));
        assert!(spec.repository.require_clean_worktree);
        assert!(!spec.patch_policy.forbidden_paths.is_empty());
    }

    #[test]
    fn rejects_missing_output_placeholder() {
        let text = r#"
schema_version = "0.1"
generator_id = "hlax64"
[repository]
path = "../hlax64"
expected_revision = "git:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
[build]
command = ["cargo", "build", "--release"]
[generation]
command = ["{generator}", "compile", "{input}"]
"#;
        let spec = parse_generator_spec(Path::new("mem"), text).expect("parse");
        let diagnostics = validate_generator_spec(&spec);
        assert!(
            diagnostics.iter().any(|d| d.contains("{output}")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn rejects_absolute_output_relative() {
        let text = r#"
schema_version = "0.1"
generator_id = "hlax64"
[repository]
path = "../hlax64"
expected_revision = "git:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
[build]
command = ["cargo", "build", "--release"]
[generation]
command = ["{generator}", "compile", "{input}", "--output", "{output}"]
output_relative = "/tmp/out.asm"
"#;
        let spec = parse_generator_spec(Path::new("mem"), text).expect("parse");
        let diagnostics = validate_generator_spec(&spec);
        assert!(
            diagnostics.iter().any(|d| d.contains("relative")),
            "{diagnostics:?}"
        );
    }
}
