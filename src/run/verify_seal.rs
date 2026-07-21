//! Shared SemASM verify + sealed evidence write path (run and ingest).

use std::path::{Path, PathBuf};

use crate::candidate::CandidateProtocol;
use crate::evidence::{
    materialize_bundle_files, sha256_digest_prefixed, write_final_sealed_evidence,
    write_sealed_evidence, EvidenceAggregator, EvidenceExpect, EvidenceReport, GeneratorMeta,
    SealBuildInput, SealEnvelope,
};
use crate::run::RunDir;
use crate::semasm::{
    match_task_requirements, CapabilityMatch, DoctorReport, SemasmDoctor, SemasmVerify,
    TargetCapabilities, VerifyError, VerifyReport,
};
use crate::task::LockedTask;

/// Outcome of verifying one candidate and sealing evidence.
#[derive(Debug)]
pub struct VerifySealOutcome {
    pub evidence: EvidenceReport,
    pub seal: SealEnvelope,
    pub source_digest: String,
    pub contract_digest: String,
    pub verify: Option<VerifyReport>,
    pub candidate_index: u32,
    pub candidate_dir: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum VerifySealError {
    #[error("io: {0}")]
    Io(String),
    #[error("candidate rejected: {0}")]
    Candidate(String),
    #[error("candidate already sealed: {0}")]
    CandidateAlreadySealed(String),
    #[error("semasm unavailable")]
    SemasmUnavailable,
    #[error("seal: {0}")]
    Seal(String),
    #[error("run directory: {0}")]
    RunDir(String),
}

/// Inputs for a single generator-agnostic verify+seal.
pub struct VerifySealInput<'a> {
    pub locked: &'a LockedTask,
    pub task_path: &'a Path,
    pub contract_path: &'a Path,
    pub source_bytes: &'a [u8],
    pub run_dir: &'a RunDir,
    pub run_id: String,
    pub protocol: &'a mut CandidateProtocol,
    pub candidate_index: u32,
    pub previous_seal_digest: Option<String>,
    pub generator: GeneratorMeta,
    pub doctor: DoctorReport,
    pub capability_match: CapabilityMatch,
    pub allow_execution: bool,
}

/// Submit candidate, run SemASM verify, aggregate evidence, write per-candidate seal + final.
pub fn verify_candidate_and_seal(
    input: VerifySealInput<'_>,
) -> Result<VerifySealOutcome, VerifySealError> {
    let target = input.locked.task().target.clone();
    let contract_bytes = std::fs::read(input.contract_path)
        .map_err(|e| VerifySealError::Io(format!("read contract: {e}")))?;
    let task_bytes = std::fs::read(input.task_path)
        .map_err(|e| VerifySealError::Io(format!("read task: {e}")))?;
    let contract_digest = sha256_digest_prefixed(&contract_bytes);
    let source_digest = sha256_digest_prefixed(input.source_bytes);
    let source_text = std::str::from_utf8(input.source_bytes)
        .map_err(|e| VerifySealError::Io(format!("source utf-8: {e}")))?
        .to_owned();

    let cand_dir = input
        .run_dir
        .create_candidate_dir(input.candidate_index)
        .map_err(|e| match e {
            crate::run::RunDirError::CandidateAlreadySealed { index, path } => {
                VerifySealError::CandidateAlreadySealed(format!("{index:04} at {}", path.display()))
            }
            other => VerifySealError::RunDir(other.to_string()),
        })?;

    let source_path = cand_dir.join("candidate.asm");
    input
        .run_dir
        .write_new_file(&source_path, source_text.as_bytes())
        .map_err(|e| VerifySealError::Io(e.to_string()))?;

    let outcome = input.protocol.submit(&source_text, &source_path, &target);
    if !outcome.accepted {
        return Err(VerifySealError::Candidate(format!(
            "{:?}",
            outcome.rejection
        )));
    }

    let binary = input
        .doctor
        .binary_path
        .as_ref()
        .ok_or(VerifySealError::SemasmUnavailable)?;

    let verify = match SemasmVerify::run(
        &source_path,
        input.contract_path,
        binary,
        &target,
        input.allow_execution,
    ) {
        Ok(report) => Some(report),
        Err(VerifyError::BinaryNotFound) => return Err(VerifySealError::SemasmUnavailable),
        Err(_) => None,
    };

    let expect = EvidenceExpect::new(target, source_digest.clone(), contract_digest.clone());
    let evidence = EvidenceAggregator::build(
        input.locked,
        Some(input.run_id),
        verify.clone(),
        Some(input.doctor),
        Some(input.capability_match),
        &expect,
    );

    let report_raw = verify.as_ref().map(|v| v.raw_json.as_str());
    materialize_bundle_files(&cand_dir, &task_bytes, &contract_bytes, report_raw)
        .map_err(|e| VerifySealError::Seal(e.to_string()))?;

    let seal = write_sealed_evidence(
        &cand_dir,
        &evidence,
        &expect,
        SealBuildInput {
            candidate_index: outcome.index,
            previous_seal_digest: input.previous_seal_digest,
            generator: input.generator,
        },
    )
    .map_err(|e| VerifySealError::Seal(e.to_string()))?;

    write_final_sealed_evidence(&input.run_dir.paths().evidence_dir, &evidence, &seal)
        .map_err(|e| VerifySealError::Seal(e.to_string()))?;

    // Best-effort append-only hardening after successful seal.
    let _ = input.run_dir.seal_candidate_readonly(&cand_dir);

    Ok(VerifySealOutcome {
        evidence,
        seal,
        source_digest,
        contract_digest,
        verify,
        candidate_index: outcome.index,
        candidate_dir: cand_dir,
    })
}

/// Convenience: doctor + capability snapshot for a locked task.
#[must_use]
pub fn doctor_and_capabilities(locked: &LockedTask) -> (DoctorReport, CapabilityMatch) {
    let target = &locked.task().target;
    let caps = TargetCapabilities::for_target(target);
    let cm = match_task_requirements(locked.task(), &caps);
    (SemasmDoctor::run(), cm)
}

/// Ingest a single external candidate (no model adapter).
#[allow(clippy::too_many_arguments)]
pub fn ingest_candidate(
    locked: &LockedTask,
    task_path: &Path,
    contract_path: &Path,
    source_path: &Path,
    run_dir: &RunDir,
    run_id: &str,
    generator_name: &str,
    max_attempts: u32,
    allow_execution: bool,
) -> Result<VerifySealOutcome, VerifySealError> {
    let source_bytes =
        std::fs::read(source_path).map_err(|e| VerifySealError::Io(format!("read source: {e}")))?;
    let (doctor, cm) = doctor_and_capabilities(locked);
    if doctor.binary_path.is_none() {
        return Err(VerifySealError::SemasmUnavailable);
    }
    let mut protocol = CandidateProtocol::with_max(&locked.task().target, max_attempts);
    verify_candidate_and_seal(VerifySealInput {
        locked,
        task_path,
        contract_path,
        source_bytes: &source_bytes,
        run_dir,
        run_id: run_id.to_owned(),
        protocol: &mut protocol,
        candidate_index: 0,
        previous_seal_digest: None,
        generator: GeneratorMeta::ingest(generator_name),
        doctor,
        capability_match: cm,
        allow_execution,
    })
}
