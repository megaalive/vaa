use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::evidence::EvidenceStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemasmDiagnostic {
    pub code: Option<String>,
    pub severity: Option<String>,
    pub message: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyReportRaw {
    pub status: String,
    pub diagnostics: Vec<SemasmDiagnostic>,
    pub coverage: Option<serde_json::Value>,
    pub target: Option<String>,
    pub contract_digest: Option<String>,
    pub source_digest: Option<String>,
    pub tool_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyReport {
    pub outcome: EvidenceStatus,
    pub raw_status: String,
    pub diagnostics: Vec<SemasmDiagnostic>,
    pub target: Option<String>,
    pub source_digest: Option<String>,
    pub contract_digest: Option<String>,
    pub tool_version: Option<String>,
    pub raw_json: String,
}

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

pub struct SemasmVerify;

impl SemasmVerify {
    pub fn run(source: &Path, contract: &Path, binary: &Path) -> Result<VerifyReport, VerifyError> {
        let output = Command::new(binary)
            .arg("agent")
            .arg("verify")
            .arg(source)
            .arg(contract)
            .arg("--format")
            .arg("json")
            .output()
            .map_err(|e| VerifyError::ProcessFailed(format!("failed to execute: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = format!("{stdout}{stderr}");

        let raw: VerifyReportRaw =
            serde_json::from_str(&combined).map_err(|e| VerifyError::ParseFailed(e.to_string()))?;

        let outcome = Self::map_status(&raw.status);

        Ok(VerifyReport {
            outcome,
            raw_status: raw.status,
            diagnostics: raw.diagnostics,
            target: raw.target,
            source_digest: raw.source_digest,
            contract_digest: raw.contract_digest,
            tool_version: raw.tool_version,
            raw_json: combined,
        })
    }

    pub fn parse_report(json: &str) -> Result<VerifyReport, VerifyError> {
        let raw: VerifyReportRaw =
            serde_json::from_str(json).map_err(|e| VerifyError::ParseFailed(e.to_string()))?;

        let outcome = Self::map_status(&raw.status);

        Ok(VerifyReport {
            outcome,
            raw_status: raw.status,
            diagnostics: raw.diagnostics,
            target: raw.target,
            source_digest: raw.source_digest,
            contract_digest: raw.contract_digest,
            tool_version: raw.tool_version,
            raw_json: json.to_owned(),
        })
    }

    fn map_status(status: &str) -> EvidenceStatus {
        match status {
            "verified" => EvidenceStatus::Verified,
            "violated" => EvidenceStatus::Violated,
            "incomplete" => EvidenceStatus::Incomplete,
            "failed" => EvidenceStatus::Failed,
            _ => EvidenceStatus::Incomplete,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_verified_report() {
        let json = r#"{
            "status": "verified",
            "diagnostics": [],
            "target": "x86_64-unknown-linux-gnu",
            "contract_digest": "sha256:abc",
            "source_digest": "sha256:def",
            "tool_version": "semasm 0.1.0"
        }"#;
        let report = SemasmVerify::parse_report(json).expect("parse");
        assert_eq!(report.outcome, EvidenceStatus::Verified);
        assert_eq!(report.target.as_deref(), Some("x86_64-unknown-linux-gnu"));
    }

    #[test]
    fn parse_violated_report() {
        let json = r#"{
            "status": "violated",
            "diagnostics": [
                {
                    "code": "ABI_CALLEE_SAVED_CLOBBER",
                    "severity": "error",
                    "message": "RBX modified but not restored",
                    "location": "source.asm:14"
                }
            ],
            "target": "x86_64-unknown-linux-gnu"
        }"#;
        let report = SemasmVerify::parse_report(json).expect("parse");
        assert_eq!(report.outcome, EvidenceStatus::Violated);
        assert_eq!(report.diagnostics.len(), 1);
    }

    #[test]
    fn parse_incomplete_report() {
        let json = r#"{"status": "incomplete", "diagnostics": []}"#;
        let report = SemasmVerify::parse_report(json).expect("parse");
        assert_eq!(report.outcome, EvidenceStatus::Incomplete);
    }

    #[test]
    fn parse_failed_report() {
        let json = r#"{"status": "failed", "diagnostics": []}"#;
        let report = SemasmVerify::parse_report(json).expect("parse");
        assert_eq!(report.outcome, EvidenceStatus::Failed);
    }

    #[test]
    fn unknown_status_maps_to_incomplete() {
        let json = r#"{"status": "unknown_thing", "diagnostics": []}"#;
        let report = SemasmVerify::parse_report(json).expect("parse");
        assert_eq!(report.outcome, EvidenceStatus::Incomplete);
    }

    #[test]
    fn malformed_json_returns_error() {
        let result = SemasmVerify::parse_report("not json");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerifyError::ParseFailed(_)));
    }
}
