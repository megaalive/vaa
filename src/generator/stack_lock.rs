//! Multi-generator stack lock (`stack.lock.toml`).

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::canonical_json::canonical_json_bytes;
use crate::generator::error::GeneratorError;

/// Accepted schema version for stack locks.
pub const STACK_LOCK_SCHEMA_VERSION: &str = "0.1";

/// Exact pins for VAA, SemASM, toolchain, and one or more generators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackLock {
    /// Document schema version.
    pub schema_version: String,
    /// VAA controller pin.
    pub vaa: ComponentPin,
    /// SemASM verifier pin.
    pub semasm: ComponentPin,
    /// Optional toolchain identity strings (informational + mismatch policy later).
    #[serde(default)]
    pub toolchain: ToolchainPin,
    /// Named generator pins (`[generators.<id>]`).
    #[serde(default)]
    pub generators: BTreeMap<String, GeneratorPin>,
}

/// Pin for VAA or SemASM (repository + exact revision; optional binary digest).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentPin {
    /// Repository URL or path label.
    pub repository: String,
    /// Exact revision (`git:<commit>`). Floating refs rejected.
    pub revision: String,
    /// Optional `sha256:<hex>` of the sealed/released binary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary_sha256: Option<String>,
}

/// Pin for one external generator under test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratorPin {
    /// Repository URL or path label.
    pub repository: String,
    /// Exact revision (`git:<commit>`).
    pub revision: String,
    /// Optional binary digest after build identity is established.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary_sha256: Option<String>,
}

/// Toolchain identity strings (not a substitute for exact generator pins).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolchainPin {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nasm: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lld: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rust: Option<String>,
}

/// Prefixed digest of the canonical stack lock encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackLockDigest {
    /// Lowercase hex SHA-256.
    pub hex: String,
}

impl StackLockDigest {
    /// Prefixed form `sha256:<hex>`.
    #[must_use]
    pub fn prefixed(&self) -> String {
        format!("sha256:{}", self.hex)
    }
}

/// Load a stack lock from disk and validate.
pub fn load_stack_lock(path: impl AsRef<Path>) -> Result<StackLock, GeneratorError> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path).map_err(|source| GeneratorError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let lock = parse_stack_lock(path, &text)?;
    let diagnostics = validate_stack_lock(&lock);
    if !diagnostics.is_empty() {
        return Err(GeneratorError::from_diagnostics(&diagnostics));
    }
    Ok(lock)
}

/// Parse stack lock TOML (path used for diagnostics only).
pub fn parse_stack_lock(path: &Path, text: &str) -> Result<StackLock, GeneratorError> {
    toml::from_str::<StackLock>(text).map_err(|error| GeneratorError::Parse {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

/// Digest over `vaa-canonical-json-v1` encoding of the lock.
#[must_use]
pub fn stack_lock_digest(lock: &StackLock) -> StackLockDigest {
    let hash = Sha256::digest(canonical_json_bytes(lock));
    StackLockDigest {
        hex: hex_encode(&hash),
    }
}

/// Validate schema and reject floating revisions.
#[must_use]
pub fn validate_stack_lock(lock: &StackLock) -> Vec<String> {
    let mut diagnostics = Vec::new();

    if lock.schema_version != STACK_LOCK_SCHEMA_VERSION {
        diagnostics.push(format!(
            "unsupported schema_version `{}` (accepts only `{STACK_LOCK_SCHEMA_VERSION}`)",
            lock.schema_version
        ));
    }

    validate_component_pin("vaa", &lock.vaa, &mut diagnostics);
    validate_component_pin("semasm", &lock.semasm, &mut diagnostics);

    if lock.generators.is_empty() {
        diagnostics
            .push("generators must contain at least one entry (`[generators.<id>]`)".to_owned());
    }

    for (id, pin) in &lock.generators {
        if id.trim().is_empty() {
            diagnostics.push("generators contains an empty key".to_owned());
            continue;
        }
        if !is_safe_generator_id(id) {
            diagnostics.push(format!(
                "generators.{id}: id must match [A-Za-z][A-Za-z0-9._-]{{0,127}}"
            ));
        }
        validate_generator_pin(id, pin, &mut diagnostics);
    }

    diagnostics
}

fn validate_component_pin(label: &str, pin: &ComponentPin, diagnostics: &mut Vec<String>) {
    if pin.repository.trim().is_empty() {
        diagnostics.push(format!("{label}.repository must not be empty"));
    }
    if let Err(msg) = validate_exact_revision(&pin.revision) {
        diagnostics.push(format!("{label}.revision: {msg}"));
    }
    if let Some(digest) = &pin.binary_sha256 {
        if let Err(msg) = validate_sha256_digest(digest) {
            diagnostics.push(format!("{label}.binary_sha256: {msg}"));
        }
    }
}

fn validate_generator_pin(id: &str, pin: &GeneratorPin, diagnostics: &mut Vec<String>) {
    if pin.repository.trim().is_empty() {
        diagnostics.push(format!("generators.{id}.repository must not be empty"));
    }
    if let Err(msg) = validate_exact_revision(&pin.revision) {
        diagnostics.push(format!("generators.{id}.revision: {msg}"));
    }
    if let Some(digest) = &pin.binary_sha256 {
        if let Err(msg) = validate_sha256_digest(digest) {
            diagnostics.push(format!("generators.{id}.binary_sha256: {msg}"));
        }
    }
}

/// Exact `git:<hex>` revisions only — reject floating names.
pub(crate) fn validate_exact_revision(revision: &str) -> Result<(), String> {
    let trimmed = revision.trim();
    if trimmed.is_empty() {
        return Err("must not be empty".to_owned());
    }
    let lower = trimmed.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "main"
            | "master"
            | "latest"
            | "head"
            | "git:main"
            | "git:master"
            | "git:latest"
            | "git:head"
    ) {
        return Err(format!(
            "floating revision `{trimmed}` is rejected (use git:<exact-commit>)"
        ));
    }
    let Some(rest) = trimmed.strip_prefix("git:") else {
        return Err(format!("must be `git:<exact-commit>`, got `{trimmed}`"));
    };
    if rest.is_empty() {
        return Err("git: prefix requires a commit id".to_owned());
    }
    if !(7..=64).contains(&rest.len()) || !rest.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("commit id must be 7–64 hex digits, got `{rest}`"));
    }
    Ok(())
}

pub(crate) fn validate_sha256_digest(digest: &str) -> Result<(), String> {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return Err("must be prefixed `sha256:<hex>`".to_owned());
    };
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("sha256 digest must be 64 hex digits".to_owned());
    }
    Ok(())
}

pub(crate) fn is_safe_generator_id(id: &str) -> bool {
    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    id.len() <= 128 && chars.all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
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
    fn loads_hlax64_stack_lock_fixture() {
        let lock = load_stack_lock(fixture("stack.lock.toml")).expect("valid lock");
        assert_eq!(lock.schema_version, STACK_LOCK_SCHEMA_VERSION);
        assert!(lock.generators.contains_key("hlax64"));
        let digest = stack_lock_digest(&lock);
        assert_eq!(digest.hex.len(), 64);
        assert!(digest.prefixed().starts_with("sha256:"));
    }

    #[test]
    fn rejects_floating_main_revision() {
        let text = r#"
schema_version = "0.1"
[vaa]
repository = "https://github.com/megaalive/vaa"
revision = "git:main"
[semasm]
repository = "https://github.com/megaalive/semasm"
revision = "git:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
[generators.hlax64]
repository = "https://github.com/megaalive/hlax64"
revision = "git:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
"#;
        let lock = parse_stack_lock(Path::new("mem"), text).expect("parse");
        let diagnostics = validate_stack_lock(&lock);
        assert!(
            diagnostics.iter().any(|d| d.contains("floating")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn rejects_empty_generators() {
        let text = r#"
schema_version = "0.1"
[vaa]
repository = "https://github.com/megaalive/vaa"
revision = "git:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
[semasm]
repository = "https://github.com/megaalive/semasm"
revision = "git:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
"#;
        let lock = parse_stack_lock(Path::new("mem"), text).expect("parse");
        let diagnostics = validate_stack_lock(&lock);
        assert!(
            diagnostics.iter().any(|d| d.contains("at least one")),
            "{diagnostics:?}"
        );
    }
}
