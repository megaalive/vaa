//! Live SemASM `status --format json` probe vs VAA embedded snapshot.
//!
//! SemASM maturity strings (`verified_in_ci`, `experimental`, …) are **not** the
//! same vocabulary as VAA [`CapabilityLevel`]. Comparison is intentionally narrow:
//! for Gate goldens, require a usable live `agent` axis — never silent-replace the
//! embedded snapshot.

use serde::{Deserialize, Serialize};

use super::capabilities::{CapabilityLevel, TargetCapabilities, CAPABILITY_SOURCE};

/// SemASM agent maturity levels that can back VAA Gate-1/2 agent-verify.
///
/// `semasm status --format json` emits **display** spellings from SemASM
/// `CapabilityLevel::as_str` (`CI-verified`, `unit-tested`), not TOML keys
/// (`verified_in_ci`, …). Accept both so probes stay tolerant.
const GATE_USABLE_AGENT: &[&str] = &[
    "CI-verified",
    "unit-tested",
    "release-qualified",
    "verified_in_ci",
    "verified_in_unit_tests",
    "release_qualified",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompareOutcome {
    Aligned,
    Drift,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveStatusCompare {
    pub outcome: CompareOutcome,
    pub target_id: String,
    pub embedded_source: String,
    pub live_agent: Option<String>,
    pub live_pipeline: Option<String>,
    pub axes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemasmStatusDocument {
    pub name: Option<String>,
    pub version: Option<String>,
    pub capability_schema: Option<String>,
    #[serde(default)]
    pub workspace_crates: Vec<String>,
    #[serde(default)]
    pub targets: Vec<SemasmStatusTarget>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemasmStatusTarget {
    pub id: String,
    pub decode: Option<String>,
    pub lower: Option<String>,
    pub abi: Option<String>,
    pub assemble: Option<String>,
    pub link: Option<String>,
    pub execute: Option<String>,
    pub pipeline: Option<String>,
    pub agent: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum StatusError {
    #[error("status JSON parse failed: {0}")]
    Parse(String),
}

/// Parse SemASM `status --format json` stdout (additive fields tolerated).
pub fn parse_status_json(stdout: &str) -> Result<SemasmStatusDocument, StatusError> {
    serde_json::from_str(stdout.trim()).map_err(|e| StatusError::Parse(e.to_string()))
}

/// Compare one VAA embedded target snapshot to a live SemASM status document.
#[must_use]
pub fn compare_live_status(
    target: &str,
    live: &SemasmStatusDocument,
    embedded: &TargetCapabilities,
) -> LiveStatusCompare {
    let mut axes = Vec::new();
    let live_target = live.targets.iter().find(|t| t.id == target);

    let Some(t) = live_target else {
        axes.push(format!("target `{target}` missing from SemASM status"));
        return LiveStatusCompare {
            outcome: CompareOutcome::Drift,
            target_id: target.to_owned(),
            embedded_source: CAPABILITY_SOURCE.to_owned(),
            live_agent: None,
            live_pipeline: None,
            axes,
        };
    };
    let (live_agent, live_pipeline) = (t.agent.clone(), t.pipeline.clone());

    if matches!(embedded.decode, CapabilityLevel::Unknown) {
        axes.push("embedded snapshot does not recognize target; agent gate check skipped".into());
        return LiveStatusCompare {
            outcome: CompareOutcome::Aligned,
            target_id: target.to_owned(),
            embedded_source: CAPABILITY_SOURCE.to_owned(),
            live_agent,
            live_pipeline,
            axes,
        };
    }

    // VAA Gate goldens claim Supported agent-verify path; live `agent` must be
    // strong enough. Do not equate SemASM decode/lower/pipeline strings to VAA levels.
    let agent = live_agent.as_deref().unwrap_or("missing");
    if embedded_claims_gate_path(embedded) && !GATE_USABLE_AGENT.contains(&agent) {
        axes.push(format!(
            "live agent={agent} is not gate-usable (need one of {GATE_USABLE_AGENT:?}); \
             embedded snapshot still claims Gate path Supported"
        ));
    }

    let outcome = if axes.is_empty() {
        CompareOutcome::Aligned
    } else {
        CompareOutcome::Drift
    };

    LiveStatusCompare {
        outcome,
        target_id: target.to_owned(),
        embedded_source: CAPABILITY_SOURCE.to_owned(),
        live_agent,
        live_pipeline,
        axes,
    }
}

fn embedded_claims_gate_path(embedded: &TargetCapabilities) -> bool {
    matches!(embedded.lower, CapabilityLevel::Supported)
        && matches!(embedded.abi_check, CapabilityLevel::Supported)
        && matches!(embedded.object_inspect, CapabilityLevel::Supported)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_status(agent_win: &str, agent_linux: &str) -> SemasmStatusDocument {
        SemasmStatusDocument {
            name: Some("semasm".into()),
            version: Some("0.0.0-test".into()),
            capability_schema: Some("0.1".into()),
            workspace_crates: vec!["semasm-cli".into()],
            targets: vec![
                SemasmStatusTarget {
                    id: "x86_64-pc-windows-msvc".into(),
                    decode: Some("partial".into()),
                    lower: Some("partial".into()),
                    abi: Some("unit-tested".into()),
                    assemble: Some("CI-verified".into()),
                    link: Some("CI-verified".into()),
                    execute: Some("CI-verified".into()),
                    pipeline: Some("experimental".into()),
                    agent: Some(agent_win.into()),
                },
                SemasmStatusTarget {
                    id: "x86_64-unknown-linux-gnu".into(),
                    decode: Some("partial".into()),
                    lower: Some("partial".into()),
                    abi: Some("unit-tested".into()),
                    assemble: Some("CI-verified".into()),
                    link: Some("CI-verified".into()),
                    execute: Some("CI-verified".into()),
                    pipeline: Some("experimental".into()),
                    agent: Some(agent_linux.into()),
                },
            ],
            notes: vec![],
        }
    }

    #[test]
    fn parse_tolerates_additive_fields() {
        let raw = r#"{
            "name":"semasm",
            "version":"1.2.3",
            "capability_schema":"0.1",
            "workspace_crates":["semasm-cli"],
            "targets":[{"id":"x86_64-pc-windows-msvc","agent":"CI-verified","extra_future":true}],
            "notes":["n"],
            "future_top": 1
        }"#;
        let doc = parse_status_json(raw).expect("parse");
        assert_eq!(doc.capability_schema.as_deref(), Some("0.1"));
        assert_eq!(doc.targets[0].agent.as_deref(), Some("CI-verified"));
    }

    #[test]
    fn aligned_when_agent_gate_usable_despite_pipeline_experimental() {
        let live = sample_status("CI-verified", "CI-verified");
        let embedded = TargetCapabilities::for_target("x86_64-pc-windows-msvc");
        let cmp = compare_live_status("x86_64-pc-windows-msvc", &live, &embedded);
        assert_eq!(cmp.outcome, CompareOutcome::Aligned);
        assert!(cmp.axes.is_empty());
        assert_eq!(cmp.live_pipeline.as_deref(), Some("experimental"));
    }

    #[test]
    fn aligned_when_agent_uses_toml_key_alias() {
        let live = sample_status("verified_in_ci", "verified_in_ci");
        let embedded = TargetCapabilities::for_target("x86_64-pc-windows-msvc");
        let cmp = compare_live_status("x86_64-pc-windows-msvc", &live, &embedded);
        assert_eq!(cmp.outcome, CompareOutcome::Aligned);
    }

    #[test]
    fn drift_when_live_agent_too_weak_for_gate_snapshot() {
        let live = sample_status("experimental", "CI-verified");
        let embedded = TargetCapabilities::for_target("x86_64-pc-windows-msvc");
        let cmp = compare_live_status("x86_64-pc-windows-msvc", &live, &embedded);
        assert_eq!(cmp.outcome, CompareOutcome::Drift);
        assert!(cmp.axes.iter().any(|a| a.contains("gate-usable")));
    }

    #[test]
    fn drift_when_target_missing() {
        let live = sample_status("verified_in_ci", "verified_in_ci");
        let embedded = TargetCapabilities::for_target("aarch64-unknown-linux-gnu");
        let cmp = compare_live_status("aarch64-unknown-linux-gnu", &live, &embedded);
        assert_eq!(cmp.outcome, CompareOutcome::Drift);
        assert!(cmp.axes.iter().any(|a| a.contains("missing")));
    }
}
