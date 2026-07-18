use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::process::{ProcessConfig, ProcessRunner};

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
        }
    }
}

pub struct BuildPipeline;

impl BuildPipeline {
    pub fn build(config: &PipelineConfig) -> BuildOutcome {
        let object_name = format!(
            "{}.o",
            config.source_path.file_stem().unwrap_or_default().to_string_lossy()
        );
        let binary_name = format!(
            "{}.bin",
            config.source_path.file_stem().unwrap_or_default().to_string_lossy()
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

        let as_cfg = ProcessConfig {
            program: config.assembler_path.clone(),
            args: as_args,
            timeout: config.timeout,
            max_output_bytes: config.max_output_bytes,
            ..ProcessConfig::default()
        };

        let as_result = ProcessRunner::run(&as_cfg);

        let (as_stdout, as_stderr, as_ok) = match &as_result {
            Ok(out) => (out.stdout.clone(), out.stderr.clone(), out.exit_code == Some(0)),
            Err(e) => (String::new(), format!("{e}"), false),
        };

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
                    assembler_digest: None,
                    linker_digest: None,
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

        let ld_cfg = ProcessConfig {
            program: config.linker_path.clone(),
            args: ld_args,
            timeout: config.timeout,
            max_output_bytes: config.max_output_bytes,
            ..ProcessConfig::default()
        };

        let ld_result = ProcessRunner::run(&ld_cfg);

        let (ld_stdout, ld_stderr, ld_ok, ld_code) = match &ld_result {
            Ok(out) => (out.stdout.clone(), out.stderr.clone(), out.exit_code == Some(0), out.exit_code),
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
                assembler_digest: None,
                linker_digest: None,
            },
            assembler_stdout: as_stdout,
            assembler_stderr: as_stderr,
            linker_stdout: ld_stdout,
            linker_stderr: ld_stderr,
            exit_code: ld_code,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn build_nonexistent_source_fails() {
        let config = PipelineConfig {
            assembler_path: PathBuf::from("nasm"),
            source_path: PathBuf::from("nonexistent_file_xyz.asm"),
            output_dir: PathBuf::from(std::env::temp_dir()),
            ..PipelineConfig::default()
        };
        let outcome = BuildPipeline::build(&config);
        assert!(!outcome.success);
    }
}
