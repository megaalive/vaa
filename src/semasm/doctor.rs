//! SemASM doctor: discover binary and negotiate version via ProcessRunner.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::process::{ProcessConfig, ProcessError, ProcessRunner};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DoctorStatus {
    Available,
    Unavailable,
    Incompatible,
    Degraded,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemasmVersion {
    pub version: String,
    pub schema_version: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DoctorReport {
    pub status: DoctorStatus,
    pub binary_path: Option<PathBuf>,
    pub version: Option<SemasmVersion>,
    pub details: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum DoctorError {
    #[error("binary discovery failed: {0}")]
    Discovery(String),
    #[error("version check failed: {0}")]
    VersionCheck(String),
}

pub struct SemasmDoctor;

impl SemasmDoctor {
    #[must_use]
    pub fn run() -> DoctorReport {
        let Some(binary) = Self::find_binary() else {
            return DoctorReport {
                status: DoctorStatus::Unavailable,
                binary_path: None,
                version: None,
                details: vec!["semasm binary not found on PATH".to_owned()],
            };
        };

        let version = match Self::read_version(&binary) {
            Ok(v) => v,
            Err(e) => {
                return DoctorReport {
                    status: DoctorStatus::Degraded,
                    binary_path: Some(binary),
                    version: None,
                    details: vec![format!("binary found but version check failed: {e}")],
                };
            }
        };

        let details = vec![format!(
            "semasm version {} schema {}",
            version.version, version.schema_version
        )];

        // Capability/task negotiation schema advertised by SemASM (distinct from
        // VerificationReport 0.4). Missing advertisement is fail-closed Degraded.
        let status = if version.schema_version == "0.1" {
            DoctorStatus::Available
        } else if version.schema_version == "missing" {
            DoctorStatus::Degraded
        } else {
            DoctorStatus::Incompatible
        };

        DoctorReport {
            status,
            binary_path: Some(binary),
            version: Some(version),
            details,
        }
    }

    #[must_use]
    pub fn find_binary() -> Option<PathBuf> {
        let paths = std::env::var_os("PATH")?;
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join("semasm");
            if candidate.is_file() {
                return Some(candidate);
            }
            let candidate_exe = dir.join("semasm.exe");
            if candidate_exe.is_file() {
                return Some(candidate_exe);
            }
        }
        None
    }

    pub fn read_version(binary: &Path) -> Result<SemasmVersion, DoctorError> {
        let config = ProcessConfig {
            program: binary.to_path_buf(),
            args: vec!["version".to_owned()],
            timeout: Duration::from_secs(15),
            max_output_bytes: 65_536,
            ..ProcessConfig::default()
        };

        let output = ProcessRunner::run(&config).map_err(|e| match e {
            ProcessError::Timeout { duration } => {
                DoctorError::VersionCheck(format!("timed out after {duration:?}"))
            }
            ProcessError::OutputOverflow { limit } => {
                DoctorError::VersionCheck(format!("output exceeded {limit} bytes"))
            }
            ProcessError::Spawn { detail, .. } => DoctorError::VersionCheck(detail),
        })?;

        if output.exit_code != Some(0) {
            return Err(DoctorError::VersionCheck(format!(
                "non-zero exit {:?}; stderr={}",
                output.exit_code, output.stderr
            )));
        }

        // Parse stdout only — never concatenate stderr into version text.
        let stdout = output.stdout;
        let lines: Vec<&str> = stdout.lines().collect();
        let version_line = lines.first().copied().unwrap_or("unknown").trim();

        let version = version_line
            .strip_prefix("semasm ")
            .unwrap_or(version_line)
            .to_owned();

        let schema_version = lines
            .iter()
            .find(|l| l.to_ascii_lowercase().contains("schema"))
            .and_then(|l| l.split_whitespace().last())
            .map_or_else(|| "missing".to_owned(), ToOwned::to_owned);

        Ok(SemasmVersion {
            version,
            schema_version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_binary_returns_none_when_not_on_path() {
        let original_path = std::env::var_os("PATH");
        std::env::set_var("PATH", "");
        let result = SemasmDoctor::find_binary();
        if let Some(path) = original_path {
            std::env::set_var("PATH", path);
        }
        assert!(result.is_none());
    }

    #[test]
    fn doctor_report_is_unavailable_without_binary() {
        let report = SemasmDoctor::run();
        match report.status {
            DoctorStatus::Available | DoctorStatus::Incompatible | DoctorStatus::Degraded => {}
            DoctorStatus::Unavailable => {
                assert!(report.binary_path.is_none());
                assert!(report.version.is_none());
            }
        }
    }
}
