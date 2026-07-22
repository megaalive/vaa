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

/// Docker/Podman argv wrapper. Still **Scaffold** isolation — not a production
/// hardened profile (no custom seccomp, no verified rootless daemon, no host
/// volume mounts in this generic wrapper). C0 deepen argv only.
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

    #[must_use]
    pub fn with_image_digest(runtime: &str, image: &str, digest: impl Into<String>) -> Self {
        Self {
            runtime: runtime.to_owned(),
            image: image.to_owned(),
            image_digest: Some(digest.into()),
        }
    }

    /// Image reference for `docker run`: `name@sha256:…` when digest is set.
    #[must_use]
    pub fn image_ref(&self) -> String {
        let Some(digest) = &self.image_digest else {
            return self.image.clone();
        };
        let digest = digest.trim();
        let digest = if digest.starts_with("sha256:") {
            digest.to_owned()
        } else {
            format!("sha256:{digest}")
        };
        // Strip tag from image name when pinning by digest.
        let name = self.image.split(':').next().unwrap_or(&self.image);
        format!("{name}@{digest}")
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

        // Fail-closed: never enable container network from this scaffold.
        let _ = config.network_disabled;
        wrapped.push("--network".to_owned());
        wrapped.push("none".to_owned());
        wrapped.push("--cap-drop".to_owned());
        wrapped.push("ALL".to_owned());
        wrapped.push("--security-opt".to_owned());
        wrapped.push("no-new-privileges".to_owned());
        wrapped.push("--user".to_owned());
        wrapped.push("65534:65534".to_owned());
        wrapped.push("--read-only".to_owned());
        wrapped.push("--tmpfs".to_owned());
        wrapped.push("/tmp:rw,noexec,nosuid,size=64m".to_owned());
        wrapped.push("--tmpfs".to_owned());
        wrapped.push("/work:rw,size=256m".to_owned());
        wrapped.push("--workdir".to_owned());
        wrapped.push("/work".to_owned());

        if let Some(mem) = config.memory_limit_bytes {
            wrapped.push("--memory".to_owned());
            wrapped.push(format!("{mem}"));
        }

        wrapped.push("--init".to_owned());
        wrapped.push(self.image_ref());
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
    fn container_backend_forces_network_none_and_cap_drop() {
        let backend = ContainerBackend::new("docker", "ubuntu:24.04");
        let cfg = SandboxConfig {
            network_disabled: false, // request ignored — fail-closed
            ..SandboxConfig::default()
        };
        let pc = backend.wrap_process("nasm", &["-v".to_owned()], &cfg);
        assert_eq!(pc.program.to_string_lossy(), "docker");
        assert!(pc.args.contains(&"--network".to_owned()));
        assert!(pc.args.contains(&"none".to_owned()));
        assert!(pc.args.contains(&"--cap-drop".to_owned()));
        assert!(pc.args.contains(&"ALL".to_owned()));
        assert!(pc.args.contains(&"--security-opt".to_owned()));
        assert!(pc.args.contains(&"no-new-privileges".to_owned()));
        assert!(pc.args.contains(&"--user".to_owned()));
        assert!(pc.args.contains(&"65534:65534".to_owned()));
        assert!(pc.args.contains(&"--read-only".to_owned()));
        assert!(pc.args.contains(&"--tmpfs".to_owned()));
        assert!(pc
            .args
            .contains(&"/tmp:rw,noexec,nosuid,size=64m".to_owned()));
        assert!(pc.args.contains(&"/work:rw,size=256m".to_owned()));
        assert!(pc.args.contains(&"--workdir".to_owned()));
        assert!(pc.args.contains(&"/work".to_owned()));
        assert!(pc.args.contains(&"ubuntu:24.04".to_owned()));
        assert!(pc.args.contains(&"nasm".to_owned()));
    }

    #[test]
    fn container_backend_prefers_image_digest_ref() {
        let backend = ContainerBackend::with_image_digest(
            "docker",
            "ubuntu:24.04",
            "sha256:deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
        );
        assert_eq!(
            backend.image_ref(),
            "ubuntu@sha256:deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
        );
        let pc = backend.wrap_process("true", &[], &SandboxConfig::default());
        assert!(pc.args.iter().any(|a| a.starts_with("ubuntu@sha256:")));
        assert!(!pc.args.iter().any(|a| a == "ubuntu:24.04"));
    }
}
