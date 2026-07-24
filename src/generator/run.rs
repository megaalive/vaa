//! End-to-end `generator-run`: guard → build/identity → generate → ingest.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::generator::error::GeneratorError;
use crate::generator::generate::{generate_candidate, GenerationOutcome, GenerationRequest};
use crate::generator::identity::{build_and_identify, GeneratorBinaryIdentity};
use crate::generator::repo_guard::{check_repository, RepoGuardConfig, RepoGuardReport};
use crate::generator::spec::{load_generator_spec, GeneratorSpec};
use crate::generator::stack_lock::{load_stack_lock, StackLock};
use crate::run::{ingest_candidate, RunDir, VerifySealError, VerifySealOutcome};
use crate::task::{load_locked_task, LockedTask, TaskError};

/// Configuration for one generator-run case.
#[derive(Debug, Clone)]
pub struct GeneratorRunConfig {
    pub spec_path: PathBuf,
    pub lock_path: Option<PathBuf>,
    pub task_path: PathBuf,
    pub contract_path: PathBuf,
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub run_base: PathBuf,
    pub repo_override: Option<PathBuf>,
    pub skip_repo_guard: bool,
    pub skip_build: bool,
    pub skip_verify: bool,
    pub allow_execution: bool,
    pub check_deterministic: bool,
    pub target_override: Option<String>,
}

/// Full outcome of `generator-run` (verify may be skipped).
#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratorRunOutcome {
    pub generator_id: String,
    pub lock: Option<StackLock>,
    pub repo_guard: Option<RepoGuardReport>,
    pub identity: GeneratorBinaryIdentity,
    pub generation: GenerationOutcome,
    pub verify: Option<VerifySummary>,
    pub run_root: Option<PathBuf>,
}

/// Compact verify summary (avoid embedding full evidence in unit tests).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifySummary {
    pub final_status: String,
    pub acceptance_digest: String,
    pub candidate_dir: PathBuf,
}

/// Errors from the generator-run pipeline.
#[derive(Debug, thiserror::Error)]
pub enum GeneratorRunError {
    #[error(transparent)]
    Generator(#[from] GeneratorError),
    #[error(transparent)]
    Task(#[from] TaskError),
    #[error(transparent)]
    Verify(#[from] VerifySealError),
    #[error("run directory error: {0}")]
    RunDir(String),
}

/// Execute guard → identity → generate → (optional) ingest/verify.
pub fn run_generator_case(
    config: &GeneratorRunConfig,
) -> Result<GeneratorRunOutcome, GeneratorRunError> {
    let spec = load_generator_spec(&config.spec_path)?;
    let lock = match &config.lock_path {
        Some(path) => Some(load_stack_lock(path)?),
        None => None,
    };

    let mut guard_config = RepoGuardConfig::from_spec(&spec, &config.spec_path)?;
    if let Some(repo) = &config.repo_override {
        guard_config.repository_path =
            std::fs::canonicalize(repo).map_err(|source| GeneratorError::Io {
                path: repo.clone(),
                source,
            })?;
    }
    let repo_root = guard_config.repository_path.clone();

    let repo_guard = if config.skip_repo_guard {
        None
    } else {
        Some(check_repository(&guard_config)?)
    };

    let identity = build_and_identify(&spec, &repo_root, config.skip_build)?;

    let target = if let Some(t) = &config.target_override {
        t.clone()
    } else {
        load_locked_task(&config.task_path)?.task().target.clone()
    };

    let generation = generate_candidate(
        &spec,
        &GenerationRequest {
            generator_binary: identity.binary_path.clone(),
            input: config.input_path.clone(),
            target,
            output: config.output_path.clone(),
            working_directory: Some(repo_root.clone()),
            clean_output: spec.generation.clean_output_directory,
            check_deterministic: config.check_deterministic,
        },
    )?;

    if config.skip_verify {
        return Ok(GeneratorRunOutcome {
            generator_id: spec.generator_id,
            lock,
            repo_guard,
            identity,
            generation,
            verify: None,
            run_root: None,
        });
    }

    let locked = load_locked_task(&config.task_path)?;
    let (verify, run_root) = verify_generated(&locked, config, &spec, &generation)?;

    Ok(GeneratorRunOutcome {
        generator_id: spec.generator_id,
        lock,
        repo_guard,
        identity,
        generation,
        verify: Some(verify),
        run_root: Some(run_root),
    })
}

fn verify_generated(
    locked: &LockedTask,
    config: &GeneratorRunConfig,
    spec: &GeneratorSpec,
    generation: &GenerationOutcome,
) -> Result<(VerifySummary, PathBuf), GeneratorRunError> {
    let run_id = crate::run::RunId::generate();
    let run_dir = RunDir::create(&config.run_base, &run_id)
        .map_err(|e| GeneratorRunError::RunDir(e.to_string()))?;
    let outcome: VerifySealOutcome = ingest_candidate(
        locked,
        &config.task_path,
        &config.contract_path,
        &generation.output_path,
        &run_dir,
        run_id.as_str(),
        &spec.generator_id,
        locked.task().budgets.max_candidates.max(1),
        config.allow_execution,
    )?;
    Ok((
        VerifySummary {
            final_status: format!("{:?}", outcome.evidence.final_status),
            acceptance_digest: outcome.seal.acceptance_digest.clone(),
            candidate_dir: outcome.candidate_dir.clone(),
        },
        run_dir.root().to_path_buf(),
    ))
}

/// Resolve a path relative to a base (for pack-relative cases).
#[must_use]
pub fn resolve_maybe_relative(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn generator_run_skip_verify_with_copy_tool() {
        let dir = std::env::temp_dir().join(format!(
            "vaa-genrun-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let tool = dir.join("tool.bin");
        let input = dir.join("in.txt");
        let output = dir.join("out.asm");
        fs::write(&tool, b"tool").unwrap();
        fs::write(&input, b"ret\n").unwrap();

        let spec_path = dir.join("generator.spec.toml");
        let spec_toml = format!(
            r#"
schema_version = "0.1"
generator_id = "fake"
kind = "other"
[repository]
path = "."
expected_revision = "git:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
require_clean_worktree = false
allow_untracked_files = true
[build]
command = ["echo", "skip"]
generator_binary = "tool.bin"
timeout_seconds = 30
[generation]
command = {command}
output_relative = "out.asm"
clean_output_directory = true
timeout_seconds = 30
"#,
            command = if cfg!(windows) {
                r#"["cmd", "/C", "copy", "/Y", "{input}", "{output}"]"#
            } else {
                r#"["cp", "{input}", "{output}"]"#
            }
        );
        fs::write(&spec_path, spec_toml).unwrap();

        // Minimal dummy task/contract paths (unused when skip_verify).
        let task = dir.join("task.vaa.toml");
        let contract = dir.join("c.sem.toml");
        fs::write(&task, "").unwrap();
        fs::write(&contract, "").unwrap();

        let outcome = run_generator_case(&GeneratorRunConfig {
            spec_path,
            lock_path: None,
            task_path: task,
            contract_path: contract,
            input_path: input,
            output_path: output.clone(),
            run_base: dir.clone(),
            repo_override: Some(dir.clone()),
            skip_repo_guard: true,
            skip_build: true,
            skip_verify: true,
            allow_execution: false,
            check_deterministic: false,
            target_override: Some("x86_64-pc-windows-msvc".to_owned()),
        })
        .expect("run");
        assert!(output.is_file());
        assert_eq!(outcome.generator_id, "fake");
        assert!(outcome.verify.is_none());
        let _ = fs::remove_dir_all(&dir);
    }
}
