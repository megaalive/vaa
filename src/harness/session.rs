//! Prepare / submit / resume session helpers for the agent harness façade.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde_json::Value;

use crate::evidence::EvidenceStatus;
use crate::harness::envelope::{
    AgentBudget, AgentCommands, AgentDigests, AgentEnvelope, AgentMode,
};
use crate::harness::feedback::{
    classify_outcome, HarnessNextAction, HarnessOutcomeClass, HarnessSubmitResult,
    HARNESS_SUBMIT_SCHEMA_VERSION,
};
use crate::process::{ProcessConfig, ProcessRunner};
use crate::semasm::doctor::{semasm_subprocess_allowed_env, SemasmDoctor, ENV_SEMASM_BIN};
use crate::semasm::verify::SemasmVerify;
use crate::sha256_digest_prefixed;
use crate::task::load_locked_task;

/// Errors from harness session operations.
#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    #[error("{0}")]
    Message(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("task: {0}")]
    Task(String),
}

/// Inputs for `prepare --mode direct-nasm`.
#[derive(Debug, Clone)]
pub struct PrepareDirectRequest {
    pub task: PathBuf,
    pub contract: PathBuf,
    pub workspace: PathBuf,
    pub seed_source: Option<PathBuf>,
    pub allow_execution_in_recipes: bool,
}

/// Inputs for `prepare --mode generator-repair`.
#[derive(Debug, Clone)]
pub struct PrepareGeneratorRequest {
    pub repair_packet: PathBuf,
    pub workspace: PathBuf,
    pub target: String,
}

/// Inputs for `submit` in direct-NASM mode.
#[derive(Debug, Clone)]
pub struct SubmitDirectRequest {
    pub task: PathBuf,
    pub contract: PathBuf,
    pub source: PathBuf,
    pub allow_execution: bool,
    pub allow_under_preconditions: bool,
    pub run_dir: Option<PathBuf>,
    pub timeout_secs: u64,
}

/// Prepare a direct-NASM workspace + agent envelope.
pub fn prepare_direct_nasm(req: &PrepareDirectRequest) -> Result<AgentEnvelope, HarnessError> {
    let locked = load_locked_task(&req.task).map_err(|e| HarnessError::Task(e.to_string()))?;
    let task = locked.task();
    fs::create_dir_all(&req.workspace)?;

    let candidate = req.workspace.join("candidate.asm");
    if let Some(seed) = &req.seed_source {
        fs::copy(seed, &candidate)?;
    } else if !candidate.exists() {
        fs::write(
            &candidate,
            format!(
                "; TODO: implement `{}` for target {}\nbits 64\ndefault rel\nsection .text\nglobal {}\n{}:\n    ret\n",
                task.entry.symbol, task.target, task.entry.symbol, task.entry.symbol
            ),
        )?;
    }

    let contract_dest = req.workspace.join("contract.sem.toml");
    fs::copy(&req.contract, &contract_dest)?;
    let task_dest = req.workspace.join("task.vaa.toml");
    fs::copy(&req.task, &task_dest)?;

    // Best-effort SemASM packet (non-fatal if semasm missing).
    let packet_path = req.workspace.join("semasm-packet.json");
    let prompt_path = req.workspace.join("prompt.md");
    let _ = try_semasm_packet(&req.contract, &task.target, Some(&candidate), &packet_path);

    let doctor = SemasmDoctor::run();
    let doctor_cmd = format!("vaa doctor --format json");
    let verify = format!(
        "semasm agent verify {} {} --target {} --format json",
        candidate.display(),
        contract_dest.display(),
        task.target
    );
    let verify_gate2 = format!("{verify} --allow-execution");

    let budget = AgentBudget {
        max_candidates: task.budgets.max_candidates.max(1),
        max_repairs_per_candidate: task.budgets.max_repairs_per_candidate,
        max_wall_time_seconds: task.budgets.max_wall_time_seconds.max(1),
    };

    let task_bytes = fs::read(&req.task)?;
    let contract_bytes = fs::read(&req.contract)?;
    let mut env = AgentEnvelope::direct_nasm(
        task.target.clone(),
        task.task_id.clone(),
        AgentCommands {
            doctor: Some(doctor_cmd),
            verify: verify.clone(),
            verify_gate2: Some(verify_gate2),
            regenerate: None,
            suite: None,
        },
        budget,
    );
    env.abi = Some(task.entry.abi.clone());
    env.writable_paths = vec![candidate.display().to_string()];
    env.workspace_dir = Some(req.workspace.display().to_string());
    env.semasm_packet_path = packet_path
        .exists()
        .then(|| packet_path.display().to_string());
    env.digests = AgentDigests {
        task: Some(sha256_digest_prefixed(&task_bytes)),
        contract: Some(sha256_digest_prefixed(&contract_bytes)),
        candidate: candidate
            .exists()
            .then(|| fs::read(&candidate).ok())
            .flatten()
            .map(|b| sha256_digest_prefixed(&b)),
    };

    let prompt = render_direct_prompt(&env, &doctor.status, req.allow_execution_in_recipes);
    fs::write(&prompt_path, prompt)?;
    env.prompt_markdown_path = Some(prompt_path.display().to_string());

    let envelope_path = req.workspace.join("agent-envelope.json");
    fs::write(&envelope_path, serde_json::to_string_pretty(&env)?)?;
    Ok(env)
}

/// Prepare from an existing repair packet (generator mode).
pub fn prepare_generator_repair(
    req: &PrepareGeneratorRequest,
) -> Result<AgentEnvelope, HarnessError> {
    fs::create_dir_all(&req.workspace)?;
    let raw = fs::read_to_string(&req.repair_packet)?;
    let packet: Value = serde_json::from_str(&raw)?;
    let task_id = packet
        .get("task_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_owned();
    let generator_id = packet
        .get("generator_id")
        .and_then(|v| v.as_str())
        .unwrap_or("generator")
        .to_owned();

    let allowed = packet
        .pointer("/repository/allowed_paths")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(str::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let forbidden = packet
        .pointer("/repository/forbidden_paths")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(str::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let build = packet
        .pointer("/commands/build")
        .and_then(|v| v.as_str())
        .unwrap_or("dotnet build")
        .to_owned();
    let regenerate = packet
        .pointer("/commands/regenerate")
        .and_then(|v| v.as_str())
        .unwrap_or("vaa suite run …")
        .to_owned();
    let verify = packet
        .pointer("/commands/verify")
        .and_then(|v| v.as_str())
        .unwrap_or("dotnet test")
        .to_owned();

    let dest = req.workspace.join("repair-packet.json");
    fs::copy(&req.repair_packet, &dest)?;
    let md = req.workspace.join("repair-packet.md");
    if let Ok(packet) = crate::generator::load_repair_packet(&req.repair_packet) {
        fs::write(&md, crate::generator::render_repair_markdown(&packet))?;
    }

    let env = AgentEnvelope {
        schema_version: crate::harness::envelope::AGENT_ENVELOPE_SCHEMA_VERSION.to_owned(),
        mode: AgentMode::GeneratorRepair,
        target: req.target.clone(),
        abi: None,
        task_id,
        run_id: None,
        writable_paths: allowed,
        forbidden_paths: forbidden,
        commands: AgentCommands {
            doctor: Some("vaa doctor --format json".into()),
            verify,
            verify_gate2: None,
            regenerate: Some(regenerate),
            suite: Some(build),
        },
        budget: AgentBudget::default(),
        digests: AgentDigests::default(),
        semasm_packet_path: None,
        repair_packet_path: Some(dest.display().to_string()),
        workspace_dir: Some(req.workspace.display().to_string()),
        prompt_markdown_path: md.exists().then(|| md.display().to_string()),
    };
    let _ = generator_id;
    let envelope_path = req.workspace.join("agent-envelope.json");
    fs::write(&envelope_path, serde_json::to_string_pretty(&env)?)?;
    Ok(env)
}

/// Submit a direct-NASM candidate through SemASM and classify the outcome.
pub fn submit_direct_nasm(req: &SubmitDirectRequest) -> Result<HarnessSubmitResult, HarnessError> {
    let locked = load_locked_task(&req.task).map_err(|e| HarnessError::Task(e.to_string()))?;
    let target = locked.task().target.clone();

    let doctor = SemasmDoctor::run();
    let Some(binary) = doctor.binary_path.clone() else {
        let (class, next, exit) = classify_outcome(
            EvidenceStatus::Failed,
            None,
            Some("TOOLCHAIN_INCOMPLETE"),
            req.allow_under_preconditions,
        );
        return Ok(HarnessSubmitResult {
            schema_version: HARNESS_SUBMIT_SCHEMA_VERSION.to_owned(),
            class,
            next_action: next,
            evidence_status: "failed".into(),
            raw_status: None,
            exit_code: exit as u8,
            message: "semasm binary not found".into(),
            failure_code: Some("TOOLCHAIN_INCOMPLETE".into()),
            candidate_digest: None,
            run_dir: req.run_dir.as_ref().map(|p| p.display().to_string()),
            may_auto_retry: class.may_auto_retry(),
        });
    };

    let source_bytes = fs::read(&req.source)?;
    let candidate_digest = sha256_digest_prefixed(&source_bytes);

    match SemasmVerify::run_with_timeout(
        &req.source,
        &req.contract,
        &binary,
        &target,
        req.allow_execution,
        req.timeout_secs,
    ) {
        Ok(report) => {
            let (class, mut next, exit) = classify_outcome(
                report.outcome,
                Some(report.raw_status.as_str()),
                None,
                req.allow_under_preconditions,
            );
            if matches!(class, HarnessOutcomeClass::ViolatedRepairable) {
                next = HarnessNextAction::EditCandidate;
            }
            Ok(HarnessSubmitResult {
                schema_version: HARNESS_SUBMIT_SCHEMA_VERSION.to_owned(),
                class,
                next_action: next,
                evidence_status: evidence_status_label(report.outcome).to_owned(),
                raw_status: Some(report.raw_status),
                exit_code: exit as u8,
                message: format!("semasm status mapped to {}", evidence_status_label(report.outcome)),
                failure_code: None,
                candidate_digest: Some(candidate_digest),
                run_dir: req.run_dir.as_ref().map(|p| p.display().to_string()),
                may_auto_retry: class.may_auto_retry(),
            })
        }
        Err(err) => {
            let failure_code = err.failure_code().map(str::to_owned);
            let (class, next, exit) = classify_outcome(
                EvidenceStatus::Failed,
                None,
                failure_code.as_deref(),
                req.allow_under_preconditions,
            );
            Ok(HarnessSubmitResult {
                schema_version: HARNESS_SUBMIT_SCHEMA_VERSION.to_owned(),
                class,
                next_action: next,
                evidence_status: "failed".into(),
                raw_status: None,
                exit_code: exit as u8,
                message: err.to_string(),
                failure_code,
                candidate_digest: Some(candidate_digest),
                run_dir: req.run_dir.as_ref().map(|p| p.display().to_string()),
                may_auto_retry: class.may_auto_retry(),
            })
        }
    }
}

/// Resume cursor summary for an existing run directory.
pub fn resume_status(run_dir: &Path) -> Result<Value, HarnessError> {
    let run = crate::run::RunDir::open(run_dir).map_err(|e| HarnessError::Message(e.to_string()))?;
    let cursor =
        crate::run::scan_resume_cursor(&run).map_err(|e| HarnessError::Message(e.to_string()))?;
    Ok(serde_json::json!({
        "schema_version": "0.1",
        "run_dir": run_dir.display().to_string(),
        "next_candidate_index": cursor.next_candidate_index,
        "previous_seal_digest": cursor.previous_seal_digest,
        "event_count": cursor.event_count,
    }))
}

fn try_semasm_packet(
    contract: &Path,
    target: &str,
    source: Option<&Path>,
    out: &Path,
) -> Result<(), HarnessError> {
    let doctor = SemasmDoctor::run();
    let Some(binary) = doctor.binary_path else {
        return Err(HarnessError::Message("semasm not found".into()));
    };
    let mut args = vec![
        "agent".into(),
        "packet".into(),
        contract.display().to_string(),
        "--target".into(),
        target.to_owned(),
        "--format".into(),
        "json".into(),
    ];
    if let Some(src) = source {
        args.push("--source".into());
        args.push(src.display().to_string());
    }
    let config = ProcessConfig {
        program: binary,
        args,
        timeout: Duration::from_secs(60),
        max_output_bytes: 4 * 1_048_576,
        allowed_env: semasm_subprocess_allowed_env(),
        ..ProcessConfig::default()
    };
    let output = ProcessRunner::run(&config).map_err(|e| HarnessError::Message(e.to_string()))?;
    if output.stdout.trim().is_empty() {
        return Err(HarnessError::Message("empty packet stdout".into()));
    }
    fs::write(out, output.stdout)?;
    Ok(())
}

fn render_direct_prompt(
    env: &AgentEnvelope,
    doctor_status: &crate::semasm::doctor::DoctorStatus,
    prefer_gate2: bool,
) -> String {
    let verify = if prefer_gate2 {
        env.commands
            .verify_gate2
            .clone()
            .unwrap_or_else(|| env.commands.verify.clone())
    } else {
        env.commands.verify.clone()
    };
    format!(
        "# Direct NASM harness task `{task}`\n\n\
Target: `{target}`\n\
Mode: `{mode}`\n\
Doctor: `{doctor:?}`\n\n\
## Writable\n\n{writable}\n\n\
## Forbidden\n\n{forbidden}\n\n\
## Verify\n\n```text\n{verify}\n```\n\n\
## Rules\n\n\
- Edit only writable paths.\n\
- Never edit `*.vaa.toml`, `*.sem.toml`, or `stack.lock.toml`.\n\
- Success requires SemASM `verified` (or allowed `verified_under_preconditions`).\n\
- `incomplete` / `execution_denied` is not success.\n\
- Fb9c arbitrary loop invariants stay locked.\n",
        task = env.task_id,
        target = env.target,
        mode = env.mode.as_str(),
        doctor = doctor_status,
        writable = env
            .writable_paths
            .iter()
            .map(|p| format!("- `{p}`"))
            .collect::<Vec<_>>()
            .join("\n"),
        forbidden = env
            .forbidden_paths
            .iter()
            .map(|p| format!("- `{p}`"))
            .collect::<Vec<_>>()
            .join("\n"),
        verify = verify,
    )
}

/// Resolve SemASM binary path for tests / adapters (honours `SEMASM_BIN`).
#[must_use]
pub fn resolve_semasm_hint() -> Option<PathBuf> {
    std::env::var_os(ENV_SEMASM_BIN).map(PathBuf::from).or_else(|| {
        Command::new("semasm")
            .arg("--version")
            .output()
            .ok()
            .and_then(|_| which_semasm())
    })
}

fn which_semasm() -> Option<PathBuf> {
    SemasmDoctor::run().binary_path
}

fn evidence_status_label(status: EvidenceStatus) -> &'static str {
    match status {
        EvidenceStatus::Verified => "verified",
        EvidenceStatus::VerifiedUnderPreconditions => "verified_under_preconditions",
        EvidenceStatus::Violated => "violated",
        EvidenceStatus::Incomplete => "incomplete",
        EvidenceStatus::Failed => "failed",
    }
}
