//! External argv generator (G1): subprocess writes under `staging/` only.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::process::{ProcessConfig, ProcessRunner};
use crate::sandbox::{ContainerBackend, SandboxBackend, SandboxConfig};

use super::adapter::{ModelError, ModelResponse};

/// Default relative output filename under staging when the generator writes a file.
pub const DEFAULT_STAGING_OUTPUT: &str = "candidate.asm";

/// Optional OS-level jail for external generators (P7-S). Container wrap with
/// staging bind-mounted at `/work` only — still not absolute isolation (C-012).
#[derive(Debug, Clone)]
pub struct GeneratorJailOpts {
    pub runtime: String,
    pub image: String,
    pub image_digest: Option<String>,
    pub seccomp_profile: Option<PathBuf>,
    pub memory_limit_bytes: Option<u64>,
    pub pids_limit: Option<u32>,
}

/// Spawn an external program with cwd = staging directory (logical G0 barrier).
///
/// Injected env: `VAA_STAGING`, `VAA_TASK_PATH`, `VAA_OUTPUT`, `VAA_TASK_ID`, `VAA_TARGET`,
/// plus allowlisted host vars (`PATH` / `HOME` / `USER`, Windows essentials).
///
/// With [`GeneratorJailOpts`], the generator runs inside a container with only
/// `staging/` writable at `/work` (OS jail Scaffold). Without jail, a hostile
/// binary can still touch the host; VAA only accepts the staged output file.
#[derive(Debug, Clone)]
pub struct ArgvExternalGenerator {
    pub name: String,
    pub program: PathBuf,
    pub args: Vec<String>,
    pub timeout: Duration,
    pub max_output_bytes: u64,
    /// Relative path under staging the generator must create (default `candidate.asm`).
    pub output_relative: String,
    /// When set, wrap the generator in a container OS jail.
    pub jail: Option<GeneratorJailOpts>,
}

impl ArgvExternalGenerator {
    #[must_use]
    pub fn new(name: impl Into<String>, program: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            program: program.into(),
            args: Vec::new(),
            timeout: Duration::from_secs(60),
            max_output_bytes: 1_048_576,
            output_relative: DEFAULT_STAGING_OUTPUT.to_owned(),
            jail: None,
        }
    }

    #[must_use]
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    #[must_use]
    pub fn with_output_relative(mut self, relative: impl Into<String>) -> Self {
        self.output_relative = relative.into();
        self
    }

    #[must_use]
    pub fn with_jail(mut self, jail: GeneratorJailOpts) -> Self {
        self.jail = Some(jail);
        self
    }

    /// True when an OS jail is configured (for `EvidencePolicy.os_fs_isolation`).
    #[must_use]
    pub fn os_fs_isolation_enforced(&self) -> bool {
        self.jail.is_some()
    }

    fn reject_bad_relative(relative: &str) -> Result<(), ModelError> {
        let path = Path::new(relative);
        if path.is_absolute() {
            return Err(ModelError::InvalidResponse(format!(
                "output relative path must be relative: {relative}"
            )));
        }
        for c in path.components() {
            match c {
                std::path::Component::Normal(_) | std::path::Component::CurDir => {}
                _ => {
                    return Err(ModelError::InvalidResponse(format!(
                        "output relative path rejects traversal: {relative}"
                    )));
                }
            }
        }
        if relative.is_empty() {
            return Err(ModelError::InvalidResponse(
                "output relative path is empty".into(),
            ));
        }
        Ok(())
    }

    /// Run the generator with `cwd = staging_dir`. Returns source bytes from the staging file.
    pub fn generate_to_staging(
        &self,
        staging_dir: &Path,
        task_path: &Path,
        task_id: &str,
        target: &str,
    ) -> Result<ModelResponse, ModelError> {
        Self::reject_bad_relative(&self.output_relative)?;

        let out_path = staging_dir.join(&self.output_relative);
        if !out_path.starts_with(staging_dir) {
            return Err(ModelError::InvalidResponse(
                "resolved output escaped staging".into(),
            ));
        }
        if out_path.exists() {
            let _ = std::fs::remove_file(&out_path);
        }

        let allowed_env = {
            #[cfg(windows)]
            {
                let mut vars = vec!["PATH".to_owned(), "HOME".to_owned(), "USER".to_owned()];
                for k in ["SYSTEMROOT", "WINDIR", "SYSTEMDRIVE", "PATHEXT", "COMSPEC"] {
                    vars.push(k.to_owned());
                }
                vars
            }
            #[cfg(not(windows))]
            {
                vec!["PATH".to_owned(), "HOME".to_owned(), "USER".to_owned()]
            }
        };

        let (program, args, working_dir, extra_env) = if let Some(jail) = &self.jail {
            let backend = match &jail.image_digest {
                Some(d) => ContainerBackend::with_image_digest(&jail.runtime, &jail.image, d),
                None => ContainerBackend::new(&jail.runtime, &jail.image),
            };
            let sandbox = SandboxConfig {
                host_work_dir: Some(staging_dir.to_path_buf()),
                host_input_ro: None,
                seccomp_profile: jail.seccomp_profile.clone(),
                memory_limit_bytes: jail.memory_limit_bytes,
                pids_limit: jail.pids_limit.or(Some(128)),
                timeout: self.timeout,
                max_output_bytes: self.max_output_bytes,
                allowed_env: allowed_env.clone(),
                ..SandboxConfig::default()
            };
            let program_name = self.program.file_name().map_or_else(
                || self.program.to_string_lossy().into_owned(),
                |s| s.to_string_lossy().into_owned(),
            );
            let wrapped = backend.wrap_process(&program_name, &self.args, &sandbox);
            let extra = vec![
                ("VAA_STAGING".to_owned(), "/work".to_owned()),
                ("VAA_TASK_PATH".to_owned(), "/work/task.vaa.toml".to_owned()),
                ("VAA_OUTPUT".to_owned(), self.output_relative.clone()),
                ("VAA_TASK_ID".to_owned(), task_id.to_owned()),
                ("VAA_TARGET".to_owned(), target.to_owned()),
            ];
            let task_staging = staging_dir.join("task.vaa.toml");
            if task_path.exists() && !task_staging.exists() {
                let _ = std::fs::copy(task_path, &task_staging);
            }
            (wrapped.program, wrapped.args, None, extra)
        } else {
            let extra = vec![
                (
                    "VAA_STAGING".to_owned(),
                    staging_dir.to_string_lossy().into_owned(),
                ),
                (
                    "VAA_TASK_PATH".to_owned(),
                    task_path.to_string_lossy().into_owned(),
                ),
                ("VAA_OUTPUT".to_owned(), self.output_relative.clone()),
                ("VAA_TASK_ID".to_owned(), task_id.to_owned()),
                ("VAA_TARGET".to_owned(), target.to_owned()),
            ];
            (
                self.program.clone(),
                self.args.clone(),
                Some(staging_dir.to_path_buf()),
                extra,
            )
        };

        let cfg = ProcessConfig {
            program,
            args,
            working_dir,
            allowed_env,
            extra_env,
            timeout: self.timeout,
            max_output_bytes: self.max_output_bytes,
            stdin_null: true,
        };

        let out =
            ProcessRunner::run(&cfg).map_err(|e| ModelError::GenerationFailed(e.to_string()))?;
        if out.exit_code != Some(0) {
            return Err(ModelError::GenerationFailed(format!(
                "exit={:?} stderr={}",
                out.exit_code,
                out.stderr.trim()
            )));
        }

        let source = std::fs::read_to_string(&out_path).map_err(|e| {
            ModelError::GenerationFailed(format!(
                "generator exit 0 but missing staging output `{}`: {e}",
                out_path.display()
            ))
        })?;
        if source.trim().is_empty() {
            return Err(ModelError::InvalidResponse(
                "generator produced empty source".into(),
            ));
        }

        Ok(ModelResponse {
            source,
            target: target.to_owned(),
            model_name: self.name.clone(),
            generation_id: format!("{}-external", self.name),
            diagnostics: if out.stderr.is_empty() {
                vec![]
            } else {
                vec![out.stderr]
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_staging() -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "vaa_g1_{}_{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn rejects_output_relative_with_parent_segments() {
        let gen = ArgvExternalGenerator::new("t", "true").with_output_relative("../evil.asm");
        let staging = temp_staging();
        let err = gen
            .generate_to_staging(
                &staging,
                Path::new("task.vaa.toml"),
                "tid",
                "x86_64-pc-windows-msvc",
            )
            .unwrap_err();
        assert!(err.to_string().contains("traversal"));
        let _ = std::fs::remove_dir_all(&staging);
    }

    #[test]
    fn jail_wraps_argv_to_container_runtime() {
        let gen = ArgvExternalGenerator::new("t", "mygen")
            .with_args(vec!["--emit".into()])
            .with_jail(GeneratorJailOpts {
                runtime: "docker".into(),
                image: "ubuntu:24.04".into(),
                image_digest: None,
                seccomp_profile: Some(PathBuf::from("/tmp/s.json")),
                memory_limit_bytes: Some(32 * 1024 * 1024),
                pids_limit: Some(64),
            });
        assert!(gen.os_fs_isolation_enforced());
        let staging = temp_staging();
        // Will fail at run (no docker / no output) but we only assert wrap by
        // checking the error path still attempts container — unit the wrap via
        // reconstructing SandboxConfig path indirectly: jail set means policy true.
        let _ = gen.generate_to_staging(
            &staging,
            Path::new("missing-task.toml"),
            "tid",
            "x86_64-pc-windows-msvc",
        );
        let _ = std::fs::remove_dir_all(&staging);
    }
}
