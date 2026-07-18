//! VAA — Verifiable Assembly Agent CLI.

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

use vaa::exit_code::ExitCode as VaaExitCode;
use vaa::task::{load_locked_task, TaskError};
use vaa::{MATURITY, TASK_SCHEMA_VERSION, VAA_VERSION};

/// Verifiable Assembly Agent command-line interface.
#[derive(Debug, Parser)]
#[command(
    name = "vaa",
    version = VAA_VERSION,
    about = "VAA: fail-closed orchestration for model-assisted assembly around SemASM",
    long_about = "VAA converts a constrained task specification into assembly \
candidates, collects evidence from SemASM and the native toolchain, and returns \
an evidence bundle. Current experimental commands: version, status, validate. \
Generation, SemASM verification, and sandbox execution are not implemented yet."
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
    }
}

fn print_status() {
    println!("vaa {VAA_VERSION}");
    println!("maturity: {MATURITY}");
    println!("form: local CLI (single binary crate + library modules)");
    println!("task schema: {TASK_SCHEMA_VERSION}");
    println!("commands: version, status, validate");
    println!("default mode: verify-only (verify pipeline not implemented yet)");
    println!("model adapter: not implemented");
    println!("SemASM integration: not implemented (see docs/implementation-baseline.md)");
    println!("supported generated target: none claimed yet");
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
                    println!("  entry: {} ({})", locked.task().entry.symbol, locked.task().entry.abi);
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
                TaskError::Io { .. } | TaskError::Parse { .. } | TaskError::Validation(_) | TaskError::ValidationMany { .. } => {
                    VaaExitCode::InvalidInput.as_std()
                }
            }
        }
    }
}

fn emit_validate_error(path: &std::path::Path, format: OutputFormat, error: &TaskError) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clap_parses_validate() {
        let cli = Cli::try_parse_from(["vaa", "validate", "task.vaa.toml"]).expect("parse");
        assert!(matches!(
            cli.command,
            Some(Commands::Validate { .. })
        ));
    }
}
