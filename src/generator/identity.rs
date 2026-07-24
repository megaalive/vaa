//! Generator binary identity (content digest).

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::evidence::sha256_digest_prefixed;
use crate::generator::error::GeneratorError;
use crate::generator::spec::GeneratorSpec;
use crate::process::{ProcessConfig, ProcessRunner};

/// Established identity for a generator binary on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratorBinaryIdentity {
    /// Absolute path to the hashed binary.
    pub binary_path: PathBuf,
    /// `sha256:<hex>` of file contents.
    pub digest: String,
    /// Byte length of the binary.
    pub size_bytes: u64,
}

/// Resolve `build.generator_binary` relative to the generator repository root.
pub fn resolve_generator_binary(
    repository_root: &Path,
    relative: &str,
) -> Result<PathBuf, GeneratorError> {
    let raw = PathBuf::from(relative);
    let joined = if raw.is_absolute() {
        raw
    } else {
        repository_root.join(raw)
    };
    if !joined.is_file() {
        return Err(GeneratorError::Validation(format!(
            "generator binary not found: {}",
            joined.display()
        )));
    }
    std::fs::canonicalize(&joined).map_err(|source| GeneratorError::Io {
        path: joined,
        source,
    })
}

/// Hash a generator binary and optionally enforce an expected digest.
pub fn establish_binary_identity(
    binary_path: &Path,
    expected_digest: Option<&str>,
) -> Result<GeneratorBinaryIdentity, GeneratorError> {
    let bytes = std::fs::read(binary_path).map_err(|source| GeneratorError::Io {
        path: binary_path.to_path_buf(),
        source,
    })?;
    let digest = sha256_digest_prefixed(&bytes);
    if let Some(expected) = expected_digest {
        if digest != expected {
            return Err(GeneratorError::Validation(format!(
                "generator binary digest mismatch: got `{digest}`, expected `{expected}`"
            )));
        }
    }
    Ok(GeneratorBinaryIdentity {
        binary_path: binary_path.to_path_buf(),
        digest,
        size_bytes: bytes.len() as u64,
    })
}

/// Build the generator using `spec.build.command` in the repository (or working_directory).
pub fn build_generator(spec: &GeneratorSpec, repository_root: &Path) -> Result<(), GeneratorError> {
    if spec.build.command.is_empty() {
        return Err(GeneratorError::Validation(
            "build.command must not be empty".to_owned(),
        ));
    }
    let cwd = match &spec.build.working_directory {
        Some(rel) => {
            let p = if Path::new(rel).is_absolute() {
                PathBuf::from(rel)
            } else {
                // working_directory in pack specs is often sibling to pack; relative to repo root.
                repository_root.join(rel)
            };
            if p.is_dir() {
                p
            } else {
                repository_root.to_path_buf()
            }
        }
        None => repository_root.to_path_buf(),
    };

    let program = &spec.build.command[0];
    let args = spec.build.command[1..].to_vec();
    let config = ProcessConfig {
        program: PathBuf::from(program),
        args,
        working_dir: Some(cwd),
        timeout: Duration::from_secs(spec.build.timeout_seconds.max(1)),
        max_output_bytes: 8 * 1024 * 1024,
        ..ProcessConfig::default()
    };
    let output = ProcessRunner::run(&config)
        .map_err(|e| GeneratorError::Validation(format!("generator build failed to start: {e}")))?;
    if output.timed_out {
        return Err(GeneratorError::Validation(format!(
            "generator build timed out after {}s",
            spec.build.timeout_seconds
        )));
    }
    if output.exit_code != Some(0) {
        return Err(GeneratorError::Validation(format!(
            "generator build exited {:?}: {}",
            output.exit_code,
            output.stderr.chars().take(2000).collect::<String>()
        )));
    }
    Ok(())
}

/// Build (optional) then establish binary identity from the spec.
pub fn build_and_identify(
    spec: &GeneratorSpec,
    repository_root: &Path,
    skip_build: bool,
) -> Result<GeneratorBinaryIdentity, GeneratorError> {
    if !skip_build {
        build_generator(spec, repository_root)?;
    }
    let relative = spec.build.generator_binary.as_deref().ok_or_else(|| {
        GeneratorError::Validation(
            "build.generator_binary is required to establish identity".to_owned(),
        )
    })?;
    let binary = resolve_generator_binary(repository_root, relative)?;
    establish_binary_identity(
        binary.as_path(),
        spec.build.expected_binary_sha256.as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn hashes_binary_and_enforces_expected() {
        let dir = std::env::temp_dir().join(format!(
            "vaa-bin-id-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let bin = dir.join("tool.bin");
        fs::write(&bin, b"hello-generator").unwrap();
        let id = establish_binary_identity(&bin, None).unwrap();
        assert!(id.digest.starts_with("sha256:"));
        assert_eq!(id.size_bytes, 15);
        let again = establish_binary_identity(&bin, Some(&id.digest)).unwrap();
        assert_eq!(again.digest, id.digest);
        let err = establish_binary_identity(
            &bin,
            Some("sha256:0000000000000000000000000000000000000000000000000000000000000000"),
        )
        .unwrap_err();
        assert!(err.to_string().contains("mismatch"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_relative_binary() {
        let dir = std::env::temp_dir().join(format!(
            "vaa-bin-res-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(dir.join("target/release")).unwrap();
        let bin = dir.join("target/release/fake.exe");
        fs::write(&bin, b"x").unwrap();
        let resolved = resolve_generator_binary(&dir, "target/release/fake.exe").unwrap();
        assert!(resolved.ends_with("fake.exe"));
        let _ = fs::remove_dir_all(&dir);
    }
}
