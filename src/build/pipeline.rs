use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::evidence::sha256_digest_prefixed;
use crate::process::{ProcessConfig, ProcessRunner};
use crate::sandbox::{ContainerBackend, SandboxBackend, SandboxConfig};

/// Default container image for `vaa build --sandbox container` (Scaffold).
pub const DEFAULT_CONTAINER_IMAGE: &str = "ubuntu:24.04";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildManifest {
    pub assembler: String,
    pub linker: String,
    pub source_path: PathBuf,
    pub object_path: PathBuf,
    pub binary_path: PathBuf,
    pub assembler_args: Vec<String>,
    pub linker_args: Vec<String>,
    pub assembler_digest: Option<String>,
    pub linker_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildOutcome {
    pub success: bool,
    pub manifest: BuildManifest,
    pub assembler_stdout: String,
    pub assembler_stderr: String,
    pub linker_stdout: String,
    pub linker_stderr: String,
    pub exit_code: Option<i32>,
}

/// Optional container wrap for assemble/link (C2 Scaffold — not hardened isolation).
#[derive(Debug, Clone)]
pub struct ContainerBuildOpts {
    pub runtime: String,
    pub image: String,
    pub image_digest: Option<String>,
    pub cpu_quota: Option<f64>,
    pub pids_limit: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub assembler_path: PathBuf,
    pub linker_path: PathBuf,
    pub source_path: PathBuf,
    pub output_dir: PathBuf,
    pub target: String,
    pub extra_as_args: Vec<String>,
    pub extra_ld_args: Vec<String>,
    pub timeout: Duration,
    pub max_output_bytes: u64,
    pub container: Option<ContainerBuildOpts>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            assembler_path: PathBuf::from("nasm"),
            linker_path: PathBuf::from("ld"),
            source_path: PathBuf::new(),
            output_dir: PathBuf::from("."),
            target: "elf64".to_owned(),
            extra_as_args: Vec::new(),
            extra_ld_args: Vec::new(),
            timeout: Duration::from_secs(60),
            max_output_bytes: 1_048_576,
            container: None,
        }
    }
}

pub struct BuildPipeline;

impl BuildPipeline {
    #[must_use]
    pub fn build(config: &PipelineConfig) -> BuildOutcome {
        let object_name = format!(
            "{}.o",
            config
                .source_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
        );
        let binary_name = format!(
            "{}.bin",
            config
                .source_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
        );
        let object_path = config.output_dir.join(&object_name);
        let binary_path = config.output_dir.join(&binary_name);

        let mut as_args = vec![
            "-f".to_owned(),
            config.target.clone(),
            "-o".to_owned(),
            object_path.to_string_lossy().to_string(),
            config.source_path.to_string_lossy().to_string(),
        ];
        as_args.extend(config.extra_as_args.clone());

        let as_cfg =
            maybe_wrap_container(&config.assembler_path.to_string_lossy(), &as_args, config);

        let as_result = ProcessRunner::run(&as_cfg);

        let (as_stdout, as_stderr, as_ok) = match &as_result {
            Ok(out) => (
                out.stdout.clone(),
                out.stderr.clone(),
                out.exit_code == Some(0),
            ),
            Err(e) => (String::new(), format!("{e}"), false),
        };

        let assembler_digest = tool_digest(&config.assembler_path);
        let linker_digest = tool_digest(&config.linker_path);

        if !as_ok {
            return BuildOutcome {
                success: false,
                manifest: BuildManifest {
                    assembler: config.assembler_path.to_string_lossy().to_string(),
                    linker: config.linker_path.to_string_lossy().to_string(),
                    source_path: config.source_path.clone(),
                    object_path,
                    binary_path,
                    assembler_args: as_cfg.args,
                    linker_args: Vec::new(),
                    assembler_digest,
                    linker_digest,
                },
                assembler_stdout: as_stdout,
                assembler_stderr: as_stderr,
                linker_stdout: String::new(),
                linker_stderr: String::new(),
                exit_code: None,
            };
        }

        let mut ld_args = vec![
            "-o".to_owned(),
            binary_path.to_string_lossy().to_string(),
            object_path.to_string_lossy().to_string(),
        ];
        ld_args.extend(config.extra_ld_args.clone());

        let ld_cfg = maybe_wrap_container(&config.linker_path.to_string_lossy(), &ld_args, config);

        let ld_result = ProcessRunner::run(&ld_cfg);

        let (ld_stdout, ld_stderr, ld_ok, ld_code) = match &ld_result {
            Ok(out) => (
                out.stdout.clone(),
                out.stderr.clone(),
                out.exit_code == Some(0),
                out.exit_code,
            ),
            Err(e) => (String::new(), format!("{e}"), false, None),
        };

        BuildOutcome {
            success: ld_ok,
            manifest: BuildManifest {
                assembler: config.assembler_path.to_string_lossy().to_string(),
                linker: config.linker_path.to_string_lossy().to_string(),
                source_path: config.source_path.clone(),
                object_path,
                binary_path,
                assembler_args: as_cfg.args,
                linker_args: ld_cfg.args,
                assembler_digest,
                linker_digest,
            },
            assembler_stdout: as_stdout,
            assembler_stderr: as_stderr,
            linker_stdout: ld_stdout,
            linker_stderr: ld_stderr,
            exit_code: ld_code,
        }
    }
}

fn maybe_wrap_container(program: &str, args: &[String], config: &PipelineConfig) -> ProcessConfig {
    let Some(opts) = &config.container else {
        return ProcessConfig {
            program: PathBuf::from(program),
            args: args.to_vec(),
            timeout: config.timeout,
            max_output_bytes: config.max_output_bytes,
            ..ProcessConfig::default()
        };
    };

    let backend = match &opts.image_digest {
        Some(d) => ContainerBackend::with_image_digest(&opts.runtime, &opts.image, d),
        None => ContainerBackend::new(&opts.runtime, &opts.image),
    };
    let sandbox = SandboxConfig {
        cpu_quota: opts.cpu_quota,
        pids_limit: opts.pids_limit,
        timeout: config.timeout,
        max_output_bytes: config.max_output_bytes,
        ..SandboxConfig::default()
    };
    backend.wrap_process(program, args, &sandbox)
}

/// SHA-256 of a resolved toolchain binary (B1). Returns `None` if unresolved.
#[must_use]
pub fn tool_digest(program: &Path) -> Option<String> {
    let resolved = resolve_tool_path(program)?;
    let bytes = std::fs::read(&resolved).ok()?;
    Some(sha256_digest_prefixed(&bytes))
}

fn resolve_tool_path(program: &Path) -> Option<PathBuf> {
    if program.is_file() {
        return Some(program.to_path_buf());
    }
    let name = program.to_str()?;
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let with_exe = dir.join(format!("{name}.exe"));
            if with_exe.is_file() {
                return Some(with_exe);
            }
        }
    }
    None
}

/// Probe `docker` then `podman` via `--version` (C2).
#[must_use]
pub fn probe_container_runtime() -> Option<String> {
    for runtime in ["docker", "podman"] {
        if ContainerBackend::new(runtime, DEFAULT_CONTAINER_IMAGE).is_available() {
            return Some(runtime.to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn build_nonexistent_source_fails() {
        let config = PipelineConfig {
            assembler_path: PathBuf::from("nasm"),
            source_path: PathBuf::from("nonexistent_file_xyz.asm"),
            output_dir: std::env::temp_dir(),
            ..PipelineConfig::default()
        };
        let outcome = BuildPipeline::build(&config);
        assert!(!outcome.success);
    }

    #[test]
    fn tool_digest_hashes_existing_file() {
        let dir = std::env::temp_dir().join(format!(
            "vaa_tool_digest_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let fake = dir.join("fake-nasm.bin");
        {
            let mut f = std::fs::File::create(&fake).unwrap();
            f.write_all(b"nasm-fake-bytes").unwrap();
        }
        let digest = tool_digest(&fake).expect("digest");
        assert!(digest.starts_with("sha256:"));
        assert_eq!(digest, tool_digest(&fake).unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn container_wrap_rewrites_program_to_runtime() {
        let config = PipelineConfig {
            container: Some(ContainerBuildOpts {
                runtime: "docker".into(),
                image: DEFAULT_CONTAINER_IMAGE.into(),
                image_digest: None,
                cpu_quota: Some(0.5),
                pids_limit: Some(128),
            }),
            ..PipelineConfig::default()
        };
        let pc = maybe_wrap_container("nasm", &["-v".into()], &config);
        assert_eq!(pc.program.to_string_lossy(), "docker");
        assert!(pc.args.contains(&"--cpus".to_owned()));
        assert!(pc.args.contains(&"0.5".to_owned()));
    }
}
