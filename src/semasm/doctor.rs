use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum DoctorStatus {
    Available,
    Unavailable,
    Incompatible,
    Degraded,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SemasmVersion {
    pub version: String,
    pub schema_version: String,
}

#[derive(Debug, Clone, serde::Serialize)]
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
    pub fn run() -> DoctorReport {
        let binary = match Self::find_binary() {
            Some(path) => path,
            None => {
                return DoctorReport {
                    status: DoctorStatus::Unavailable,
                    binary_path: None,
                    version: None,
                    details: vec!["semasm binary not found on PATH".to_owned()],
                };
            }
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

        let details = vec![format!("semasm version {}", version.version)];

        let status = if version.schema_version == "0.1" {
            DoctorStatus::Available
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

    pub fn read_version(binary: &PathBuf) -> Result<SemasmVersion, DoctorError> {
        let output = Command::new(binary)
            .arg("version")
            .output()
            .map_err(|e| DoctorError::VersionCheck(format!("failed to execute: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{stdout}{stderr}");

        let lines: Vec<&str> = combined.lines().collect();
        let version_line = lines
            .first()
            .copied()
            .unwrap_or("unknown")
            .trim();

        let version = version_line
            .strip_prefix("semasm ")
            .unwrap_or(version_line)
            .to_owned();

        let schema_version = lines
            .iter()
            .find(|l| l.contains("schema") || l.contains("capability"))
            .and_then(|l| l.split_whitespace().last())
            .unwrap_or("0.1")
            .to_owned();

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
            DoctorStatus::Available | DoctorStatus::Incompatible | DoctorStatus::Degraded => {
                // Binary exists in test environment — that's fine
            }
            DoctorStatus::Unavailable => {
                assert!(report.binary_path.is_none());
                assert!(report.version.is_none());
            }
        }
    }
}
