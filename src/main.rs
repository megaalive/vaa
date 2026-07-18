//! VAA — Verifiable Assembly Agent.
//!
//! This binary is the offline controller shell. Functional verification,
//! generation, and sandbox pipelines are added in later pull requests.
//! Until those land, this crate intentionally exposes only bootstrap commands
//! and makes no claim that assembly has been verified.

#![forbid(unsafe_code)]

use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Package version embedded at compile time.
pub const VAA_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maturity label for the current tree. Keep this honest.
pub const MATURITY: &str = "experimental-bootstrap";

/// Verifiable Assembly Agent command-line interface.
#[derive(Debug, Parser)]
#[command(
    name = "vaa",
    version = VAA_VERSION,
    about = "VAA: fail-closed orchestration for model-assisted assembly around SemASM",
    long_about = "VAA converts a constrained task specification into assembly \
candidates, collects evidence from SemASM and the native toolchain, and returns \
an evidence bundle. This bootstrap build only provides version and status \
commands. It does not verify assembly and does not call a model."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Print version and bootstrap maturity (no functional claims).
    Version,
    /// Show high-level project status for this bootstrap tree.
    Status,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Status) {
        Commands::Version => {
            println!("vaa {VAA_VERSION}");
            ExitCode::SUCCESS
        }
        Commands::Status => {
            print_status();
            ExitCode::SUCCESS
        }
    }
}

fn print_status() {
    println!("vaa {VAA_VERSION}");
    println!("maturity: {MATURITY}");
    println!("form: local CLI bootstrap (single binary crate)");
    println!("default mode: verify-only (not yet implemented)");
    println!("model adapter: not implemented");
    println!("SemASM integration: not implemented (see docs/implementation-baseline.md)");
    println!("supported generated target: none claimed in this bootstrap");
    println!("note: absence of errors here is not evidence that any assembly is verified");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_constant_is_nonempty() {
        assert!(!VAA_VERSION.is_empty());
    }

    #[test]
    fn maturity_is_bootstrap_not_production() {
        assert!(MATURITY.contains("bootstrap") || MATURITY.contains("experimental"));
        assert!(!MATURITY.contains("production"));
    }

    #[test]
    fn clap_parses_version_subcommand() {
        let cli = Cli::try_parse_from(["vaa", "version"]).expect("parse version");
        assert!(matches!(cli.command, Some(Commands::Version)));
    }

    #[test]
    fn clap_parses_status_subcommand() {
        let cli = Cli::try_parse_from(["vaa", "status"]).expect("parse status");
        assert!(matches!(cli.command, Some(Commands::Status)));
    }
}
