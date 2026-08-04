//! Shared SemASM verify + sealed evidence write path (run and ingest).

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::candidate::CandidateProtocol;
use crate::evidence::{
    append_seal_log, materialize_bundle_files, sha256_digest_prefixed, write_final_sealed_evidence,
    write_sealed_evidence, EvidenceAggregator, EvidenceExpect, EvidenceReport, GeneratorMeta,
    ObjectInspectionOutcome, SealBuildInput, SealEnvelope, SealLogEntry,
};
use crate::inspect::ArtifactInspector;
use crate::process::{ProcessConfig, ProcessRunner};
use crate::run::RunDir;
use crate::semasm::{
    match_task_requirements, CapabilityMatch, DoctorReport, SemasmDoctor, SemasmVerify,
    TargetCapabilities, VerifyError, VerifyReport,
};
use crate::task::LockedTask;
use std::time::Duration;

#[derive(Serialize)]
struct ExternalVectorDocument<'a> {
    schema_version: &'static str,
    contract_digest: &'a str,
    target: &'a str,
    routine_symbol: &'a str,
    cases: Vec<ExternalVectorCase<'a>>,
}

#[derive(Serialize)]
struct ExternalVectorCase<'a> {
    id: &'a str,
    inputs: std::collections::BTreeMap<&'a str, serde_json::Value>,
}

/// Outcome of verifying one candidate and sealing evidence.
#[derive(Debug)]
pub struct VerifySealOutcome {
    pub evidence: EvidenceReport,
    pub seal: SealEnvelope,
    pub source_digest: String,
    pub contract_digest: String,
    pub verify: Option<VerifyReport>,
    /// When SemASM returned `agent_failure` instead of a VerificationReport.
    pub agent_failure_code: Option<String>,
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
    /// Assembler used for object-inspect (NASM x86_64 / GAS AArch64|RV).
    pub assembler: crate::harness::AssemblerFlavor,
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
    // UTF-8 BOM breaks NASM ("label '' alone on a line"); normalize before seal.
    let source_bytes = strip_utf8_bom(input.source_bytes);
    let source_digest = sha256_digest_prefixed(source_bytes);
    let source_text = std::str::from_utf8(source_bytes)
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

    let source_path = cand_dir.join(input.assembler.candidate_filename());
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

    let vectors = build_external_vectors(input.locked, &contract_digest, &target)?;
    let vectors_path = vectors.as_ref().map(|(path, _)| path.as_path());
    let expected_vectors_digest = vectors.as_ref().map(|(_, digest)| digest.as_str());
    let mut agent_failure_code = None;
    let verify_result = if let Some(vectors_path) = vectors_path {
        SemasmVerify::run_with_vectors(
            &source_path,
            input.contract_path,
            binary,
            &target,
            input.allow_execution,
            vectors_path,
        )
    } else {
        SemasmVerify::run(
            &source_path,
            input.contract_path,
            binary,
            &target,
            input.allow_execution,
        )
    };
    if let Some(vectors_path) = vectors_path {
        let _ = std::fs::remove_file(vectors_path);
    }
    let verify = match verify_result {
        Ok(mut report) => {
            if let Some(expected_digest) = expected_vectors_digest {
                enforce_task_vector_evidence(&mut report, input.locked, expected_digest);
            }
            Some(report)
        }
        Err(VerifyError::BinaryNotFound) => return Err(VerifySealError::SemasmUnavailable),
        Err(e) => {
            // Do not invent a VerificationReport. Keep the SemASM failure code so
            // harness classification can treat candidate assemble failures as
            // repairable instead of a silent "verification report missing".
            agent_failure_code = e.failure_code().map(str::to_owned);
            eprintln!("warning: semasm agent verify failed: {e}");
            None
        }
    };

    let mut expect = EvidenceExpect::new(
        target.clone(),
        source_digest.clone(),
        contract_digest.clone(),
    );
    if input.locked.task().verification.require_object_inspection {
        expect.object_inspection = Some(assemble_and_inspect_with(
            &source_path,
            &cand_dir,
            &target,
            input.assembler,
        ));
    }
    if input.locked.task().verification.require_reproducible_build {
        let (matched, details) = crate::build::reproducible_build_check(&source_path, &target);
        expect.reproducible_build =
            Some(crate::evidence::ReproducibleBuildOutcome { matched, details });
    }

    let evidence = EvidenceAggregator::build(
        input.locked,
        Some(input.run_id.clone()),
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

    let log_entry = SealLogEntry::from_seal(
        &input.run_id,
        &input.locked.task().task_id,
        evidence.final_status,
        &seal,
    );
    append_seal_log(&input.run_dir.paths().evidence_dir, &log_entry)
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
        agent_failure_code,
        candidate_index: outcome.index,
        candidate_dir: cand_dir,
    })
}

/// Materialize the input-only SemASM vector document for schema 0.2 tasks.
pub fn build_external_vectors(
    locked: &LockedTask,
    contract_digest: &str,
    target: &str,
) -> Result<Option<(PathBuf, String)>, VerifySealError> {
    let task = locked.task();
    if task.schema_version != "0.2" || task.tests.is_empty() {
        return Ok(None);
    }
    let cases = task
        .tests
        .iter()
        .map(|test| ExternalVectorCase {
            id: &test.name,
            inputs: test
                .input
                .iter()
                .map(|(name, value)| (name.as_str(), toml_value_to_json(value)))
                .collect(),
        })
        .collect();
    let document = ExternalVectorDocument {
        schema_version: "0.1",
        contract_digest,
        target,
        routine_symbol: &task.entry.symbol,
        cases,
    };
    let canonical = crate::canonical_json::canonical_json_bytes(&document);
    let digest = sha256_digest_prefixed(&canonical);
    let path = std::env::temp_dir().join(format!(
        "vaa-vectors-{}-{}.json",
        std::process::id(),
        &digest[7..19]
    ));
    let pretty = serde_json::to_vec_pretty(&document)
        .map_err(|error| VerifySealError::Io(format!("serialize external vectors: {error}")))?;
    std::fs::write(&path, pretty)
        .map_err(|error| VerifySealError::Io(format!("write external vectors: {error}")))?;
    Ok(Some((path, digest)))
}

fn toml_value_to_json(value: &crate::task::TomlValue) -> serde_json::Value {
    match value {
        crate::task::TomlValue::Null => serde_json::Value::Null,
        crate::task::TomlValue::Bool(value) => serde_json::json!(value),
        crate::task::TomlValue::Integer(value) => serde_json::json!(value),
        crate::task::TomlValue::String(value) => serde_json::json!(value),
        crate::task::TomlValue::Array(values) => {
            serde_json::Value::Array(values.iter().map(toml_value_to_json).collect())
        }
    }
}

/// Fail the mapped report when schema 0.2 vector evidence is incomplete or mismatched.
pub fn enforce_task_vector_evidence(
    report: &mut VerifyReport,
    locked: &LockedTask,
    expected_digest: &str,
) {
    let task = locked.task();
    let raw = serde_json::from_str::<crate::semasm::verify::VerifyReportRaw>(&report.raw_json).ok();
    let valid = raw
        .as_ref()
        .and_then(|raw| raw.vector_set.as_ref())
        .is_some_and(|set| {
            set.external_document_digest.as_deref() == Some(expected_digest)
                && set.external_case_count == task.tests.len()
                && task.tests.iter().all(|test| {
                    let binding = set.cases.iter().find(|case| {
                        case.origin == "external"
                            && case.external_case_id.as_deref() == Some(test.name.as_str())
                    });
                    let expected = match test.expected {
                        crate::task::TomlValue::Integer(value) => value.to_string(),
                        _ => return false,
                    };
                    binding.is_some_and(|binding| {
                        raw.as_ref()
                            .and_then(|raw| raw.behavior.as_ref())
                            .is_some_and(|behavior| {
                                behavior.cases.iter().any(|case| {
                                    case.name == binding.name
                                        && case.expected == expected
                                        && case.passed
                                })
                            })
                    })
                })
        });
    if !valid {
        report.outcome = crate::evidence::EvidenceStatus::Failed;
        report
            .diagnostics
            .push(crate::semasm::verify::SemasmDiagnostic {
                code: Some("TASK_VECTOR_EVIDENCE_MISMATCH".into()),
                severity: Some("error".into()),
                message: "SemASM external-vector evidence does not match locked schema 0.2 tests"
                    .into(),
                location: None,
            });
    }
}

/// Drop a leading UTF-8 BOM (`U+FEFF`) if present.
#[must_use]
pub fn strip_utf8_bom(bytes: &[u8]) -> &[u8] {
    match bytes {
        [0xEF, 0xBB, 0xBF, rest @ ..] => rest,
        other => other,
    }
}

/// Convenience: doctor + capability snapshot for a locked task.
#[must_use]
pub fn doctor_and_capabilities(locked: &LockedTask) -> (DoctorReport, CapabilityMatch) {
    let target = &locked.task().target;
    let caps = TargetCapabilities::for_target(target);
    let cm = match_task_requirements(locked.task(), &caps);
    (SemasmDoctor::run(), cm)
}

/// Assemble candidate to `.o` and run [`ArtifactInspector`] (I0).
#[must_use]
pub fn assemble_and_inspect(
    source_path: &Path,
    out_dir: &Path,
    target: &str,
) -> ObjectInspectionOutcome {
    assemble_and_inspect_with(
        source_path,
        out_dir,
        target,
        crate::harness::AssemblerFlavor::Nasm,
    )
}

/// Assemble with an explicit [`crate::harness::AssemblerFlavor`].
#[must_use]
pub fn assemble_and_inspect_with(
    source_path: &Path,
    out_dir: &Path,
    target: &str,
    assembler: crate::harness::AssemblerFlavor,
) -> ObjectInspectionOutcome {
    let object_path = out_dir.join("candidate.o");
    let program = assembler.default_program(target);
    let args = assembler.assemble_args(source_path, &object_path, target);
    let tool = assembler.as_str();
    let cfg = ProcessConfig {
        program,
        args,
        timeout: Duration::from_secs(60),
        max_output_bytes: 1_048_576,
        ..ProcessConfig::default()
    };
    match ProcessRunner::run(&cfg) {
        Ok(out) if out.exit_code == Some(0) => match ArtifactInspector::inspect(&object_path) {
            Ok(info) => ObjectInspectionOutcome {
                error: None,
                has_wxorx: info.has_wxorx,
                has_executable_stack: info.has_executable_stack,
                format: info.format,
            },
            Err(e) => ObjectInspectionOutcome {
                error: Some(format!("inspect failed: {e}")),
                has_wxorx: false,
                has_executable_stack: false,
                format: "unknown".into(),
            },
        },
        Ok(out) => ObjectInspectionOutcome {
            error: Some(format!(
                "{tool} failed code={:?} stderr={}",
                out.exit_code, out.stderr
            )),
            has_wxorx: false,
            has_executable_stack: false,
            format: "none".into(),
        },
        Err(e) => ObjectInspectionOutcome {
            error: Some(format!("{tool} invoke failed: {e}")),
            has_wxorx: false,
            has_executable_stack: false,
            format: "none".into(),
        },
    }
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
        assembler: crate::harness::AssemblerFlavor::Nasm,
    })
}

#[cfg(test)]
mod tests {
    use super::strip_utf8_bom;

    #[test]
    fn strip_utf8_bom_drops_prefix() {
        let with = b"\xEF\xBB\xBF; hello";
        assert_eq!(strip_utf8_bom(with), b"; hello");
        assert_eq!(strip_utf8_bom(b"; hello"), b"; hello");
        assert_eq!(strip_utf8_bom(b""), b"");
    }
}
