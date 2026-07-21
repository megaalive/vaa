use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use super::status::EvidenceStatus;
use crate::semasm::capabilities::CapabilityMatch;
use crate::semasm::doctor::DoctorReport;
use crate::semasm::verify::VerifyReport;
use crate::task::LockedTask;

#[derive(Debug, Clone, Serialize)]
pub struct CheckOutcome {
    pub check_name: String,
    pub required: bool,
    pub passed: bool,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
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
}

pub struct EvidenceAggregator;

impl EvidenceAggregator {
    #[must_use]
    pub fn build(
        task: &LockedTask,
        run_id: Option<String>,
        verify_report: Option<VerifyReport>,
        doctor: Option<DoctorReport>,
        capability_match: Option<CapabilityMatch>,
    ) -> EvidenceReport {
        let mut checks: Vec<CheckOutcome> = Vec::new();

        checks.push(CheckOutcome {
            check_name: "task_valid".to_owned(),
            required: true,
            passed: true,
            details: None,
        });

        if let Some(ref doc) = doctor {
            let passed = doc.status == crate::semasm::DoctorStatus::Available;
            checks.push(CheckOutcome {
                check_name: "semasm_available".to_owned(),
                required: true,
                passed,
                details: Some(format!("{:?}", doc.status)),
            });
        }

        if let Some(ref cm) = capability_match {
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

        if let Some(ref vr) = verify_report {
            let passed = vr.outcome == EvidenceStatus::Verified;
            checks.push(CheckOutcome {
                check_name: "semasm_verification".to_owned(),
                required: true,
                passed,
                details: Some(format!("{:?}", vr.outcome)),
            });
        }

        let required_failures: Vec<&CheckOutcome> =
            checks.iter().filter(|c| c.required && !c.passed).collect();

        // Prefer the SemASM-mapped verify outcome when a report was parsed.
        // `execution_denied` → Incomplete must not be collapsed to Violated.
        let final_status = if required_failures.is_empty() {
            EvidenceStatus::Verified
        } else if let Some(vr) = verify_report.as_ref() {
            match vr.outcome {
                EvidenceStatus::Verified => EvidenceStatus::Incomplete,
                other => other,
            }
        } else {
            EvidenceStatus::Incomplete
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
        }
    }
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

    #[test]
    fn aggregator_verified_when_all_pass() {
        let task = sample_locked_task();
        let report = EvidenceAggregator::build(
            &task,
            None,
            Some(VerifyReport {
                outcome: EvidenceStatus::Verified,
                raw_status: "verified".to_owned(),
                diagnostics: vec![],
                target: Some("x86_64-unknown-linux-gnu".to_owned()),
                source_digest: None,
                contract_digest: None,
                tool_version: None,
                raw_json: "{}".to_owned(),
            }),
            None,
            Some(CapabilityMatch {
                compatible: true,
                missing: vec![],
                insufficient: vec![],
            }),
        );
        assert_eq!(report.final_status, EvidenceStatus::Verified);
        assert!(report.summary.contains("Accepted"));
    }

    #[test]
    fn aggregator_violated_when_verify_fails() {
        let task = sample_locked_task();
        let report = EvidenceAggregator::build(
            &task,
            None,
            Some(VerifyReport {
                outcome: EvidenceStatus::Violated,
                raw_status: "violated".to_owned(),
                diagnostics: vec![],
                target: None,
                source_digest: None,
                contract_digest: None,
                tool_version: None,
                raw_json: "{}".to_owned(),
            }),
            None,
            None,
        );
        assert_eq!(report.final_status, EvidenceStatus::Violated);
    }

    #[test]
    fn aggregator_preserves_execution_denied_as_incomplete() {
        let task = sample_locked_task();
        let report = EvidenceAggregator::build(
            &task,
            None,
            Some(VerifyReport {
                outcome: EvidenceStatus::Incomplete,
                raw_status: "execution_denied".to_owned(),
                diagnostics: vec![],
                target: Some("x86_64-pc-windows-msvc".to_owned()),
                source_digest: Some("sha256:aa".to_owned()),
                contract_digest: Some("sha256:bb".to_owned()),
                tool_version: Some("semasm 0.1.0".to_owned()),
                raw_json: "{}".to_owned(),
            }),
            None,
            Some(CapabilityMatch {
                compatible: true,
                missing: vec![],
                insufficient: vec![],
            }),
        );
        assert_eq!(report.final_status, EvidenceStatus::Incomplete);
    }
}
