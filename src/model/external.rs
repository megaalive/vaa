//! External argv generator (G1): subprocess writes under `staging/` only.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::process::{ProcessConfig, ProcessRunner};

use super::adapter::{ModelError, ModelResponse};

/// Default relative output filename under staging when the generator writes a file.
pub const DEFAULT_STAGING_OUTPUT: &str = "candidate.asm";

/// Spawn an external program with cwd = staging directory (logical G0 barrier).
///
/// Injected env: `VAA_STAGING`, `VAA_TASK_PATH`, `VAA_OUTPUT`, `VAA_TASK_ID`, `VAA_TARGET`,
/// plus allowlisted host vars (`PATH` / `HOME` / `USER`, Windows essentials).
///
/// **Not** OS-level FS isolation — a hostile binary can still touch the host;
/// VAA only accepts the staged output file via the RunDir API.
#[derive(Debug, Clone)]
pub struct ArgvExternalGenerator {
    pub name: String,
    pub program: PathBuf,
    pub args: Vec<String>,
    pub timeout: Duration,
    pub max_output_bytes: u64,
    /// Relative path under staging the generator must create (default `candidate.asm`).
    pub output_relative: String,
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
            let mut vars = vec!["PATH".to_owned(), "HOME".to_owned(), "USER".to_owned()];
            #[cfg(windows)]
            {
                for k in ["SYSTEMROOT", "WINDIR", "SYSTEMDRIVE", "PATHEXT", "COMSPEC"] {
                    vars.push(k.to_owned());
                }
            }
            vars
        };

        let extra_env = vec![
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

        let cfg = ProcessConfig {
            program: self.program.clone(),
            args: self.args.clone(),
            working_dir: Some(staging_dir.to_path_buf()),
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
                Path::new("task.toml"),
                "t",
                "x86_64-pc-windows-msvc",
            )
            .unwrap_err();
        assert!(err.to_string().contains("traversal"), "{err}");
        let _ = std::fs::remove_dir_all(&staging);
    }

    #[test]
    fn external_generator_writes_candidate_asm() {
        let staging = temp_staging();
        let task = staging.join("dummy.vaa.toml");
        std::fs::write(&task, b"placeholder").unwrap();

        #[cfg(windows)]
        let (program, args) = (
            PathBuf::from("powershell"),
            vec![
                "-NoProfile".into(),
                "-Command".into(),
                "Set-Content -Path candidate.asm -Value \"xor eax, eax`nret`n\" -NoNewline".into(),
            ],
        );
        #[cfg(not(windows))]
        let (program, args) = (
            PathBuf::from("sh"),
            vec![
                "-c".into(),
                "printf 'xor eax, eax\\nret\\n' > candidate.asm".into(),
            ],
        );

        let gen = ArgvExternalGenerator::new("ci-g1", program).with_args(args);
        let resp = gen
            .generate_to_staging(
                &staging,
                &task,
                "count-byte-win64-v1",
                "x86_64-pc-windows-msvc",
            )
            .expect("generator");
        assert!(resp.source.contains("ret"));
        assert_eq!(resp.model_name, "ci-g1");
        assert!(staging.join("candidate.asm").is_file());
        let _ = std::fs::remove_dir_all(&staging);
    }
}
