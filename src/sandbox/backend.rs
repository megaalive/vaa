use std::path::PathBuf;
use std::time::Duration;

use crate::process::ProcessConfig;

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub image_digest: Option<String>,
    pub network_disabled: bool,
    pub memory_limit_bytes: Option<u64>,
    pub cpu_quota: Option<f64>,
    pub timeout: Duration,
    pub max_output_bytes: u64,
    pub allowed_env: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            image_digest: None,
            network_disabled: true,
            memory_limit_bytes: None,
            cpu_quota: None,
            timeout: Duration::from_secs(60),
            max_output_bytes: 1_048_576,
            allowed_env: vec!["PATH".to_owned(), "HOME".to_owned()],
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("sandbox backend unavailable: {0}")]
    Unavailable(String),
    #[error("sandbox process failed: {0}")]
    ProcessFailed(String),
    #[error("sandbox timed out")]
    Timeout,
    #[error("network access denied")]
    NetworkDenied,
}

pub trait SandboxBackend {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
    fn image_digest(&self) -> Option<String>;
    fn wrap_process(&self, program: &str, args: &[String], config: &SandboxConfig)
        -> ProcessConfig;
}

pub struct LocalBackend;

impl SandboxBackend for LocalBackend {
    fn name(&self) -> &str {
        "local"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn image_digest(&self) -> Option<String> {
        None
    }

    fn wrap_process(
        &self,
        program: &str,
        args: &[String],
        config: &SandboxConfig,
    ) -> ProcessConfig {
        ProcessConfig {
            program: PathBuf::from(program),
            args: args.to_vec(),
            working_dir: None,
            allowed_env: config.allowed_env.clone(),
            timeout: config.timeout,
            max_output_bytes: config.max_output_bytes,
            stdin_null: true,
        }
    }
}

pub struct ContainerBackend {
    pub runtime: String,
    pub image: String,
    pub image_digest: Option<String>,
}

impl ContainerBackend {
    #[must_use]
    pub fn new(runtime: &str, image: &str) -> Self {
        Self {
            runtime: runtime.to_owned(),
            image: image.to_owned(),
            image_digest: None,
        }
    }
}

impl SandboxBackend for ContainerBackend {
    fn name(&self) -> &str {
        &self.runtime
    }

    fn is_available(&self) -> bool {
        let cfg = ProcessConfig {
            program: PathBuf::from(&self.runtime),
            args: vec!["--version".to_owned()],
            timeout: Duration::from_secs(5),
            ..ProcessConfig::default()
        };
        crate::process::ProcessRunner::run(&cfg).is_ok()
    }

    fn image_digest(&self) -> Option<String> {
        self.image_digest.clone()
    }

    fn wrap_process(
        &self,
        program: &str,
        args: &[String],
        config: &SandboxConfig,
    ) -> ProcessConfig {
        let mut wrapped = vec!["run".to_owned(), "--rm".to_owned()];

        if config.network_disabled {
            wrapped.push("--network".to_owned());
            wrapped.push("none".to_owned());
        }

        if let Some(mem) = config.memory_limit_bytes {
            wrapped.push("--memory".to_owned());
            wrapped.push(format!("{mem}"));
        }

        wrapped.push("--init".to_owned());
        wrapped.push(self.image.clone());
        wrapped.push(program.to_owned());
        wrapped.extend(args.iter().cloned());

        ProcessConfig {
            program: PathBuf::from(&self.runtime),
            args: wrapped,
            working_dir: None,
            allowed_env: config.allowed_env.clone(),
            timeout: config.timeout,
            max_output_bytes: config.max_output_bytes,
            stdin_null: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_backend_always_available() {
        let backend = LocalBackend;
        assert!(backend.is_available());
        assert_eq!(backend.name(), "local");
        assert!(backend.image_digest().is_none());
    }

    #[test]
    fn local_backend_wraps_process() {
        let backend = LocalBackend;
        let cfg = SandboxConfig::default();
        let pc = backend.wrap_process("nasm", &["-v".to_owned()], &cfg);
        assert_eq!(pc.program.to_string_lossy(), "nasm");
        assert_eq!(pc.args, ["-v"]);
    }

    #[test]
    fn container_backend_wraps_with_docker_args() {
        let backend = ContainerBackend::new("docker", "ubuntu:24.04");
        let cfg = SandboxConfig::default();
        let pc = backend.wrap_process("nasm", &["-v".to_owned()], &cfg);
        assert_eq!(pc.program.to_string_lossy(), "docker");
        assert!(pc.args.contains(&"run".to_owned()));
        assert!(pc.args.contains(&"--network".to_owned()));
        assert!(pc.args.contains(&"none".to_owned()));
        assert!(pc.args.contains(&"ubuntu:24.04".to_owned()));
        assert!(pc.args.contains(&"nasm".to_owned()));
    }
}
