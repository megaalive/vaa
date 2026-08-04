//! SemASM `agent verify` adapter: stdout-only VerificationReport / agent_failure parse (0.4+).

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

/// Tolerant subset of SemASM [`VerificationReport`] schema 0.4+.
///
/// Unknown nested fields (`semantic`, `behavior`, `alias_analysis`, …) are
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
    #[serde(default)]
    pub vector_set: Option<SemasmVectorSet>,
    #[serde(default)]
    pub behavior: Option<SemasmBehavior>,
}

/// SemASM 0.6 vector-set binding used to prove task cases were included.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemasmVectorSet {
    pub external_document_digest: Option<String>,
    pub builtin_case_count: usize,
    pub external_case_count: usize,
    pub cases: Vec<SemasmVectorCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemasmVectorCase {
    pub name: String,
    pub origin: String,
    pub external_case_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemasmBehavior {
    pub cases: Vec<SemasmBehaviorCase>,
    pub all_passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemasmBehaviorCase {
    pub name: String,
    pub passed: bool,
    pub expected: String,
    pub observed: String,
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
    /// Structured early failure from SemASM (stdout JSON, not a VerificationReport).
    #[error("semasm agent_failure {code}: {message}")]
    AgentFailure {
        code: String,
        message: String,
        stage: Option<String>,
        retryability: Option<String>,
        raw_json: String,
    },
}

impl VerifyError {
    /// Stable failure code when available.
    #[must_use]
    pub fn failure_code(&self) -> Option<&str> {
        match self {
            Self::AgentFailure { code, .. } => Some(code.as_str()),
            Self::Timeout => Some("TIMEOUT"),
            _ => None,
        }
    }
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
        Self::run_with_timeout(source, contract, binary, target, allow_execution, 120)
    }

    /// Run verify with an additive SemASM external-vector document.
    pub fn run_with_vectors(
        source: &Path,
        contract: &Path,
        binary: &Path,
        target: &str,
        allow_execution: bool,
        vectors_file: &Path,
    ) -> Result<VerifyReport, VerifyError> {
        Self::run_with_timeout_and_vectors(
            source,
            contract,
            binary,
            target,
            allow_execution,
            120,
            Some(vectors_file),
        )
    }

    /// Like [`Self::run`] with an explicit subprocess timeout (seconds).
    pub fn run_with_timeout(
        source: &Path,
        contract: &Path,
        binary: &Path,
        target: &str,
        allow_execution: bool,
        timeout_secs: u64,
    ) -> Result<VerifyReport, VerifyError> {
        Self::run_with_timeout_and_vectors(
            source,
            contract,
            binary,
            target,
            allow_execution,
            timeout_secs,
            None,
        )
    }

    fn run_with_timeout_and_vectors(
        source: &Path,
        contract: &Path,
        binary: &Path,
        target: &str,
        allow_execution: bool,
        timeout_secs: u64,
        vectors_file: Option<&Path>,
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
        if let Some(vectors_file) = vectors_file {
            args.push("--vectors-file".to_owned());
            args.push(vectors_file.to_string_lossy().into_owned());
        }
        let config = ProcessConfig {
            program: binary.to_path_buf(),
            args,
            timeout: Duration::from_secs(timeout_secs.max(1)),
            max_output_bytes: 4 * 1_048_576,
            // TEMP/TMP + Windows roots required; PATH/HOME/USER alone fails scratch dir.
            allowed_env: crate::semasm::doctor::semasm_subprocess_allowed_env(),
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

    /// Run SemASM verify through [`crate::sandbox::ExecutionSandbox`] (LocalBackend).
    ///
    /// Sets up the Gate-2 isolation wire (I2). LocalBackend is a process wrapper,
    /// not container isolation (C-012). Fail-closed when sandbox cannot run.
    pub fn run_sandboxed(
        source: &Path,
        contract: &Path,
        binary: &Path,
        target: &str,
        allow_execution: bool,
    ) -> Result<VerifyReport, VerifyError> {
        Self::run_sandboxed_with_optional_vectors(
            source,
            contract,
            binary,
            target,
            allow_execution,
            None,
        )
    }

    /// Sandboxed verify with an additive external-vector document.
    pub fn run_sandboxed_with_vectors(
        source: &Path,
        contract: &Path,
        binary: &Path,
        target: &str,
        allow_execution: bool,
        vectors_file: &Path,
    ) -> Result<VerifyReport, VerifyError> {
        Self::run_sandboxed_with_optional_vectors(
            source,
            contract,
            binary,
            target,
            allow_execution,
            Some(vectors_file),
        )
    }

    fn run_sandboxed_with_optional_vectors(
        source: &Path,
        contract: &Path,
        binary: &Path,
        target: &str,
        allow_execution: bool,
        vectors_file: Option<&Path>,
    ) -> Result<VerifyReport, VerifyError> {
        use crate::sandbox::exec::ExecutionError;
        use crate::sandbox::{ExecutionSandbox, LocalBackend};

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
        if let Some(vectors_file) = vectors_file {
            args.push("--vectors-file".to_owned());
            args.push(vectors_file.to_string_lossy().into_owned());
        }

        let mut sandbox = ExecutionSandbox::new(Box::new(LocalBackend));
        sandbox.enable();
        let result = sandbox
            .run(binary, &args, Duration::from_secs(120))
            .map_err(|e| match e {
                ExecutionError::NotEnabled => {
                    VerifyError::ProcessFailed("execution sandbox not enabled".into())
                }
                ExecutionError::SandboxUnavailable(name) => {
                    VerifyError::ProcessFailed(format!("sandbox unavailable: {name}"))
                }
                ExecutionError::BinaryNotFound(p) => {
                    VerifyError::ProcessFailed(format!("semasm binary not found in sandbox: {p}"))
                }
                ExecutionError::ProcessError(ProcessError::Timeout { .. }) => VerifyError::Timeout,
                ExecutionError::ProcessError(other) => {
                    VerifyError::ProcessFailed(other.to_string())
                }
            })?;

        if result.timed_out {
            return Err(VerifyError::Timeout);
        }

        let stdout = result.stdout;
        let stderr = result.stderr;
        if stdout.trim().is_empty() {
            return Err(VerifyError::ParseFailed(format!(
                "empty stdout from sandboxed semasm; stderr={stderr}"
            )));
        }
        Self::parse_report(&stdout).map_err(|err| match err {
            VerifyError::ParseFailed(msg) => {
                VerifyError::ParseFailed(format!("{msg}; stderr={stderr}"))
            }
            other => other,
        })
    }

    /// Parse a SemASM VerificationReport **or** agent_failure envelope (stdout body only).
    pub fn parse_report(json: &str) -> Result<VerifyReport, VerifyError> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|e| VerifyError::ParseFailed(e.to_string()))?;

        if value.get("kind").and_then(|k| k.as_str()) == Some("agent_failure") {
            let code = value
                .get("code")
                .and_then(|c| c.as_str())
                .unwrap_or("UNKNOWN")
                .to_owned();
            let message = value
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("agent_failure")
                .to_owned();
            let stage = value
                .get("stage")
                .and_then(|s| s.as_str())
                .map(str::to_owned);
            let retryability = value
                .get("retryability")
                .and_then(|s| s.as_str())
                .map(str::to_owned);
            return Err(VerifyError::AgentFailure {
                code,
                message,
                stage,
                retryability,
                raw_json: json.to_owned(),
            });
        }

        let raw: VerifyReportRaw =
            serde_json::from_value(value).map_err(|e| VerifyError::ParseFailed(e.to_string()))?;

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

    /// Accept VerificationReport schemas in `[0.4, 0.7)`.
    fn check_schema_version(version: Option<&str>) -> Result<(), VerifyError> {
        let Some(version) = version else {
            return Err(VerifyError::ParseFailed(
                "missing schema_version (required >=0.4,<0.7)".to_owned(),
            ));
        };
        if !schema_version_compatible(version) {
            return Err(VerifyError::ParseFailed(format!(
                "unsupported VerificationReport schema_version `{version}` (accepted >=0.4,<0.7)"
            )));
        }
        Ok(())
    }

    /// Map SemASM `VerificationReport.status` to VAA evidence vocabulary.
    fn map_status(status: &str) -> EvidenceStatus {
        match status {
            "verified" => EvidenceStatus::Verified,
            "verified_under_preconditions" => EvidenceStatus::VerifiedUnderPreconditions,
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
    fn parse_verified_under_preconditions_maps_distinct_status() {
        let json = minimal("verified_under_preconditions").replace("0.4", "0.5");
        let report = SemasmVerify::parse_report(&json).expect("parse");
        assert_eq!(report.outcome, EvidenceStatus::VerifiedUnderPreconditions);
        assert_ne!(report.outcome, EvidenceStatus::Verified);
        assert_eq!(report.schema_version.as_deref(), Some("0.5"));
    }

    #[test]
    fn parses_schema_0_6_external_vector_binding() {
        let json = format!(
            r#"{{
                "schema_version":"0.6",
                "status":"verified",
                "vector_set":{{
                    "external_document_digest":"sha256:{}",
                    "builtin_case_count":5,
                    "external_case_count":1,
                    "cases":[{{"name":"external:four","origin":"external","external_case_id":"four"}}]
                }},
                "behavior":{{
                    "all_passed":true,
                    "cases":[{{"name":"external:four","passed":true,"expected":"10","observed":"10"}}]
                }}
            }}"#,
            "c".repeat(64)
        );
        let report = SemasmVerify::parse_report(&json).expect("schema 0.6 report");
        let raw: VerifyReportRaw = serde_json::from_str(&report.raw_json).unwrap();
        let set = raw.vector_set.expect("vector set");
        assert_eq!(set.external_case_count, 1);
        assert_eq!(set.cases[0].external_case_id.as_deref(), Some("four"));
        assert_eq!(raw.behavior.expect("behavior").cases[0].expected, "10");
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
    fn golden_sum_i64_execution_denied_report_deserializes() {
        let json = include_str!(
            "../../fixtures/semasm/reports/verification-report-sum_i64.execution_denied.json"
        );
        let report = SemasmVerify::parse_report(json).expect("sum_i64 golden parse");
        assert_eq!(report.outcome, EvidenceStatus::Incomplete);
        assert_eq!(report.raw_status, "execution_denied");
        let raw: serde_json::Value =
            serde_json::from_str(&report.raw_json).expect("raw_json is JSON");
        assert_eq!(
            raw["behavior_oracle"]["id"],
            "builtin.buffer.wrapping_sum_i64"
        );
        assert_eq!(raw["behavior_oracle"]["version"], 2);
        assert_eq!(raw["behavior_oracle"]["proof_basis"], "oracle_and_vectors");
        assert_eq!(
            raw["behavior_oracle"]["contract_ensures"],
            serde_json::json!(["true"])
        );
    }

    #[test]
    fn golden_sum_i64_verified_report_deserializes() {
        let json =
            include_str!("../../fixtures/semasm/reports/verification-report-sum_i64.verified.json");
        let report = SemasmVerify::parse_report(json).expect("sum_i64 verified golden parse");
        assert_eq!(report.outcome, EvidenceStatus::Verified);
        assert_eq!(report.raw_status, "verified");
        let raw: serde_json::Value =
            serde_json::from_str(&report.raw_json).expect("raw_json is JSON");
        assert_eq!(
            raw["behavior_oracle"]["id"],
            "builtin.buffer.wrapping_sum_i64"
        );
        assert_eq!(raw["behavior_oracle"]["version"], 2);
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

    #[test]
    fn parse_agent_failure_envelope() {
        let json = r#"{
            "schema_version": "0.1",
            "kind": "agent_failure",
            "code": "UNSUPPORTED_SHAPE",
            "stage": "unsupported_shape",
            "message": "no vectors",
            "retryability": "never"
        }"#;
        let err = SemasmVerify::parse_report(json).expect_err("must be agent_failure");
        match &err {
            VerifyError::AgentFailure { code, message, .. } => {
                assert_eq!(code, "UNSUPPORTED_SHAPE");
                assert_eq!(message, "no vectors");
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(err.failure_code(), Some("UNSUPPORTED_SHAPE"));
    }
}
