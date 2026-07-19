#![forbid(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines
)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

use vaa::exit_code::ExitCode as VaaExitCode;
use vaa::task::{load_locked_task, TaskError};
use vaa::{
    ArtifactInspector, BuildPipeline, EvidenceAggregator, EvidenceStatus, FixtureModelAdapter,
    ModelAdapter, PipelineConfig, SemasmDoctor, SemasmVerify, TargetCapabilities, VerifyError,
    MATURITY, TASK_SCHEMA_VERSION, VAA_VERSION,
};

/// Verifiable Assembly Agent command-line interface.
#[derive(Debug, Parser)]
#[command(
    name = "vaa",
    version = VAA_VERSION,
    about = "VAA: fail-closed orchestration for model-assisted assembly around SemASM",
    long_about = "VAA converts a constrained task specification into assembly \
candidates, collects evidence from SemASM and the native toolchain, and returns \
an evidence bundle."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Print version.
    Version,
    /// Show high-level project status.
    Status,
    /// Parse and validate a `task.vaa.toml` file (schema 0.1).
    Validate {
        /// Path to the task file.
        task: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
        /// Include the locked task content digest in the output.
        #[arg(long, default_value_t = true)]
        show_digest: bool,
    },
    /// Check SemASM binary, version, and schema compatibility.
    Doctor {
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Query target capabilities from SemASM.
    Capabilities {
        /// Target triple to inspect.
        #[arg(long)]
        target: String,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Verify an assembly source against a locked task.
    Verify {
        /// Path to the locked task file.
        task: PathBuf,
        /// Path to the assembly source file.
        #[arg(long)]
        source: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Generate assembly from a locked task via model adapter.
    Generate {
        /// Path to the locked task file.
        task: PathBuf,
        /// Output path for generated source.
        #[arg(long)]
        output: PathBuf,
    },
    /// Assemble and link a source file.
    Build {
        /// Path to the assembly source file.
        source: PathBuf,
        /// Output directory.
        #[arg(long, default_value = ".")]
        output_dir: PathBuf,
        /// Target format.
        #[arg(long, default_value = "elf64")]
        target: String,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Inspect a compiled artifact.
    Inspect {
        /// Path to the artifact.
        artifact: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    /// Human-readable text.
    Terminal,
    /// Machine-readable JSON object.
    Json,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Status) {
        Commands::Version => {
            println!("vaa {VAA_VERSION}");
            VaaExitCode::Success.as_std()
        }
        Commands::Status => {
            print_status();
            VaaExitCode::Success.as_std()
        }
        Commands::Validate {
            task,
            format,
            show_digest,
        } => validate_command(&task, format, show_digest),
        Commands::Doctor { format } => doctor_command(format),
        Commands::Capabilities { target, format } => capabilities_command(&target, format),
        Commands::Verify {
            task,
            source,
            format,
        } => verify_command(&task, &source, format),
        Commands::Generate { task, output } => generate_command(&task, &output),
        Commands::Build {
            source,
            output_dir,
            target,
            format,
        } => build_command(&source, &output_dir, &target, format),
        Commands::Inspect { artifact, format } => inspect_command(&artifact, format),
    }
}

fn print_status() {
    println!("vaa {VAA_VERSION}");
    println!("maturity: {MATURITY}");
    println!("form: local CLI (single binary crate + library modules)");
    println!("task schema: {TASK_SCHEMA_VERSION}");
    println!("commands: version, status, validate, doctor, capabilities, verify, generate, build, inspect");
    println!("default mode: verify-only");
    println!("model adapter: fixture adapter available");
    println!("SemASM integration: doctor and capability adapters available");
    println!("build pipeline: nasm + ld (needs toolchain on PATH)");
    println!("note: absence of errors here is not evidence that any assembly is verified");
}

fn validate_command(path: &std::path::Path, format: OutputFormat, show_digest: bool) -> ExitCode {
    match load_locked_task(path) {
        Ok(locked) => {
            match format {
                OutputFormat::Terminal => {
                    println!("ok: task `{}` is valid", locked.task().task_id);
                    println!("  schema_version: {}", locked.task().schema_version);
                    println!("  target: {}", locked.task().target);
                    println!("  artifact_kind: {:?}", locked.task().artifact_kind);
                    println!(
                        "  entry: {} ({})",
                        locked.task().entry.symbol,
                        locked.task().entry.abi
                    );
                    println!("  tests: {}", locked.task().tests.len());
                    if show_digest {
                        println!("  digest: {}", locked.digest().prefixed());
                    }
                }
                OutputFormat::Json => {
                    let body = serde_json::json!({
                        "ok": true,
                        "path": path,
                        "task_id": locked.task().task_id,
                        "schema_version": locked.task().schema_version,
                        "target": locked.task().target,
                        "artifact_kind": locked.task().artifact_kind,
                        "entry_symbol": locked.task().entry.symbol,
                        "entry_abi": locked.task().entry.abi,
                        "test_count": locked.task().tests.len(),
                        "digest": if show_digest {
                            Some(locked.digest().prefixed())
                        } else {
                            None
                        },
                    });
                    println!("{body}");
                }
            }
            VaaExitCode::Success.as_std()
        }
        Err(error) => {
            emit_validate_error(path, format, &error);
            match error {
                TaskError::Io { .. }
                | TaskError::Parse { .. }
                | TaskError::Validation(_)
                | TaskError::ValidationMany { .. } => VaaExitCode::InvalidInput.as_std(),
            }
        }
    }
}

fn emit_validate_error(path: &Path, format: OutputFormat, error: &TaskError) {
    match format {
        OutputFormat::Terminal => {
            eprintln!("error: failed to validate `{}`", path.display());
            eprintln!("{error}");
        }
        OutputFormat::Json => {
            let body = serde_json::json!({
                "ok": false,
                "path": path,
                "error": error.to_string(),
            });
            println!("{body}");
        }
    }
}

fn doctor_command(format: OutputFormat) -> ExitCode {
    let report = SemasmDoctor::run();
    match format {
        OutputFormat::Terminal => {
            println!("VAA Doctor Report");
            println!("  status: {:?}", report.status);
            if let Some(path) = &report.binary_path {
                println!("  binary: {}", path.display());
            } else {
                println!("  binary: not found");
            }
            if let Some(ver) = &report.version {
                println!("  version: {}", ver.version);
                println!("  schema: {}", ver.schema_version);
            }
            for detail in &report.details {
                println!("  {detail}");
            }
        }
        OutputFormat::Json => {
            let body = serde_json::json!({
                "status": format!("{:?}", report.status),
                "binary_path": report.binary_path,
                "version": report.version.as_ref().map(|v| v.version.clone()),
                "schema_version": report.version.as_ref().map(|v| v.schema_version.clone()),
                "details": report.details,
            });
            println!("{body}");
        }
    }
    match report.status {
        vaa::DoctorStatus::Available => VaaExitCode::Success.as_std(),
        vaa::DoctorStatus::Incompatible | vaa::DoctorStatus::Unavailable => {
            VaaExitCode::DependencyIncompatible.as_std()
        }
        vaa::DoctorStatus::Degraded => VaaExitCode::ToolFailure.as_std(),
    }
}

fn capabilities_command(target: &str, format: OutputFormat) -> ExitCode {
    let caps = TargetCapabilities::for_target(target);
    match format {
        OutputFormat::Terminal => {
            println!("Target: {}", caps.target_id);
            println!("  decode:         {:?}", caps.decode);
            println!("  lower:          {:?}", caps.lower);
            println!("  abi_check:      {:?}", caps.abi_check);
            println!("  object_inspect: {:?}", caps.object_inspect);
            println!("  assemble:       {:?}", caps.assemble);
            println!("  link:           {:?}", caps.link);
            println!("  sandbox_run:    {:?}", caps.sandbox_run);
            println!("  digest: {}", caps.digest());
        }
        OutputFormat::Json => {
            let body = serde_json::json!({
                "target_id": caps.target_id,
                "decode": format!("{:?}", caps.decode),
                "lower": format!("{:?}", caps.lower),
                "abi_check": format!("{:?}", caps.abi_check),
                "object_inspect": format!("{:?}", caps.object_inspect),
                "assemble": format!("{:?}", caps.assemble),
                "link": format!("{:?}", caps.link),
                "sandbox_run": format!("{:?}", caps.sandbox_run),
                "digest": caps.digest(),
            });
            println!("{body}");
        }
    }
    VaaExitCode::Success.as_std()
}

fn verify_command(task_path: &Path, source_path: &Path, format: OutputFormat) -> ExitCode {
    let locked = match load_locked_task(task_path) {
        Ok(t) => t,
        Err(error) => {
            emit_validate_error(task_path, format, &error);
            return VaaExitCode::InvalidInput.as_std();
        }
    };

    let target = &locked.task().target;
    let caps = TargetCapabilities::for_target(target);
    let cm = vaa::match_task_requirements(locked.task(), &caps);

    let doctor = SemasmDoctor::run();

    let verify_result = match doctor.binary_path.as_ref() {
        Some(binary) => SemasmVerify::run(source_path, task_path, binary),
        None => Err(VerifyError::BinaryNotFound),
    };

    let verify_report = match verify_result {
        Ok(report) => Some(report),
        Err(e) => {
            let mut checks = Vec::new();
            checks.push(vaa::CheckOutcome {
                check_name: "task_valid".to_owned(),
                required: true,
                passed: true,
                details: None,
            });
            checks.push(vaa::CheckOutcome {
                check_name: "semasm_available".to_owned(),
                required: true,
                passed: matches!(doctor.status, vaa::DoctorStatus::Available),
                details: Some(format!("{:?}", doctor.status)),
            });
            checks.push(vaa::CheckOutcome {
                check_name: "target_capability_match".to_owned(),
                required: true,
                passed: cm.compatible,
                details: if cm.compatible {
                    None
                } else {
                    let mut msgs = cm.insufficient.clone();
                    msgs.extend(cm.missing.clone());
                    Some(msgs.join("; "))
                },
            });
            checks.push(vaa::CheckOutcome {
                check_name: "semasm_verification".to_owned(),
                required: true,
                passed: false,
                details: Some(format!("verify error: {e}")),
            });
            let report = vaa::EvidenceReport {
                task_id: locked.task().task_id.clone(),
                task_digest: locked.digest().prefixed(),
                target: target.clone(),
                timestamp: iso_timestamp(),
                run_id: None,
                doctor: Some(doctor),
                capability_match: Some(cm),
                verify_report: None,
                checks,
                final_status: EvidenceStatus::Incomplete,
                summary: format!("Verification failed: {e}"),
            };
            return emit_evidence_report(&report, format);
        }
    };

    let report = EvidenceAggregator::build(&locked, None, verify_report, Some(doctor), Some(cm));
    emit_evidence_report(&report, format)
}

fn emit_evidence_report(report: &vaa::EvidenceReport, format: OutputFormat) -> ExitCode {
    match format {
        OutputFormat::Terminal => {
            println!("Task: {} ({})", report.task_id, report.task_digest);
            println!("Target: {}", report.target);
            println!("Timestamp: {}", report.timestamp);
            println!("Status: {:?}", report.final_status);
            println!("{}", report.summary);
            println!("Checks:");
            for check in &report.checks {
                let mark = if check.passed { "PASS" } else { "FAIL" };
                println!("  [{mark}] {}", check.check_name);
                if let Some(ref details) = check.details {
                    if !check.passed {
                        println!("        {details}");
                    }
                }
            }
        }
        OutputFormat::Json => {
            let body = serde_json::to_value(report).expect("serialize report");
            println!("{body}");
        }
    }
    VaaExitCode::from_raw(report.final_status.exit_code()).as_std()
}

fn generate_command(task_path: &PathBuf, output_path: &PathBuf) -> ExitCode {
    let locked = match load_locked_task(task_path) {
        Ok(t) => t,
        Err(error) => {
            eprintln!("error: failed to load task `{}`", task_path.display());
            eprintln!("{error}");
            return VaaExitCode::InvalidInput.as_std();
        }
    };

    let mut adapter = FixtureModelAdapter::new("fixture");
    adapter.add_response(
        &format!("{}::{}", locked.task().task_id, locked.task().target),
        &format!("; Auto-generated by VAA fixture model\n; Task: {}\n; Target: {}\n\nsection .text\nglobal {}\n{}:\n    ret\n",
            locked.task().task_id,
            locked.task().target,
            locked.task().entry.symbol,
            locked.task().entry.symbol),
    );

    match adapter.generate("prompt", &locked.task().task_id, &locked.task().target) {
        Ok(resp) => {
            if let Err(e) = std::fs::write(output_path, &resp.source) {
                eprintln!("error: failed to write `{}`: {e}", output_path.display());
                return VaaExitCode::ToolFailure.as_std();
            }
            println!(
                "generated `{}` (model: {}, id: {})",
                output_path.display(),
                resp.model_name,
                resp.generation_id
            );
            VaaExitCode::Success.as_std()
        }
        Err(e) => {
            eprintln!("error: generation failed: {e}");
            VaaExitCode::ToolFailure.as_std()
        }
    }
}

fn build_command(source: &Path, output_dir: &Path, target: &str, format: OutputFormat) -> ExitCode {
    let config = PipelineConfig {
        source_path: source.to_path_buf(),
        output_dir: output_dir.to_path_buf(),
        target: target.to_owned(),
        ..PipelineConfig::default()
    };

    let outcome = BuildPipeline::build(&config);

    match format {
        OutputFormat::Terminal => {
            if outcome.success {
                println!("Build succeeded");
                println!("  object: {}", outcome.manifest.object_path.display());
                println!("  binary: {}", outcome.manifest.binary_path.display());
            } else {
                eprintln!("Build failed");
                if !outcome.assembler_stderr.is_empty() {
                    eprintln!("  assembler: {}", outcome.assembler_stderr.trim());
                }
                if !outcome.linker_stderr.is_empty() {
                    eprintln!("  linker: {}", outcome.linker_stderr.trim());
                }
            }
        }
        OutputFormat::Json => {
            let body = serde_json::to_value(&outcome).unwrap_or_default();
            println!("{body}");
        }
    }

    if outcome.success {
        VaaExitCode::Success.as_std()
    } else {
        VaaExitCode::ToolFailure.as_std()
    }
}

fn inspect_command(artifact: &Path, format: OutputFormat) -> ExitCode {
    match ArtifactInspector::inspect(artifact) {
        Ok(info) => {
            match format {
                OutputFormat::Terminal => {
                    println!("Artifact: {}", info.path);
                    println!("  size: {} bytes", info.size_bytes);
                    println!("  format: {}", info.format);
                    println!("  architecture: {}", info.architecture);
                    println!("  executable: {}", info.is_executable);
                    println!("  sections: {}", info.section_count);
                    println!("  symbols: {}", info.symbol_count);
                    println!("  imports: {}", info.import_count);
                    println!("  exec stack: {}", info.has_executable_stack);
                    println!("  W^X violation: {}", info.has_wxorx);
                    for w in &info.warnings {
                        println!("  warning: {w}");
                    }
                }
                OutputFormat::Json => {
                    let body = serde_json::to_value(&info).unwrap_or_default();
                    println!("{body}");
                }
            }
            VaaExitCode::Success.as_std()
        }
        Err(e) => {
            eprintln!("error: inspection failed: {e}");
            VaaExitCode::ToolFailure.as_std()
        }
    }
}

fn iso_timestamp() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before epoch");
    let secs = dur.as_secs();
    let subsec = dur.subsec_millis();
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}.{subsec:03}Z")
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d_ = doy - (153 * mp + 2) / 5 + 1;
    let m_ = if mp < 10 { mp + 3 } else { mp - 9 };
    let y_ = if m_ <= 2 { y + 1 } else { y };
    (y_, m_ as u32, d_ as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clap_parses_validate() {
        let cli = Cli::try_parse_from(["vaa", "validate", "task.vaa.toml"]).expect("parse");
        assert!(matches!(cli.command, Some(Commands::Validate { .. })));
    }

    #[test]
    fn clap_parses_doctor() {
        let cli = Cli::try_parse_from(["vaa", "doctor"]).expect("parse");
        assert!(matches!(cli.command, Some(Commands::Doctor { .. })));
    }

    #[test]
    fn clap_parses_capabilities() {
        let cli = Cli::try_parse_from([
            "vaa",
            "capabilities",
            "--target",
            "x86_64-unknown-linux-gnu",
        ])
        .expect("parse");
        assert!(matches!(cli.command, Some(Commands::Capabilities { .. })));
    }

    #[test]
    fn clap_parses_verify() {
        let cli = Cli::try_parse_from([
            "vaa",
            "verify",
            "task.vaa.toml",
            "--source",
            "candidate.asm",
        ])
        .expect("parse");
        assert!(matches!(cli.command, Some(Commands::Verify { .. })));
    }

    #[test]
    fn clap_parses_generate() {
        let cli = Cli::try_parse_from(["vaa", "generate", "task.vaa.toml", "--output", "out.asm"])
            .expect("parse");
        assert!(matches!(cli.command, Some(Commands::Generate { .. })));
    }

    #[test]
    fn clap_parses_build() {
        let cli = Cli::try_parse_from(["vaa", "build", "source.asm", "--output-dir", "out"])
            .expect("parse");
        assert!(matches!(cli.command, Some(Commands::Build { .. })));
    }

    #[test]
    fn clap_parses_inspect() {
        let cli = Cli::try_parse_from(["vaa", "inspect", "artifact.o"]).expect("parse");
        assert!(matches!(cli.command, Some(Commands::Inspect { .. })));
    }
}
