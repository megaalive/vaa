use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::process::{ProcessError, ProcessRunner};

use super::backend::{SandboxBackend, SandboxConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub signal: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("execution not enabled; pass --allow-execution to opt in")]
    NotEnabled,
    #[error("sandbox unavailable: {0}")]
    SandboxUnavailable(String),
    #[error("process error: {0}")]
    ProcessError(#[from] ProcessError),
    #[error("binary not found: {0}")]
    BinaryNotFound(String),
}

pub struct ExecutionSandbox {
    pub enabled: bool,
    pub backend: Box<dyn SandboxBackend>,
    pub config: SandboxConfig,
}

impl ExecutionSandbox {
    pub fn new(backend: Box<dyn SandboxBackend>) -> Self {
        Self {
            enabled: false,
            backend,
            config: SandboxConfig::default(),
        }
    }

    pub fn with_config(backend: Box<dyn SandboxBackend>, config: SandboxConfig) -> Self {
        Self {
            enabled: false,
            backend,
            config,
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn run(&self, binary: &Path, args: &[String], timeout: Duration) -> Result<ExecutionResult, ExecutionError> {
        if !self.enabled {
            return Err(ExecutionError::NotEnabled);
        }

        if !binary.is_absolute() && !binary.exists() {
            let found = find_on_path(binary);
            if found.is_none() && !binary.exists() {
                return Err(ExecutionError::BinaryNotFound(
                    binary.to_string_lossy().to_string(),
                ));
            }
        } else if !binary.exists() {
            return Err(ExecutionError::BinaryNotFound(
                binary.to_string_lossy().to_string(),
            ));
        }

        if !self.backend.is_available() {
            return Err(ExecutionError::SandboxUnavailable(
                self.backend.name().to_owned(),
            ));
        }

        let mut sandbox_cfg = self.config.clone();
        sandbox_cfg.timeout = timeout;
        sandbox_cfg.network_disabled = true;

        let pc = self.backend.wrap_process(
            &binary.to_string_lossy(),
            args,
            &sandbox_cfg,
        );

        let output = ProcessRunner::run(&pc)?;

        let signal = if output.exit_code.is_none() && !output.timed_out {
            Some("unknown".to_owned())
        } else {
            None
        };

        Ok(ExecutionResult {
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
            timed_out: output.timed_out,
            signal,
        })
    }
}

fn find_on_path(name: &Path) -> Option<PathBuf> {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
            let with_ext = format!("{}.exe", name.to_string_lossy());
            let candidate_exe = dir.join(&with_ext);
            if candidate_exe.is_file() {
                return Some(candidate_exe);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::backend::LocalBackend;
    use std::path::PathBuf;

    #[test]
    fn execution_requires_opt_in() {
        let backend = Box::new(LocalBackend);
        let sandbox = ExecutionSandbox::new(backend);
        let result = sandbox.run(Path::new("test"), &[], Duration::from_secs(5));
        assert!(matches!(result, Err(ExecutionError::NotEnabled)));
    }

    #[test]
    fn execution_fails_on_missing_binary() {
        let backend = Box::new(LocalBackend);
        let mut sandbox = ExecutionSandbox::new(backend);
        sandbox.enable();
        let result = sandbox.run(
            Path::new("nonexistent_binary_xyz"),
            &[],
            Duration::from_secs(5),
        );
        assert!(matches!(result, Err(ExecutionError::BinaryNotFound(_))));
    }

    #[test]
    fn enabled_sandbox_runs_command() {
        let backend = Box::new(LocalBackend);
        let mut sandbox = ExecutionSandbox::new(backend);
        sandbox.enable();
        let (program, args) = if cfg!(windows) {
            (PathBuf::from("cmd.exe"), vec!["/c".to_owned(), "echo".to_owned(), "ok".to_owned()])
        } else {
            (PathBuf::from("echo"), vec!["ok".to_owned()])
        };
        let result = sandbox.run(&program, &args, Duration::from_secs(5));
        assert!(result.is_ok(), "expected Ok, got {result:?}");
    }
}
