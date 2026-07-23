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
    ingest_candidate, keygen_seal, probe_live_for_target, run_fixture_loop, sha256_digest_prefixed,
    verify_bundle, verify_chain, verify_seal, verify_transparency_against_run,
    write_transparency_file, ArtifactInspector, BuildPipeline, EvidenceAggregator, EvidenceExpect,
    EvidenceStatus, FixtureModelAdapter, ModelAdapter, PipelineConfig, RunConfig, RunDir, RunId,
    SemasmDoctor, SemasmVerify, TargetCapabilities, VerifyError, MATURITY, TASK_SCHEMA_VERSION,
    VAA_VERSION,
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
        /// Path to the SemASM `*.sem.toml` contract (not the VAA task file).
        #[arg(long)]
        contract: PathBuf,
        /// Forward `--allow-execution` to SemASM (opt-in behavioral verify).
        #[arg(long, default_value_t = false)]
        allow_execution: bool,
        /// Opt-in local content-addressed cache (PR-020). Default off for deterministic Gate CI.
        #[arg(long, default_value_t = false)]
        cache: bool,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Run fixture-driven generate → verify → repair → evidence loop.
    Run {
        /// Path to the locked task file.
        task: PathBuf,
        /// Path to the SemASM `*.sem.toml` contract.
        #[arg(long)]
        contract: PathBuf,
        /// Directory that will contain the run folder.
        #[arg(long, default_value = ".")]
        run_dir: PathBuf,
        /// Wrong candidate source (first generation).
        #[arg(long)]
        wrong: PathBuf,
        /// Repaired candidate source (second generation).
        #[arg(long)]
        repaired: PathBuf,
        /// Forward `--allow-execution` to SemASM (opt-in behavioral verify).
        #[arg(long, default_value_t = false)]
        allow_execution: bool,
        /// Resume an existing run directory (E1); skips already-sealed candidates.
        #[arg(long)]
        resume: Option<PathBuf>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Ingest an external candidate (no model) and seal evidence.
    Ingest {
        /// Path to the locked task file.
        task: PathBuf,
        /// Path to the SemASM `*.sem.toml` contract.
        #[arg(long)]
        contract: PathBuf,
        /// Path to the candidate assembly source.
        #[arg(long)]
        source: PathBuf,
        /// Untrusted generator name for seal attribution.
        #[arg(long, default_value = "external")]
        generator: String,
        /// Directory that will contain the run folder.
        #[arg(long, default_value = ".")]
        run_dir: PathBuf,
        /// Forward `--allow-execution` to SemASM (opt-in behavioral verify).
        #[arg(long, default_value_t = false)]
        allow_execution: bool,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Evidence seal utilities.
    Evidence {
        #[command(subcommand)]
        command: EvidenceCommands,
    },
    /// Generate assembly from a locked task via model adapter.
    Generate {
        /// Path to the locked task file.
        task: PathBuf,
        /// Output path for generated source (required unless `--run-dir` is set).
        #[arg(long)]
        output: Option<PathBuf>,
        /// Run base directory: create a run and write default output under `staging/`.
        #[arg(long)]
        run_dir: Option<PathBuf>,
        /// External generator program (G1). Requires `--run-dir`. Writes `candidate.asm` under staging.
        #[arg(long)]
        command: Option<PathBuf>,
        /// Arguments forwarded to `--command` (use `--` before them).
        #[arg(last = true)]
        command_args: Vec<String>,
        /// Opt-in live OpenAI-compatible generate (requires feature `live-model` + `VAA_MODEL_API_KEY`).
        #[arg(long, default_value_t = false)]
        live: bool,
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
        /// Sandbox backend: `local` (default) or `container` (Docker/Podman Scaffold).
        #[arg(long, value_enum, default_value_t = BuildSandboxMode::Local)]
        sandbox: BuildSandboxMode,
        /// Container image (default `ubuntu:24.04`). Also `VAA_CONTAINER_IMAGE`.
        #[arg(long, env = "VAA_CONTAINER_IMAGE")]
        container_image: Option<String>,
        /// Optional image digest pin (`sha256:…`). Also `VAA_CONTAINER_IMAGE_DIGEST`.
        #[arg(long, env = "VAA_CONTAINER_IMAGE_DIGEST")]
        container_image_digest: Option<String>,
        /// Container runtime binary (`docker` or `podman`). Also `VAA_CONTAINER_RUNTIME`.
        #[arg(long, env = "VAA_CONTAINER_RUNTIME")]
        container_runtime: Option<String>,
        /// Docker/Podman `--cpus` quota when `--sandbox container`.
        #[arg(long)]
        cpu_quota: Option<f64>,
        /// Opt-in local content-addressed build cache (PR-020).
        #[arg(long, default_value_t = false)]
        cache: bool,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Local content-addressed cache utilities (PR-020).
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
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

#[derive(Debug, Subcommand)]
enum CacheCommands {
    /// Print cache root and entry counts.
    Status,
}

#[derive(Debug, Subcommand)]
enum EvidenceCommands {
    /// Verify `evidence.json` against `evidence.seal.json` (JSON drift only).
    CheckSeal {
        /// Path to evidence.json.
        evidence: PathBuf,
        /// Path to evidence.seal.json.
        seal: PathBuf,
    },
    /// Re-hash on-disk artifacts in a bundle directory against sealed digests.
    VerifyBundle {
        /// Directory containing task/contract/candidate/report/evidence/seal.
        bundle_dir: PathBuf,
    },
    /// Verify the full candidate seal chain for a run directory.
    VerifyChain {
        /// Run directory containing `candidates/` and `evidence/final*.json`.
        run_dir: PathBuf,
    },
    /// Export digests for external storage (CI artifact / Git note).
    ExportTransparency {
        /// Run directory.
        run_dir: PathBuf,
        /// Output JSON path (`vaa-transparency-v1`).
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Verify an exported transparency file against a live run directory.
    VerifyTransparency {
        /// Path to transparency JSON.
        file: PathBuf,
        /// Run directory to compare against.
        #[arg(long)]
        against: PathBuf,
    },
    /// Generate a 32-byte hex Ed25519 seed file for optional seal signing.
    KeygenSeal {
        /// Output path for the hex seed file.
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum BuildSandboxMode {
    Local,
    Container,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
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
            contract,
            allow_execution,
            cache,
            format,
        } => verify_command(&task, &source, &contract, allow_execution, cache, format),
        Commands::Run {
            task,
            contract,
            run_dir,
            wrong,
            repaired,
            allow_execution,
            resume,
            format,
        } => run_command(
            &task,
            &contract,
            &run_dir,
            &wrong,
            &repaired,
            allow_execution,
            resume.as_deref(),
            format,
        ),
        Commands::Ingest {
            task,
            contract,
            source,
            generator,
            run_dir,
            allow_execution,
            format,
        } => ingest_command(
            &task,
            &contract,
            &source,
            &generator,
            &run_dir,
            allow_execution,
            format,
        ),
        Commands::Evidence { command } => match command {
            EvidenceCommands::CheckSeal { evidence, seal } => check_seal_command(&evidence, &seal),
            EvidenceCommands::VerifyBundle { bundle_dir } => verify_bundle_command(&bundle_dir),
            EvidenceCommands::VerifyChain { run_dir } => verify_chain_command(&run_dir),
            EvidenceCommands::ExportTransparency { run_dir, output } => {
                export_transparency_command(&run_dir, &output)
            }
            EvidenceCommands::VerifyTransparency { file, against } => {
                verify_transparency_command(&file, &against)
            }
            EvidenceCommands::KeygenSeal { out } => keygen_seal_command(&out),
        },
        Commands::Generate {
            task,
            output,
            run_dir,
            command,
            command_args,
            live,
        } => generate_command(
            &task,
            output.as_deref(),
            run_dir.as_deref(),
            command.as_deref(),
            &command_args,
            live,
        ),
        Commands::Build {
            source,
            output_dir,
            target,
            sandbox,
            container_image,
            container_image_digest,
            container_runtime,
            cpu_quota,
            cache,
            format,
        } => build_command(
            &source,
            &output_dir,
            &target,
            sandbox,
            container_image.as_deref(),
            container_image_digest.as_deref(),
            container_runtime.as_deref(),
            cpu_quota,
            cache,
            format,
        ),
        Commands::Cache { command } => match command {
            CacheCommands::Status => cache_status_command(),
        },
        Commands::Inspect { artifact, format } => inspect_command(&artifact, format),
    }
}

fn print_status() {
    let container = vaa::probe_container_runtime();
    println!("vaa {VAA_VERSION}");
    println!("maturity: {MATURITY}");
    println!("form: local CLI (single binary crate + library modules)");
    println!("task schema: {TASK_SCHEMA_VERSION}");
    println!("commands: version, status, validate, doctor, capabilities, verify, run, ingest, evidence, generate, build, cache, inspect");
    println!("default mode: verify-only (run=fixture; ingest=external; live LLM opt-in)");
    println!(
        "model adapter: fixture default; --live needs --features live-model + VAA_MODEL_API_KEY"
    );
    println!(
        "cache: local `.vaa/cache` opt-in via --cache / VAA_CACHE_DIR (PR-020; not remote log)"
    );
    println!("SemASM integration: doctor + verify via ProcessRunner (stdout-only report 0.4)");
    println!("evidence: integrity seals (check-seal=JSON drift; verify-bundle=artifact rehash)");
    println!("evidence note: opt-in Ed25519 when VAA_SEAL_SIGNING_KEY is set (not a trust root)");
    println!("SemASM execution: default static-only; pass --allow-execution for Gate-2 Verified");
    println!("build pipeline: nasm + ld (needs toolchain on PATH); --sandbox container = Scaffold");
    println!(
        "container_runtime: {}",
        container.as_deref().unwrap_or("unavailable")
    );
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
    let evidence_policy = vaa::EvidencePolicy::vaa_g0();
    let container_runtime = vaa::probe_container_runtime();
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
            if let Some(probe) = &report.live_probe {
                println!("  live_probe:");
                if let Some(v) = &probe.semasm_version {
                    println!("    semasm_version: {v}");
                }
                if let Some(s) = &probe.capability_schema {
                    println!("    capability_schema: {s}");
                }
                for cmp in &probe.compares {
                    println!(
                        "    {}: {:?} agent={:?} pipeline={:?}",
                        cmp.target_id, cmp.outcome, cmp.live_agent, cmp.live_pipeline
                    );
                    for axis in &cmp.axes {
                        println!("      - {axis}");
                    }
                }
            }
            println!(
                "  container_runtime: {} (Scaffold; not hardened isolation)",
                container_runtime.as_deref().unwrap_or("unavailable")
            );
            println!("  evidence_policy:");
            println!(
                "    generator_staging: {}",
                evidence_policy.generator_staging
            );
            println!("    evidence_writes: {}", evidence_policy.evidence_writes);
            println!(
                "    rundir_protected_zone: {}",
                evidence_policy.rundir_protected_zone
            );
            println!(
                "    os_fs_isolation: {} (logical G0 barrier only)",
                evidence_policy.os_fs_isolation
            );
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
                "live_probe": report.live_probe,
                "container_runtime": container_runtime,
                "evidence_policy": evidence_policy,
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
    let live = probe_live_for_target(target);
    match format {
        OutputFormat::Terminal => {
            println!("Target: {}", caps.target_id);
            println!("  source:         {}", vaa::CAPABILITY_SOURCE);
            println!("  decode:         {:?}", caps.decode);
            println!("  lower:          {:?}", caps.lower);
            println!("  abi_check:      {:?}", caps.abi_check);
            println!("  object_inspect: {:?}", caps.object_inspect);
            println!("  assemble:       {:?}", caps.assemble);
            println!("  link:           {:?}", caps.link);
            println!("  sandbox_run:    {:?}", caps.sandbox_run);
            println!("  digest: {}", caps.digest());
            println!("note: embedded agent-verify snapshot; not live SemASM capabilities.toml");
            if let Some((doc, cmp)) = &live {
                println!(
                    "live_probe: schema={:?} version={:?} compare={:?} agent={:?} pipeline={:?}",
                    doc.capability_schema,
                    doc.version,
                    cmp.outcome,
                    cmp.live_agent,
                    cmp.live_pipeline
                );
                for axis in &cmp.axes {
                    println!("  - {axis}");
                }
            } else {
                println!("live_probe: unavailable (semasm not on PATH or status JSON failed)");
            }
        }
        OutputFormat::Json => {
            let live_json = live.as_ref().map(|(doc, cmp)| {
                serde_json::json!({
                    "semasm_version": doc.version,
                    "capability_schema": doc.capability_schema,
                    "compare": cmp,
                })
            });
            let body = serde_json::json!({
                "source": vaa::CAPABILITY_SOURCE,
                "target_id": caps.target_id,
                "decode": format!("{:?}", caps.decode),
                "lower": format!("{:?}", caps.lower),
                "abi_check": format!("{:?}", caps.abi_check),
                "object_inspect": format!("{:?}", caps.object_inspect),
                "assemble": format!("{:?}", caps.assemble),
                "link": format!("{:?}", caps.link),
                "sandbox_run": format!("{:?}", caps.sandbox_run),
                "digest": caps.digest(),
                "live_probe": live_json,
            });
            println!("{body}");
        }
    }
    VaaExitCode::Success.as_std()
}

fn verify_command(
    task_path: &Path,
    source_path: &Path,
    contract_path: &Path,
    allow_execution: bool,
    use_cache: bool,
    format: OutputFormat,
) -> ExitCode {
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

    let source_bytes = match std::fs::read(source_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("error: cannot read source: {error}");
            return VaaExitCode::ToolFailure.as_std();
        }
    };
    let contract_bytes = match std::fs::read(contract_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("error: cannot read contract: {error}");
            return VaaExitCode::ToolFailure.as_std();
        }
    };
    let source_digest = sha256_digest_prefixed(&source_bytes);
    let contract_digest = sha256_digest_prefixed(&contract_bytes);

    let cache_materials = || {
        let semasm_version = doctor
            .version
            .as_ref()
            .map_or_else(|| "unknown".to_owned(), |v| v.version.clone());
        vaa::VerificationKeyMaterials {
            source_digest: source_digest.clone(),
            contract_digest: contract_digest.clone(),
            task_digest: locked.digest().prefixed(),
            target: target.clone(),
            semasm_version,
            allow_execution,
            capability_source: vaa::CAPABILITY_SOURCE.to_owned(),
        }
    };

    let verify_report = {
        let mut cached: Option<vaa::VerifyReport> = None;
        if use_cache
            && matches!(
                doctor.status,
                vaa::DoctorStatus::Available | vaa::DoctorStatus::Degraded
            )
            && cm.compatible
        {
            let store = vaa::CacheStore::open(vaa::resolve_cache_root());
            let mat = cache_materials();
            // Prefer exact status reuse (including Incomplete); never promote to Verified via policy.
            if let Ok((_rec, raw)) = store.get_verification(&mat, false) {
                if let Ok(parsed) = SemasmVerify::parse_report(&raw) {
                    cached = Some(parsed);
                }
            }
        }

        if let Some(report) = cached {
            report
        } else {
            let verify_result = match doctor.binary_path.as_ref() {
                Some(binary) => {
                    SemasmVerify::run(source_path, contract_path, binary, target, allow_execution)
                }
                None => Err(VerifyError::BinaryNotFound),
            };
            match verify_result {
                Ok(report) => {
                    if use_cache {
                        let store = vaa::CacheStore::open(vaa::resolve_cache_root());
                        let status = match report.outcome {
                            EvidenceStatus::Verified => "Verified",
                            EvidenceStatus::Violated => "Violated",
                            EvidenceStatus::Incomplete => "Incomplete",
                            EvidenceStatus::Failed => "Failed",
                        };
                        let _ = store.put_verification(
                            &cache_materials(),
                            status,
                            &report.raw_json,
                            Some(&report.raw_status),
                        );
                    }
                    report
                }
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
                        passed: matches!(
                            doctor.status,
                            vaa::DoctorStatus::Available | vaa::DoctorStatus::Degraded
                        ),
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
                        final_status: EvidenceStatus::Failed,
                        summary: format!("Verification failed: {e}"),
                    };
                    return emit_evidence_report(&report, format);
                }
            }
        }
    };

    let mut expect = EvidenceExpect::new(target.clone(), source_digest, contract_digest);
    if locked.task().verification.require_object_inspection {
        let inspect_dir = std::env::temp_dir().join(format!(
            "vaa_verify_inspect_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        let _ = std::fs::create_dir_all(&inspect_dir);
        expect.object_inspection =
            Some(vaa::assemble_and_inspect(source_path, &inspect_dir, target));
        let _ = std::fs::remove_dir_all(&inspect_dir);
    }

    let report = EvidenceAggregator::build(
        &locked,
        None,
        Some(verify_report),
        Some(doctor),
        Some(cm),
        &expect,
    );
    emit_evidence_report(&report, format)
}

fn cache_status_command() -> ExitCode {
    let store = vaa::CacheStore::open(vaa::resolve_cache_root());
    let _ = store.ensure_layout();
    let stats = store.stats();
    println!("cache root: {}", stats.root);
    println!("blobs: {}", stats.blobs);
    println!("verification entries: {}", stats.verification_entries);
    println!("build entries: {}", stats.build_entries);
    println!("note: local content-addressed store ≠ remote immutable log");
    VaaExitCode::Success.as_std()
}

fn run_command(
    task_path: &Path,
    contract_path: &Path,
    run_base: &Path,
    wrong_path: &Path,
    repaired_path: &Path,
    allow_execution: bool,
    resume: Option<&Path>,
    format: OutputFormat,
) -> ExitCode {
    let wrong = match std::fs::read_to_string(wrong_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read wrong candidate: {e}");
            return VaaExitCode::InvalidInput.as_std();
        }
    };
    let repaired = match std::fs::read_to_string(repaired_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read repaired candidate: {e}");
            return VaaExitCode::InvalidInput.as_std();
        }
    };

    let config = RunConfig {
        task_path,
        contract_path,
        run_base,
        fixture_sources: vec![wrong, repaired],
        max_attempts: 4,
        allow_execution,
        resume_root: resume,
    };

    match run_fixture_loop(&config) {
        Ok(outcome) => {
            if format == OutputFormat::Terminal {
                println!("Run root: {}", outcome.run_root.display());
                println!("Candidates accepted: {}", outcome.candidates_accepted);
                println!("Transitions: {}", outcome.transitions);
            }
            emit_evidence_report(&outcome.evidence, format)
        }
        Err(e) => {
            eprintln!("error: {e}");
            if matches!(e, vaa::RunError::BudgetExhausted(_)) {
                VaaExitCode::BudgetExhausted.as_std()
            } else {
                VaaExitCode::ToolFailure.as_std()
            }
        }
    }
}

fn ingest_command(
    task_path: &Path,
    contract_path: &Path,
    source_path: &Path,
    generator: &str,
    run_base: &Path,
    allow_execution: bool,
    format: OutputFormat,
) -> ExitCode {
    let locked = match load_locked_task(task_path) {
        Ok(t) => t,
        Err(error) => {
            emit_validate_error(task_path, format, &error);
            return VaaExitCode::InvalidInput.as_std();
        }
    };

    let run_id = RunId::generate();
    let run_dir = match RunDir::create(run_base, &run_id) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: run dir: {e}");
            return VaaExitCode::ToolFailure.as_std();
        }
    };

    let mut events = vaa::EventLog::new(run_dir.event_log_path().to_path_buf());
    let _ = events.record(vaa::EventKind::RunStarted {
        task_id: locked.task().task_id.clone(),
        task_digest: locked.digest().prefixed(),
    });
    let _ = events.record(vaa::EventKind::CandidateSubmitted {
        index: 0,
        source_path: source_path.display().to_string(),
    });
    let _ = events.record(vaa::EventKind::VerificationStarted);

    match ingest_candidate(
        &locked,
        task_path,
        contract_path,
        source_path,
        &run_dir,
        run_id.as_str(),
        generator,
        locked.task().budgets.max_candidates.max(1),
        allow_execution,
    ) {
        Ok(outcome) => {
            let _ = events.record(vaa::EventKind::CandidateAccepted {
                index: outcome.candidate_index,
            });
            let _ = events.record(vaa::EventKind::VerificationCompleted {
                outcome: format!("{:?}", outcome.evidence.final_status),
            });
            let _ = events.record(vaa::EventKind::RunFinished {
                outcome: format!("{:?}", outcome.evidence.final_status),
                candidate_count: 1,
            });
            if format == OutputFormat::Terminal {
                println!("Run root: {}", run_dir.root().display());
                println!("Candidate dir: {}", outcome.candidate_dir.display());
                println!("Acceptance digest: {}", outcome.seal.acceptance_digest);
                println!("Envelope digest: {}", outcome.seal.envelope_digest);
                println!(
                    "Generator: {} / {}",
                    outcome.seal.provenance.generator.kind, outcome.seal.provenance.generator.name
                );
            }
            emit_evidence_report(&outcome.evidence, format)
        }
        Err(e) => {
            let _ = events.record(vaa::EventKind::Error {
                message: e.to_string(),
            });
            eprintln!("error: {e}");
            VaaExitCode::ToolFailure.as_std()
        }
    }
}

fn check_seal_command(evidence: &Path, seal: &Path) -> ExitCode {
    match verify_seal(evidence, seal) {
        Ok(()) => {
            println!("ok: evidence/seal JSON integrity verified (not artifact rehash)");
            VaaExitCode::Success.as_std()
        }
        Err(e) => {
            eprintln!("error: seal check failed: {e}");
            VaaExitCode::ToolFailure.as_std()
        }
    }
}

fn verify_bundle_command(bundle_dir: &Path) -> ExitCode {
    match verify_bundle(bundle_dir) {
        Ok(envelope) => {
            println!("ok: bundle verified against sealed digests");
            println!("  acceptance_digest: {}", envelope.acceptance_digest);
            println!("  envelope_digest: {}", envelope.envelope_digest);
            VaaExitCode::Success.as_std()
        }
        Err(e) => {
            eprintln!("error: bundle verify failed: {e}");
            VaaExitCode::ToolFailure.as_std()
        }
    }
}

fn verify_chain_command(run_dir: &Path) -> ExitCode {
    match verify_chain(run_dir) {
        Ok(report) => {
            println!(
                "ok: seal chain verified ({} candidates)",
                report.candidate_count
            );
            println!(
                "  last acceptance_digest: {}",
                report.last_acceptance_digest
            );
            println!("  last envelope_digest: {}", report.last_envelope_digest);
            VaaExitCode::Success.as_std()
        }
        Err(e) => {
            eprintln!("error: chain verify failed: {e}");
            VaaExitCode::ToolFailure.as_std()
        }
    }
}

fn export_transparency_command(run_dir: &Path, output: &Path) -> ExitCode {
    match write_transparency_file(run_dir, output) {
        Ok(doc) => {
            println!(
                "ok: transparency exported ({} entries) → {}",
                doc.entries.len(),
                output.display()
            );
            println!("  schema: {}", doc.schema_version);
            println!("  final_envelope_digest: {}", doc.final_envelope_digest);
            VaaExitCode::Success.as_std()
        }
        Err(e) => {
            eprintln!("error: transparency export failed: {e}");
            VaaExitCode::ToolFailure.as_std()
        }
    }
}

fn verify_transparency_command(file: &Path, against: &Path) -> ExitCode {
    match verify_transparency_against_run(file, against) {
        Ok(()) => {
            println!("ok: transparency matches run digests");
            VaaExitCode::Success.as_std()
        }
        Err(e) => {
            eprintln!("error: transparency verify failed: {e}");
            VaaExitCode::ToolFailure.as_std()
        }
    }
}

fn keygen_seal_command(out: &Path) -> ExitCode {
    match keygen_seal(out) {
        Ok((pk_hex, pk_b64)) => {
            println!("ok: wrote Ed25519 seed → {}", out.display());
            println!("  public_key_hex: {pk_hex}");
            println!("  public_key_b64: {pk_b64}");
            println!("  set VAA_SEAL_SIGNING_KEY={}", out.display());
            VaaExitCode::Success.as_std()
        }
        Err(e) => {
            eprintln!("error: keygen-seal failed: {e}");
            VaaExitCode::ToolFailure.as_std()
        }
    }
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

fn generate_command(
    task_path: &Path,
    output_path: Option<&Path>,
    run_base: Option<&Path>,
    command: Option<&Path>,
    command_args: &[String],
    live: bool,
) -> ExitCode {
    let locked = match load_locked_task(task_path) {
        Ok(t) => t,
        Err(error) => {
            eprintln!("error: failed to load task `{}`", task_path.display());
            eprintln!("{error}");
            return VaaExitCode::InvalidInput.as_std();
        }
    };

    if command.is_some() && live {
        eprintln!("error: `--command` and `--live` are mutually exclusive");
        return VaaExitCode::InvalidInput.as_std();
    }

    if command.is_some() && run_base.is_none() {
        eprintln!("error: `--command` requires `--run-dir` (G1 staging cwd)");
        return VaaExitCode::InvalidInput.as_std();
    }

    let rundir = match run_base {
        Some(base) => match RunDir::create(base, &RunId::generate()) {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!(
                    "error: failed to create run dir under `{}`: {e}",
                    base.display()
                );
                return VaaExitCode::ToolFailure.as_std();
            }
        },
        None => None,
    };

    if let Some(prog) = command {
        let rd = rundir.as_ref().expect("run-dir checked");
        let relative = vaa::DEFAULT_STAGING_OUTPUT.to_owned();
        let _ = output_path; // G1 always stages `candidate.asm`; explicit --output outside staging ignored.

        let gen = vaa::ArgvExternalGenerator::new("external-argv", prog)
            .with_args(command_args.to_vec())
            .with_output_relative(relative.clone());

        return match gen.generate_to_staging(
            &rd.paths().staging_dir,
            task_path,
            &locked.task().task_id,
            &locked.task().target,
        ) {
            Ok(resp) => match rd.write_staging(&relative, resp.source.as_bytes()) {
                Ok(written) => {
                    println!(
                        "generated `{}` (model: {}, id: {}, kind: external-argv)",
                        written.display(),
                        resp.model_name,
                        resp.generation_id
                    );
                    VaaExitCode::Success.as_std()
                }
                Err(e) => {
                    eprintln!("error: staging write failed: {e}");
                    VaaExitCode::ToolFailure.as_std()
                }
            },
            Err(e) => {
                eprintln!("error: external generator failed: {e}");
                VaaExitCode::ToolFailure.as_std()
            }
        };
    }

    let resolved_output: PathBuf = match (output_path, rundir.as_ref()) {
        (Some(path), Some(rd)) if rd.is_protected_path(path) => {
            eprintln!(
                "error: output `{}` is in the protected evidence zone",
                path.display()
            );
            return VaaExitCode::InvalidInput.as_std();
        }
        (Some(path), _) => path.to_path_buf(),
        (None, Some(rd)) => {
            let name = format!("{}.asm", locked.task().task_id);
            match rd.staging_join(&name) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("error: staging path: {e}");
                    return VaaExitCode::ToolFailure.as_std();
                }
            }
        }
        (None, None) => {
            eprintln!("error: provide `--output <path>` or `--run-dir <base>`");
            return VaaExitCode::InvalidInput.as_std();
        }
    };

    let response = if live {
        #[cfg(feature = "live-model")]
        {
            let Ok(cfg) = vaa::LiveModelConfig::from_env() else {
                eprintln!(
                    "error: `--live` requires VAA_MODEL_API_KEY (optional VAA_MODEL_BASE_URL / VAA_MODEL_NAME)"
                );
                return VaaExitCode::InvalidInput.as_std();
            };
            let prompt = vaa::build_generation_prompt(
                &locked.task().task_id,
                &locked.task().target,
                &locked.task().entry.symbol,
                &locked.task().entry.abi,
                &locked.task().behavior.summary,
            );
            let adapter = vaa::OpenAiCompatibleAdapter::new(cfg);
            match adapter.generate(&prompt, &locked.task().task_id, &locked.task().target) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("error: live generation failed: {e}");
                    return VaaExitCode::ToolFailure.as_std();
                }
            }
        }
        #[cfg(not(feature = "live-model"))]
        {
            let _ = locked;
            eprintln!(
                "error: `--live` requires building with `--features live-model` (see DEPENDENCIES.md)"
            );
            return VaaExitCode::InvalidInput.as_std();
        }
    } else {
        let mut adapter = FixtureModelAdapter::new("fixture");
        adapter.add_response(
            &format!("{}::{}", locked.task().task_id, locked.task().target),
            &format!(
                "; Auto-generated by VAA fixture model\n; Task: {}\n; Target: {}\n\nsection .text\nglobal {}\n{}:\n    ret\n",
                locked.task().task_id,
                locked.task().target,
                locked.task().entry.symbol,
                locked.task().entry.symbol
            ),
        );
        match adapter.generate("prompt", &locked.task().task_id, &locked.task().target) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("error: generation failed: {e}");
                return VaaExitCode::ToolFailure.as_std();
            }
        }
    };

    let kind = if live {
        "live-openai-compatible"
    } else {
        "fixture"
    };
    write_generated_source(rundir.as_ref(), &resolved_output, &response, kind)
}

fn write_generated_source(
    rundir: Option<&RunDir>,
    resolved_output: &Path,
    resp: &vaa::ModelResponse,
    kind: &str,
) -> ExitCode {
    if let Some(rd) = rundir {
        if resolved_output.starts_with(&rd.paths().staging_dir) {
            let rel = resolved_output
                .strip_prefix(&rd.paths().staging_dir)
                .unwrap_or(resolved_output);
            let rel_str = rel.to_string_lossy();
            return match rd.write_staging(rel_str.as_ref(), resp.source.as_bytes()) {
                Ok(written) => {
                    println!(
                        "generated `{}` (model: {}, id: {}, kind: {kind})",
                        written.display(),
                        resp.model_name,
                        resp.generation_id
                    );
                    VaaExitCode::Success.as_std()
                }
                Err(e) => {
                    eprintln!("error: staging write failed: {e}");
                    VaaExitCode::ToolFailure.as_std()
                }
            };
        }
    }
    if let Some(parent) = resolved_output.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("error: failed to create `{}`: {e}", parent.display());
                return VaaExitCode::ToolFailure.as_std();
            }
        }
    }
    if let Err(e) = std::fs::write(resolved_output, &resp.source) {
        eprintln!(
            "error: failed to write `{}`: {e}",
            resolved_output.display()
        );
        return VaaExitCode::ToolFailure.as_std();
    }
    println!(
        "generated `{}` (model: {}, id: {}, kind: {kind})",
        resolved_output.display(),
        resp.model_name,
        resp.generation_id
    );
    VaaExitCode::Success.as_std()
}

#[allow(clippy::too_many_arguments)]
fn build_command(
    source: &Path,
    output_dir: &Path,
    target: &str,
    sandbox: BuildSandboxMode,
    container_image: Option<&str>,
    container_image_digest: Option<&str>,
    container_runtime: Option<&str>,
    cpu_quota: Option<f64>,
    use_cache: bool,
    format: OutputFormat,
) -> ExitCode {
    let container = match sandbox {
        BuildSandboxMode::Local => None,
        BuildSandboxMode::Container => {
            let runtime = container_runtime
                .map(str::to_owned)
                .or_else(vaa::probe_container_runtime)
                .unwrap_or_else(|| "docker".to_owned());
            Some(vaa::ContainerBuildOpts {
                runtime,
                image: container_image
                    .unwrap_or(vaa::DEFAULT_CONTAINER_IMAGE)
                    .to_owned(),
                image_digest: container_image_digest.map(str::to_owned),
                cpu_quota,
                pids_limit: Some(256),
            })
        }
    };

    let config = PipelineConfig {
        source_path: source.to_path_buf(),
        output_dir: output_dir.to_path_buf(),
        target: target.to_owned(),
        container,
        ..PipelineConfig::default()
    };

    // Opt-in build cache: restore object/binary when toolchain digests match.
    if use_cache {
        if let Ok(source_bytes) = std::fs::read(source) {
            let source_digest = sha256_digest_prefixed(&source_bytes);
            let as_digest = vaa::tool_digest(Path::new("nasm")).unwrap_or_default();
            let ld_digest = vaa::tool_digest(Path::new("ld")).unwrap_or_default();
            let mat = vaa::BuildKeyMaterials {
                source_digest,
                target: target.to_owned(),
                assembler_digest: as_digest,
                linker_digest: ld_digest,
                assembler_args_fingerprint: vaa::args_fingerprint(&[
                    "-f".into(),
                    target.to_owned(),
                ]),
                linker_args_fingerprint: vaa::args_fingerprint(&[]),
                container_image_digest: container_image_digest.unwrap_or("").to_owned(),
            };
            let store = vaa::CacheStore::open(vaa::resolve_cache_root());
            if let Ok(arts) = store.get_build(&mat) {
                let _ = std::fs::create_dir_all(output_dir);
                let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
                let object_path = output_dir.join(format!("{stem}.o"));
                let binary_path = output_dir.join(format!("{stem}.bin"));
                if std::fs::write(&object_path, &arts.object).is_ok() {
                    if let Some(bin) = arts.binary {
                        let _ = std::fs::write(&binary_path, bin);
                    }
                    match format {
                        OutputFormat::Terminal => {
                            println!("Build cache hit");
                            println!("  object: {}", object_path.display());
                            println!("  binary: {}", binary_path.display());
                        }
                        OutputFormat::Json => {
                            println!(
                                "{}",
                                serde_json::json!({
                                    "success": true,
                                    "cache_hit": true,
                                    "object": object_path,
                                    "binary": binary_path,
                                })
                            );
                        }
                    }
                    return VaaExitCode::Success.as_std();
                }
            }
        }
    }

    let outcome = BuildPipeline::build(&config);

    if use_cache && outcome.success {
        if let (Ok(source_bytes), Ok(object_bytes)) = (
            std::fs::read(source),
            std::fs::read(&outcome.manifest.object_path),
        ) {
            let binary_bytes = std::fs::read(&outcome.manifest.binary_path).ok();
            let mat = vaa::BuildKeyMaterials {
                source_digest: sha256_digest_prefixed(&source_bytes),
                target: target.to_owned(),
                assembler_digest: outcome
                    .manifest
                    .assembler_digest
                    .clone()
                    .unwrap_or_default(),
                linker_digest: outcome.manifest.linker_digest.clone().unwrap_or_default(),
                assembler_args_fingerprint: vaa::args_fingerprint(&[
                    "-f".into(),
                    target.to_owned(),
                ]),
                linker_args_fingerprint: vaa::args_fingerprint(&[]),
                container_image_digest: container_image_digest.unwrap_or("").to_owned(),
            };
            let store = vaa::CacheStore::open(vaa::resolve_cache_root());
            let manifest_json = serde_json::to_string(&outcome.manifest).unwrap_or_default();
            let _ = store.put_build(&mat, &object_bytes, binary_bytes.as_deref(), &manifest_json);
        }
    }

    match format {
        OutputFormat::Terminal => {
            if outcome.success {
                println!("Build succeeded");
                println!("  object: {}", outcome.manifest.object_path.display());
                println!("  binary: {}", outcome.manifest.binary_path.display());
                if let Some(d) = &outcome.manifest.assembler_digest {
                    println!("  assembler_digest: {d}");
                }
                if let Some(d) = &outcome.manifest.linker_digest {
                    println!("  linker_digest: {d}");
                }
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
            "--contract",
            "contract.sem.toml",
        ])
        .expect("parse");
        assert!(matches!(cli.command, Some(Commands::Verify { .. })));
    }

    #[test]
    fn clap_parses_run() {
        let cli = Cli::try_parse_from([
            "vaa",
            "run",
            "task.vaa.toml",
            "--contract",
            "c.sem.toml",
            "--wrong",
            "w.asm",
            "--repaired",
            "r.asm",
        ])
        .expect("parse");
        assert!(matches!(cli.command, Some(Commands::Run { .. })));
    }

    #[test]
    fn clap_parses_run_resume() {
        let cli = Cli::try_parse_from([
            "vaa",
            "run",
            "task.vaa.toml",
            "--contract",
            "c.sem.toml",
            "--wrong",
            "w.asm",
            "--repaired",
            "r.asm",
            "--resume",
            "target/vaa-runs/existing",
        ])
        .expect("parse");
        match cli.command {
            Some(Commands::Run {
                resume: Some(path), ..
            }) => assert!(path.ends_with("existing")),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn clap_parses_ingest() {
        let cli = Cli::try_parse_from([
            "vaa",
            "ingest",
            "task.vaa.toml",
            "--contract",
            "c.sem.toml",
            "--source",
            "cand.asm",
            "--generator",
            "cryptopt-like",
        ])
        .expect("parse");
        assert!(matches!(cli.command, Some(Commands::Ingest { .. })));
    }

    #[test]
    fn clap_parses_evidence_check_seal() {
        let cli = Cli::try_parse_from([
            "vaa",
            "evidence",
            "check-seal",
            "evidence.json",
            "evidence.seal.json",
        ])
        .expect("parse");
        assert!(matches!(
            cli.command,
            Some(Commands::Evidence {
                command: EvidenceCommands::CheckSeal { .. }
            })
        ));
    }

    #[test]
    fn clap_parses_evidence_verify_bundle() {
        let cli = Cli::try_parse_from(["vaa", "evidence", "verify-bundle", "candidates/0000"])
            .expect("parse");
        assert!(matches!(
            cli.command,
            Some(Commands::Evidence {
                command: EvidenceCommands::VerifyBundle { .. }
            })
        ));
    }

    #[test]
    fn clap_parses_evidence_verify_chain() {
        let cli = Cli::try_parse_from(["vaa", "evidence", "verify-chain", "target/vaa-runs/run-1"])
            .expect("parse");
        assert!(matches!(
            cli.command,
            Some(Commands::Evidence {
                command: EvidenceCommands::VerifyChain { .. }
            })
        ));
    }

    #[test]
    fn clap_parses_evidence_export_transparency() {
        let cli = Cli::try_parse_from([
            "vaa",
            "evidence",
            "export-transparency",
            "target/vaa-runs/run-1",
            "-o",
            "transparency.json",
        ])
        .expect("parse");
        assert!(matches!(
            cli.command,
            Some(Commands::Evidence {
                command: EvidenceCommands::ExportTransparency { .. }
            })
        ));
    }

    #[test]
    fn clap_parses_evidence_verify_transparency() {
        let cli = Cli::try_parse_from([
            "vaa",
            "evidence",
            "verify-transparency",
            "transparency.json",
            "--against",
            "target/vaa-runs/run-1",
        ])
        .expect("parse");
        assert!(matches!(
            cli.command,
            Some(Commands::Evidence {
                command: EvidenceCommands::VerifyTransparency { .. }
            })
        ));
    }

    #[test]
    fn clap_parses_evidence_keygen_seal() {
        let cli = Cli::try_parse_from(["vaa", "evidence", "keygen-seal", "--out", "seal.seed"])
            .expect("parse");
        assert!(matches!(
            cli.command,
            Some(Commands::Evidence {
                command: EvidenceCommands::KeygenSeal { .. }
            })
        ));
    }

    #[test]
    fn clap_parses_generate() {
        let cli = Cli::try_parse_from(["vaa", "generate", "task.vaa.toml", "--output", "out.asm"])
            .expect("parse");
        assert!(matches!(cli.command, Some(Commands::Generate { .. })));
    }

    #[test]
    fn clap_parses_generate_run_dir() {
        let cli = Cli::try_parse_from([
            "vaa",
            "generate",
            "task.vaa.toml",
            "--run-dir",
            "target/vaa-runs",
        ])
        .expect("parse");
        assert!(matches!(
            cli.command,
            Some(Commands::Generate {
                run_dir: Some(_),
                output: None,
                ..
            })
        ));
    }

    #[test]
    fn clap_parses_generate_external_command() {
        let cli = Cli::try_parse_from([
            "vaa",
            "generate",
            "task.vaa.toml",
            "--run-dir",
            "target/vaa-runs",
            "--command",
            "python",
            "--",
            "gen.py",
        ])
        .expect("parse");
        match cli.command {
            Some(Commands::Generate {
                command: Some(cmd),
                command_args,
                run_dir: Some(_),
                ..
            }) => {
                assert!(cmd.ends_with("python"));
                assert_eq!(command_args, vec!["gen.py".to_owned()]);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn clap_parses_generate_live() {
        let cli = Cli::try_parse_from([
            "vaa",
            "generate",
            "task.vaa.toml",
            "--output",
            "out.asm",
            "--live",
        ])
        .expect("parse");
        assert!(matches!(
            cli.command,
            Some(Commands::Generate { live: true, .. })
        ));
    }

    #[test]
    fn clap_parses_build() {
        let cli = Cli::try_parse_from(["vaa", "build", "source.asm", "--output-dir", "out"])
            .expect("parse");
        assert!(matches!(cli.command, Some(Commands::Build { .. })));
    }

    #[test]
    fn clap_parses_cache_status() {
        let cli = Cli::try_parse_from(["vaa", "cache", "status"]).expect("parse");
        assert!(matches!(
            cli.command,
            Some(Commands::Cache {
                command: CacheCommands::Status
            })
        ));
    }

    #[test]
    fn clap_parses_verify_cache_flag() {
        let cli = Cli::try_parse_from([
            "vaa",
            "verify",
            "task.vaa.toml",
            "--source",
            "a.asm",
            "--contract",
            "c.sem.toml",
            "--cache",
        ])
        .expect("parse");
        assert!(matches!(
            cli.command,
            Some(Commands::Verify { cache: true, .. })
        ));
    }

    #[test]
    fn clap_parses_inspect() {
        let cli = Cli::try_parse_from(["vaa", "inspect", "artifact.o"]).expect("parse");
        assert!(matches!(cli.command, Some(Commands::Inspect { .. })));
    }
}
