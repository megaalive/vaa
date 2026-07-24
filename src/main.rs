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
        /// Run SemASM via ExecutionSandbox (LocalBackend scaffold). Sets
        /// `execution_isolation=sandbox`. Fail-closed if sandbox cannot run.
        #[arg(long, default_value_t = false)]
        execution_sandbox: bool,
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
        /// Wrap `--command` in a container OS jail (staging bind at `/work` only).
        #[arg(long, default_value_t = false)]
        generator_jail: bool,
    },
    /// CryptOpt-like search loop via ingest (fixture mutator; no embed CryptOpt).
    Search {
        /// Locked task file.
        task: PathBuf,
        /// Seed assembly source.
        seed: PathBuf,
        /// Run base directory.
        #[arg(long)]
        run_dir: PathBuf,
        /// Max candidates to try (also capped by task `max_candidates`).
        #[arg(long, default_value_t = 8)]
        budget: u32,
        /// Mutator: `nop-slide`, `nop-before-ret`, or external `--mutator-command`.
        #[arg(long, default_value = "nop-slide")]
        mutator: String,
        /// Optional external mutator program (reads seed on stdin, writes asm on stdout).
        #[arg(long)]
        mutator_command: Option<PathBuf>,
        /// Ingest each staged candidate through SemASM verify/seal (Tranche T).
        #[arg(long, default_value_t = false)]
        ingest: bool,
        /// SemASM contract (required with `--ingest`).
        #[arg(long)]
        contract: Option<PathBuf>,
        /// Opt-in SemASM `--allow-execution` (Gate-2). Default CI stays Gate-1 Incomplete.
        #[arg(long, default_value_t = false)]
        allow_execution: bool,
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
        /// Docker/Podman `--memory` bytes when `--sandbox container`.
        #[arg(long)]
        memory_limit_bytes: Option<u64>,
        /// Disable C1 host bind mounts (`/input`+`/work`) for container builds.
        #[arg(long, default_value_t = false)]
        no_container_binds: bool,
        /// Apply bundled seccomp profile (written under output dir). Linux-oriented.
        #[arg(long, default_value_t = false)]
        seccomp: bool,
        /// Fail closed unless runtime probe looks rootless.
        #[arg(long, default_value_t = false)]
        require_rootless: bool,
        /// Opt-in local content-addressed build cache (PR-020).
        #[arg(long, default_value_t = false)]
        cache: bool,
        /// Twin-build same-host reproducibility check (PR-021).
        #[arg(long, default_value_t = false)]
        check_reproducible: bool,
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
    /// External generator bridge (stack lock + generator spec).
    Generator {
        #[command(subcommand)]
        command: GeneratorCommands,
    },
    /// Build → generate → verify one case (no model). Alias: `compiler-run`.
    #[command(name = "generator-run", alias = "compiler-run")]
    GeneratorRun {
        /// Path to `ExternalGeneratorSpec` TOML.
        #[arg(long)]
        spec: PathBuf,
        /// Optional stack lock path.
        #[arg(long)]
        lock: Option<PathBuf>,
        /// Locked task file.
        #[arg(long)]
        task: PathBuf,
        /// SemASM contract.
        #[arg(long)]
        contract: PathBuf,
        /// Generator primary input (`{input}`).
        #[arg(long)]
        input: PathBuf,
        /// Generated assembly output (`{output}`).
        #[arg(long)]
        output: PathBuf,
        /// Run base directory for evidence (required unless `--skip-verify`).
        #[arg(long, default_value = ".")]
        run_dir: PathBuf,
        /// Override generator repository path.
        #[arg(long)]
        repo: Option<PathBuf>,
        /// Skip revision/worktree guard.
        #[arg(long, default_value_t = false)]
        skip_repo_guard: bool,
        /// Skip build; hash existing binary only.
        #[arg(long, default_value_t = false)]
        skip_build: bool,
        /// Generate only (no SemASM ingest/verify).
        #[arg(long, default_value_t = false)]
        skip_verify: bool,
        /// Twin-run generation digest check.
        #[arg(long, default_value_t = false)]
        check_deterministic: bool,
        /// Forward `--allow-execution` to SemASM.
        #[arg(long, default_value_t = false)]
        allow_execution: bool,
        /// Override `{target}` (default: task target).
        #[arg(long)]
        target: Option<String>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Regression suite commands (`validate` / `run`).
    Suite {
        #[command(subcommand)]
        command: SuiteCommands,
    },
    /// Patch evidence commands.
    Patch {
        #[command(subcommand)]
        command: PatchCommands,
    },
    /// Repair packet commands (agent-facing generator repair briefs).
    Repair {
        #[command(subcommand)]
        command: RepairCommands,
    },
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)] // transient CLI parse type; Export carries many fields
enum RepairCommands {
    /// Build a repair packet (JSON + optional Markdown) from a failing status.
    Export {
        /// Path to the generator spec TOML.
        #[arg(long)]
        spec: PathBuf,
        /// Task/case id the packet is for.
        #[arg(long)]
        task_id: String,
        /// Failure status string (e.g. `Violated`, `BehaviorFailed`).
        #[arg(long)]
        status: String,
        /// Human-readable failure message.
        #[arg(long)]
        message: String,
        /// Stable diagnostic code (e.g. `ABI_CALLEE_SAVED_001`).
        #[arg(long)]
        diagnostic_code: Option<String>,
        /// Offending instruction offset (e.g. `0x17`).
        #[arg(long)]
        instruction_offset: Option<String>,
        /// Path of the generated artifact (relative to run dir).
        #[arg(long, default_value = "candidate.asm")]
        artifact: String,
        /// Digest of the generated artifact (`sha256:…`).
        #[arg(long)]
        artifact_digest: String,
        /// Optional generator input reference (e.g. `input.hlx:4`).
        #[arg(long)]
        map_input: Option<String>,
        /// Optional IR node reference (e.g. `CompareSigned#12`).
        #[arg(long)]
        map_ir: Option<String>,
        /// Optional generator source reference (e.g. `src/backend/x64/emitter.rs:214`).
        #[arg(long)]
        map_source: Option<String>,
        /// Optional `candidate.map.json`; joined by `--instruction-offset`
        /// to auto-fill source mapping (fallback: assembly context only).
        #[arg(long)]
        map: Option<PathBuf>,
        /// Regenerate command shown to the agent.
        #[arg(long)]
        regenerate_command: String,
        /// Verify command shown to the agent.
        #[arg(long)]
        verify_command: String,
        /// Output packet JSON path.
        #[arg(long)]
        output: PathBuf,
        /// Also write a Markdown sibling next to the JSON.
        #[arg(long, default_value_t = true)]
        markdown: bool,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Load and structurally verify a repair packet JSON.
    Verify {
        /// Path to repair packet JSON.
        file: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Render agent/editor repair rules Markdown from a generator spec.
    Rules {
        /// Path to the generator spec TOML.
        #[arg(long)]
        spec: PathBuf,
        /// One-case regeneration command template.
        #[arg(long)]
        regenerate_command: String,
        /// Per-case verification command template.
        #[arg(long)]
        verify_command: String,
        /// Full regression suite command.
        #[arg(long)]
        suite_command: String,
        /// Override the build command (default: spec build argv).
        #[arg(long)]
        build_command: Option<String>,
        /// Write Markdown here instead of stdout.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Output format (`terminal` prints the Markdown itself).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
}

#[derive(Debug, Subcommand)]
enum PatchCommands {
    /// Verify a patch evidence JSON file (structure + honesty invariants).
    #[command(name = "evidence-verify")]
    EvidenceVerify {
        /// Path to patch evidence JSON.
        file: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Build patch evidence JSON from suite evidence + changed paths.
    #[command(name = "evidence-build")]
    EvidenceBuild {
        /// Path to suite evidence JSON (from `vaa suite run --output`).
        #[arg(long)]
        suite_evidence: PathBuf,
        /// Base revision (`git:<commit>`).
        #[arg(long)]
        base: String,
        /// Patched revision (`git:<commit>`).
        #[arg(long)]
        patched: String,
        /// Generator binary digest (`sha256:…`).
        #[arg(long)]
        generator_binary_digest: String,
        /// Changed file path (repeatable).
        #[arg(long = "changed")]
        changed: Vec<String>,
        /// Optional path to generator spec (for forbidden/allowed paths).
        #[arg(long)]
        spec: Option<PathBuf>,
        /// Output patch evidence JSON path.
        #[arg(long)]
        output: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
}

#[derive(Debug, Subcommand)]
enum SuiteCommands {
    /// Parse and validate a suite manifest.
    Validate {
        /// Path to `*.vaa-suite.toml`.
        suite: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
        /// Include suite manifest digest.
        #[arg(long, default_value_t = true)]
        show_digest: bool,
    },
    /// Run all required cases and emit suite evidence JSON summary.
    Run {
        /// Path to `*.vaa-suite.toml`.
        suite: PathBuf,
        /// Override generator repository path.
        #[arg(long)]
        repo: Option<PathBuf>,
        /// Run base directory.
        #[arg(long, default_value = ".")]
        run_dir: PathBuf,
        /// Skip revision/worktree guard.
        #[arg(long, default_value_t = false)]
        skip_repo_guard: bool,
        /// Skip build; hash existing binary only.
        #[arg(long, default_value_t = false)]
        skip_build: bool,
        /// Generate only (no SemASM ingest/verify) — suite status will be incomplete.
        #[arg(long, default_value_t = false)]
        skip_verify: bool,
        /// Twin-run generation digest check.
        #[arg(long, default_value_t = false)]
        check_deterministic: bool,
        /// Forward `--allow-execution` to SemASM.
        #[arg(long, default_value_t = false)]
        allow_execution: bool,
        /// Optional path to write suite evidence JSON.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
}

#[derive(Debug, Subcommand)]
enum GeneratorCommands {
    /// Parse and validate a `stack.lock.toml`.
    ValidateLock {
        /// Path to the stack lock file.
        lock: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
        /// Include the lock content digest in the output.
        #[arg(long, default_value_t = true)]
        show_digest: bool,
    },
    /// Parse and validate an `ExternalGeneratorSpec` TOML.
    ValidateSpec {
        /// Path to the generator spec file.
        spec: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Check generator repository revision + clean worktree (+ path policy).
    CheckRepo {
        /// Path to the generator spec file.
        spec: PathBuf,
        /// Override repository path (otherwise resolve from spec).
        #[arg(long)]
        repo: Option<PathBuf>,
        /// Skip allow/deny path policy on dirty entries.
        #[arg(long, default_value_t = false)]
        no_path_policy: bool,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Hash a generator binary (and optionally build first).
    Identity {
        /// Path to the generator spec file.
        spec: PathBuf,
        /// Override repository path.
        #[arg(long)]
        repo: Option<PathBuf>,
        /// Skip `build.command` and only hash an existing binary.
        #[arg(long, default_value_t = false)]
        skip_build: bool,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Run locked deterministic generation (no SemASM verify).
    Generate {
        /// Path to the generator spec file.
        spec: PathBuf,
        /// Generator binary path (`{generator}`).
        #[arg(long)]
        generator: PathBuf,
        /// Primary input path (`{input}`).
        #[arg(long)]
        input: PathBuf,
        /// Output assembly path (`{output}`).
        #[arg(long)]
        output: PathBuf,
        /// Target label (`{target}`).
        #[arg(long, default_value = "x86_64-pc-windows-msvc")]
        target: String,
        /// Twin-run and require identical digests.
        #[arg(long, default_value_t = false)]
        check_deterministic: bool,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Build → generate → verify one case (same as `vaa generator-run`).
    Run {
        /// Path to `ExternalGeneratorSpec` TOML.
        #[arg(long)]
        spec: PathBuf,
        /// Optional stack lock path.
        #[arg(long)]
        lock: Option<PathBuf>,
        /// Locked task file.
        #[arg(long)]
        task: PathBuf,
        /// SemASM contract.
        #[arg(long)]
        contract: PathBuf,
        /// Generator primary input (`{input}`).
        #[arg(long)]
        input: PathBuf,
        /// Generated assembly output (`{output}`).
        #[arg(long)]
        output: PathBuf,
        /// Run base directory for evidence.
        #[arg(long, default_value = ".")]
        run_dir: PathBuf,
        /// Override generator repository path.
        #[arg(long)]
        repo: Option<PathBuf>,
        /// Skip revision/worktree guard.
        #[arg(long, default_value_t = false)]
        skip_repo_guard: bool,
        /// Skip build; hash existing binary only.
        #[arg(long, default_value_t = false)]
        skip_build: bool,
        /// Generate only (no SemASM ingest/verify).
        #[arg(long, default_value_t = false)]
        skip_verify: bool,
        /// Twin-run generation digest check.
        #[arg(long, default_value_t = false)]
        check_deterministic: bool,
        /// Forward `--allow-execution` to SemASM.
        #[arg(long, default_value_t = false)]
        allow_execution: bool,
        /// Override `{target}` (default: task target).
        #[arg(long)]
        target: Option<String>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Check changed paths against generator patch_policy allow/deny lists.
    CheckPaths {
        /// Path to the generator spec file.
        spec: PathBuf,
        /// Changed file path (repeatable).
        #[arg(long = "changed", required = true)]
        changed: Vec<String>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Triage a status string for generator-vs-verifier repair routing.
    Triage {
        /// Status string (e.g. `Verified`, `Incomplete`, `BehaviorFailed`).
        status: String,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Join an instruction offset or assembly line against `candidate.map.json`.
    MapJoin {
        /// Path to the source map JSON.
        map: PathBuf,
        /// Instruction offset to join (hex `0x…` or decimal).
        #[arg(long, conflicts_with = "line")]
        offset: Option<String>,
        /// 1-based assembly line to join.
        #[arg(long)]
        line: Option<u64>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// List or look up stable diagnostic codes (plan §12 registry).
    Diagnostics {
        /// Optional exact code to look up (e.g. `ABI_CALLEE_SAVED_001`).
        code: Option<String>,
        /// Filter by category prefix (e.g. `ABI`, `GEN`, `POLICY`).
        #[arg(long)]
        category: Option<String>,
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
    /// Publish transparency JSON to a Rekor-compatible log (opt-in network).
    PublishRekor {
        /// Path to `vaa-transparency-v1` JSON.
        file: PathBuf,
        /// Rekor base URL (also `VAA_REKOR_URL`).
        #[arg(
            long,
            env = "VAA_REKOR_URL",
            default_value = "https://rekor.sigstore.dev"
        )]
        rekor_url: String,
        /// Dry-run: build the entry payload without HTTP.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Verify a Rekor entry UUID against a local transparency file (opt-in network).
    VerifyRekor {
        /// Path to transparency JSON.
        file: PathBuf,
        /// Rekor entry UUID.
        #[arg(long)]
        uuid: String,
        /// Rekor base URL (also `VAA_REKOR_URL`).
        #[arg(
            long,
            env = "VAA_REKOR_URL",
            default_value = "https://rekor.sigstore.dev"
        )]
        rekor_url: String,
    },
    /// Fulcio keyless DSSE over transparency JSON (opt-in; not Gate).
    FulcioSign {
        /// Path to `vaa-transparency-v1` JSON.
        file: PathBuf,
        /// Fulcio base URL (also `VAA_FULCIO_URL`).
        #[arg(
            long,
            env = "VAA_FULCIO_URL",
            default_value = "https://fulcio.sigstore.dev"
        )]
        fulcio_url: String,
        /// OIDC identity token (also `VAA_OIDC_TOKEN`). Required unless `--dry-run`.
        #[arg(long, env = "VAA_OIDC_TOKEN")]
        oidc_token: Option<String>,
        /// Offline mock Fulcio (no network). Gate stays offline.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        /// Output path for DSSE + cert chain JSON (default: `<file>.fulcio.json`).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Probe seal durability class for a path (local-durable / best-effort / refuse).
    DurabilityProbe {
        /// Directory to probe (default: cwd).
        #[arg(long)]
        path: Option<PathBuf>,
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
    // Clap's derived parser for this CLI is large; Windows default stack (~1 MiB)
    // overflows during `Cli::parse`. Run the CLI body on a larger stack.
    const STACK: usize = 8 * 1024 * 1024;
    match std::thread::Builder::new()
        .name("vaa-main".into())
        .stack_size(STACK)
        .spawn(run_cli)
    {
        Ok(handle) => handle.join().unwrap_or_else(|_| {
            eprintln!("error: vaa main thread panicked");
            VaaExitCode::ToolFailure.as_std()
        }),
        Err(e) => {
            eprintln!("error: failed to start vaa main thread: {e}");
            VaaExitCode::ToolFailure.as_std()
        }
    }
}

fn run_cli() -> ExitCode {
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
            execution_sandbox,
            cache,
            format,
        } => verify_command(
            &task,
            &source,
            &contract,
            allow_execution,
            execution_sandbox,
            cache,
            format,
        ),
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
            EvidenceCommands::PublishRekor {
                file,
                rekor_url,
                dry_run,
            } => publish_rekor_command(&file, &rekor_url, dry_run),
            EvidenceCommands::VerifyRekor {
                file,
                uuid,
                rekor_url,
            } => verify_rekor_command(&file, &uuid, &rekor_url),
            EvidenceCommands::FulcioSign {
                file,
                fulcio_url,
                oidc_token,
                dry_run,
                output,
            } => fulcio_sign_command(
                &file,
                &fulcio_url,
                oidc_token.as_deref(),
                dry_run,
                output.as_deref(),
            ),
            EvidenceCommands::DurabilityProbe { path } => durability_probe_command(path.as_deref()),
        },
        Commands::Generate {
            task,
            output,
            run_dir,
            command,
            command_args,
            live,
            generator_jail,
        } => generate_command(
            &task,
            output.as_deref(),
            run_dir.as_deref(),
            command.as_deref(),
            &command_args,
            live,
            generator_jail,
        ),
        Commands::Search {
            task,
            seed,
            run_dir,
            budget,
            mutator,
            mutator_command,
            ingest,
            contract,
            allow_execution,
        } => search_command(
            &task,
            &seed,
            &run_dir,
            budget,
            &mutator,
            mutator_command.as_deref(),
            ingest,
            contract.as_deref(),
            allow_execution,
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
            memory_limit_bytes,
            no_container_binds,
            seccomp,
            require_rootless,
            cache,
            check_reproducible,
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
            memory_limit_bytes,
            !no_container_binds,
            seccomp,
            require_rootless,
            cache,
            check_reproducible,
            format,
        ),
        Commands::Cache { command } => match command {
            CacheCommands::Status => cache_status_command(),
        },
        Commands::Inspect { artifact, format } => inspect_command(&artifact, format),
        Commands::Generator { command } => match command {
            GeneratorCommands::ValidateLock {
                lock,
                format,
                show_digest,
            } => generator_validate_lock_command(&lock, format, show_digest),
            GeneratorCommands::ValidateSpec { spec, format } => {
                generator_validate_spec_command(&spec, format)
            }
            GeneratorCommands::CheckRepo {
                spec,
                repo,
                no_path_policy,
                format,
            } => generator_check_repo_command(&spec, repo.as_deref(), no_path_policy, format),
            GeneratorCommands::Identity {
                spec,
                repo,
                skip_build,
                format,
            } => generator_identity_command(&spec, repo.as_deref(), skip_build, format),
            GeneratorCommands::Generate {
                spec,
                generator,
                input,
                output,
                target,
                check_deterministic,
                format,
            } => generator_generate_command(
                &spec,
                &generator,
                &input,
                &output,
                &target,
                check_deterministic,
                format,
            ),
            GeneratorCommands::Run {
                spec,
                lock,
                task,
                contract,
                input,
                output,
                run_dir,
                repo,
                skip_repo_guard,
                skip_build,
                skip_verify,
                check_deterministic,
                allow_execution,
                target,
                format,
            } => {
                use vaa::generator::GeneratorRunConfig;
                generator_run_command(
                    &GeneratorRunConfig {
                        spec_path: spec,
                        lock_path: lock,
                        task_path: task,
                        contract_path: contract,
                        input_path: input,
                        output_path: output,
                        run_base: run_dir,
                        repo_override: repo,
                        skip_repo_guard,
                        skip_build,
                        skip_verify,
                        allow_execution,
                        check_deterministic,
                        target_override: target,
                    },
                    format,
                )
            }
            GeneratorCommands::CheckPaths {
                spec,
                changed,
                format,
            } => generator_check_paths_command(&spec, &changed, format),
            GeneratorCommands::Triage { status, format } => {
                generator_triage_command(&status, format)
            }
            GeneratorCommands::MapJoin {
                map,
                offset,
                line,
                format,
            } => generator_map_join_command(&map, offset.as_deref(), line, format),
            GeneratorCommands::Diagnostics {
                code,
                category,
                format,
            } => generator_diagnostics_command(code.as_deref(), category.as_deref(), format),
        },
        Commands::GeneratorRun {
            spec,
            lock,
            task,
            contract,
            input,
            output,
            run_dir,
            repo,
            skip_repo_guard,
            skip_build,
            skip_verify,
            check_deterministic,
            allow_execution,
            target,
            format,
        } => {
            use vaa::generator::GeneratorRunConfig;
            generator_run_command(
                &GeneratorRunConfig {
                    spec_path: spec,
                    lock_path: lock,
                    task_path: task,
                    contract_path: contract,
                    input_path: input,
                    output_path: output,
                    run_base: run_dir,
                    repo_override: repo,
                    skip_repo_guard,
                    skip_build,
                    skip_verify,
                    allow_execution,
                    check_deterministic,
                    target_override: target,
                },
                format,
            )
        }
        Commands::Suite { command } => match command {
            SuiteCommands::Validate {
                suite,
                format,
                show_digest,
            } => suite_validate_command(&suite, format, show_digest),
            SuiteCommands::Run {
                suite,
                repo,
                run_dir,
                skip_repo_guard,
                skip_build,
                skip_verify,
                check_deterministic,
                allow_execution,
                output,
                format,
            } => {
                use vaa::generator::SuiteRunConfig;
                suite_run_command(
                    &SuiteRunConfig {
                        suite_path: suite,
                        repo_override: repo,
                        run_base: run_dir,
                        skip_repo_guard,
                        skip_build,
                        skip_verify,
                        allow_execution,
                        check_deterministic,
                    },
                    output.as_deref(),
                    format,
                )
            }
        },
        Commands::Patch { command } => match command {
            PatchCommands::EvidenceVerify { file, format } => {
                patch_evidence_verify_command(&file, format)
            }
            PatchCommands::EvidenceBuild {
                suite_evidence,
                base,
                patched,
                generator_binary_digest,
                changed,
                spec,
                output,
                format,
            } => patch_evidence_build_command(
                &suite_evidence,
                &base,
                &patched,
                &generator_binary_digest,
                &changed,
                spec.as_deref(),
                &output,
                format,
            ),
        },
        Commands::Repair { command } => match command {
            RepairCommands::Export {
                spec,
                task_id,
                status,
                message,
                diagnostic_code,
                instruction_offset,
                artifact,
                artifact_digest,
                map_input,
                map_ir,
                map_source,
                map,
                regenerate_command,
                verify_command,
                output,
                markdown,
                format,
            } => {
                let config = RepairExportConfig {
                    spec,
                    task_id,
                    status,
                    message,
                    diagnostic_code,
                    instruction_offset,
                    artifact,
                    artifact_digest,
                    map_input,
                    map_ir,
                    map_source,
                    map,
                    regenerate_command,
                    verify_command,
                    output,
                    markdown,
                };
                repair_export_command(&config, format)
            }
            RepairCommands::Verify { file, format } => repair_verify_command(&file, format),
            RepairCommands::Rules {
                spec,
                regenerate_command,
                verify_command,
                suite_command,
                build_command,
                output,
                format,
            } => repair_rules_command(
                &spec,
                &regenerate_command,
                &verify_command,
                &suite_command,
                build_command.as_deref(),
                output.as_deref(),
                format,
            ),
        },
    }
}

fn print_status() {
    let container = vaa::probe_container_runtime();
    println!("vaa {VAA_VERSION}");
    println!("maturity: {MATURITY}");
    println!("form: local CLI (single binary crate + library modules)");
    println!("task schema: {TASK_SCHEMA_VERSION}");
    println!("commands: version, status, validate, doctor, capabilities, verify, run, ingest, evidence, generate, build, cache, inspect, generator, generator-run, suite, patch, repair");
    println!("default mode: verify-only (run=fixture; ingest=external; live LLM opt-in)");
    println!(
        "model adapter: fixture default; --live needs --features live-model + VAA_MODEL_API_KEY"
    );
    println!(
        "cache: local `.vaa/cache` opt-in via --cache / VAA_CACHE_DIR (PR-020; not remote log)"
    );
    println!("generator bridge: P0+P1 complete (suite/patch/paths/triage); P2 repair packet next");
    println!("SemASM integration: doctor + verify via ProcessRunner (stdout-only report 0.4)");
    println!("evidence: integrity seals (check-seal=JSON drift; verify-bundle=artifact rehash)");
    println!("evidence note: opt-in Ed25519 when VAA_SEAL_SIGNING_KEY is set (practice; not a trust root)");
    println!("trust ops (G5): signer_kind labels practice-ed25519|sigstore-dsse|hsm-pkcs11; production trust root locked");
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

fn generator_validate_lock_command(
    path: &Path,
    format: OutputFormat,
    show_digest: bool,
) -> ExitCode {
    use vaa::generator::{load_stack_lock, stack_lock_digest, GeneratorError};

    match load_stack_lock(path) {
        Ok(lock) => {
            let digest = stack_lock_digest(&lock);
            let generator_ids: Vec<&String> = lock.generators.keys().collect();
            match format {
                OutputFormat::Terminal => {
                    println!("ok: stack lock `{}` is valid", path.display());
                    println!("  schema_version: {}", lock.schema_version);
                    println!("  vaa.revision: {}", lock.vaa.revision);
                    println!("  semasm.revision: {}", lock.semasm.revision);
                    println!("  generators: {}", generator_ids.len());
                    for id in &generator_ids {
                        if let Some(pin) = lock.generators.get(*id) {
                            println!("    {id}: {}", pin.revision);
                        }
                    }
                    if show_digest {
                        println!("  digest: {}", digest.prefixed());
                    }
                }
                OutputFormat::Json => {
                    let body = serde_json::json!({
                        "ok": true,
                        "path": path,
                        "schema_version": lock.schema_version,
                        "vaa_revision": lock.vaa.revision,
                        "semasm_revision": lock.semasm.revision,
                        "generators": generator_ids,
                        "digest": if show_digest {
                            Some(digest.prefixed())
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
            emit_generator_error(path, format, &error);
            match error {
                GeneratorError::Io { .. }
                | GeneratorError::Parse { .. }
                | GeneratorError::Validation(_)
                | GeneratorError::ValidationMany { .. } => VaaExitCode::InvalidInput.as_std(),
            }
        }
    }
}

fn generator_validate_spec_command(path: &Path, format: OutputFormat) -> ExitCode {
    use vaa::generator::{load_generator_spec, GeneratorError};

    match load_generator_spec(path) {
        Ok(spec) => {
            match format {
                OutputFormat::Terminal => {
                    println!("ok: generator spec `{}` is valid", path.display());
                    println!("  schema_version: {}", spec.schema_version);
                    println!("  generator_id: {}", spec.generator_id);
                    if let Some(kind) = &spec.kind {
                        println!("  kind: {kind}");
                    }
                    println!("  repository.path: {}", spec.repository.path);
                    println!(
                        "  repository.expected_revision: {}",
                        spec.repository.expected_revision
                    );
                    println!("  build.command: {:?}", spec.build.command);
                    println!("  generation.command: {:?}", spec.generation.command);
                }
                OutputFormat::Json => {
                    let body = serde_json::json!({
                        "ok": true,
                        "path": path,
                        "schema_version": spec.schema_version,
                        "generator_id": spec.generator_id,
                        "kind": spec.kind,
                        "repository_path": spec.repository.path,
                        "expected_revision": spec.repository.expected_revision,
                        "build_command": spec.build.command,
                        "generation_command": spec.generation.command,
                    });
                    println!("{body}");
                }
            }
            VaaExitCode::Success.as_std()
        }
        Err(error) => {
            emit_generator_error(path, format, &error);
            match error {
                GeneratorError::Io { .. }
                | GeneratorError::Parse { .. }
                | GeneratorError::Validation(_)
                | GeneratorError::ValidationMany { .. } => VaaExitCode::InvalidInput.as_std(),
            }
        }
    }
}

fn emit_generator_error(path: &Path, format: OutputFormat, error: &vaa::generator::GeneratorError) {
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

fn generator_check_repo_command(
    spec_path: &Path,
    repo_override: Option<&Path>,
    no_path_policy: bool,
    format: OutputFormat,
) -> ExitCode {
    use vaa::generator::{check_repository, load_generator_spec, GeneratorError, RepoGuardConfig};

    let spec = match load_generator_spec(spec_path) {
        Ok(s) => s,
        Err(error) => {
            emit_generator_error(spec_path, format, &error);
            return VaaExitCode::InvalidInput.as_std();
        }
    };

    let mut config = match RepoGuardConfig::from_spec(&spec, spec_path) {
        Ok(c) => c,
        Err(error) => {
            emit_generator_error(spec_path, format, &error);
            return VaaExitCode::InvalidInput.as_std();
        }
    };
    if let Some(repo) = repo_override {
        config.repository_path = match std::fs::canonicalize(repo) {
            Ok(p) => p,
            Err(source) => {
                let error = GeneratorError::Io {
                    path: repo.to_path_buf(),
                    source,
                };
                emit_generator_error(repo, format, &error);
                return VaaExitCode::InvalidInput.as_std();
            }
        };
    }
    config.check_path_policy = !no_path_policy;

    match check_repository(&config) {
        Ok(report) => {
            match format {
                OutputFormat::Terminal => {
                    println!("ok: repository guard passed");
                    println!("  path: {}", report.repository_path.display());
                    println!("  head: git:{}", report.head_revision);
                    println!("  expected: {}", report.expected_revision);
                    println!("  worktree_clean: {}", report.worktree_clean);
                }
                OutputFormat::Json => {
                    let body = serde_json::json!({
                        "ok": true,
                        "report": report,
                    });
                    println!("{body}");
                }
            }
            VaaExitCode::Success.as_std()
        }
        Err(error) => {
            emit_generator_error(spec_path, format, &error);
            VaaExitCode::InvalidInput.as_std()
        }
    }
}

fn generator_identity_command(
    spec_path: &Path,
    repo_override: Option<&Path>,
    skip_build: bool,
    format: OutputFormat,
) -> ExitCode {
    use vaa::generator::{
        build_and_identify, load_generator_spec, resolve_repository_path, GeneratorError,
    };

    let spec = match load_generator_spec(spec_path) {
        Ok(s) => s,
        Err(error) => {
            emit_generator_error(spec_path, format, &error);
            return VaaExitCode::InvalidInput.as_std();
        }
    };

    let repo = if let Some(repo) = repo_override {
        match std::fs::canonicalize(repo) {
            Ok(p) => p,
            Err(source) => {
                emit_generator_error(
                    repo,
                    format,
                    &GeneratorError::Io {
                        path: repo.to_path_buf(),
                        source,
                    },
                );
                return VaaExitCode::InvalidInput.as_std();
            }
        }
    } else {
        match resolve_repository_path(spec_path, &spec.repository.path) {
            Ok(p) => p,
            Err(error) => {
                emit_generator_error(spec_path, format, &error);
                return VaaExitCode::InvalidInput.as_std();
            }
        }
    };

    match build_and_identify(&spec, &repo, skip_build) {
        Ok(identity) => {
            match format {
                OutputFormat::Terminal => {
                    println!("ok: generator binary identity established");
                    println!("  path: {}", identity.binary_path.display());
                    println!("  digest: {}", identity.digest);
                    println!("  size_bytes: {}", identity.size_bytes);
                }
                OutputFormat::Json => {
                    let body = serde_json::json!({
                        "ok": true,
                        "identity": identity,
                    });
                    println!("{body}");
                }
            }
            VaaExitCode::Success.as_std()
        }
        Err(error) => {
            emit_generator_error(spec_path, format, &error);
            VaaExitCode::InvalidInput.as_std()
        }
    }
}

fn generator_generate_command(
    spec_path: &Path,
    generator: &Path,
    input: &Path,
    output: &Path,
    target: &str,
    check_deterministic: bool,
    format: OutputFormat,
) -> ExitCode {
    use vaa::generator::{generate_candidate, load_generator_spec, GenerationRequest};

    let spec = match load_generator_spec(spec_path) {
        Ok(s) => s,
        Err(error) => {
            emit_generator_error(spec_path, format, &error);
            return VaaExitCode::InvalidInput.as_std();
        }
    };

    let request = GenerationRequest {
        generator_binary: generator.to_path_buf(),
        input: input.to_path_buf(),
        target: target.to_owned(),
        output: output.to_path_buf(),
        working_directory: None,
        clean_output: spec.generation.clean_output_directory,
        check_deterministic,
    };

    match generate_candidate(&spec, &request) {
        Ok(outcome) => {
            match format {
                OutputFormat::Terminal => {
                    println!("ok: candidate generated");
                    println!("  output: {}", outcome.output_path.display());
                    println!("  digest: {}", outcome.candidate_digest);
                    println!("  size_bytes: {}", outcome.size_bytes);
                    println!("  deterministic_checked: {}", outcome.deterministic_checked);
                    println!("  command: {:?}", outcome.command);
                }
                OutputFormat::Json => {
                    let body = serde_json::json!({
                        "ok": true,
                        "outcome": outcome,
                    });
                    println!("{body}");
                }
            }
            VaaExitCode::Success.as_std()
        }
        Err(error) => {
            emit_generator_error(spec_path, format, &error);
            VaaExitCode::InvalidInput.as_std()
        }
    }
}

fn generator_run_command(
    config: &vaa::generator::GeneratorRunConfig,
    format: OutputFormat,
) -> ExitCode {
    use vaa::generator::{run_generator_case, GeneratorRunError};

    match run_generator_case(config) {
        Ok(outcome) => {
            match format {
                OutputFormat::Terminal => {
                    println!("ok: generator-run completed");
                    println!("  generator_id: {}", outcome.generator_id);
                    println!("  binary_digest: {}", outcome.identity.digest);
                    println!(
                        "  candidate_digest: {}",
                        outcome.generation.candidate_digest
                    );
                    println!("  output: {}", outcome.generation.output_path.display());
                    if let Some(verify) = &outcome.verify {
                        println!("  final_status: {}", verify.final_status);
                        println!("  acceptance_digest: {}", verify.acceptance_digest);
                        println!("  candidate_dir: {}", verify.candidate_dir.display());
                    } else {
                        println!("  verify: skipped");
                    }
                    if let Some(root) = &outcome.run_root {
                        println!("  run_root: {}", root.display());
                    }
                }
                OutputFormat::Json => {
                    let body = serde_json::json!({
                        "ok": true,
                        "outcome": outcome,
                    });
                    println!("{body}");
                }
            }
            VaaExitCode::Success.as_std()
        }
        Err(error) => {
            match format {
                OutputFormat::Terminal => {
                    eprintln!("error: generator-run failed");
                    eprintln!("{error}");
                }
                OutputFormat::Json => {
                    let body = serde_json::json!({
                        "ok": false,
                        "error": error.to_string(),
                    });
                    println!("{body}");
                }
            }
            match error {
                GeneratorRunError::Generator(_) | GeneratorRunError::Task(_) => {
                    VaaExitCode::InvalidInput.as_std()
                }
                GeneratorRunError::Verify(_) | GeneratorRunError::RunDir(_) => {
                    VaaExitCode::ToolFailure.as_std()
                }
            }
        }
    }
}

fn suite_validate_command(path: &Path, format: OutputFormat, show_digest: bool) -> ExitCode {
    use vaa::generator::{load_suite_manifest, suite_manifest_digest};

    match load_suite_manifest(path) {
        Ok(suite) => {
            let digest = suite_manifest_digest(&suite);
            match format {
                OutputFormat::Terminal => {
                    println!("ok: suite `{}` is valid", suite.suite_id);
                    println!("  schema_version: {}", suite.schema_version);
                    println!("  target: {}", suite.target);
                    println!("  generator.spec: {}", suite.generator.spec);
                    println!("  required_cases: {}", suite.required_cases.len());
                    if show_digest {
                        println!("  digest: {digest}");
                    }
                }
                OutputFormat::Json => {
                    let body = serde_json::json!({
                        "ok": true,
                        "suite_id": suite.suite_id,
                        "schema_version": suite.schema_version,
                        "target": suite.target,
                        "required_cases": suite.required_cases,
                        "digest": if show_digest { Some(digest) } else { None },
                    });
                    println!("{body}");
                }
            }
            VaaExitCode::Success.as_std()
        }
        Err(error) => {
            emit_generator_error(path, format, &error);
            VaaExitCode::InvalidInput.as_std()
        }
    }
}

fn suite_run_command(
    config: &vaa::generator::SuiteRunConfig,
    output: Option<&Path>,
    format: OutputFormat,
) -> ExitCode {
    use vaa::generator::{run_suite, SuiteStatus};

    match run_suite(config) {
        Ok(report) => {
            if let Some(out) = output {
                if let Err(e) = std::fs::write(
                    out,
                    serde_json::to_vec_pretty(&report.evidence).unwrap_or_default(),
                ) {
                    eprintln!("error: write suite evidence: {e}");
                    return VaaExitCode::ToolFailure.as_std();
                }
            }
            match format {
                OutputFormat::Terminal => {
                    println!("ok: suite run finished");
                    println!("  suite_id: {}", report.evidence.suite_id);
                    println!("  status: {:?}", report.evidence.status);
                    println!("  suite_digest: {}", report.evidence.suite_digest);
                    println!("  cases: {}", report.evidence.cases.len());
                    for case in &report.evidence.cases {
                        println!("    {}: {}", case.case_id, case.status);
                    }
                }
                OutputFormat::Json => {
                    let body = serde_json::json!({
                        "ok": true,
                        "report": report,
                    });
                    println!("{body}");
                }
            }
            match report.evidence.status {
                SuiteStatus::Accepted => VaaExitCode::Success.as_std(),
                SuiteStatus::Rejected | SuiteStatus::Incomplete | SuiteStatus::Failed => {
                    VaaExitCode::ToolFailure.as_std()
                }
            }
        }
        Err(error) => {
            match format {
                OutputFormat::Terminal => {
                    eprintln!("error: suite run failed");
                    eprintln!("{error}");
                }
                OutputFormat::Json => {
                    let body = serde_json::json!({
                        "ok": false,
                        "error": error.to_string(),
                    });
                    println!("{body}");
                }
            }
            VaaExitCode::InvalidInput.as_std()
        }
    }
}

fn generator_check_paths_command(
    spec_path: &Path,
    changed: &[String],
    format: OutputFormat,
) -> ExitCode {
    use vaa::generator::{check_path_policy, load_generator_spec};

    let spec = match load_generator_spec(spec_path) {
        Ok(s) => s,
        Err(error) => {
            emit_generator_error(spec_path, format, &error);
            return VaaExitCode::InvalidInput.as_std();
        }
    };
    let report = check_path_policy(changed, &spec.patch_policy);
    match format {
        OutputFormat::Terminal => {
            if report.ok {
                println!(
                    "ok: path policy passed ({} path(s))",
                    report.changed_files.len()
                );
            } else {
                eprintln!("error: path policy violated");
                for v in &report.violations {
                    eprintln!("  - {v}");
                }
            }
        }
        OutputFormat::Json => {
            let body = serde_json::json!({
                "ok": report.ok,
                "report": report,
            });
            println!("{body}");
        }
    }
    if report.ok {
        VaaExitCode::Success.as_std()
    } else {
        VaaExitCode::InvalidInput.as_std()
    }
}

fn generator_triage_command(status: &str, format: OutputFormat) -> ExitCode {
    use vaa::generator::triage_status;

    let decision = triage_status(status);
    match format {
        OutputFormat::Terminal => {
            println!("ok: triage");
            println!("  status: {status}");
            println!("  class: {:?}", decision.class);
            println!(
                "  suggest_generator_repair: {}",
                decision.suggest_generator_repair
            );
            println!("  rationale: {}", decision.rationale);
        }
        OutputFormat::Json => {
            let body = serde_json::json!({
                "ok": true,
                "status": status,
                "decision": decision,
            });
            println!("{body}");
        }
    }
    VaaExitCode::Success.as_std()
}

fn generator_map_join_command(
    map_path: &Path,
    offset: Option<&str>,
    line: Option<u64>,
    format: OutputFormat,
) -> ExitCode {
    use vaa::generator::{join_by_assembly_line, join_by_offset, load_source_map};

    let map = match load_source_map(map_path) {
        Ok(m) => m,
        Err(error) => {
            emit_generator_error(map_path, format, &error);
            return VaaExitCode::InvalidInput.as_std();
        }
    };

    let (key, entry) = match (offset, line) {
        (Some(offset), _) => (format!("offset {offset}"), join_by_offset(&map, offset)),
        (None, Some(line)) => (format!("line {line}"), join_by_assembly_line(&map, line)),
        (None, None) => {
            match format {
                OutputFormat::Terminal => {
                    println!("ok: source map valid");
                    println!("  entries: {}", map.entries.len());
                    if let Some(rev) = &map.generator_revision {
                        println!("  generator_revision: {rev}");
                    }
                }
                OutputFormat::Json => {
                    let body = serde_json::json!({
                        "ok": true,
                        "entries": map.entries.len(),
                        "generator_revision": map.generator_revision,
                    });
                    println!("{body}");
                }
            }
            return VaaExitCode::Success.as_std();
        }
    };

    if let Some(entry) = entry {
        match format {
            OutputFormat::Terminal => {
                println!("ok: joined {key}");
                if let Some(v) = entry.assembly_line {
                    println!("  assembly_line: {v}");
                }
                if let Some(v) = &entry.instruction_offset {
                    println!("  instruction_offset: {v}");
                }
                if let Some(v) = &entry.generator_input {
                    println!("  generator_input: {v}");
                }
                if let Some(v) = &entry.ir_node {
                    println!("  ir_node: {v}");
                }
                if let Some(v) = &entry.generator_source {
                    println!("  generator_source: {v}");
                }
            }
            OutputFormat::Json => {
                let body = serde_json::json!({
                    "ok": true,
                    "joined": true,
                    "entry": entry,
                });
                println!("{body}");
            }
        }
        return VaaExitCode::Success.as_std();
    }

    // Plan §13.3: absence of a mapping is reported, not an error status.
    match format {
        OutputFormat::Terminal => {
            println!("ok: no entry for {key} (fallback: assembly context only)");
        }
        OutputFormat::Json => {
            let body = serde_json::json!({
                "ok": true,
                "joined": false,
            });
            println!("{body}");
        }
    }
    VaaExitCode::Success.as_std()
}

fn generator_diagnostics_command(
    code: Option<&str>,
    category: Option<&str>,
    format: OutputFormat,
) -> ExitCode {
    use vaa::generator::{
        diagnostics_by_category, lookup_diagnostic, DiagnosticView, DIAGNOSTIC_REGISTRY,
    };

    if let Some(code) = code {
        if let Some(entry) = lookup_diagnostic(code) {
            match format {
                OutputFormat::Terminal => {
                    println!("ok: diagnostic code registered");
                    println!("  code: {}", entry.code);
                    println!("  category: {}", entry.category);
                    println!("  triage: {:?}", entry.triage);
                    println!("  description: {}", entry.description);
                }
                OutputFormat::Json => {
                    let body = serde_json::json!({
                        "ok": true,
                        "diagnostic": DiagnosticView::from(entry),
                    });
                    println!("{body}");
                }
            }
            return VaaExitCode::Success.as_std();
        }
        match format {
            OutputFormat::Terminal => {
                eprintln!("error: diagnostic code `{code}` is not registered");
            }
            OutputFormat::Json => {
                let body = serde_json::json!({
                    "ok": false,
                    "error": format!("diagnostic code `{code}` is not registered"),
                });
                println!("{body}");
            }
        }
        return VaaExitCode::InvalidInput.as_std();
    }

    let entries: Vec<&vaa::generator::DiagnosticEntry> = match category {
        Some(cat) => diagnostics_by_category(cat),
        None => DIAGNOSTIC_REGISTRY.iter().collect(),
    };
    match format {
        OutputFormat::Terminal => {
            println!("ok: {} diagnostic code(s)", entries.len());
            for entry in &entries {
                println!(
                    "  {:<38} {:<8} {:?}: {}",
                    entry.code, entry.category, entry.triage, entry.description
                );
            }
        }
        OutputFormat::Json => {
            let views: Vec<DiagnosticView> =
                entries.iter().map(|e| DiagnosticView::from(*e)).collect();
            let body = serde_json::json!({
                "ok": true,
                "count": views.len(),
                "diagnostics": views,
            });
            println!("{body}");
        }
    }
    VaaExitCode::Success.as_std()
}

fn patch_evidence_verify_command(path: &Path, format: OutputFormat) -> ExitCode {
    use vaa::generator::verify_patch_evidence_file;

    match verify_patch_evidence_file(path) {
        Ok(evidence) => {
            match format {
                OutputFormat::Terminal => {
                    println!("ok: patch evidence verified");
                    println!("  status: {:?}", evidence.status);
                    println!("  suite_id: {}", evidence.suite_id);
                    println!("  patch_digest: {}", evidence.patch_digest);
                    println!(
                        "  generator_binary_digest: {}",
                        evidence.generator_binary_digest
                    );
                    println!(
                        "  forbidden_paths_changed: {}",
                        evidence.forbidden_paths_changed.len()
                    );
                }
                OutputFormat::Json => {
                    let body = serde_json::json!({
                        "ok": true,
                        "evidence": evidence,
                    });
                    println!("{body}");
                }
            }
            VaaExitCode::Success.as_std()
        }
        Err(error) => {
            emit_generator_error(path, format, &error);
            VaaExitCode::InvalidInput.as_std()
        }
    }
}

fn patch_evidence_build_command(
    suite_evidence_path: &Path,
    base: &str,
    patched: &str,
    generator_binary_digest: &str,
    changed: &[String],
    spec_path: Option<&Path>,
    output: &Path,
    format: OutputFormat,
) -> ExitCode {
    use vaa::generator::{
        build_patch_evidence, load_generator_spec, write_patch_evidence, PatchEvidenceInput,
        PatchPolicy, PatchStatus, SuiteEvidence,
    };

    let suite_bytes = match std::fs::read(suite_evidence_path) {
        Ok(b) => b,
        Err(source) => {
            emit_generator_error(
                suite_evidence_path,
                format,
                &vaa::generator::GeneratorError::Io {
                    path: suite_evidence_path.to_path_buf(),
                    source,
                },
            );
            return VaaExitCode::InvalidInput.as_std();
        }
    };
    let suite: SuiteEvidence = match serde_json::from_slice(&suite_bytes) {
        Ok(s) => s,
        Err(error) => {
            emit_generator_error(
                suite_evidence_path,
                format,
                &vaa::generator::GeneratorError::Parse {
                    path: suite_evidence_path.to_path_buf(),
                    message: error.to_string(),
                },
            );
            return VaaExitCode::InvalidInput.as_std();
        }
    };

    let patch_policy = if let Some(spec) = spec_path {
        match load_generator_spec(spec) {
            Ok(s) => s.patch_policy,
            Err(error) => {
                emit_generator_error(spec, format, &error);
                return VaaExitCode::InvalidInput.as_std();
            }
        }
    } else {
        PatchPolicy::default()
    };

    let evidence = match build_patch_evidence(&PatchEvidenceInput {
        base_revision: base.to_owned(),
        patched_revision: patched.to_owned(),
        changed_files: changed.to_vec(),
        patch_policy,
        generator_binary_digest: generator_binary_digest.to_owned(),
        generator_spec_digest: None,
        stack_lock_digest: suite.stack_lock_digest.clone(),
        suite_id: suite.suite_id.clone(),
        suite_digest: suite.suite_digest.clone(),
        suite_status: suite.status,
        patch_bytes: None,
    }) {
        Ok(e) => e,
        Err(error) => {
            emit_generator_error(output, format, &error);
            return VaaExitCode::InvalidInput.as_std();
        }
    };

    if let Err(error) = write_patch_evidence(output, &evidence) {
        emit_generator_error(output, format, &error);
        return VaaExitCode::ToolFailure.as_std();
    }

    match format {
        OutputFormat::Terminal => {
            println!("ok: wrote patch evidence {}", output.display());
            println!("  status: {:?}", evidence.status);
            println!("  patch_digest: {}", evidence.patch_digest);
        }
        OutputFormat::Json => {
            let body = serde_json::json!({
                "ok": true,
                "path": output,
                "evidence": evidence,
            });
            println!("{body}");
        }
    }

    match evidence.status {
        PatchStatus::Accepted => VaaExitCode::Success.as_std(),
        PatchStatus::Rejected | PatchStatus::Incomplete | PatchStatus::Failed => {
            VaaExitCode::ToolFailure.as_std()
        }
    }
}

struct RepairExportConfig {
    spec: PathBuf,
    task_id: String,
    status: String,
    message: String,
    diagnostic_code: Option<String>,
    instruction_offset: Option<String>,
    artifact: String,
    artifact_digest: String,
    map_input: Option<String>,
    map_ir: Option<String>,
    map_source: Option<String>,
    map: Option<PathBuf>,
    regenerate_command: String,
    verify_command: String,
    output: PathBuf,
    markdown: bool,
}

fn repair_export_command(config: &RepairExportConfig, format: OutputFormat) -> ExitCode {
    use vaa::generator::{
        build_repair_packet, entry_to_repair_mapping, join_by_offset, load_generator_spec,
        load_source_map, write_repair_packet, RepairCommands as Commands, RepairPacketInput,
        RepairSourceMapping,
    };

    let spec = match load_generator_spec(&config.spec) {
        Ok(s) => s,
        Err(error) => {
            emit_generator_error(&config.spec, format, &error);
            return VaaExitCode::InvalidInput.as_std();
        }
    };

    // Explicit --map-* flags first; a map file join can override them.
    let mut source_mapping =
        if config.map_input.is_some() || config.map_ir.is_some() || config.map_source.is_some() {
            Some(RepairSourceMapping {
                generator_input: config.map_input.clone(),
                ir_node: config.map_ir.clone(),
                generator_source: config.map_source.clone(),
            })
        } else {
            None
        };
    // A missing/unjoinable entry never fails the export (plan §13.3
    // fallback: assembly context only). A malformed map file does fail.
    if let Some(map_path) = &config.map {
        match load_source_map(map_path) {
            Ok(map) => {
                if let Some(offset) = &config.instruction_offset {
                    if let Some(entry) = join_by_offset(&map, offset) {
                        source_mapping = Some(entry_to_repair_mapping(entry));
                    } else {
                        eprintln!(
                            "note: no source map entry for offset {offset}; packet keeps assembly context only"
                        );
                    }
                } else {
                    eprintln!(
                        "note: --map given without --instruction-offset; packet keeps assembly context only"
                    );
                }
            }
            Err(error) => {
                emit_generator_error(map_path, format, &error);
                return VaaExitCode::InvalidInput.as_std();
            }
        }
    }

    let input = RepairPacketInput {
        task_id: config.task_id.clone(),
        status: config.status.clone(),
        message: config.message.clone(),
        diagnostic_code: config.diagnostic_code.clone(),
        instruction_offset: config.instruction_offset.clone(),
        artifact_path: config.artifact.clone(),
        artifact_digest: config.artifact_digest.clone(),
        source_mapping,
        commands: Commands {
            build: spec.build.command.join(" "),
            regenerate: config.regenerate_command.clone(),
            verify: config.verify_command.clone(),
        },
    };

    let packet = match build_repair_packet(&spec, &input) {
        Ok(p) => p,
        Err(error) => {
            emit_generator_error(&config.output, format, &error);
            return VaaExitCode::InvalidInput.as_std();
        }
    };

    if let Err(error) = write_repair_packet(&config.output, &packet, config.markdown) {
        emit_generator_error(&config.output, format, &error);
        return VaaExitCode::ToolFailure.as_std();
    }

    match format {
        OutputFormat::Terminal => {
            println!("ok: wrote repair packet {}", config.output.display());
            println!("  task_id: {}", packet.task_id);
            println!("  classification: {}", packet.failure.classification);
            if config.markdown {
                println!(
                    "  markdown: {}",
                    config.output.with_extension("md").display()
                );
            }
        }
        OutputFormat::Json => {
            let body = serde_json::json!({
                "ok": true,
                "path": config.output,
                "packet": packet,
            });
            println!("{body}");
        }
    }
    VaaExitCode::Success.as_std()
}

#[allow(clippy::too_many_arguments)]
fn repair_rules_command(
    spec_path: &Path,
    regenerate_command: &str,
    verify_command: &str,
    suite_command: &str,
    build_command: Option<&str>,
    output: Option<&Path>,
    format: OutputFormat,
) -> ExitCode {
    use vaa::generator::{
        load_generator_spec, render_agent_rules, write_agent_rules, RuleCommands,
    };

    let spec = match load_generator_spec(spec_path) {
        Ok(s) => s,
        Err(error) => {
            emit_generator_error(spec_path, format, &error);
            return VaaExitCode::InvalidInput.as_std();
        }
    };
    let commands = RuleCommands {
        build: build_command.map_or_else(
            || spec.build.command.join(" "),
            std::borrow::ToOwned::to_owned,
        ),
        regenerate: regenerate_command.to_owned(),
        verify: verify_command.to_owned(),
        suite: suite_command.to_owned(),
    };

    if let Some(output) = output {
        if let Err(error) = write_agent_rules(output, &spec, &commands) {
            emit_generator_error(output, format, &error);
            return VaaExitCode::ToolFailure.as_std();
        }
        match format {
            OutputFormat::Terminal => {
                println!("ok: wrote agent rules {}", output.display());
                println!("  generator_id: {}", spec.generator_id);
            }
            OutputFormat::Json => {
                let body = serde_json::json!({
                    "ok": true,
                    "path": output,
                    "generator_id": spec.generator_id,
                });
                println!("{body}");
            }
        }
        return VaaExitCode::Success.as_std();
    }

    let rendered = render_agent_rules(&spec, &commands);
    match format {
        OutputFormat::Terminal => println!("{rendered}"),
        OutputFormat::Json => {
            let body = serde_json::json!({
                "ok": true,
                "generator_id": spec.generator_id,
                "markdown": rendered,
            });
            println!("{body}");
        }
    }
    VaaExitCode::Success.as_std()
}

fn repair_verify_command(path: &Path, format: OutputFormat) -> ExitCode {
    use vaa::generator::load_repair_packet;

    match load_repair_packet(path) {
        Ok(packet) => {
            match format {
                OutputFormat::Terminal => {
                    println!("ok: repair packet verified");
                    println!("  task_id: {}", packet.task_id);
                    println!("  generator_id: {}", packet.generator_id);
                    println!("  classification: {}", packet.failure.classification);
                }
                OutputFormat::Json => {
                    let body = serde_json::json!({
                        "ok": true,
                        "packet": packet,
                    });
                    println!("{body}");
                }
            }
            VaaExitCode::Success.as_std()
        }
        Err(error) => {
            emit_generator_error(path, format, &error);
            VaaExitCode::InvalidInput.as_std()
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
                "    os_fs_isolation: {} (true only when generator jail enforced)",
                evidence_policy.os_fs_isolation
            );
            println!(
                "  execution: default Gate = SemASM --allow-execution (semasm_host); opt-in --execution-sandbox = LocalBackend process wrapper (sandbox + backend=local; ≠ container; C-012)"
            );
            println!(
                "  trust: integrity seals + opt-in practice Ed25519 / SoftHSM smoke / Rekor+Fulcio clients — not a production trust root (G5 ops proof)"
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
                "execution": {
                    "default": "semasm_host",
                    "opt_in_execution_sandbox": "local",
                    "note": "LocalBackend ≠ container; C-012; Verified ≠ isolation",
                },
                "trust_policy": {
                    "integrity": true,
                    "authenticity": "opt-in practice",
                    "signer_kinds": ["practice-ed25519", "sigstore-dsse", "hsm-pkcs11"],
                    "production_trust_root": false,
                    "note": "SoftHSM ≠ hardware HSM; Fulcio ≠ Verified; authenticity ≠ semantic truth",
                },
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
    execution_sandbox: bool,
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
                    if execution_sandbox {
                        SemasmVerify::run_sandboxed(
                            source_path,
                            contract_path,
                            binary,
                            target,
                            allow_execution,
                        )
                    } else {
                        SemasmVerify::run(
                            source_path,
                            contract_path,
                            binary,
                            target,
                            allow_execution,
                        )
                    }
                }
                None => Err(VerifyError::BinaryNotFound),
            };
            match verify_result {
                Ok(report) => {
                    if use_cache {
                        let store = vaa::CacheStore::open(vaa::resolve_cache_root());
                        let status = match report.outcome {
                            EvidenceStatus::Verified => "Verified",
                            EvidenceStatus::VerifiedUnderPreconditions => {
                                "VerifiedUnderPreconditions"
                            }
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
                        execution_isolation: if execution_sandbox {
                            "sandbox".to_owned()
                        } else {
                            "semasm_host".to_owned()
                        },
                        execution_sandbox_backend: if execution_sandbox {
                            Some("local".to_owned())
                        } else {
                            None
                        },
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
    if locked.task().verification.require_reproducible_build {
        let (matched, details) = vaa::reproducible_build_check(source_path, target);
        expect.reproducible_build = Some(vaa::ReproducibleBuildOutcome { matched, details });
    }

    let mut report = EvidenceAggregator::build(
        &locked,
        None,
        Some(verify_report),
        Some(doctor),
        Some(cm),
        &expect,
    );
    report.execution_isolation = if execution_sandbox {
        "sandbox".to_owned()
    } else {
        "semasm_host".to_owned()
    };
    report.execution_sandbox_backend = if execution_sandbox {
        Some("local".to_owned())
    } else {
        None
    };
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

fn durability_probe_command(path: Option<&Path>) -> ExitCode {
    let dir = path.map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let report = vaa::probe_durability(&dir);
    println!("durability probe: {}", report.path);
    println!("  class: {:?}", report.class);
    for d in &report.details {
        println!("  - {d}");
    }
    println!(
        "  may_claim_verified: {}",
        vaa::may_claim_verified(report.class)
    );
    println!("note: not a formal proof of filesystem correctness");
    if matches!(report.class, vaa::DurabilityClass::RefuseVerified) {
        VaaExitCode::ToolFailure.as_std()
    } else {
        VaaExitCode::Success.as_std()
    }
}

fn publish_rekor_command(file: &Path, rekor_url: &str, dry_run: bool) -> ExitCode {
    let doc = match vaa::read_transparency_file(file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: read transparency: {e}");
            return VaaExitCode::InvalidInput.as_std();
        }
    };
    let payload = match serde_json::to_vec(&doc) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: serialize transparency: {e}");
            return VaaExitCode::ToolFailure.as_std();
        }
    };
    let Ok(seed) = std::env::var(vaa::ENV_SEAL_SIGNING_KEY) else {
        eprintln!("error: set VAA_SEAL_SIGNING_KEY to sign DSSE before Rekor publish");
        return VaaExitCode::InvalidInput.as_std();
    };
    let raw = match std::fs::read_to_string(&seed) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: read signing key: {e}");
            return VaaExitCode::ToolFailure.as_std();
        }
    };
    let hex: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    let mut seed_bytes = [0u8; 32];
    if hex.len() != 64 {
        eprintln!("error: signing seed must be 64 hex chars");
        return VaaExitCode::InvalidInput.as_std();
    }
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk).unwrap();
        seed_bytes[i] = u8::from_str_radix(s, 16).unwrap_or(0);
    }
    let signer = vaa::SigstoreDsseSigner::from_seed(seed_bytes);
    let dsse = match signer.sign_payload(vaa::DSSE_PAYLOAD_TYPE_TRANSPARENCY, &payload) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: dsse sign: {e}");
            return VaaExitCode::ToolFailure.as_std();
        }
    };
    let dsse_path = file.with_extension("dsse.json");
    if let Err(e) = vaa::write_dsse_file(&dsse_path, &dsse) {
        eprintln!("error: write dsse: {e}");
        return VaaExitCode::ToolFailure.as_std();
    }

    let result = if dry_run {
        vaa::publish_dsse(&vaa::MockRekorTransport::new(), &dsse, true)
    } else {
        #[cfg(feature = "rekor")]
        {
            let transport = vaa::UreqRekorTransport {
                base_url: rekor_url.to_owned(),
            };
            vaa::publish_dsse(&transport, &dsse, false)
        }
        #[cfg(not(feature = "rekor"))]
        {
            let _ = rekor_url;
            eprintln!(
                "error: live Rekor requires `--features rekor` (use --dry-run for offline payload)"
            );
            return VaaExitCode::ToolFailure.as_std();
        }
    };

    match result {
        Ok(r) => {
            println!(
                "ok: rekor publish uuid={} digest={}",
                r.uuid, r.entry_digest
            );
            println!("  dsse: {}", dsse_path.display());
            println!("  dry_run: {}", r.dry_run);
            println!("note: Rekor entry ≠ SemASM Verified");
            VaaExitCode::Success.as_std()
        }
        Err(e) => {
            eprintln!("error: rekor publish: {e}");
            VaaExitCode::ToolFailure.as_std()
        }
    }
}

fn verify_rekor_command(file: &Path, uuid: &str, rekor_url: &str) -> ExitCode {
    let dsse_path = file.with_extension("dsse.json");
    let Some(dsse) = std::fs::read_to_string(&dsse_path)
        .ok()
        .and_then(|s| serde_json::from_str::<vaa::DsseEnvelope>(&s).ok())
    else {
        eprintln!(
            "error: missing/invalid DSSE at {} (publish-rekor first)",
            dsse_path.display()
        );
        return VaaExitCode::InvalidInput.as_std();
    };
    if let Err(e) = vaa::verify_dsse_envelope(&dsse) {
        eprintln!("error: dsse verify: {e}");
        return VaaExitCode::ToolFailure.as_std();
    }

    #[cfg(feature = "rekor")]
    {
        let transport = vaa::UreqRekorTransport {
            base_url: rekor_url.to_owned(),
        };
        match vaa::verify_entry_matches_dsse(&transport, uuid, &dsse) {
            Ok(()) => {
                println!("ok: rekor entry {uuid} matches DSSE");
                VaaExitCode::Success.as_std()
            }
            Err(e) => {
                eprintln!("error: rekor verify: {e}");
                VaaExitCode::ToolFailure.as_std()
            }
        }
    }
    #[cfg(not(feature = "rekor"))]
    {
        let _ = (uuid, rekor_url);
        eprintln!("error: live Rekor verify requires `--features rekor`");
        VaaExitCode::ToolFailure.as_std()
    }
}

fn fulcio_sign_command(
    file: &Path,
    fulcio_url: &str,
    oidc_token: Option<&str>,
    dry_run: bool,
    output: Option<&Path>,
) -> ExitCode {
    let payload = match std::fs::read(file) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: read {}: {e}", file.display());
            return VaaExitCode::InvalidInput.as_std();
        }
    };

    let mut seed_bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut seed_bytes);

    let result = if dry_run {
        let token = oidc_token.map_or_else(vaa::dry_run_oidc_token, str::to_owned);
        vaa::keyless_sign_transparency(
            &vaa::MockFulcioTransport::new(),
            &token,
            &payload,
            seed_bytes,
        )
    } else {
        let Some(token) = oidc_token else {
            eprintln!("error: live Fulcio requires --oidc-token / VAA_OIDC_TOKEN (or --dry-run)");
            return VaaExitCode::InvalidInput.as_std();
        };
        #[cfg(feature = "fulcio")]
        {
            let transport = vaa::UreqFulcioTransport {
                base_url: fulcio_url.to_owned(),
            };
            vaa::keyless_sign_transparency(&transport, token, &payload, seed_bytes)
        }
        #[cfg(not(feature = "fulcio"))]
        {
            let _ = (fulcio_url, token);
            eprintln!(
                "error: live Fulcio requires `--features fulcio` (use --dry-run for offline mock)"
            );
            return VaaExitCode::ToolFailure.as_std();
        }
    };

    match result {
        Ok(r) => {
            let out = output.map_or_else(|| file.with_extension("fulcio.json"), PathBuf::from);
            let doc = serde_json::json!({
                "schema": "vaa-fulcio-dsse-v1",
                "certificate_chain_pem": r.certificate_chain_pem,
                "public_key_b64": r.public_key_b64,
                "dsse": r.dsse,
                "dry_run": dry_run,
            });
            if let Err(e) =
                std::fs::write(&out, serde_json::to_vec_pretty(&doc).unwrap_or_default())
            {
                eprintln!("error: write {}: {e}", out.display());
                return VaaExitCode::ToolFailure.as_std();
            }
            println!("ok: fulcio keyless DSSE -> {}", out.display());
            println!("note: Fulcio identity attest ≠ SemASM Verified");
            VaaExitCode::Success.as_std()
        }
        Err(e) => {
            eprintln!("error: fulcio sign: {e}");
            VaaExitCode::ToolFailure.as_std()
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn search_command(
    task_path: &Path,
    seed: &Path,
    run_dir: &Path,
    budget: u32,
    mutator: &str,
    mutator_command: Option<&Path>,
    ingest: bool,
    contract: Option<&Path>,
    allow_execution: bool,
) -> ExitCode {
    if ingest && contract.is_none() {
        eprintln!("error: --ingest requires --contract");
        return VaaExitCode::InvalidInput.as_std();
    }
    let locked = match load_locked_task(task_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: load task: {e}");
            return VaaExitCode::InvalidInput.as_std();
        }
    };
    let seed_asm = match std::fs::read_to_string(seed) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: read seed: {e}");
            return VaaExitCode::InvalidInput.as_std();
        }
    };
    let ingest_cfg = contract.map(|contract_path| vaa::SearchIngestConfig {
        task_path,
        contract_path,
        allow_execution,
    });
    if ingest && ingest_cfg.is_none() {
        eprintln!("error: --ingest requires --contract");
        return VaaExitCode::InvalidInput.as_std();
    }
    let ingest_cfg = if ingest { ingest_cfg } else { None };
    match vaa::run_search(
        &locked,
        &seed_asm,
        run_dir,
        budget,
        mutator,
        mutator_command,
        false,
        ingest_cfg,
    ) {
        Ok(report) => {
            println!(
                "search: attempts={} verified={} reason={}",
                report.attempts.len(),
                report.verified,
                report.stopped_reason
            );
            for a in &report.attempts {
                println!("  [{:>4}] {} {}", a.index, a.status, a.source_digest);
            }
            if report.verified {
                println!("note: SemASM Verified only — not CryptOpt; Gate-2 allow-execution path");
            } else {
                println!(
                    "note: CryptOpt-like search loop only — Incomplete≠Verified; not formal superoptimization"
                );
            }
            VaaExitCode::Success.as_std()
        }
        Err(e) => {
            eprintln!("error: search failed: {e}");
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
    generator_jail: bool,
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

    if generator_jail && command.is_none() {
        eprintln!("error: `--generator-jail` requires `--command`");
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
        let _ = output_path;

        let mut gen = vaa::ArgvExternalGenerator::new("external-argv", prog)
            .with_args(command_args.to_vec())
            .with_output_relative(relative.clone());
        if generator_jail {
            let runtime = vaa::probe_container_runtime().unwrap_or_else(|| "docker".to_owned());
            let seccomp = {
                let path = rd.paths().staging_dir.join("vaa-seccomp.json");
                let _ = vaa::write_default_seccomp_profile(&path);
                Some(path)
            };
            gen = gen.with_jail(vaa::GeneratorJailOpts {
                runtime,
                image: std::env::var("VAA_CONTAINER_IMAGE")
                    .unwrap_or_else(|_| vaa::DEFAULT_CONTAINER_IMAGE.to_owned()),
                image_digest: std::env::var("VAA_CONTAINER_IMAGE_DIGEST").ok(),
                seccomp_profile: seccomp,
                memory_limit_bytes: Some(256 * 1024 * 1024),
                pids_limit: Some(128),
            });
        }

        return match gen.generate_to_staging(
            &rd.paths().staging_dir,
            task_path,
            &locked.task().task_id,
            &locked.task().target,
        ) {
            Ok(resp) => match rd.write_staging(&relative, resp.source.as_bytes()) {
                Ok(written) => {
                    let kind = if generator_jail {
                        "external-argv+os-jail"
                    } else {
                        "external-argv"
                    };
                    println!(
                        "generated `{}` (model: {}, id: {}, kind: {kind})",
                        written.display(),
                        resp.model_name,
                        resp.generation_id
                    );
                    if generator_jail {
                        println!("note: os_fs_isolation enforced via container jail (not absolute isolation)");
                    }
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

#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
fn build_command(
    source: &Path,
    output_dir: &Path,
    target: &str,
    sandbox: BuildSandboxMode,
    container_image: Option<&str>,
    container_image_digest: Option<&str>,
    container_runtime: Option<&str>,
    cpu_quota: Option<f64>,
    memory_limit_bytes: Option<u64>,
    bind_host_paths: bool,
    use_seccomp: bool,
    require_rootless: bool,
    use_cache: bool,
    check_reproducible: bool,
    format: OutputFormat,
) -> ExitCode {
    if check_reproducible {
        let config = PipelineConfig {
            source_path: source.to_path_buf(),
            output_dir: output_dir.to_path_buf(),
            target: target.to_owned(),
            ..PipelineConfig::default()
        };
        return match vaa::check_reproducible(&config) {
            Ok(report) => {
                match format {
                    OutputFormat::Terminal => {
                        if report.matched {
                            println!("reproducible: matched (same-host twin build)");
                        } else {
                            eprintln!("reproducible: mismatch");
                            for m in &report.mismatches {
                                eprintln!("  {m}");
                            }
                        }
                        println!("note: not a cross-host bit-identical claim");
                    }
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_value(&report).unwrap_or_default());
                    }
                }
                if report.matched {
                    VaaExitCode::Success.as_std()
                } else {
                    VaaExitCode::ToolFailure.as_std()
                }
            }
            Err(e) => {
                eprintln!("error: reproducible check failed: {e}");
                VaaExitCode::ToolFailure.as_std()
            }
        };
    }

    let container = match sandbox {
        BuildSandboxMode::Local => None,
        BuildSandboxMode::Container => {
            let runtime = container_runtime
                .map(str::to_owned)
                .or_else(vaa::probe_container_runtime)
                .unwrap_or_else(|| "docker".to_owned());
            if require_rootless {
                match vaa::probe_rootless_runtime(&runtime) {
                    Ok(true) => {}
                    Ok(false) => {
                        eprintln!(
                            "error: --require-rootless set but `{runtime}` does not look rootless"
                        );
                        return VaaExitCode::ToolFailure.as_std();
                    }
                    Err(e) => {
                        eprintln!("error: rootless probe failed: {e}");
                        return VaaExitCode::ToolFailure.as_std();
                    }
                }
            }
            let seccomp_profile = if use_seccomp {
                let path = output_dir.join("vaa-seccomp.json");
                if let Err(e) = vaa::write_default_seccomp_profile(&path) {
                    eprintln!("error: write seccomp profile: {e}");
                    return VaaExitCode::ToolFailure.as_std();
                }
                Some(path)
            } else {
                None
            };
            Some(vaa::ContainerBuildOpts {
                runtime,
                image: container_image
                    .unwrap_or(vaa::DEFAULT_CONTAINER_IMAGE)
                    .to_owned(),
                image_digest: container_image_digest.map(str::to_owned),
                cpu_quota,
                pids_limit: Some(256),
                memory_limit_bytes,
                bind_host_paths,
                seccomp_profile,
                require_rootless,
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
