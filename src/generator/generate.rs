//! Deterministic candidate generation from a locked generator command.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::evidence::sha256_digest_prefixed;
use crate::generator::error::GeneratorError;
use crate::generator::spec::GeneratorSpec;
use crate::process::{ProcessConfig, ProcessRunner};

/// Inputs required to expand and run a generation command.
#[derive(Debug, Clone)]
pub struct GenerationRequest {
    /// Absolute path to the generator binary (`{generator}`).
    pub generator_binary: PathBuf,
    /// Absolute path to the primary input (`{input}`).
    pub input: PathBuf,
    /// Target triple or ABI label (`{target}`).
    pub target: String,
    /// Absolute path for the generated assembly (`{output}`).
    pub output: PathBuf,
    /// Optional working directory override.
    pub working_directory: Option<PathBuf>,
    /// When true, delete `output` before generation if it exists.
    pub clean_output: bool,
    /// When true, run generation twice and require identical digests.
    pub check_deterministic: bool,
}

/// Result of a locked generation run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationOutcome {
    /// Expanded argv actually executed.
    pub command: Vec<String>,
    /// Output path produced.
    pub output_path: PathBuf,
    /// `sha256:<hex>` of output bytes.
    pub candidate_digest: String,
    /// Output size in bytes.
    pub size_bytes: u64,
    /// True when a twin run was performed and matched.
    pub deterministic_checked: bool,
}

/// Expand placeholders in a command template.
#[must_use]
pub fn expand_generation_command(
    template: &[String],
    placeholders: &BTreeMap<&str, &str>,
) -> Vec<String> {
    template
        .iter()
        .map(|part| {
            let mut out = part.clone();
            for (key, value) in placeholders {
                let token = format!("{{{key}}}");
                out = out.replace(&token, value);
            }
            out
        })
        .collect()
}

/// Run the locked generation command and hash the candidate.
pub fn generate_candidate(
    spec: &GeneratorSpec,
    request: &GenerationRequest,
) -> Result<GenerationOutcome, GeneratorError> {
    if !request.input.is_file() {
        return Err(GeneratorError::Validation(format!(
            "generation input not found: {}",
            request.input.display()
        )));
    }
    if !request.generator_binary.is_file() {
        return Err(GeneratorError::Validation(format!(
            "generator binary not found: {}",
            request.generator_binary.display()
        )));
    }

    if let Some(parent) = request.output.parent() {
        std::fs::create_dir_all(parent).map_err(|source| GeneratorError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    if request.clean_output && request.output.exists() {
        std::fs::remove_file(&request.output).map_err(|source| GeneratorError::Io {
            path: request.output.clone(),
            source,
        })?;
    } else if request.output.exists() && !request.clean_output {
        return Err(GeneratorError::Validation(format!(
            "output already exists and clean_output_directory=false: {}",
            request.output.display()
        )));
    }

    let generator = path_str(&request.generator_binary);
    let input = path_str(&request.input);
    let output = path_str(&request.output);
    let mut placeholders = BTreeMap::new();
    placeholders.insert("generator", generator.as_str());
    placeholders.insert("input", input.as_str());
    placeholders.insert("output", output.as_str());
    placeholders.insert("target", request.target.as_str());

    let command = expand_generation_command(&spec.generation.command, &placeholders);
    if command.is_empty() {
        return Err(GeneratorError::Validation(
            "generation.command expanded to empty argv".to_owned(),
        ));
    }

    let cwd = request
        .working_directory
        .clone()
        .or_else(|| {
            spec.generation
                .working_directory
                .as_ref()
                .map(PathBuf::from)
        })
        .or_else(|| request.generator_binary.parent().map(Path::to_path_buf));

    run_generation_argv(&command, cwd.as_deref(), spec.generation.timeout_seconds)?;

    if !request.output.is_file() {
        return Err(GeneratorError::Validation(format!(
            "generation did not produce output: {}",
            request.output.display()
        )));
    }

    let first = hash_file(&request.output)?;
    let mut deterministic_checked = false;
    if request.check_deterministic {
        std::fs::remove_file(&request.output).map_err(|source| GeneratorError::Io {
            path: request.output.clone(),
            source,
        })?;
        run_generation_argv(&command, cwd.as_deref(), spec.generation.timeout_seconds)?;
        let second = hash_file(&request.output)?;
        if second.digest != first.digest {
            return Err(GeneratorError::Validation(format!(
                "candidate non-deterministic: first `{}` != second `{}`",
                first.digest, second.digest
            )));
        }
        deterministic_checked = true;
    }

    Ok(GenerationOutcome {
        command,
        output_path: request.output.clone(),
        candidate_digest: first.digest,
        size_bytes: first.size_bytes,
        deterministic_checked,
    })
}

struct FileHash {
    digest: String,
    size_bytes: u64,
}

fn hash_file(path: &Path) -> Result<FileHash, GeneratorError> {
    let bytes = std::fs::read(path).map_err(|source| GeneratorError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(FileHash {
        digest: sha256_digest_prefixed(&bytes),
        size_bytes: bytes.len() as u64,
    })
}

fn run_generation_argv(
    command: &[String],
    cwd: Option<&Path>,
    timeout_seconds: u64,
) -> Result<(), GeneratorError> {
    let program = &command[0];
    let args = command[1..].to_vec();
    let config = ProcessConfig {
        program: PathBuf::from(program),
        args,
        working_dir: cwd.map(Path::to_path_buf),
        timeout: Duration::from_secs(timeout_seconds.max(1)),
        max_output_bytes: 4 * 1024 * 1024,
        ..ProcessConfig::default()
    };
    let output = ProcessRunner::run(&config)
        .map_err(|e| GeneratorError::Validation(format!("generation failed to start: {e}")))?;
    if output.timed_out {
        return Err(GeneratorError::Validation(format!(
            "generation timed out after {timeout_seconds}s"
        )));
    }
    if output.exit_code != Some(0) {
        return Err(GeneratorError::Validation(format!(
            "generation exited {:?}: {}",
            output.exit_code,
            output.stderr.chars().take(2000).collect::<String>()
        )));
    }
    Ok(())
}

fn path_str(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::spec::{
        BuildSpec, GenerationSpec, GeneratorRepository, GeneratorSpec, IdentityPolicy, PatchPolicy,
    };
    use std::fs;

    fn sample_spec(command: Vec<String>) -> GeneratorSpec {
        GeneratorSpec {
            schema_version: "0.1".to_owned(),
            generator_id: "fake".to_owned(),
            kind: Some("other".to_owned()),
            repository: GeneratorRepository {
                path: ".".to_owned(),
                expected_revision: "git:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
                require_clean_worktree: false,
                allow_untracked_files: true,
            },
            build: BuildSpec {
                command: vec!["true".to_owned()],
                working_directory: None,
                generator_binary: Some("tool".to_owned()),
                expected_binary_sha256: None,
                timeout_seconds: 60,
            },
            generation: GenerationSpec {
                command,
                working_directory: None,
                output_relative: "candidate.asm".to_owned(),
                clean_output_directory: true,
                timeout_seconds: 60,
            },
            identity: IdentityPolicy::default(),
            patch_policy: PatchPolicy::default(),
        }
    }

    #[test]
    fn expands_placeholders() {
        let mut map = BTreeMap::new();
        map.insert("generator", "g.exe");
        map.insert("input", "in.hlx");
        map.insert("output", "out.asm");
        map.insert("target", "x86_64");
        let expanded = expand_generation_command(
            &[
                "{generator}".to_owned(),
                "compile".to_owned(),
                "{input}".to_owned(),
                "--output".to_owned(),
                "{output}".to_owned(),
                "--target".to_owned(),
                "{target}".to_owned(),
            ],
            &map,
        );
        assert_eq!(
            expanded,
            vec!["g.exe", "compile", "in.hlx", "--output", "out.asm", "--target", "x86_64"]
        );
    }

    #[test]
    fn generates_candidate_via_copy_tool() {
        let dir = std::env::temp_dir().join(format!(
            "vaa-gen-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let tool = dir.join("tool.bin");
        let input = dir.join("input.txt");
        let output = dir.join("out.asm");
        fs::write(&tool, b"tool").unwrap();
        fs::write(&input, b"; generated\nret\n").unwrap();

        // Use the current process's ability to copy via a small Rust-less approach:
        // on Windows `cmd /C copy`, elsewhere `cp`.
        let command = if cfg!(windows) {
            vec![
                "cmd".to_owned(),
                "/C".to_owned(),
                "copy".to_owned(),
                "/Y".to_owned(),
                "{input}".to_owned(),
                "{output}".to_owned(),
            ]
        } else {
            vec!["cp".to_owned(), "{input}".to_owned(), "{output}".to_owned()]
        };
        let spec = sample_spec(command);
        let outcome = generate_candidate(
            &spec,
            &GenerationRequest {
                generator_binary: tool,
                input,
                target: "x86_64-pc-windows-msvc".to_owned(),
                output: output.clone(),
                working_directory: Some(dir.clone()),
                clean_output: true,
                check_deterministic: true,
            },
        )
        .expect("generate");
        assert!(output.is_file());
        assert!(outcome.candidate_digest.starts_with("sha256:"));
        assert!(outcome.deterministic_checked);
        let _ = fs::remove_dir_all(&dir);
    }
}
