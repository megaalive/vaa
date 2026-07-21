//! SemASM `agent verify` adapter: stdout-only VerificationReport 0.4 parse.

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::evidence::{schema_version_compatible, EvidenceStatus};
use crate::process::{ProcessConfig, ProcessError, ProcessRunner};

/// Optional diagnostic entry when present in older/fictional payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemasmDiagnostic {
    pub code: Option<String>,
    pub severity: Option<String>,
    pub message: String,
    pub location: Option<String>,
}

/// Tolerant subset of SemASM [`VerificationReport`] schema 0.4.
///
/// Unknown nested fields (`semantic`, `behavior`, `behavior_oracle`, …) are
/// ignored by serde so the adapter stays compatible with additive report growth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyReportRaw {
    pub status: String,
    #[serde(default)]
    pub schema_version: Option<String>,
    #[serde(default)]
    pub diagnostics: Vec<SemasmDiagnostic>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub contract_digest: Option<String>,
    #[serde(default)]
    pub source_digest: Option<String>,
    #[serde(default)]
    pub tool_version: Option<String>,
}

/// Mapped verification report for the evidence aggregator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyReport {
    pub outcome: EvidenceStatus,
    pub raw_status: String,
    pub schema_version: Option<String>,
    pub diagnostics: Vec<SemasmDiagnostic>,
    pub target: Option<String>,
    pub source_digest: Option<String>,
    pub contract_digest: Option<String>,
    pub tool_version: Option<String>,
    pub raw_json: String,
}

/// Errors from invoking or parsing SemASM verify.
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("semasm binary not found")]
    BinaryNotFound,
    #[error("verification process failed: {0}")]
    ProcessFailed(String),
    #[error("failed to parse verification output: {0}")]
    ParseFailed(String),
    #[error("verification timed out")]
    Timeout,
}

/// Subprocess adapter for `semasm agent verify --format json`.
pub struct SemasmVerify;

impl SemasmVerify {
    /// Run SemASM verify via [`ProcessRunner`] and parse JSON from **stdout only**.
    pub fn run(
        source: &Path,
        contract: &Path,
        binary: &Path,
        target: &str,
        allow_execution: bool,
    ) -> Result<VerifyReport, VerifyError> {
        let mut args = vec![
            "agent".to_owned(),
            "verify".to_owned(),
            source.to_string_lossy().into_owned(),
            contract.to_string_lossy().into_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            "--target".to_owned(),
            target.to_owned(),
        ];
        if allow_execution {
            args.push("--allow-execution".to_owned());
        }
        let config = ProcessConfig {
            program: binary.to_path_buf(),
            args,
            timeout: Duration::from_secs(120),
            max_output_bytes: 4 * 1_048_576,
            ..ProcessConfig::default()
        };

        let output = ProcessRunner::run(&config).map_err(|e| match e {
            ProcessError::Timeout { .. } => VerifyError::Timeout,
            ProcessError::OutputOverflow { limit } => {
                VerifyError::ProcessFailed(format!("output exceeded {limit} bytes"))
            }
            ProcessError::Spawn { detail, .. } => VerifyError::ProcessFailed(detail),
        })?;

        let stdout = output.stdout;
        let stderr = output.stderr;

        if stdout.trim().is_empty() {
            return Err(VerifyError::ParseFailed(format!(
                "empty stdout from semasm (no VerificationReport); stderr={stderr}"
            )));
        }

        Self::parse_report(&stdout).map_err(|err| match err {
            VerifyError::ParseFailed(msg) => {
                VerifyError::ParseFailed(format!("{msg}; stderr={stderr}"))
            }
            other => other,
        })
    }

    /// Parse a SemASM VerificationReport JSON document (stdout body only).
    pub fn parse_report(json: &str) -> Result<VerifyReport, VerifyError> {
        let raw: VerifyReportRaw =
            serde_json::from_str(json).map_err(|e| VerifyError::ParseFailed(e.to_string()))?;

        Self::check_schema_version(raw.schema_version.as_deref())?;

        let outcome = Self::map_status(&raw.status);

        Ok(VerifyReport {
            outcome,
            raw_status: raw.status,
            schema_version: raw.schema_version,
            diagnostics: raw.diagnostics,
            target: raw.target,
            source_digest: raw.source_digest,
            contract_digest: raw.contract_digest,
            tool_version: raw.tool_version,
            raw_json: json.to_owned(),
        })
    }

    /// Accept only VerificationReport schemas in `[0.4, 0.5)`.
    fn check_schema_version(version: Option<&str>) -> Result<(), VerifyError> {
        let Some(version) = version else {
            return Err(VerifyError::ParseFailed(
                "missing schema_version (required >=0.4,<0.5)".to_owned(),
            ));
        };
        if !schema_version_compatible(version) {
            return Err(VerifyError::ParseFailed(format!(
                "unsupported VerificationReport schema_version `{version}` (accepted >=0.4,<0.5)"
            )));
        }
        Ok(())
    }

    /// Map SemASM `VerificationReport.status` to VAA 4-outcome vocabulary.
    fn map_status(status: &str) -> EvidenceStatus {
        match status {
            "verified" => EvidenceStatus::Verified,
            "semantic_failed" | "executable_failed" | "behavior_failed" => EvidenceStatus::Violated,
            "execution_denied" => EvidenceStatus::Incomplete,
            _ => EvidenceStatus::Failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal(status: &str) -> String {
        format!(
            r#"{{
                "schema_version": "0.4",
                "status": "{status}",
                "target": "x86_64-unknown-linux-gnu",
                "tool_version": "semasm 0.1.0",
                "contract_digest": "sha256:{}",
                "source_digest": "sha256:{}"
            }}"#,
            "a".repeat(64),
            "b".repeat(64)
        )
    }

    #[test]
    fn parse_verified_report() {
        let report = SemasmVerify::parse_report(&minimal("verified")).expect("parse");
        assert_eq!(report.outcome, EvidenceStatus::Verified);
        assert_eq!(report.target.as_deref(), Some("x86_64-unknown-linux-gnu"));
        assert_eq!(report.tool_version.as_deref(), Some("semasm 0.1.0"));
        assert_eq!(report.schema_version.as_deref(), Some("0.4"));
    }

    #[test]
    fn parse_semantic_failed_maps_to_violated() {
        let report = SemasmVerify::parse_report(&minimal("semantic_failed")).expect("parse");
        assert_eq!(report.outcome, EvidenceStatus::Violated);
    }

    #[test]
    fn parse_executable_failed_maps_to_violated() {
        let report = SemasmVerify::parse_report(&minimal("executable_failed")).expect("parse");
        assert_eq!(report.outcome, EvidenceStatus::Violated);
    }

    #[test]
    fn parse_behavior_failed_maps_to_violated() {
        let report = SemasmVerify::parse_report(&minimal("behavior_failed")).expect("parse");
        assert_eq!(report.outcome, EvidenceStatus::Violated);
    }

    #[test]
    fn parse_execution_denied_maps_to_incomplete() {
        let report = SemasmVerify::parse_report(&minimal("execution_denied")).expect("parse");
        assert_eq!(report.outcome, EvidenceStatus::Incomplete);
    }

    #[test]
    fn unknown_status_maps_to_failed() {
        let report = SemasmVerify::parse_report(&minimal("unknown_thing")).expect("parse");
        assert_eq!(report.outcome, EvidenceStatus::Failed);
    }

    #[test]
    fn major_schema_mismatch_is_failed_closed() {
        let json = r#"{
            "schema_version": "1.0",
            "status": "verified"
        }"#;
        let err = SemasmVerify::parse_report(json).expect_err("major 1 rejected");
        assert!(matches!(err, VerifyError::ParseFailed(_)));
    }

    #[test]
    fn schema_0_3_rejected() {
        let json = r#"{"schema_version":"0.3","status":"verified"}"#;
        assert!(SemasmVerify::parse_report(json).is_err());
    }

    #[test]
    fn missing_schema_rejected() {
        let json = r#"{"status":"verified"}"#;
        assert!(SemasmVerify::parse_report(json).is_err());
    }

    #[test]
    fn golden_execution_denied_report_deserializes() {
        let json = include_str!(
            "../../fixtures/semasm/reports/verification-report-count_byte.execution_denied.json"
        );
        let report = SemasmVerify::parse_report(json).expect("golden parse");
        assert_eq!(report.outcome, EvidenceStatus::Incomplete);
        assert_eq!(report.raw_status, "execution_denied");
        assert!(report
            .tool_version
            .as_deref()
            .is_some_and(|v| v.starts_with("semasm ")));
        assert!(report
            .contract_digest
            .as_deref()
            .is_some_and(|v| v.starts_with("sha256:") && v.len() == 7 + 64));
        assert!(report
            .source_digest
            .as_deref()
            .is_some_and(|v| v.starts_with("sha256:") && v.len() == 7 + 64));
    }

    #[test]
    fn stderr_noise_must_not_be_concatenated_for_parse() {
        let stdout = minimal("execution_denied");
        let with_stderr_prefix = format!("execution denied: human message\n{stdout}");
        assert!(SemasmVerify::parse_report(&with_stderr_prefix).is_err());
        assert!(SemasmVerify::parse_report(&stdout).is_ok());
    }

    #[test]
    fn malformed_json_returns_error() {
        let result = SemasmVerify::parse_report("not json");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerifyError::ParseFailed(_)));
    }

    #[test]
    fn missing_status_returns_error() {
        let result = SemasmVerify::parse_report(r#"{"schema_version":"0.4"}"#);
        assert!(matches!(result.unwrap_err(), VerifyError::ParseFailed(_)));
    }
}
