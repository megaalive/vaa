//! Sealed evidence envelope — acceptance digests that generators cannot rewrite.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::report::{sha256_digest_prefixed, CheckOutcome, EvidenceExpect, EvidenceReport};
use super::status::EvidenceStatus;

/// Seal schema for the deterministic payload.
pub const SEAL_SCHEMA_VERSION: &str = "0.1";

/// Untrusted attribution for who proposed the candidate (not part of acceptance logic).
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

/// Deterministic seal body (no volatile timestamp).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SealPayloadV01 {
    pub schema_version: String,
    pub task_id: String,
    pub task_digest: String,
    pub target: String,
    pub run_id: Option<String>,
    pub contract_digest: String,
    pub source_digest: String,
    /// Digest of SemASM `raw_json`, or literal `none`.
    pub semasm_report_digest: String,
    pub final_status: EvidenceStatus,
    pub checks: Vec<CheckOutcome>,
    pub generator: GeneratorMeta,
}

/// On-disk seal envelope: payload + its digest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SealEnvelope {
    pub schema_version: String,
    pub seal_digest: String,
    pub payload: SealPayloadV01,
}

#[derive(Debug, thiserror::Error)]
pub enum SealError {
    #[error("io: {0}")]
    Io(String),
    #[error("json: {0}")]
    Json(String),
    #[error("seal digest mismatch")]
    DigestMismatch,
    #[error("evidence does not match seal payload: {0}")]
    EvidenceMismatch(String),
}

/// Build a seal payload from a finished evidence report and identity expect.
#[must_use]
pub fn build_seal_payload(
    report: &EvidenceReport,
    expect: &EvidenceExpect,
    generator: GeneratorMeta,
) -> SealPayloadV01 {
    let semasm_report_digest = report.verify_report.as_ref().map_or_else(
        || "none".to_owned(),
        |vr| sha256_digest_prefixed(vr.raw_json.as_bytes()),
    );

    let mut checks = report.checks.clone();
    checks.sort_by(|a, b| a.check_name.cmp(&b.check_name));

    SealPayloadV01 {
        schema_version: SEAL_SCHEMA_VERSION.to_owned(),
        task_id: report.task_id.clone(),
        task_digest: report.task_digest.clone(),
        target: report.target.clone(),
        run_id: report.run_id.clone(),
        contract_digest: expect.expected_contract_digest.clone(),
        source_digest: expect.expected_source_digest.clone(),
        semasm_report_digest,
        final_status: report.final_status,
        checks,
        generator,
    }
}

/// SHA-256 over canonical JSON of the payload (`sha256:` + hex).
#[must_use]
pub fn seal_digest_of(payload: &SealPayloadV01) -> String {
    let canonical = canonical_json_bytes(payload);
    sha256_digest_prefixed(&canonical)
}

/// Wrap payload with its digest.
#[must_use]
pub fn seal_envelope(payload: SealPayloadV01) -> SealEnvelope {
    let seal_digest = seal_digest_of(&payload);
    SealEnvelope {
        schema_version: SEAL_SCHEMA_VERSION.to_owned(),
        seal_digest,
        payload,
    }
}

/// Write `evidence.json` + `evidence.seal.json` under `evidence_dir`.
pub fn write_sealed_evidence(
    evidence_dir: &Path,
    report: &EvidenceReport,
    expect: &EvidenceExpect,
    generator: GeneratorMeta,
) -> Result<SealEnvelope, SealError> {
    fs::create_dir_all(evidence_dir).map_err(|e| SealError::Io(e.to_string()))?;

    let evidence_path = evidence_dir.join("evidence.json");
    let seal_path = evidence_dir.join("evidence.seal.json");

    let body = serde_json::to_string_pretty(report).map_err(|e| SealError::Json(e.to_string()))?;
    fs::write(&evidence_path, body).map_err(|e| SealError::Io(e.to_string()))?;

    let envelope = seal_envelope(build_seal_payload(report, expect, generator));
    let seal_body =
        serde_json::to_string_pretty(&envelope).map_err(|e| SealError::Json(e.to_string()))?;
    fs::write(&seal_path, seal_body).map_err(|e| SealError::Io(e.to_string()))?;

    Ok(envelope)
}

/// Verify seal digest integrity and cross-check against `evidence.json`.
pub fn verify_seal(evidence_path: &Path, seal_path: &Path) -> Result<(), SealError> {
    let seal_raw = fs::read_to_string(seal_path).map_err(|e| SealError::Io(e.to_string()))?;
    let envelope: SealEnvelope =
        serde_json::from_str(&seal_raw).map_err(|e| SealError::Json(e.to_string()))?;

    let recomputed = seal_digest_of(&envelope.payload);
    if recomputed != envelope.seal_digest {
        return Err(SealError::DigestMismatch);
    }

    let evidence_raw =
        fs::read_to_string(evidence_path).map_err(|e| SealError::Io(e.to_string()))?;
    let report: EvidenceReport =
        serde_json::from_str(&evidence_raw).map_err(|e| SealError::Json(e.to_string()))?;

    cross_check_evidence(&report, &envelope.payload)
}

fn cross_check_evidence(
    report: &EvidenceReport,
    payload: &SealPayloadV01,
) -> Result<(), SealError> {
    if report.task_id != payload.task_id {
        return Err(SealError::EvidenceMismatch("task_id".into()));
    }
    if report.task_digest != payload.task_digest {
        return Err(SealError::EvidenceMismatch("task_digest".into()));
    }
    if report.target != payload.target {
        return Err(SealError::EvidenceMismatch("target".into()));
    }
    if report.run_id != payload.run_id {
        return Err(SealError::EvidenceMismatch("run_id".into()));
    }
    if report.final_status != payload.final_status {
        return Err(SealError::EvidenceMismatch("final_status".into()));
    }

    let mut report_checks = report.checks.clone();
    report_checks.sort_by(|a, b| a.check_name.cmp(&b.check_name));
    if report_checks.len() != payload.checks.len() {
        return Err(SealError::EvidenceMismatch("checks length".into()));
    }
    for (a, b) in report_checks.iter().zip(payload.checks.iter()) {
        if a.check_name != b.check_name || a.passed != b.passed || a.required != b.required {
            return Err(SealError::EvidenceMismatch(format!(
                "check {}",
                a.check_name
            )));
        }
    }

    if let Some(vr) = &report.verify_report {
        let report_digest = sha256_digest_prefixed(vr.raw_json.as_bytes());
        if report_digest != payload.semasm_report_digest {
            return Err(SealError::EvidenceMismatch("semasm_report_digest".into()));
        }
        if vr.source_digest.as_deref() != Some(payload.source_digest.as_str()) {
            return Err(SealError::EvidenceMismatch("source_digest".into()));
        }
        if vr.contract_digest.as_deref() != Some(payload.contract_digest.as_str()) {
            return Err(SealError::EvidenceMismatch("contract_digest".into()));
        }
    } else if payload.semasm_report_digest != "none" {
        return Err(SealError::EvidenceMismatch(
            "semasm_report_digest expected none".into(),
        ));
    }

    Ok(())
}

fn canonical_json_bytes<T: Serialize>(value: &T) -> Vec<u8> {
    let value = serde_json::to_value(value).expect("serialize");
    let canonical = sort_value(value);
    serde_json::to_vec(&canonical).expect("canonical JSON")
}

fn sort_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort_unstable();
            let mut out = serde_json::Map::new();
            for key in keys {
                let child = map.get(&key).cloned().expect("key exists");
                out.insert(key, sort_value(child));
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(sort_value).collect())
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semasm::capabilities::CapabilityMatch;
    use crate::semasm::doctor::{DoctorReport, DoctorStatus, SemasmVersion};
    use crate::semasm::verify::VerifyReport;
    use crate::task::load_locked_task;
    use std::path::PathBuf;

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

    use crate::evidence::EvidenceAggregator;

    #[test]
    fn seal_digest_is_stable() {
        let (report, expect) = sample_report();
        let gen = GeneratorMeta::ingest("unit");
        let a = seal_digest_of(&build_seal_payload(&report, &expect, gen.clone()));
        let b = seal_digest_of(&build_seal_payload(&report, &expect, gen));
        assert_eq!(a, b);
        assert!(a.starts_with("sha256:"));
    }

    #[test]
    fn check_seal_rejects_final_status_tamper() {
        let (mut report, expect) = sample_report();
        let dir = tempfile_dir("vaa_seal_ok");
        write_sealed_evidence(&dir, &report, &expect, GeneratorMeta::ingest("unit")).unwrap();

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
            SealError::EvidenceMismatch(_) | SealError::DigestMismatch
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn check_seal_rejects_source_digest_tamper_in_seal() {
        let (report, expect) = sample_report();
        let dir = tempfile_dir("vaa_seal_src");
        let mut envelope =
            write_sealed_evidence(&dir, &report, &expect, GeneratorMeta::ingest("unit")).unwrap();
        envelope.payload.source_digest =
            "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_owned();
        // Recompute digest so envelope is self-consistent but mismatches evidence digests
        envelope.seal_digest = seal_digest_of(&envelope.payload);
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
            GeneratorMeta::fixture("fixture", None),
        )
        .unwrap();
        verify_seal(&dir.join("evidence.json"), &dir.join("evidence.seal.json")).unwrap();
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
