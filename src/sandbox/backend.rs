use std::path::PathBuf;
use std::time::Duration;

use crate::process::ProcessConfig;

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub image_digest: Option<String>,
    pub network_disabled: bool,
    pub memory_limit_bytes: Option<u64>,
    pub cpu_quota: Option<f64>,
    /// Optional Docker/Podman `--pids-limit` (Scaffold; not a security claim).
    pub pids_limit: Option<u32>,
    pub timeout: Duration,
    pub max_output_bytes: u64,
    pub allowed_env: Vec<String>,
    /// Host directory bind-mounted at `/work` (rw). When set, replaces tmpfs `/work`.
    pub host_work_dir: Option<PathBuf>,
    /// Host directory bind-mounted at `/input` (ro).
    pub host_input_ro: Option<PathBuf>,
    /// Optional seccomp JSON profile path (`--security-opt seccomp=<path>`).
    pub seccomp_profile: Option<PathBuf>,
    /// When true, `ContainerBackend::wrap_process` still emits argv; callers must
    /// probe rootless separately via [`crate::sandbox::probe_rootless_runtime`].
    pub require_rootless: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            image_digest: None,
            network_disabled: true,
            memory_limit_bytes: None,
            cpu_quota: None,
            pids_limit: None,
            timeout: Duration::from_secs(60),
            max_output_bytes: 1_048_576,
            allowed_env: vec!["PATH".to_owned(), "HOME".to_owned()],
            host_work_dir: None,
            host_input_ro: None,
            seccomp_profile: None,
            require_rootless: false,
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
            extra_env: Vec::new(),
            timeout: config.timeout,
            max_output_bytes: config.max_output_bytes,
            stdin_null: true,
        }
    }
}

/// Docker/Podman argv wrapper. Hardening increments (seccomp profile, C1 binds)
/// are real argv; this is still **not** absolute isolation (architecture C-012).
/// Verified rootless is opt-in via doctor/`require_rootless`, not assumed.
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
        if let Some(profile) = &config.seccomp_profile {
            wrapped.push("--security-opt".to_owned());
            wrapped.push(format!("seccomp={}", profile.display()));
        }
        wrapped.push("--user".to_owned());
        wrapped.push("65534:65534".to_owned());
        wrapped.push("--read-only".to_owned());
        wrapped.push("--tmpfs".to_owned());
        wrapped.push("/tmp:rw,noexec,nosuid,size=64m".to_owned());

        if let Some(input) = &config.host_input_ro {
            wrapped.push("--mount".to_owned());
            wrapped.push(format!(
                "type=bind,src={},dst=/input,ro=true",
                input.display()
            ));
        }

        if let Some(work) = &config.host_work_dir {
            wrapped.push("--mount".to_owned());
            wrapped.push(format!(
                "type=bind,src={},dst=/work,ro=false",
                work.display()
            ));
        } else {
            wrapped.push("--tmpfs".to_owned());
            wrapped.push("/work:rw,size=256m".to_owned());
        }

        wrapped.push("--workdir".to_owned());
        wrapped.push("/work".to_owned());

        if let Some(mem) = config.memory_limit_bytes {
            wrapped.push("--memory".to_owned());
            wrapped.push(format!("{mem}"));
        }

        if let Some(cpus) = config.cpu_quota {
            wrapped.push("--cpus".to_owned());
            wrapped.push(format!("{cpus}"));
        }

        if let Some(pids) = config.pids_limit {
            wrapped.push("--pids-limit".to_owned());
            wrapped.push(format!("{pids}"));
        }

        // require_rootless is enforced by callers/doctor before wrap; keep argv honest.
        let _ = config.require_rootless;

        wrapped.push("--init".to_owned());
        wrapped.push(self.image_ref());
        wrapped.push(program.to_owned());
        wrapped.extend(args.iter().cloned());

        ProcessConfig {
            program: PathBuf::from(&self.runtime),
            args: wrapped,
            working_dir: None,
            allowed_env: config.allowed_env.clone(),
            extra_env: Vec::new(),
            timeout: config.timeout,
            max_output_bytes: config.max_output_bytes,
            stdin_null: true,
        }
    }
}

/// Bundled default seccomp profile (JSON). Restrictive baseline for Linux
/// containers — still not a certified security product.
pub const DEFAULT_SECCOMP_PROFILE_JSON: &str =
    include_str!("../../assets/seccomp/vaa-default.json");

/// Write the bundled seccomp profile to `path` (parents created).
pub fn write_default_seccomp_profile(path: &std::path::Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, DEFAULT_SECCOMP_PROFILE_JSON)
}

/// Best-effort rootless probe: `podman info` / `docker info` stdout contains
/// `rootless` hints. Returns `Ok(true)` when evidence of rootless is found,
/// `Ok(false)` when runtime works but looks rootful, `Err` when unavailable.
pub fn probe_rootless_runtime(runtime: &str) -> Result<bool, SandboxError> {
    let cfg = ProcessConfig {
        program: PathBuf::from(runtime),
        args: vec!["info".to_owned()],
        timeout: Duration::from_secs(15),
        max_output_bytes: 1_048_576,
        ..ProcessConfig::default()
    };
    let out = crate::process::ProcessRunner::run(&cfg)
        .map_err(|e| SandboxError::Unavailable(e.to_string()))?;
    if out.exit_code != Some(0) {
        return Err(SandboxError::Unavailable(format!(
            "{runtime} info exit={:?}",
            out.exit_code
        )));
    }
    let blob = format!("{}\n{}", out.stdout, out.stderr).to_ascii_lowercase();
    Ok(blob.contains("rootless: true")
        || blob.contains("rootless\": true")
        || blob.contains("rootlessmode")
        || (runtime == "podman" && blob.contains("rootless")))
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
    fn container_backend_ops_deny_network_socket_and_credential_env() {
        let backend = ContainerBackend::new("docker", "ubuntu:24.04");
        let cfg = SandboxConfig::default();
        let pc = backend.wrap_process("nasm", &["-v".to_owned()], &cfg);
        // Network + capability ops (argv proof; not a runtime pen-test).
        assert!(pc.args.windows(2).any(|w| w == ["--network", "none"]));
        assert!(pc.args.windows(2).any(|w| w == ["--cap-drop", "ALL"]));
        // Never bind the Docker socket.
        assert!(
            !pc.args
                .iter()
                .any(|a| a.contains("docker.sock") || a.contains("podman.sock")),
            "socket mount leaked into argv: {:?}",
            pc.args
        );
        // Default allowed_env must not forward credential-shaped keys.
        for forbidden in [
            "AWS_SECRET_ACCESS_KEY",
            "AWS_SESSION_TOKEN",
            "GITHUB_TOKEN",
            "VAA_SEAL_SIGNING_KEY",
            "DOCKER_AUTH_CONFIG",
        ] {
            assert!(
                !pc.allowed_env.iter().any(|e| e == forbidden),
                "credential env {forbidden} must not be in default allowed_env"
            );
        }
        assert!(pc.allowed_env.iter().any(|e| e == "PATH"));
        assert!(pc.allowed_env.iter().any(|e| e == "HOME"));
    }

    #[test]
    fn local_backend_default_env_excludes_credential_keys() {
        let backend = LocalBackend;
        let cfg = SandboxConfig::default();
        let pc = backend.wrap_process("semasm", &["version".to_owned()], &cfg);
        assert_eq!(backend.name(), "local");
        for forbidden in [
            "AWS_SECRET_ACCESS_KEY",
            "VAA_SEAL_SIGNING_KEY",
            "GITHUB_TOKEN",
        ] {
            assert!(
                !pc.allowed_env.iter().any(|e| e == forbidden),
                "credential env {forbidden} must not be in LocalBackend default allowed_env"
            );
        }
    }

    #[test]
    fn container_backend_bind_mounts_replace_work_tmpfs() {
        let backend = ContainerBackend::new("docker", "ubuntu:24.04");
        let cfg = SandboxConfig {
            host_work_dir: Some(PathBuf::from("/host/work")),
            host_input_ro: Some(PathBuf::from("/host/input")),
            ..SandboxConfig::default()
        };
        let pc = backend.wrap_process("nasm", &["-v".to_owned()], &cfg);
        assert!(pc.args.contains(&"--mount".to_owned()));
        assert!(pc
            .args
            .iter()
            .any(|a| a.contains("dst=/input") && a.contains("ro=true")));
        assert!(pc
            .args
            .iter()
            .any(|a| a.contains("dst=/work") && a.contains("ro=false")));
        assert!(!pc.args.contains(&"/work:rw,size=256m".to_owned()));
        assert!(pc
            .args
            .contains(&"/tmp:rw,noexec,nosuid,size=64m".to_owned()));
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

    #[test]
    fn container_backend_honors_cpu_quota_and_pids_limit() {
        let backend = ContainerBackend::new("docker", "ubuntu:24.04");
        let cfg = SandboxConfig {
            cpu_quota: Some(1.5),
            pids_limit: Some(256),
            memory_limit_bytes: Some(64 * 1024 * 1024),
            ..SandboxConfig::default()
        };
        let pc = backend.wrap_process("nasm", &["-v".to_owned()], &cfg);
        assert!(pc.args.contains(&"--cpus".to_owned()));
        assert!(pc.args.contains(&"1.5".to_owned()));
        assert!(pc.args.contains(&"--pids-limit".to_owned()));
        assert!(pc.args.contains(&"256".to_owned()));
        assert!(pc.args.contains(&"--memory".to_owned()));
        assert!(pc.args.contains(&(64 * 1024 * 1024).to_string()));
    }

    #[test]
    fn container_backend_emits_seccomp_security_opt() {
        let backend = ContainerBackend::new("docker", "ubuntu:24.04");
        let cfg = SandboxConfig {
            seccomp_profile: Some(PathBuf::from("/tmp/vaa-seccomp.json")),
            ..SandboxConfig::default()
        };
        let pc = backend.wrap_process("nasm", &["-v".to_owned()], &cfg);
        assert!(pc.args.iter().any(|a| a == "seccomp=/tmp/vaa-seccomp.json"));
    }

    #[test]
    fn bundled_seccomp_profile_is_json() {
        let v: serde_json::Value =
            serde_json::from_str(DEFAULT_SECCOMP_PROFILE_JSON).expect("seccomp json");
        assert_eq!(
            v.get("defaultAction").and_then(|x| x.as_str()),
            Some("SCMP_ACT_ERRNO")
        );
    }
}
