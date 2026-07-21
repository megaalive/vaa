//! Sealed evidence envelope — integrity digests, not cryptographic attestation.
//!
//! See [`docs/seal.md`](../../docs/seal.md). Schema **0.2** separates
//! `acceptance_digest` (technical truth) from `envelope_digest` (includes provenance).

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::canonical_json::{canonical_json_bytes, CANONICALIZATION_ID, DIGEST_ALGORITHM_ID};

use super::report::{sha256_digest_prefixed, CheckOutcome, EvidenceExpect, EvidenceReport};
use super::status::EvidenceStatus;

/// Seal schema version (acceptance / envelope split).
pub const SEAL_SCHEMA_VERSION: &str = "0.2";

/// Untrusted attribution for who proposed the candidate (not part of acceptance).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneratorMeta {
    /// Source kind (`fixture`, `ingest`, `external`, …).
    pub kind: String,
    /// Human / tool name.
    pub name: String,
    /// Optional generation id from a model adapter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_id: Option<String>,
}

impl GeneratorMeta {
    #[must_use]
    pub fn ingest(name: impl Into<String>) -> Self {
        Self {
            kind: "ingest".to_owned(),
            name: name.into(),
            generation_id: None,
        }
    }

    #[must_use]
    pub fn fixture(name: impl Into<String>, generation_id: Option<String>) -> Self {
        Self {
            kind: "fixture".to_owned(),
            name: name.into(),
            generation_id,
        }
    }
}

/// Technical acceptance body — hashed into [`SealEnvelope::acceptance_digest`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AcceptanceBody {
    pub task_digest: String,
    pub target: String,
    pub contract_digest: String,
    pub source_digest: String,
    /// Digest of SemASM report bytes, or literal `none`.
    pub semasm_report_digest: String,
    pub final_status: EvidenceStatus,
    pub checks: Vec<CheckOutcome>,
}

/// Run / generator provenance — hashed into [`SealEnvelope::envelope_digest`] only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvenanceBody {
    pub task_id: String,
    pub run_id: Option<String>,
    pub generator: GeneratorMeta,
    pub candidate_index: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_seal_digest: Option<String>,
}

/// On-disk seal envelope (schema 0.2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SealEnvelope {
    pub schema_version: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub acceptance_digest: String,
    pub envelope_digest: String,
    pub acceptance: AcceptanceBody,
    pub provenance: ProvenanceBody,
}

#[derive(Debug, Serialize)]
struct EnvelopeHashBody<'a> {
    acceptance: &'a AcceptanceBody,
    provenance: &'a ProvenanceBody,
}

#[derive(Debug, thiserror::Error)]
pub enum SealError {
    #[error("io: {0}")]
    Io(String),
    #[error("json: {0}")]
    Json(String),
    #[error("acceptance digest mismatch")]
    AcceptanceDigestMismatch,
    #[error("envelope digest mismatch")]
    EnvelopeDigestMismatch,
    #[error("unsupported seal schema: {0}")]
    UnsupportedSchema(String),
    #[error("evidence does not match seal payload: {0}")]
    EvidenceMismatch(String),
    #[error("bundle: {0}")]
    Bundle(String),
}

/// Inputs for building a seal envelope.
#[derive(Debug, Clone)]
pub struct SealBuildInput {
    pub candidate_index: u32,
    pub previous_seal_digest: Option<String>,
    pub generator: GeneratorMeta,
}

/// Build acceptance + provenance + digests from a finished evidence report.
#[must_use]
pub fn build_seal_envelope(
    report: &EvidenceReport,
    expect: &EvidenceExpect,
    input: SealBuildInput,
) -> SealEnvelope {
    let semasm_report_digest = report.verify_report.as_ref().map_or_else(
        || "none".to_owned(),
        |vr| sha256_digest_prefixed(vr.raw_json.as_bytes()),
    );

    let mut checks = report.checks.clone();
    checks.sort_by(|a, b| a.check_name.cmp(&b.check_name));

    let acceptance = AcceptanceBody {
        task_digest: report.task_digest.clone(),
        target: report.target.clone(),
        contract_digest: expect.expected_contract_digest.clone(),
        source_digest: expect.expected_source_digest.clone(),
        semasm_report_digest,
        final_status: report.final_status,
        checks,
    };
    let provenance = ProvenanceBody {
        task_id: report.task_id.clone(),
        run_id: report.run_id.clone(),
        generator: input.generator,
        candidate_index: input.candidate_index,
        previous_seal_digest: input.previous_seal_digest,
    };

    seal_envelope(acceptance, provenance)
}

/// SHA-256 over canonical JSON of the acceptance body.
#[must_use]
pub fn acceptance_digest_of(acceptance: &AcceptanceBody) -> String {
    sha256_digest_prefixed(&canonical_json_bytes(acceptance))
}

/// SHA-256 over canonical JSON of `{acceptance, provenance}`.
#[must_use]
pub fn envelope_digest_of(acceptance: &AcceptanceBody, provenance: &ProvenanceBody) -> String {
    let body = EnvelopeHashBody {
        acceptance,
        provenance,
    };
    sha256_digest_prefixed(&canonical_json_bytes(&body))
}

/// Wrap acceptance + provenance with both digests.
#[must_use]
pub fn seal_envelope(acceptance: AcceptanceBody, provenance: ProvenanceBody) -> SealEnvelope {
    let acceptance_digest = acceptance_digest_of(&acceptance);
    let envelope_digest = envelope_digest_of(&acceptance, &provenance);
    SealEnvelope {
        schema_version: SEAL_SCHEMA_VERSION.to_owned(),
        canonicalization: CANONICALIZATION_ID.to_owned(),
        digest_algorithm: DIGEST_ALGORITHM_ID.to_owned(),
        acceptance_digest,
        envelope_digest,
        acceptance,
        provenance,
    }
}

/// Atomically write `evidence.json` + `evidence.seal.json` (seal rename last).
pub fn write_sealed_evidence(
    evidence_dir: &Path,
    report: &EvidenceReport,
    expect: &EvidenceExpect,
    input: SealBuildInput,
) -> Result<SealEnvelope, SealError> {
    fs::create_dir_all(evidence_dir).map_err(|e| SealError::Io(e.to_string()))?;

    let envelope = build_seal_envelope(report, expect, input);
    let evidence_body =
        serde_json::to_string_pretty(report).map_err(|e| SealError::Json(e.to_string()))?;
    let seal_body =
        serde_json::to_string_pretty(&envelope).map_err(|e| SealError::Json(e.to_string()))?;

    atomic_write_pair(
        &evidence_dir.join("evidence.json"),
        evidence_body.as_bytes(),
        &evidence_dir.join("evidence.seal.json"),
        seal_body.as_bytes(),
    )?;

    Ok(envelope)
}

/// Write final acceptance markers under `evidence/` (copies of the latest pair).
pub fn write_final_sealed_evidence(
    evidence_dir: &Path,
    report: &EvidenceReport,
    envelope: &SealEnvelope,
) -> Result<(), SealError> {
    fs::create_dir_all(evidence_dir).map_err(|e| SealError::Io(e.to_string()))?;
    let evidence_body =
        serde_json::to_string_pretty(report).map_err(|e| SealError::Json(e.to_string()))?;
    let seal_body =
        serde_json::to_string_pretty(envelope).map_err(|e| SealError::Json(e.to_string()))?;
    atomic_write_pair(
        &evidence_dir.join("final.json"),
        evidence_body.as_bytes(),
        &evidence_dir.join("final.seal.json"),
        seal_body.as_bytes(),
    )
}

/// Verify seal digests and cross-check against `evidence.json`.
pub fn verify_seal(evidence_path: &Path, seal_path: &Path) -> Result<(), SealError> {
    let seal_raw = fs::read_to_string(seal_path).map_err(|e| SealError::Io(e.to_string()))?;
    let envelope: SealEnvelope =
        serde_json::from_str(&seal_raw).map_err(|e| SealError::Json(e.to_string()))?;

    if envelope.schema_version != SEAL_SCHEMA_VERSION {
        return Err(SealError::UnsupportedSchema(envelope.schema_version));
    }
    if envelope.canonicalization != CANONICALIZATION_ID {
        return Err(SealError::Bundle(format!(
            "canonicalization {}",
            envelope.canonicalization
        )));
    }
    if envelope.digest_algorithm != DIGEST_ALGORITHM_ID {
        return Err(SealError::Bundle(format!(
            "digest_algorithm {}",
            envelope.digest_algorithm
        )));
    }

    let acceptance_digest = acceptance_digest_of(&envelope.acceptance);
    if acceptance_digest != envelope.acceptance_digest {
        return Err(SealError::AcceptanceDigestMismatch);
    }
    let envelope_digest = envelope_digest_of(&envelope.acceptance, &envelope.provenance);
    if envelope_digest != envelope.envelope_digest {
        return Err(SealError::EnvelopeDigestMismatch);
    }

    let evidence_raw =
        fs::read_to_string(evidence_path).map_err(|e| SealError::Io(e.to_string()))?;
    let report: EvidenceReport =
        serde_json::from_str(&evidence_raw).map_err(|e| SealError::Json(e.to_string()))?;

    cross_check_evidence(&report, &envelope)
}

fn cross_check_evidence(report: &EvidenceReport, envelope: &SealEnvelope) -> Result<(), SealError> {
    let acceptance = &envelope.acceptance;
    let provenance = &envelope.provenance;

    if report.task_id != provenance.task_id {
        return Err(SealError::EvidenceMismatch("task_id".into()));
    }
    if report.task_digest != acceptance.task_digest {
        return Err(SealError::EvidenceMismatch("task_digest".into()));
    }
    if report.target != acceptance.target {
        return Err(SealError::EvidenceMismatch("target".into()));
    }
    if report.run_id != provenance.run_id {
        return Err(SealError::EvidenceMismatch("run_id".into()));
    }
    if report.final_status != acceptance.final_status {
        return Err(SealError::EvidenceMismatch("final_status".into()));
    }

    let mut report_checks = report.checks.clone();
    report_checks.sort_by(|a, b| a.check_name.cmp(&b.check_name));
    if report_checks.len() != acceptance.checks.len() {
        return Err(SealError::EvidenceMismatch("checks length".into()));
    }
    for (a, b) in report_checks.iter().zip(acceptance.checks.iter()) {
        if a.check_name != b.check_name || a.passed != b.passed || a.required != b.required {
            return Err(SealError::EvidenceMismatch(format!(
                "check {}",
                a.check_name
            )));
        }
    }

    if let Some(vr) = &report.verify_report {
        let report_digest = sha256_digest_prefixed(vr.raw_json.as_bytes());
        if report_digest != acceptance.semasm_report_digest {
            return Err(SealError::EvidenceMismatch("semasm_report_digest".into()));
        }
        if vr.source_digest.as_deref() != Some(acceptance.source_digest.as_str()) {
            return Err(SealError::EvidenceMismatch("source_digest".into()));
        }
        if vr.contract_digest.as_deref() != Some(acceptance.contract_digest.as_str()) {
            return Err(SealError::EvidenceMismatch("contract_digest".into()));
        }
    } else if acceptance.semasm_report_digest != "none" {
        return Err(SealError::EvidenceMismatch(
            "semasm_report_digest expected none".into(),
        ));
    }

    Ok(())
}

/// Write evidence then seal using tmp + fsync + rename; seal rename is the commit marker.
fn atomic_write_pair(
    evidence_path: &Path,
    evidence_bytes: &[u8],
    seal_path: &Path,
    seal_bytes: &[u8],
) -> Result<(), SealError> {
    let evidence_tmp = tmp_sibling(evidence_path);
    let seal_tmp = tmp_sibling(seal_path);

    write_tmp_fsync(&evidence_tmp, evidence_bytes)?;
    write_tmp_fsync(&seal_tmp, seal_bytes)?;

    // Replace destinations if present (Windows rename fails when dest exists).
    let _ = fs::remove_file(evidence_path);
    fs::rename(&evidence_tmp, evidence_path).map_err(|e| SealError::Io(e.to_string()))?;

    let _ = fs::remove_file(seal_path);
    fs::rename(&seal_tmp, seal_path).map_err(|e| SealError::Io(e.to_string()))?;

    Ok(())
}

fn tmp_sibling(path: &Path) -> PathBuf {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("file");
    path.with_file_name(format!("{name}.tmp"))
}

fn write_tmp_fsync(path: &Path, bytes: &[u8]) -> Result<(), SealError> {
    let mut file = File::create(path).map_err(|e| SealError::Io(e.to_string()))?;
    file.write_all(bytes)
        .map_err(|e| SealError::Io(e.to_string()))?;
    file.flush().map_err(|e| SealError::Io(e.to_string()))?;
    file.sync_all().map_err(|e| SealError::Io(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::EvidenceAggregator;
    use crate::semasm::capabilities::CapabilityMatch;
    use crate::semasm::doctor::{DoctorReport, DoctorStatus, SemasmVersion};
    use crate::semasm::verify::VerifyReport;
    use crate::task::load_locked_task;

    fn sample_report() -> (EvidenceReport, EvidenceExpect) {
        let task = load_locked_task(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/tasks/sum_i64.vaa.toml"),
        )
        .expect("task");
        let expect = EvidenceExpect::new(
            task.task().target.clone(),
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        let verify = VerifyReport {
            outcome: EvidenceStatus::Incomplete,
            raw_status: "execution_denied".to_owned(),
            schema_version: Some("0.4".to_owned()),
            diagnostics: vec![],
            target: Some(task.task().target.clone()),
            source_digest: Some(expect.expected_source_digest.clone()),
            contract_digest: Some(expect.expected_contract_digest.clone()),
            tool_version: Some("semasm 0.1.0".to_owned()),
            raw_json: r#"{"status":"execution_denied"}"#.to_owned(),
        };
        let report = EvidenceAggregator::build(
            &task,
            Some("run-1".to_owned()),
            Some(verify),
            Some(DoctorReport {
                status: DoctorStatus::Available,
                binary_path: Some(PathBuf::from("semasm")),
                version: Some(SemasmVersion {
                    version: "0.1.0".to_owned(),
                    schema_version: "0.1".to_owned(),
                }),
                details: vec![],
            }),
            Some(CapabilityMatch {
                compatible: true,
                missing: vec![],
                insufficient: vec![],
            }),
            &expect,
        );
        (report, expect)
    }

    fn build_input(generator: GeneratorMeta) -> SealBuildInput {
        SealBuildInput {
            candidate_index: 0,
            previous_seal_digest: None,
            generator,
        }
    }

    #[test]
    fn acceptance_digest_stable_across_provenance() {
        let (report, expect) = sample_report();
        let a = build_seal_envelope(
            &report,
            &expect,
            SealBuildInput {
                candidate_index: 0,
                previous_seal_digest: None,
                generator: GeneratorMeta::ingest("unit"),
            },
        );
        let mut report2 = report.clone();
        report2.run_id = Some("run-2".to_owned());
        let b = build_seal_envelope(
            &report2,
            &expect,
            SealBuildInput {
                candidate_index: 1,
                previous_seal_digest: Some(a.envelope_digest.clone()),
                generator: GeneratorMeta::ingest("other"),
            },
        );
        assert_eq!(a.acceptance_digest, b.acceptance_digest);
        assert_ne!(a.envelope_digest, b.envelope_digest);
    }

    #[test]
    fn check_seal_rejects_final_status_tamper() {
        let (mut report, expect) = sample_report();
        let dir = tempfile_dir("vaa_seal_ok");
        write_sealed_evidence(
            &dir,
            &report,
            &expect,
            build_input(GeneratorMeta::ingest("unit")),
        )
        .unwrap();

        report.final_status = EvidenceStatus::Verified;
        fs::write(
            dir.join("evidence.json"),
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .unwrap();

        let err = verify_seal(&dir.join("evidence.json"), &dir.join("evidence.seal.json"))
            .expect_err("tamper detected");
        assert!(matches!(
            err,
            SealError::EvidenceMismatch(_)
                | SealError::AcceptanceDigestMismatch
                | SealError::EnvelopeDigestMismatch
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn check_seal_rejects_source_digest_tamper_in_seal() {
        let (report, expect) = sample_report();
        let dir = tempfile_dir("vaa_seal_src");
        let mut envelope = write_sealed_evidence(
            &dir,
            &report,
            &expect,
            build_input(GeneratorMeta::ingest("unit")),
        )
        .unwrap();
        envelope.acceptance.source_digest =
            "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_owned();
        envelope.acceptance_digest = acceptance_digest_of(&envelope.acceptance);
        envelope.envelope_digest = envelope_digest_of(&envelope.acceptance, &envelope.provenance);
        fs::write(
            dir.join("evidence.seal.json"),
            serde_json::to_string_pretty(&envelope).unwrap(),
        )
        .unwrap();

        let err = verify_seal(&dir.join("evidence.json"), &dir.join("evidence.seal.json"))
            .expect_err("source mismatch");
        assert!(matches!(err, SealError::EvidenceMismatch(_)));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn round_trip_seal_verifies() {
        let (report, expect) = sample_report();
        let dir = tempfile_dir("vaa_seal_round");
        write_sealed_evidence(
            &dir,
            &report,
            &expect,
            build_input(GeneratorMeta::fixture("fixture", None)),
        )
        .unwrap();
        verify_seal(&dir.join("evidence.json"), &dir.join("evidence.seal.json")).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_write_creates_final_names_not_tmp() {
        let (report, expect) = sample_report();
        let dir = tempfile_dir("vaa_seal_atomic");
        write_sealed_evidence(
            &dir,
            &report,
            &expect,
            build_input(GeneratorMeta::ingest("unit")),
        )
        .unwrap();
        assert!(dir.join("evidence.json").exists());
        assert!(dir.join("evidence.seal.json").exists());
        assert!(!dir.join("evidence.json.tmp").exists());
        assert!(!dir.join("evidence.seal.json.tmp").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    fn tempfile_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "{prefix}_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
