//! SemASM doctor: discover binary and negotiate version via ProcessRunner.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::process::{ProcessConfig, ProcessError, ProcessRunner};

use super::capabilities::TargetCapabilities;
use super::status::{
    compare_live_status, parse_status_json, CompareOutcome, LiveStatusCompare, SemasmStatusDocument,
};

/// Gate goldens probed when a live SemASM status document is available.
const DOCTOR_COMPARE_TARGETS: &[&str] = &["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu"];

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
pub struct LiveProbeSummary {
    pub semasm_version: Option<String>,
    pub capability_schema: Option<String>,
    pub compares: Vec<LiveStatusCompare>,
}

/// Static honesty about VAA run-dir write policy (G0). Not OS isolation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EvidencePolicy {
    /// Relative layout hint for generator output.
    pub generator_staging: String,
    /// Who may write sealed evidence files.
    pub evidence_writes: String,
    /// `RunDir` rejects protected-zone writes via its public API.
    pub rundir_protected_zone: bool,
    /// OS ACL / process sandbox for generators — still false.
    pub os_fs_isolation: bool,
}

impl EvidencePolicy {
    #[must_use]
    pub fn vaa_g0() -> Self {
        Self {
            generator_staging: "run_dir/staging".to_owned(),
            evidence_writes: "seal_module_only".to_owned(),
            rundir_protected_zone: true,
            os_fs_isolation: false,
        }
    }

    /// Reported when a generator OS jail is actually configured/enforced.
    #[must_use]
    pub fn vaa_os_jail() -> Self {
        Self {
            generator_staging: "run_dir/staging".to_owned(),
            evidence_writes: "seal_module_only".to_owned(),
            rundir_protected_zone: true,
            os_fs_isolation: true,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DoctorReport {
    pub status: DoctorStatus,
    pub binary_path: Option<PathBuf>,
    pub version: Option<SemasmVersion>,
    pub details: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live_probe: Option<LiveProbeSummary>,
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
                live_probe: None,
            };
        };

        let (version_str, version_notes) = match Self::read_version_string(&binary) {
            Ok((v, notes)) => (v, notes),
            Err(e) => {
                return DoctorReport {
                    status: DoctorStatus::Degraded,
                    binary_path: Some(binary),
                    version: None,
                    details: vec![format!("binary found but version check failed: {e}")],
                    live_probe: None,
                };
            }
        };

        let mut details = version_notes;
        let (schema_version, live_probe, status_notes) = match Self::read_status(&binary) {
            Ok(doc) => {
                let probe = summarize_live_probe(&doc);
                let schema = doc
                    .capability_schema
                    .clone()
                    .unwrap_or_else(|| "missing".to_owned());
                let mut notes = Vec::new();
                for cmp in &probe.compares {
                    if cmp.outcome == CompareOutcome::Drift {
                        notes.push(format!(
                            "live status drift on {}: {}",
                            cmp.target_id,
                            cmp.axes.join("; ")
                        ));
                    }
                }
                (schema, Some(probe), notes)
            }
            Err(e) => {
                details.push(format!("status --format json failed: {e}"));
                ("missing".to_owned(), None, Vec::new())
            }
        };
        details.extend(status_notes);

        details.push(format!(
            "semasm version {version_str} capability_schema {schema_version}"
        ));

        let version = SemasmVersion {
            version: version_str,
            schema_version: schema_version.clone(),
        };

        let has_drift = live_probe.as_ref().is_some_and(|p| {
            p.compares
                .iter()
                .any(|c| c.outcome == CompareOutcome::Drift)
        });

        let status = if schema_version != "0.1" && schema_version != "missing" {
            DoctorStatus::Incompatible
        } else if schema_version == "missing" || has_drift {
            DoctorStatus::Degraded
        } else {
            DoctorStatus::Available
        };

        DoctorReport {
            status,
            binary_path: Some(binary),
            version: Some(version),
            details,
            live_probe,
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

    fn read_version_string(binary: &Path) -> Result<(String, Vec<String>), DoctorError> {
        let config = ProcessConfig {
            program: binary.to_path_buf(),
            args: vec![
                "version".to_owned(),
                "--format".to_owned(),
                "json".to_owned(),
            ],
            timeout: Duration::from_secs(15),
            max_output_bytes: 65_536,
            allowed_env: doctor_allowed_env(),
            ..ProcessConfig::default()
        };

        match run_semasm(&config) {
            Ok(stdout) => {
                if let Ok(v) = parse_version_json(&stdout) {
                    return Ok((v, vec!["version via --format json".to_owned()]));
                }
                let text = parse_version_text(&stdout);
                Ok((
                    text,
                    vec!["version JSON parse failed; fell back to terminal text".to_owned()],
                ))
            }
            Err(json_err) => {
                // Older SemASM without --format json.
                let config = ProcessConfig {
                    program: binary.to_path_buf(),
                    args: vec!["version".to_owned()],
                    timeout: Duration::from_secs(15),
                    max_output_bytes: 65_536,
                    allowed_env: doctor_allowed_env(),
                    ..ProcessConfig::default()
                };
                let stdout = run_semasm(&config).map_err(|_| json_err)?;
                Ok((
                    parse_version_text(&stdout),
                    vec!["version --format json unavailable; used terminal version".to_owned()],
                ))
            }
        }
    }

    fn read_status(binary: &Path) -> Result<SemasmStatusDocument, DoctorError> {
        let config = ProcessConfig {
            program: binary.to_path_buf(),
            args: vec![
                "status".to_owned(),
                "--format".to_owned(),
                "json".to_owned(),
            ],
            timeout: Duration::from_secs(15),
            max_output_bytes: 1_048_576,
            allowed_env: doctor_allowed_env(),
            ..ProcessConfig::default()
        };
        let stdout = run_semasm(&config)?;
        parse_status_json(&stdout).map_err(|e| DoctorError::VersionCheck(e.to_string()))
    }
}

fn doctor_allowed_env() -> Vec<String> {
    vec![
        "PATH".to_owned(),
        "HOME".to_owned(),
        "USER".to_owned(),
        "SYSTEMROOT".to_owned(),
        "WINDIR".to_owned(),
        "SYSTEMDRIVE".to_owned(),
        "PATHEXT".to_owned(),
        "COMSPEC".to_owned(),
    ]
}

fn run_semasm(config: &ProcessConfig) -> Result<String, DoctorError> {
    let output = ProcessRunner::run(config).map_err(|e| match e {
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
    Ok(output.stdout)
}

fn parse_version_json(stdout: &str) -> Result<String, DoctorError> {
    let value: serde_json::Value = serde_json::from_str(stdout.trim())
        .map_err(|e| DoctorError::VersionCheck(format!("version JSON: {e}")))?;
    value
        .get("version")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            DoctorError::VersionCheck("version JSON missing string field `version`".into())
        })
}

fn parse_version_text(stdout: &str) -> String {
    let version_line = stdout.lines().next().unwrap_or("unknown").trim();
    version_line
        .strip_prefix("semasm ")
        .unwrap_or(version_line)
        .to_owned()
}

fn summarize_live_probe(doc: &SemasmStatusDocument) -> LiveProbeSummary {
    let compares = DOCTOR_COMPARE_TARGETS
        .iter()
        .map(|target| {
            let embedded = TargetCapabilities::for_target(target);
            compare_live_status(target, doc, &embedded)
        })
        .collect();
    LiveProbeSummary {
        semasm_version: doc.version.clone(),
        capability_schema: doc.capability_schema.clone(),
        compares,
    }
}

/// Probe live status for a single target (used by `vaa capabilities`).
#[must_use]
pub fn probe_live_for_target(target: &str) -> Option<(SemasmStatusDocument, LiveStatusCompare)> {
    let binary = SemasmDoctor::find_binary()?;
    let doc = SemasmDoctor::read_status(&binary).ok()?;
    let embedded = TargetCapabilities::for_target(target);
    let cmp = compare_live_status(target, &doc, &embedded);
    Some((doc, cmp))
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
        let original_path = std::env::var_os("PATH");
        std::env::set_var("PATH", "");
        let report = SemasmDoctor::run();
        if let Some(path) = original_path {
            std::env::set_var("PATH", path);
        }
        assert_eq!(report.status, DoctorStatus::Unavailable);
        assert!(report.binary_path.is_none());
        assert!(report.version.is_none());
        assert!(report.live_probe.is_none());
    }

    #[test]
    fn parse_version_json_reads_version_field() {
        let v = parse_version_json(r#"{"name":"semasm","version":"9.9.9"}"#).expect("json");
        assert_eq!(v, "9.9.9");
    }

    #[test]
    fn evidence_policy_g0_is_honest() {
        let p = EvidencePolicy::vaa_g0();
        assert_eq!(p.generator_staging, "run_dir/staging");
        assert_eq!(p.evidence_writes, "seal_module_only");
        assert!(p.rundir_protected_zone);
        assert!(!p.os_fs_isolation);
    }

    #[test]
    fn evidence_policy_os_jail_flips_flag() {
        let p = EvidencePolicy::vaa_os_jail();
        assert!(p.os_fs_isolation);
    }

    #[test]
    fn parse_version_text_strips_prefix() {
        assert_eq!(parse_version_text("semasm 1.2.3\n"), "1.2.3");
    }
}
