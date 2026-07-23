//! Fail-closed evidence aggregation with SemASM identity cross-checks.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::status::EvidenceStatus;
use crate::semasm::capabilities::CapabilityMatch;
use crate::semasm::doctor::DoctorReport;
use crate::semasm::verify::VerifyReport;
use crate::task::LockedTask;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckOutcome {
    pub check_name: String,
    pub required: bool,
    pub passed: bool,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceReport {
    pub task_id: String,
    pub task_digest: String,
    pub target: String,
    pub timestamp: String,
    pub run_id: Option<String>,

    pub doctor: Option<DoctorReport>,
    pub capability_match: Option<CapabilityMatch>,
    pub verify_report: Option<VerifyReport>,

    pub checks: Vec<CheckOutcome>,
    pub final_status: EvidenceStatus,
    pub summary: String,
    /// How SemASM was invoked for this evidence: `semasm_host` (direct) or
    /// `sandbox` (via [`crate::sandbox::ExecutionSandbox`]). Not absolute isolation.
    #[serde(default = "default_execution_isolation")]
    pub execution_isolation: String,
}

fn default_execution_isolation() -> String {
    "semasm_host".to_owned()
}

/// Expected identity for binding a SemASM report to the locked run.
#[derive(Debug, Clone)]
pub struct EvidenceExpect {
    /// Locked task target triple.
    pub expected_target: String,
    /// `sha256:` digest of the candidate source bytes submitted for verify.
    pub expected_source_digest: String,
    /// `sha256:` digest of the SemASM contract file bytes.
    pub expected_contract_digest: String,
    /// Result of optional object inspection (I0), when policy requires it.
    pub object_inspection: Option<ObjectInspectionOutcome>,
    /// Result of optional twin-build reproducibility check (PR-021).
    pub reproducible_build: Option<ReproducibleBuildOutcome>,
}

/// Outcome of same-host twin build comparison (PR-021).
#[derive(Debug, Clone)]
pub struct ReproducibleBuildOutcome {
    pub matched: bool,
    pub details: String,
}

/// Outcome of assembling + inspecting a candidate object (I0).
#[derive(Debug, Clone)]
pub struct ObjectInspectionOutcome {
    pub error: Option<String>,
    pub has_wxorx: bool,
    pub has_executable_stack: bool,
    pub format: String,
}

impl EvidenceExpect {
    #[must_use]
    pub fn new(
        expected_target: impl Into<String>,
        expected_source_digest: impl Into<String>,
        expected_contract_digest: impl Into<String>,
    ) -> Self {
        Self {
            expected_target: expected_target.into(),
            expected_source_digest: expected_source_digest.into(),
            expected_contract_digest: expected_contract_digest.into(),
            object_inspection: None,
            reproducible_build: None,
        }
    }
}

/// Prefixed SHA-256 digest (`sha256:` + lowercase hex).
#[must_use]
pub fn sha256_digest_prefixed(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(7 + 64);
    out.push_str("sha256:");
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

pub struct EvidenceAggregator;

impl EvidenceAggregator {
    /// Build a fail-closed evidence report.
    ///
    /// Missing doctor / capability / verify inputs become **failed required
    /// checks** (never silently omitted). Identity mismatches force `Failed`
    /// even when SemASM reported `verified`.
    #[must_use]
    pub fn build(
        task: &LockedTask,
        run_id: Option<String>,
        verify_report: Option<VerifyReport>,
        doctor: Option<DoctorReport>,
        capability_match: Option<CapabilityMatch>,
        expect: &EvidenceExpect,
    ) -> EvidenceReport {
        let mut checks: Vec<CheckOutcome> = Vec::new();

        checks.push(CheckOutcome {
            check_name: "task_valid".to_owned(),
            required: true,
            passed: true,
            details: None,
        });

        match &doctor {
            Some(doc) => {
                // Degraded = binary usable but schema not advertised; still runnable.
                let passed = matches!(
                    doc.status,
                    crate::semasm::DoctorStatus::Available | crate::semasm::DoctorStatus::Degraded
                );
                checks.push(CheckOutcome {
                    check_name: "semasm_available".to_owned(),
                    required: true,
                    passed,
                    details: Some(format!("{:?}", doc.status)),
                });
            }
            None => checks.push(CheckOutcome {
                check_name: "semasm_available".to_owned(),
                required: true,
                passed: false,
                details: Some("doctor report missing".to_owned()),
            }),
        }

        match &capability_match {
            Some(cm) => {
                checks.push(CheckOutcome {
                    check_name: "target_capability_match".to_owned(),
                    required: true,
                    passed: cm.compatible,
                    details: if cm.compatible {
                        None
                    } else {
                        let mut msgs = cm.insufficient.clone();
                        msgs.extend(cm.missing.clone());
                        Some(msgs.join("; "))
                    },
                });
            }
            None => checks.push(CheckOutcome {
                check_name: "target_capability_match".to_owned(),
                required: true,
                passed: false,
                details: Some("capability match missing".to_owned()),
            }),
        }

        match &verify_report {
            Some(vr) => {
                let passed = vr.outcome == EvidenceStatus::Verified;
                checks.push(CheckOutcome {
                    check_name: "semasm_verification".to_owned(),
                    required: true,
                    passed,
                    details: Some(format!("{:?}", vr.outcome)),
                });

                let schema_ok = vr
                    .schema_version
                    .as_deref()
                    .is_some_and(schema_version_compatible);
                checks.push(CheckOutcome {
                    check_name: "report_schema_compatible".to_owned(),
                    required: true,
                    passed: schema_ok,
                    details: Some(
                        vr.schema_version
                            .clone()
                            .unwrap_or_else(|| "missing".to_owned()),
                    ),
                });

                let target_ok = vr.target.as_deref() == Some(expect.expected_target.as_str());
                checks.push(CheckOutcome {
                    check_name: "report_target_matches".to_owned(),
                    required: true,
                    passed: target_ok,
                    details: Some(format!(
                        "expected={} actual={:?}",
                        expect.expected_target, vr.target
                    )),
                });

                let source_ok =
                    vr.source_digest.as_deref() == Some(expect.expected_source_digest.as_str());
                checks.push(CheckOutcome {
                    check_name: "report_source_digest_matches".to_owned(),
                    required: true,
                    passed: source_ok,
                    details: Some(format!(
                        "expected={} actual={:?}",
                        expect.expected_source_digest, vr.source_digest
                    )),
                });

                let contract_ok =
                    vr.contract_digest.as_deref() == Some(expect.expected_contract_digest.as_str());
                checks.push(CheckOutcome {
                    check_name: "report_contract_digest_matches".to_owned(),
                    required: true,
                    passed: contract_ok,
                    details: Some(format!(
                        "expected={} actual={:?}",
                        expect.expected_contract_digest, vr.contract_digest
                    )),
                });

                let tool_ok = vr
                    .tool_version
                    .as_deref()
                    .is_some_and(|v| v.starts_with("semasm "));
                checks.push(CheckOutcome {
                    check_name: "report_tool_identity_allowed".to_owned(),
                    required: true,
                    passed: tool_ok,
                    details: Some(format!("{:?}", vr.tool_version)),
                });
            }
            None => checks.push(CheckOutcome {
                check_name: "semasm_verification".to_owned(),
                required: true,
                passed: false,
                details: Some("verification report missing".to_owned()),
            }),
        }

        if task.task().verification.require_object_inspection {
            match &expect.object_inspection {
                None => checks.push(CheckOutcome {
                    check_name: "object_inspection".to_owned(),
                    required: true,
                    passed: false,
                    details: Some("required but not performed".to_owned()),
                }),
                Some(oi) => {
                    if let Some(err) = &oi.error {
                        checks.push(CheckOutcome {
                            check_name: "object_inspection".to_owned(),
                            required: true,
                            passed: false,
                            details: Some(err.clone()),
                        });
                    } else {
                        let clean = !oi.has_wxorx && !oi.has_executable_stack;
                        checks.push(CheckOutcome {
                            check_name: "object_inspection".to_owned(),
                            required: true,
                            passed: clean,
                            details: Some(format!(
                                "format={} wxorx={} exec_stack={}",
                                oi.format, oi.has_wxorx, oi.has_executable_stack
                            )),
                        });
                    }
                }
            }
        }

        if task.task().verification.require_reproducible_build {
            match &expect.reproducible_build {
                None => checks.push(CheckOutcome {
                    check_name: "reproducible_build".to_owned(),
                    required: true,
                    passed: false,
                    details: Some("required but not performed".to_owned()),
                }),
                Some(rb) => checks.push(CheckOutcome {
                    check_name: "reproducible_build".to_owned(),
                    required: true,
                    passed: rb.matched,
                    details: Some(rb.details.clone()),
                }),
            }
        }

        let required_failures: Vec<&CheckOutcome> =
            checks.iter().filter(|c| c.required && !c.passed).collect();

        let identity_failed = required_failures.iter().any(|c| {
            matches!(
                c.check_name.as_str(),
                "report_schema_compatible"
                    | "report_target_matches"
                    | "report_source_digest_matches"
                    | "report_contract_digest_matches"
                    | "report_tool_identity_allowed"
            )
        });

        let final_status = if identity_failed {
            EvidenceStatus::Failed
        } else if required_failures.is_empty() {
            EvidenceStatus::Verified
        } else if verify_report.is_none() {
            EvidenceStatus::Failed
        } else if let Some(vr) = verify_report.as_ref() {
            match vr.outcome {
                EvidenceStatus::Verified => EvidenceStatus::Incomplete,
                other => other,
            }
        } else {
            EvidenceStatus::Failed
        };

        let summary = if final_status == EvidenceStatus::Verified {
            format!(
                "Accepted under policy `{}`; all {} required checks completed.",
                task.task().task_id,
                checks.iter().filter(|c| c.required).count(),
            )
        } else {
            format!(
                "Rejected: {} of {} required checks failed.",
                required_failures.len(),
                checks.iter().filter(|c| c.required).count(),
            )
        };

        EvidenceReport {
            task_id: task.task().task_id.clone(),
            task_digest: task.digest().prefixed(),
            target: task.task().target.clone(),
            timestamp: iso_timestamp(),
            run_id,
            doctor,
            capability_match,
            verify_report,
            checks,
            final_status,
            summary,
            execution_isolation: default_execution_isolation(),
        }
    }
}

/// Accepted VerificationReport schemas: major 0, minor >= 4 and < 5 (i.e. 0.4.x).
#[must_use]
pub fn schema_version_compatible(version: &str) -> bool {
    let mut parts = version.split('.');
    let Some(major) = parts.next().and_then(|p| p.parse::<u32>().ok()) else {
        return false;
    };
    let Some(minor) = parts.next().and_then(|p| p.parse::<u32>().ok()) else {
        return false;
    };
    major == 0 && (4..5).contains(&minor)
}

fn iso_timestamp() -> String {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before epoch");
    let secs = dur.as_secs();
    let subsec = dur.subsec_millis();
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}.{subsec:03}Z")
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d_ = doy - (153 * mp + 2) / 5 + 1;
    let m_ = if mp < 10 { mp + 3 } else { mp - 9 };
    let y_ = if m_ <= 2 { y + 1 } else { y };
    (y_, m_ as u32, d_ as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semasm::capabilities::CapabilityMatch;
    use crate::semasm::doctor::{DoctorReport, DoctorStatus, SemasmVersion};
    use crate::semasm::verify::VerifyReport;
    use crate::task::{load_locked_task, LockedTask};
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("tasks")
            .join(name)
    }

    fn sample_locked_task() -> LockedTask {
        load_locked_task(fixture("sum_i64.vaa.toml")).expect("valid fixture")
    }

    fn expect_for(task: &LockedTask) -> EvidenceExpect {
        let mut expect = EvidenceExpect::new(
            task.task().target.clone(),
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        );
        if task.task().verification.require_object_inspection {
            expect.object_inspection = Some(ObjectInspectionOutcome {
                error: None,
                has_wxorx: false,
                has_executable_stack: false,
                format: "Elf".into(),
            });
        }
        if task.task().verification.require_reproducible_build {
            expect.reproducible_build = Some(ReproducibleBuildOutcome {
                matched: true,
                details: "unit twin match".into(),
            });
        }
        expect
    }

    fn available_doctor() -> DoctorReport {
        DoctorReport {
            status: DoctorStatus::Available,
            binary_path: Some(PathBuf::from("semasm")),
            version: Some(SemasmVersion {
                version: "0.1.0".to_owned(),
                schema_version: "0.1".to_owned(),
            }),
            details: vec![],
            live_probe: None,
        }
    }

    fn ok_verify(task: &LockedTask, expect: &EvidenceExpect) -> VerifyReport {
        VerifyReport {
            outcome: EvidenceStatus::Verified,
            raw_status: "verified".to_owned(),
            schema_version: Some("0.4".to_owned()),
            diagnostics: vec![],
            target: Some(task.task().target.clone()),
            source_digest: Some(expect.expected_source_digest.clone()),
            contract_digest: Some(expect.expected_contract_digest.clone()),
            tool_version: Some("semasm 0.1.0".to_owned()),
            raw_json: "{}".to_owned(),
        }
    }

    #[test]
    fn aggregator_verified_when_all_pass() {
        let task = sample_locked_task();
        let expect = expect_for(&task);
        let report = EvidenceAggregator::build(
            &task,
            None,
            Some(ok_verify(&task, &expect)),
            Some(available_doctor()),
            Some(CapabilityMatch {
                compatible: true,
                missing: vec![],
                insufficient: vec![],
            }),
            &expect,
        );
        assert_eq!(report.final_status, EvidenceStatus::Verified);
        assert!(report.summary.contains("Accepted"));
    }

    #[test]
    fn aggregator_reproducible_build_mismatch_blocks_verified() {
        let task = sample_locked_task();
        assert!(task.task().verification.require_reproducible_build);
        let mut expect = expect_for(&task);
        expect.reproducible_build = Some(ReproducibleBuildOutcome {
            matched: false,
            details: "object_digest mismatch".into(),
        });
        let report = EvidenceAggregator::build(
            &task,
            None,
            Some(ok_verify(&task, &expect)),
            Some(available_doctor()),
            Some(CapabilityMatch {
                compatible: true,
                missing: vec![],
                insufficient: vec![],
            }),
            &expect,
        );
        assert_ne!(report.final_status, EvidenceStatus::Verified);
        assert!(report
            .checks
            .iter()
            .any(|c| c.check_name == "reproducible_build" && !c.passed));
    }

    #[test]
    fn aggregator_object_inspection_fails_on_wxorx() {
        let task = sample_locked_task();
        assert!(task.task().verification.require_object_inspection);
        let mut expect = expect_for(&task);
        expect.object_inspection = Some(ObjectInspectionOutcome {
            error: None,
            has_wxorx: true,
            has_executable_stack: false,
            format: "Elf".into(),
        });
        let report = EvidenceAggregator::build(
            &task,
            None,
            Some(ok_verify(&task, &expect)),
            Some(available_doctor()),
            Some(CapabilityMatch {
                compatible: true,
                missing: vec![],
                insufficient: vec![],
            }),
            &expect,
        );
        assert_ne!(report.final_status, EvidenceStatus::Verified);
        assert!(report
            .checks
            .iter()
            .any(|c| c.check_name == "object_inspection" && !c.passed));
    }

    #[test]
    fn aggregator_never_verified_when_all_options_missing() {
        let task = sample_locked_task();
        let expect = expect_for(&task);
        let report = EvidenceAggregator::build(&task, None, None, None, None, &expect);
        assert_ne!(report.final_status, EvidenceStatus::Verified);
        assert_eq!(report.final_status, EvidenceStatus::Failed);
        assert!(report
            .checks
            .iter()
            .any(|c| c.check_name == "semasm_verification" && !c.passed));
    }

    #[test]
    fn aggregator_failed_on_source_digest_mismatch() {
        let task = sample_locked_task();
        let expect = expect_for(&task);
        let mut verify = ok_verify(&task, &expect);
        verify.source_digest = Some("sha256:deadbeef".to_owned());
        let report = EvidenceAggregator::build(
            &task,
            None,
            Some(verify),
            Some(available_doctor()),
            Some(CapabilityMatch {
                compatible: true,
                missing: vec![],
                insufficient: vec![],
            }),
            &expect,
        );
        assert_eq!(report.final_status, EvidenceStatus::Failed);
        assert!(report
            .checks
            .iter()
            .any(|c| c.check_name == "report_source_digest_matches" && !c.passed));
    }

    #[test]
    fn aggregator_violated_when_verify_fails() {
        let task = sample_locked_task();
        let expect = expect_for(&task);
        let report = EvidenceAggregator::build(
            &task,
            None,
            Some(VerifyReport {
                outcome: EvidenceStatus::Violated,
                raw_status: "semantic_failed".to_owned(),
                schema_version: Some("0.4".to_owned()),
                diagnostics: vec![],
                target: Some(task.task().target.clone()),
                source_digest: Some(expect.expected_source_digest.clone()),
                contract_digest: Some(expect.expected_contract_digest.clone()),
                tool_version: Some("semasm 0.1.0".to_owned()),
                raw_json: "{}".to_owned(),
            }),
            Some(available_doctor()),
            Some(CapabilityMatch {
                compatible: true,
                missing: vec![],
                insufficient: vec![],
            }),
            &expect,
        );
        assert_eq!(report.final_status, EvidenceStatus::Violated);
    }

    #[test]
    fn aggregator_preserves_execution_denied_as_incomplete() {
        let task = sample_locked_task();
        let expect = EvidenceExpect::new(
            task.task().target.clone(),
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        let report = EvidenceAggregator::build(
            &task,
            None,
            Some(VerifyReport {
                outcome: EvidenceStatus::Incomplete,
                raw_status: "execution_denied".to_owned(),
                schema_version: Some("0.4".to_owned()),
                diagnostics: vec![],
                target: Some(task.task().target.clone()),
                source_digest: Some(expect.expected_source_digest.clone()),
                contract_digest: Some(expect.expected_contract_digest.clone()),
                tool_version: Some("semasm 0.1.0".to_owned()),
                raw_json: "{}".to_owned(),
            }),
            Some(available_doctor()),
            Some(CapabilityMatch {
                compatible: true,
                missing: vec![],
                insufficient: vec![],
            }),
            &expect,
        );
        assert_eq!(report.final_status, EvidenceStatus::Incomplete);
    }

    #[test]
    fn schema_pin_accepts_0_4() {
        assert!(schema_version_compatible("0.4"));
        assert!(!schema_version_compatible("0.3"));
        assert!(!schema_version_compatible("0.5"));
        assert!(!schema_version_compatible("1.0"));
    }
}
