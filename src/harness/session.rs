//! Prepare / submit / resume session helpers for the agent harness façade.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde_json::Value;

use crate::candidate::CandidateProtocol;
use crate::evidence::{EvidenceStatus, GeneratorMeta};
use crate::generator::{
    build_patch_evidence, load_repair_packet, path_policy_violations, run_suite,
    write_patch_evidence, PatchEvidenceInput, PatchPolicy, PatchStatus, SuiteRunConfig,
    SuiteStatus,
};
use crate::harness::assembler::AssemblerFlavor;
use crate::harness::envelope::{
    default_allowed_operations, AgentBudget, AgentCommands, AgentDigests, AgentEnvelope, AgentMode,
};
use crate::harness::feedback::{
    classify_outcome, enrich_repair_feedback, HarnessNextAction, HarnessOutcomeClass,
    HarnessSubmitResult, HARNESS_SUBMIT_SCHEMA_VERSION,
};
use crate::harness::target_profile::write_target_profile;
use crate::process::{ProcessConfig, ProcessRunner};
use crate::run::{
    doctor_and_capabilities, scan_resume_cursor, verify_candidate_and_seal, EventKind, EventLog,
    RunDir, RunId, VerifySealInput,
};
use crate::semasm::admission::CAPABILITY_SNAPSHOT_DIGEST;
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

/// Inputs for `prepare --mode direct-nasm` (assembler flavor may be gas later).
#[derive(Debug, Clone)]
pub struct PrepareDirectRequest {
    pub task: PathBuf,
    pub contract: PathBuf,
    pub workspace: PathBuf,
    pub seed_source: Option<PathBuf>,
    pub allow_execution_in_recipes: bool,
    pub assembler: AssemblerFlavor,
    pub run_dir: Option<PathBuf>,
}

/// Inputs for `prepare --mode generator-repair`.
#[derive(Debug, Clone)]
pub struct PrepareGeneratorRequest {
    pub repair_packet: PathBuf,
    pub workspace: PathBuf,
    pub target: String,
}

/// Inputs for `submit` in direct-assembly mode.
#[derive(Debug, Clone)]
pub struct SubmitDirectRequest {
    pub task: PathBuf,
    pub contract: PathBuf,
    pub source: PathBuf,
    pub allow_execution: bool,
    pub allow_under_preconditions: bool,
    /// When set, create/open a run directory and seal the candidate.
    pub run_dir: Option<PathBuf>,
    /// Parent directory used when creating a fresh run (`RunDir::create`).
    pub run_base: Option<PathBuf>,
    pub timeout_secs: u64,
    pub assembler: AssemblerFlavor,
    pub idempotency_key: Option<String>,
}

/// Inputs for `submit` in generator-repair mode.
#[derive(Debug, Clone)]
pub struct SubmitGeneratorRequest {
    pub repair_packet: PathBuf,
    pub workspace: PathBuf,
    pub changed_files: Vec<String>,
    pub patched_revision: String,
    pub base_revision: Option<String>,
    pub suite: Option<PathBuf>,
    pub suite_evidence: Option<PathBuf>,
    pub run_base: Option<PathBuf>,
    pub repo: Option<PathBuf>,
    pub allow_execution: bool,
    pub skip_build: bool,
    pub skip_repo_guard: bool,
    pub generator_binary_digest: Option<String>,
}

/// Prepare a direct-assembly workspace + agent envelope.
pub fn prepare_direct_nasm(req: &PrepareDirectRequest) -> Result<AgentEnvelope, HarnessError> {
    let locked = load_locked_task(&req.task).map_err(|e| HarnessError::Task(e.to_string()))?;
    let task = locked.task();
    req.assembler
        .ensure_supported_for(&task.target)
        .map_err(HarnessError::Message)?;
    fs::create_dir_all(&req.workspace)?;

    let candidate_name = req.assembler.candidate_filename();
    let candidate = req.workspace.join(candidate_name);
    if let Some(seed) = &req.seed_source {
        fs::copy(seed, &candidate)?;
    } else if !candidate.exists() {
        fs::write(
            &candidate,
            req.assembler.seed_stub(&task.entry.symbol, &task.target),
        )?;
    }

    let contract_dest = req.workspace.join("contract.sem.toml");
    fs::copy(&req.contract, &contract_dest)?;
    let task_dest = req.workspace.join("task.vaa.toml");
    fs::copy(&req.task, &task_dest)?;

    let packet_path = req.workspace.join("semasm-packet.json");
    let prompt_path = req.workspace.join("prompt.md");
    let _ = try_semasm_packet(&req.contract, &task.target, Some(&candidate), &packet_path);

    let doctor = SemasmDoctor::run();
    let doctor_cmd = "vaa doctor --format json".to_owned();
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

    let sealed = req
        .run_dir
        .as_ref()
        .and_then(|p| RunDir::open(p).ok())
        .and_then(|r| r.resume_cursor().ok())
        .map_or(0, |c| c.next_candidate_index);
    let remaining = budget.max_candidates.saturating_sub(sealed);

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
    env.assembler = req.assembler;
    env.abi = Some(task.entry.abi.clone());
    env.writable_paths = vec![candidate.display().to_string()];
    env.workspace_dir = Some(req.workspace.display().to_string());
    env.remaining_attempts = Some(remaining);
    env.semasm_packet_path = packet_path
        .exists()
        .then(|| packet_path.display().to_string());
    let (profile_path, profile_digest) = write_target_profile(&req.workspace, &task.target)?;
    let feedback_path = req.workspace.join("feedback.json");
    let work_packet_path = req.workspace.join("work-packet.json");

    env.allowed_operations = default_allowed_operations();
    env.target_profile_path = Some(profile_path.display().to_string());
    env.feedback_path = Some(feedback_path.display().to_string());
    env.digests = AgentDigests {
        task: Some(sha256_digest_prefixed(&task_bytes)),
        contract: Some(sha256_digest_prefixed(&contract_bytes)),
        candidate: candidate
            .exists()
            .then(|| fs::read(&candidate).ok())
            .flatten()
            .map(|b| sha256_digest_prefixed(&b)),
        capability_snapshot: Some(CAPABILITY_SNAPSHOT_DIGEST.to_owned()),
        target_profile: Some(profile_digest),
    };
    if let Some(run) = &req.run_dir {
        env.events_path = Some(run.join("events.jsonl").display().to_string());
        env.run_id = Some(
            run.file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
        );
    }

    let prompt = render_direct_prompt(&env, &doctor.status, req.allow_execution_in_recipes);
    fs::write(&prompt_path, prompt)?;
    env.prompt_markdown_path = Some(prompt_path.display().to_string());

    // Paths must be set before serializing so both files carry them.
    let envelope_path = req.workspace.join("agent-envelope.json");
    env.work_packet_path = Some(work_packet_path.display().to_string());
    let pretty = serde_json::to_string_pretty(&env)?;
    fs::write(&envelope_path, &pretty)?;
    fs::write(&work_packet_path, &pretty)?;
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
    if let Ok(packet) = load_repair_packet(&req.repair_packet) {
        fs::write(&md, crate::generator::render_repair_markdown(&packet))?;
    }

    let (profile_path, profile_digest) = write_target_profile(&req.workspace, &req.target)?;
    let feedback_path = req.workspace.join("feedback.json");
    let work_packet_path = req.workspace.join("work-packet.json");
    let mut env = AgentEnvelope {
        schema_version: crate::harness::envelope::AGENT_ENVELOPE_SCHEMA_VERSION.to_owned(),
        mode: AgentMode::GeneratorRepair,
        target: req.target.clone(),
        abi: None,
        assembler: AssemblerFlavor::Nasm,
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
        remaining_attempts: None,
        latest_failure: packet
            .pointer("/failure/message")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        digests: AgentDigests {
            capability_snapshot: Some(CAPABILITY_SNAPSHOT_DIGEST.to_owned()),
            target_profile: Some(profile_digest),
            ..AgentDigests::default()
        },
        semasm_packet_path: None,
        repair_packet_path: Some(dest.display().to_string()),
        workspace_dir: Some(req.workspace.display().to_string()),
        prompt_markdown_path: md.exists().then(|| md.display().to_string()),
        events_path: None,
        allowed_operations: default_allowed_operations(),
        target_profile_path: Some(profile_path.display().to_string()),
        feedback_path: Some(feedback_path.display().to_string()),
        work_packet_path: None,
    };
    let envelope_path = req.workspace.join("agent-envelope.json");
    env.work_packet_path = Some(work_packet_path.display().to_string());
    let pretty = serde_json::to_string_pretty(&env)?;
    fs::write(&envelope_path, &pretty)?;
    fs::write(&work_packet_path, &pretty)?;
    Ok(env)
}

/// Submit a direct-assembly candidate through SemASM (optional ingest/seal).
pub fn submit_direct_nasm(req: &SubmitDirectRequest) -> Result<HarnessSubmitResult, HarnessError> {
    let locked = load_locked_task(&req.task).map_err(|e| HarnessError::Task(e.to_string()))?;
    let target = locked.task().target.clone();
    req.assembler
        .ensure_supported_for(&target)
        .map_err(HarnessError::Message)?;

    let source_bytes = fs::read(&req.source)?;
    let candidate_digest = sha256_digest_prefixed(&source_bytes);
    let feedback_dir = req.source.parent().map(Path::to_path_buf);

    // Seal path: create/open run dir and verify_candidate_and_seal.
    if req.run_dir.is_some() || req.run_base.is_some() {
        let mut result = submit_direct_with_seal(req, &locked, &source_bytes, &candidate_digest)?;
        persist_feedback(&mut result, feedback_dir.as_deref())?;
        return Ok(result);
    }

    let doctor = SemasmDoctor::run();
    let Some(binary) = doctor.binary_path.clone() else {
        let mut result = submit_result_from_failure(
            "TOOLCHAIN_INCOMPLETE",
            "semasm binary not found",
            Some(candidate_digest),
            None,
            req.assembler,
            req.allow_under_preconditions,
        );
        persist_feedback(&mut result, feedback_dir.as_deref())?;
        return Ok(result);
    };

    let mut result = match SemasmVerify::run_with_timeout(
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
            HarnessSubmitResult {
                schema_version: HARNESS_SUBMIT_SCHEMA_VERSION.to_owned(),
                class,
                next_action: next,
                evidence_status: evidence_status_label(report.outcome).to_owned(),
                raw_status: Some(report.raw_status),
                exit_code: exit as u8,
                message: format!(
                    "semasm status mapped to {}",
                    evidence_status_label(report.outcome)
                ),
                failure_code: None,
                candidate_digest: Some(candidate_digest),
                run_dir: None,
                run_id: None,
                candidate_index: None,
                candidate_dir: None,
                seal_digest: None,
                patch_evidence_path: None,
                assembler: Some(req.assembler.as_str().to_owned()),
                may_auto_retry: class.may_auto_retry(),
                failure: None,
                counterexample: None,
                candidate_delta: None,
                repair_focus: None,
                feedback_path: None,
            }
        }
        Err(err) => {
            let failure_code = err.failure_code().map(str::to_owned);
            let (class, next, exit) = classify_outcome(
                EvidenceStatus::Failed,
                None,
                failure_code.as_deref(),
                req.allow_under_preconditions,
            );
            HarnessSubmitResult {
                schema_version: HARNESS_SUBMIT_SCHEMA_VERSION.to_owned(),
                class,
                next_action: next,
                evidence_status: "failed".into(),
                raw_status: None,
                exit_code: exit as u8,
                message: err.to_string(),
                failure_code,
                candidate_digest: Some(candidate_digest),
                run_dir: None,
                run_id: None,
                candidate_index: None,
                candidate_dir: None,
                seal_digest: None,
                patch_evidence_path: None,
                assembler: Some(req.assembler.as_str().to_owned()),
                may_auto_retry: class.may_auto_retry(),
                failure: None,
                counterexample: None,
                candidate_delta: None,
                repair_focus: None,
                feedback_path: None,
            }
        }
    };
    persist_feedback(&mut result, feedback_dir.as_deref())?;
    Ok(result)
}

fn submit_direct_with_seal(
    req: &SubmitDirectRequest,
    locked: &crate::task::LockedTask,
    source_bytes: &[u8],
    candidate_digest: &str,
) -> Result<HarnessSubmitResult, HarnessError> {
    let (run_dir, run_id, created) = open_or_create_run_dir(req)?;
    let cursor = run_dir
        .resume_cursor()
        .map_err(|e| HarnessError::Message(e.to_string()))?;
    let max_attempts = locked.task().budgets.max_candidates.max(1);
    if cursor.next_candidate_index >= max_attempts {
        return Ok(submit_result_from_failure(
            "BUDGET_EXHAUSTED",
            "max_candidates exhausted; refuse to reseal",
            Some(candidate_digest.to_owned()),
            Some(run_dir.root().display().to_string()),
            req.assembler,
            req.allow_under_preconditions,
        ));
    }

    let (doctor, capability_match) = doctor_and_capabilities(locked);
    if doctor.binary_path.is_none() {
        return Ok(submit_result_from_failure(
            "TOOLCHAIN_INCOMPLETE",
            "semasm binary not found",
            Some(candidate_digest.to_owned()),
            Some(run_dir.root().display().to_string()),
            req.assembler,
            req.allow_under_preconditions,
        ));
    }

    let mut events = EventLog::open_existing(run_dir.event_log_path().to_path_buf());
    if created {
        let _ = events.record(EventKind::RunStarted {
            task_id: locked.task().task_id.clone(),
            task_digest: sha256_digest_prefixed(&fs::read(&req.task).unwrap_or_default()),
        });
    }
    let _ = events.record(EventKind::CandidateSubmitted {
        index: cursor.next_candidate_index,
        source_path: req.source.display().to_string(),
    });

    let mut protocol = CandidateProtocol::with_max(&locked.task().target, max_attempts);
    protocol.seed_resume(cursor.next_candidate_index);

    match verify_candidate_and_seal(VerifySealInput {
        locked,
        task_path: &req.task,
        contract_path: &req.contract,
        source_bytes,
        run_dir: &run_dir,
        run_id: run_id.clone(),
        protocol: &mut protocol,
        candidate_index: cursor.next_candidate_index,
        previous_seal_digest: cursor.previous_seal_digest.clone(),
        generator: GeneratorMeta::ingest(
            req.idempotency_key
                .clone()
                .unwrap_or_else(|| "harness-submit".into()),
        ),
        doctor,
        capability_match,
        allow_execution: req.allow_execution,
        assembler: req.assembler,
    }) {
        Ok(outcome) => {
            let raw = outcome.verify.as_ref().map(|v| v.raw_status.as_str());
            let (class, mut next, exit) = classify_outcome(
                outcome.evidence.final_status,
                raw,
                None,
                req.allow_under_preconditions,
            );
            if matches!(class, HarnessOutcomeClass::ViolatedRepairable) {
                next = HarnessNextAction::EditCandidate;
            }
            let _ = events.record(EventKind::VerificationCompleted {
                outcome: evidence_status_label(outcome.evidence.final_status).to_owned(),
            });
            let _ = events.record(EventKind::Info {
                message: format!(
                    "harness class={} seal={}",
                    class.as_str(),
                    outcome.seal.envelope_digest
                ),
            });
            Ok(HarnessSubmitResult {
                schema_version: HARNESS_SUBMIT_SCHEMA_VERSION.to_owned(),
                class,
                next_action: next,
                evidence_status: evidence_status_label(outcome.evidence.final_status).to_owned(),
                raw_status: raw.map(str::to_owned),
                exit_code: exit as u8,
                message: format!(
                    "sealed candidate {:04} as {}",
                    outcome.candidate_index,
                    evidence_status_label(outcome.evidence.final_status)
                ),
                failure_code: None,
                candidate_digest: Some(candidate_digest.to_owned()),
                run_dir: Some(run_dir.root().display().to_string()),
                run_id: Some(run_id),
                candidate_index: Some(outcome.candidate_index),
                candidate_dir: Some(outcome.candidate_dir.display().to_string()),
                seal_digest: Some(outcome.seal.envelope_digest),
                patch_evidence_path: None,
                assembler: Some(req.assembler.as_str().to_owned()),
                may_auto_retry: class.may_auto_retry(),
                failure: None,
                counterexample: None,
                candidate_delta: None,
                repair_focus: None,
                feedback_path: None,
            })
        }
        Err(err) => {
            let code = match &err {
                crate::run::VerifySealError::SemasmUnavailable => Some("TOOLCHAIN_INCOMPLETE"),
                crate::run::VerifySealError::CandidateAlreadySealed(_) => Some("ALREADY_SEALED"),
                _ => None,
            };
            let _ = events.record(EventKind::Error {
                message: err.to_string(),
            });
            Ok(submit_result_from_failure(
                code.unwrap_or("SEAL_FAILED"),
                &err.to_string(),
                Some(candidate_digest.to_owned()),
                Some(run_dir.root().display().to_string()),
                req.assembler,
                req.allow_under_preconditions,
            ))
        }
    }
}

fn open_or_create_run_dir(
    req: &SubmitDirectRequest,
) -> Result<(RunDir, String, bool), HarnessError> {
    if let Some(existing) = &req.run_dir {
        if existing.is_dir() {
            let run = RunDir::open(existing).map_err(|e| HarnessError::Message(e.to_string()))?;
            let id = existing
                .file_name()
                .map_or_else(|| "run".into(), |s| s.to_string_lossy().into_owned());
            return Ok((run, id, false));
        }
    }
    let base = req
        .run_base
        .clone()
        .or_else(|| {
            req.run_dir
                .as_ref()
                .and_then(|p| p.parent().map(Path::to_path_buf))
        })
        .unwrap_or_else(|| std::env::temp_dir().join("vaa-harness-runs"));
    fs::create_dir_all(&base)?;
    let run_id = RunId::generate();
    let run = RunDir::create(&base, &run_id).map_err(|e| HarnessError::Message(e.to_string()))?;
    Ok((run, run_id.to_string(), true))
}

/// Submit a generator repair: path policy → optional suite → patch evidence.
pub fn submit_generator_repair(
    req: &SubmitGeneratorRequest,
) -> Result<HarnessSubmitResult, HarnessError> {
    fs::create_dir_all(&req.workspace)?;
    let packet =
        load_repair_packet(&req.repair_packet).map_err(|e| HarnessError::Message(e.to_string()))?;

    let policy = PatchPolicy {
        allowed_paths: packet.repository.allowed_paths.clone(),
        forbidden_paths: packet.repository.forbidden_paths.clone(),
    };
    let violations = path_policy_violations(&req.changed_files, &policy);
    if !violations.is_empty() {
        let (class, next, exit) =
            classify_outcome(EvidenceStatus::Failed, None, Some("FORBIDDEN_PATH"), false);
        let mut result = HarnessSubmitResult {
            schema_version: HARNESS_SUBMIT_SCHEMA_VERSION.to_owned(),
            class,
            next_action: next,
            evidence_status: "failed".into(),
            raw_status: None,
            exit_code: exit as u8,
            message: format!(
                "authority/forbidden path mutation rejected: {}",
                violations.join(", ")
            ),
            failure_code: Some("FORBIDDEN_PATH".into()),
            candidate_digest: None,
            run_dir: req.run_base.as_ref().map(|p| p.display().to_string()),
            run_id: None,
            candidate_index: None,
            candidate_dir: None,
            seal_digest: None,
            patch_evidence_path: None,
            assembler: Some(AssemblerFlavor::Nasm.as_str().to_owned()),
            may_auto_retry: false,
            failure: None,
            counterexample: None,
            candidate_delta: None,
            repair_focus: None,
            feedback_path: None,
        };
        persist_feedback(&mut result, Some(req.workspace.as_path()))?;
        return Ok(result);
    }

    // Suite evidence: run live suite, or load prior suite-evidence JSON.
    let (suite_id, suite_digest, suite_status, binary_digest, run_root) = if let Some(suite_path) =
        &req.suite
    {
        let run_base = req
            .run_base
            .clone()
            .unwrap_or_else(|| req.workspace.join("suite-runs"));
        fs::create_dir_all(&run_base)?;
        let report = run_suite(&SuiteRunConfig {
            suite_path: suite_path.clone(),
            repo_override: req.repo.clone(),
            run_base: run_base.clone(),
            skip_repo_guard: req.skip_repo_guard,
            skip_build: req.skip_build,
            skip_verify: false,
            allow_execution: req.allow_execution,
            check_deterministic: false,
        })
        .map_err(|e| HarnessError::Message(e.to_string()))?;
        (
            report.evidence.suite_id,
            report.evidence.suite_digest,
            report.evidence.status,
            report.evidence.generator_binary_digest.unwrap_or_else(|| {
                req.generator_binary_digest
                    .clone()
                    .unwrap_or_else(|| "sha256:unknown".into())
            }),
            Some(run_base.display().to_string()),
        )
    } else if let Some(path) = &req.suite_evidence {
        let raw = fs::read_to_string(path)?;
        let v: Value = serde_json::from_str(&raw)?;
        let suite_id = v
            .get("suite_id")
            .and_then(|x| x.as_str())
            .unwrap_or("unknown")
            .to_owned();
        let suite_digest = v
            .get("suite_digest")
            .and_then(|x| x.as_str())
            .unwrap_or("sha256:00")
            .to_owned();
        let status_str = v.get("status").and_then(|x| x.as_str()).unwrap_or("failed");
        let suite_status = match status_str {
            "accepted" => SuiteStatus::Accepted,
            "rejected" => SuiteStatus::Rejected,
            "incomplete" => SuiteStatus::Incomplete,
            _ => SuiteStatus::Failed,
        };
        let binary = v
            .get("generator_binary_digest")
            .and_then(|x| x.as_str())
            .map(str::to_owned)
            .or_else(|| req.generator_binary_digest.clone())
            .unwrap_or_else(|| "sha256:unknown".into());
        (suite_id, suite_digest, suite_status, binary, None)
    } else {
        let mut result = HarnessSubmitResult {
            schema_version: HARNESS_SUBMIT_SCHEMA_VERSION.to_owned(),
            class: HarnessOutcomeClass::IncompleteCoverage,
            next_action: HarnessNextAction::Abort,
            evidence_status: "incomplete".into(),
            raw_status: None,
            exit_code: crate::exit_code::ExitCode::Incomplete as u8,
            message: "generator submit requires --suite or --suite-evidence for acceptance".into(),
            failure_code: Some("SUITE_REQUIRED".into()),
            candidate_digest: None,
            run_dir: None,
            run_id: None,
            candidate_index: None,
            candidate_dir: None,
            seal_digest: None,
            patch_evidence_path: None,
            assembler: Some(AssemblerFlavor::Nasm.as_str().to_owned()),
            may_auto_retry: false,
            failure: None,
            counterexample: None,
            candidate_delta: None,
            repair_focus: None,
            feedback_path: None,
        };
        persist_feedback(&mut result, Some(req.workspace.as_path()))?;
        return Ok(result);
    };

    let base_revision = req
        .base_revision
        .clone()
        .unwrap_or_else(|| packet.repository.base_revision.clone());
    let patch = build_patch_evidence(&PatchEvidenceInput {
        base_revision,
        patched_revision: req.patched_revision.clone(),
        changed_files: req.changed_files.clone(),
        patch_policy: policy,
        generator_binary_digest: binary_digest,
        generator_spec_digest: None,
        stack_lock_digest: None,
        suite_id,
        suite_digest,
        suite_status,
        patch_bytes: None,
    })
    .map_err(|e| HarnessError::Message(e.to_string()))?;

    let patch_path = req.workspace.join("patch-evidence.json");
    write_patch_evidence(&patch_path, &patch).map_err(|e| HarnessError::Message(e.to_string()))?;

    let (class, next, exit) = match patch.status {
        PatchStatus::Accepted => (
            HarnessOutcomeClass::Accepted,
            HarnessNextAction::Done,
            crate::exit_code::ExitCode::Success,
        ),
        PatchStatus::Rejected => (
            HarnessOutcomeClass::ViolatedRepairable,
            HarnessNextAction::EditGenerator,
            crate::exit_code::ExitCode::Violated,
        ),
        PatchStatus::Incomplete => (
            HarnessOutcomeClass::IncompleteCoverage,
            HarnessNextAction::Abort,
            crate::exit_code::ExitCode::Incomplete,
        ),
        PatchStatus::Failed => {
            if patch.forbidden_paths_changed.is_empty() {
                (
                    HarnessOutcomeClass::Failed,
                    HarnessNextAction::EditGenerator,
                    crate::exit_code::ExitCode::ToolFailure,
                )
            } else {
                (
                    HarnessOutcomeClass::PolicyBlocked,
                    HarnessNextAction::StopPolicy,
                    crate::exit_code::ExitCode::SecurityBlock,
                )
            }
        }
    };

    let mut result = HarnessSubmitResult {
        schema_version: HARNESS_SUBMIT_SCHEMA_VERSION.to_owned(),
        class,
        next_action: next,
        evidence_status: format!("{:?}", patch.status).to_ascii_lowercase(),
        raw_status: Some(format!("{suite_status:?}").to_ascii_lowercase()),
        exit_code: exit as u8,
        message: format!(
            "generator repair patch status={:?} suite={:?}",
            patch.status, suite_status
        ),
        failure_code: None,
        candidate_digest: None,
        run_dir: run_root,
        run_id: None,
        candidate_index: None,
        candidate_dir: None,
        seal_digest: None,
        patch_evidence_path: Some(patch_path.display().to_string()),
        assembler: Some(AssemblerFlavor::Nasm.as_str().to_owned()),
        may_auto_retry: class.may_auto_retry(),
        failure: None,
        counterexample: None,
        candidate_delta: None,
        repair_focus: None,
        feedback_path: None,
    };
    persist_feedback(&mut result, Some(req.workspace.as_path()))?;
    Ok(result)
}

/// Resume / status snapshot for an existing run directory.
pub fn resume_status(run_dir: &Path) -> Result<Value, HarnessError> {
    let run = RunDir::open(run_dir).map_err(|e| HarnessError::Message(e.to_string()))?;
    let cursor = scan_resume_cursor(&run).map_err(|e| HarnessError::Message(e.to_string()))?;
    let events_path = run.event_log_path().display().to_string();
    let evidence_dir = run.paths().evidence_dir.display().to_string();
    let last_events: Vec<Value> = fs::read_to_string(run.event_log_path())
        .unwrap_or_default()
        .lines()
        .rev()
        .take(5)
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    Ok(serde_json::json!({
        "schema_version": "0.1",
        "run_dir": run_dir.display().to_string(),
        "next_candidate_index": cursor.next_candidate_index,
        "previous_seal_digest": cursor.previous_seal_digest,
        "event_count": cursor.event_count,
        "events_path": events_path,
        "evidence_dir": evidence_dir,
        "recent_events": last_events,
        "artifacts": {
            "events_jsonl": events_path,
            "evidence_dir": evidence_dir,
        }
    }))
}

fn submit_result_from_failure(
    code: &str,
    message: &str,
    candidate_digest: Option<String>,
    run_dir: Option<String>,
    assembler: AssemblerFlavor,
    allow_under_preconditions: bool,
) -> HarnessSubmitResult {
    let (class, next, exit) = classify_outcome(
        EvidenceStatus::Failed,
        None,
        Some(code),
        allow_under_preconditions,
    );
    HarnessSubmitResult {
        schema_version: HARNESS_SUBMIT_SCHEMA_VERSION.to_owned(),
        class,
        next_action: next,
        evidence_status: "failed".into(),
        raw_status: None,
        exit_code: exit as u8,
        message: message.to_owned(),
        failure_code: Some(code.to_owned()),
        candidate_digest,
        run_dir,
        run_id: None,
        candidate_index: None,
        candidate_dir: None,
        seal_digest: None,
        patch_evidence_path: None,
        assembler: Some(assembler.as_str().to_owned()),
        may_auto_retry: class.may_auto_retry(),
        failure: None,
        counterexample: None,
        candidate_delta: None,
        repair_focus: None,
        feedback_path: None,
    }
}

fn persist_feedback(
    result: &mut HarnessSubmitResult,
    dir: Option<&Path>,
) -> Result<(), HarnessError> {
    enrich_repair_feedback(result);
    let Some(dir) = dir else {
        return Ok(());
    };
    fs::create_dir_all(dir)?;
    let path = dir.join("feedback.json");
    fs::write(&path, serde_json::to_string_pretty(result)?)?;
    result.feedback_path = Some(path.display().to_string());
    // Re-write so the on-disk copy includes feedback_path.
    fs::write(&path, serde_json::to_string_pretty(result)?)?;
    Ok(())
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
        "# Direct assembly harness task `{task}`\n\n\
Target: `{target}`\n\
Mode: `{mode}`\n\
Assembler: `{assembler}` (gas supported for aarch64/riscv64; x86_64 remains nasm)\n\
Doctor: `{doctor:?}`\n\
Remaining attempts: {remaining}\n\n\
## Writable\n\n{writable}\n\n\
## Forbidden\n\n{forbidden}\n\n\
## Verify\n\n```text\n{verify}\n```\n\n\
## Rules\n\n\
- Edit only writable paths.\n\
- Never edit `*.vaa.toml`, `*.sem.toml`, or `stack.lock.toml`.\n\
- Success requires SemASM `verified` (or allowed `verified_under_preconditions`).\n\
- `incomplete` / `execution_denied` is not success.\n\
- Fb9c arbitrary loop invariants stay locked.\n\
- Do not claim unsupported assembler/target pairings (gas is aarch64/riscv64 only).\n",
        task = env.task_id,
        target = env.target,
        mode = env.mode.as_str(),
        assembler = env.assembler.as_str(),
        doctor = doctor_status,
        remaining = env
            .remaining_attempts
            .map_or_else(|| "unknown".into(), |n| n.to_string()),
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
    std::env::var_os(ENV_SEMASM_BIN)
        .map(PathBuf::from)
        .or_else(|| {
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
