//! Fluent Agent Surface — correctness-preserving optimization.
//!
//! Rank sealed candidates by deterministic size/cost metrics **only after**
//! at least one accepted/verified-class candidate exists. Violated / incomplete /
//! failed candidates never win. Fast-level checks are not acceptance for ranking.
//!
//! Metric integrity (H1): never invent `object_file_bytes` from source text;
//! primary metric must be present on every eligible candidate; mixed metric
//! bases are not comparable.

use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::canonical_json::canonical_json_bytes;
use crate::evidence::{
    sha256_digest_prefixed, EvidenceStatus, SealEnvelope, BUNDLE_EVIDENCE, BUNDLE_REPORT,
    BUNDLE_SEAL, BUNDLE_SOURCE, BUNDLE_SOURCE_GAS,
};
use crate::run::{RunDir, RunDirError};

/// Objective / selection-evidence schema version (H1 metric integrity).
pub const OBJECTIVE_SCHEMA_VERSION: &str = "0.2";
/// Prior schema still accepted on load (mapped to 0.2 metric names).
pub const OBJECTIVE_SCHEMA_VERSION_LEGACY: &str = "0.1";
/// On-disk selection evidence filename under the run directory.
pub const SELECTION_EVIDENCE_FILE: &str = "selection-evidence.json";

/// Optimization metric keys (smaller is better for all current metrics).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveMetric {
    /// Assembled object file size only — never derived from source text.
    #[serde(alias = "object_bytes")]
    ObjectFileBytes,
    /// Source assembly text size (explicit; not interchangeable with object).
    SourceTextBytes,
    InstructionCount,
    StackBytes,
}

impl ObjectiveMetric {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ObjectFileBytes => "object_file_bytes",
            Self::SourceTextBytes => "source_text_bytes",
            Self::InstructionCount => "instruction_count",
            Self::StackBytes => "stack_bytes",
        }
    }
}

/// Provenance of a measured metric value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricBasis {
    ObjectFile,
    SourceText,
    InstructionReport,
    StackReport,
}

impl MetricBasis {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::ObjectFile => "object_file",
            Self::SourceText => "source_text",
            Self::InstructionReport => "instruction_report",
            Self::StackReport => "stack_report",
        }
    }
}

/// Parsed optimize objective (`objective.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Objective {
    pub schema_version: String,
    pub primary: ObjectiveMetric,
    #[serde(default)]
    pub secondary: Vec<ObjectiveMetric>,
    pub must_preserve_status: bool,
    pub max_candidates: u32,
}

/// Metrics + correctness for one sealed candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateMetrics {
    pub index: u32,
    pub correctness_status: EvidenceStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_file_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_text_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_bytes: Option<u64>,
    /// Basis for the primary metric when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric_basis: Option<MetricBasis>,
    pub seal_digest: String,
}

/// Winner metrics embedded in selection evidence (`objective` field).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedObjectiveView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_file_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_text_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric_basis: Option<MetricBasis>,
    /// Tool that produced the ranking (binds metric interpretation).
    pub tool_version: String,
}

/// Rejected candidate row in selection evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectedCandidate {
    pub index: u32,
    pub reason: String,
}

/// Sealed selection evidence written to the run directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionEvidence {
    pub schema_version: String,
    pub selected_candidate: u32,
    pub correctness_status: String,
    pub objective: SelectedObjectiveView,
    pub rejected_candidates: Vec<RejectedCandidate>,
    pub objective_digest: String,
    /// Digest of metric values + bases + tool version for the eligible set.
    pub metric_evidence_digest: String,
    pub seal_digest: String,
}

/// Errors from objective parse / ranking / selection.
#[derive(Debug, Error)]
pub enum OptimizeError {
    #[error("{0}")]
    Message(String),
    #[error("I/O on `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    RunDir(#[from] RunDirError),
}

impl OptimizeError {
    fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

/// Load and validate an objective TOML file.
pub fn load_objective(path: &Path) -> Result<Objective, OptimizeError> {
    let raw = fs::read_to_string(path).map_err(|source| OptimizeError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    parse_objective_toml(&raw)
}

/// Parse objective TOML bytes/string and validate.
pub fn parse_objective_toml(raw: &str) -> Result<Objective, OptimizeError> {
    let mut objective: Objective =
        toml::from_str(raw).map_err(|e| OptimizeError::msg(format!("objective TOML: {e}")))?;
    if objective.schema_version == OBJECTIVE_SCHEMA_VERSION_LEGACY {
        objective.schema_version = OBJECTIVE_SCHEMA_VERSION.to_owned();
    }
    validate_objective(&objective)?;
    Ok(objective)
}

/// Fail-closed objective validation (`must_preserve_status` always true).
pub fn validate_objective(objective: &Objective) -> Result<(), OptimizeError> {
    if objective.schema_version != OBJECTIVE_SCHEMA_VERSION {
        return Err(OptimizeError::msg(format!(
            "unsupported objective schema_version `{}` (expected {OBJECTIVE_SCHEMA_VERSION})",
            objective.schema_version
        )));
    }
    if !objective.must_preserve_status {
        return Err(OptimizeError::msg(
            "must_preserve_status must be true (correctness-preserving ranking)",
        ));
    }
    if objective.max_candidates == 0 || objective.max_candidates > 9999 {
        return Err(OptimizeError::msg(
            "max_candidates must be in 1..=9999".to_owned(),
        ));
    }
    Ok(())
}

/// SHA-256 digest of the canonical objective body.
#[must_use]
pub fn objective_digest(objective: &Objective) -> String {
    sha256_digest_prefixed(&canonical_json_bytes(objective))
}

fn tool_version() -> String {
    format!("vaa {}", env!("CARGO_PKG_VERSION"))
}

/// Digest binding eligible metric values, bases, and tool version.
#[must_use]
pub fn metric_evidence_digest(eligible: &[CandidateMetrics], primary: ObjectiveMetric) -> String {
    #[derive(Serialize)]
    struct Row<'a> {
        index: u32,
        primary: &'a str,
        value: Option<u64>,
        basis: Option<&'a str>,
        object_file_bytes: Option<u64>,
        source_text_bytes: Option<u64>,
        instruction_count: Option<u64>,
        stack_bytes: Option<u64>,
    }
    #[derive(Serialize)]
    struct Body<'a> {
        tool_version: String,
        primary: &'a str,
        candidates: Vec<Row<'a>>,
    }
    let body = Body {
        tool_version: tool_version(),
        primary: primary.as_str(),
        candidates: eligible
            .iter()
            .map(|c| Row {
                index: c.index,
                primary: primary.as_str(),
                value: metric_value(c, primary),
                basis: c.metric_basis.as_ref().map(MetricBasis::as_str),
                object_file_bytes: c.object_file_bytes,
                source_text_bytes: c.source_text_bytes,
                instruction_count: c.instruction_count,
                stack_bytes: c.stack_bytes,
            })
            .collect(),
    };
    sha256_digest_prefixed(&canonical_json_bytes(&body))
}

/// Human / JSON label for an evidence status (snake_case).
#[must_use]
pub fn status_label(status: EvidenceStatus) -> &'static str {
    match status {
        EvidenceStatus::Verified => "verified",
        EvidenceStatus::VerifiedUnderPreconditions => "verified_under_preconditions",
        EvidenceStatus::Violated => "violated",
        EvidenceStatus::Incomplete => "incomplete",
        EvidenceStatus::Failed => "failed",
    }
}

fn rejection_reason(status: EvidenceStatus) -> &'static str {
    match status {
        EvidenceStatus::Violated => "behavior_failed",
        EvidenceStatus::Incomplete => "incomplete",
        EvidenceStatus::Failed => "failed",
        EvidenceStatus::VerifiedUnderPreconditions => "verified_under_preconditions_not_allowed",
        EvidenceStatus::Verified => "not_selected",
    }
}

fn is_accepted(status: EvidenceStatus, allow_under_preconditions: bool) -> bool {
    match status {
        EvidenceStatus::Verified => true,
        EvidenceStatus::VerifiedUnderPreconditions => allow_under_preconditions,
        EvidenceStatus::Violated | EvidenceStatus::Incomplete | EvidenceStatus::Failed => false,
    }
}

fn metric_value(c: &CandidateMetrics, metric: ObjectiveMetric) -> Option<u64> {
    match metric {
        ObjectiveMetric::ObjectFileBytes => c.object_file_bytes,
        ObjectiveMetric::SourceTextBytes => c.source_text_bytes,
        ObjectiveMetric::InstructionCount => c.instruction_count,
        ObjectiveMetric::StackBytes => c.stack_bytes,
    }
}

fn primary_basis(c: &CandidateMetrics, primary: ObjectiveMetric) -> Option<MetricBasis> {
    match primary {
        ObjectiveMetric::ObjectFileBytes => c.object_file_bytes.map(|_| MetricBasis::ObjectFile),
        ObjectiveMetric::SourceTextBytes => c.source_text_bytes.map(|_| MetricBasis::SourceText),
        ObjectiveMetric::InstructionCount => {
            c.instruction_count.map(|_| MetricBasis::InstructionReport)
        }
        ObjectiveMetric::StackBytes => c.stack_bytes.map(|_| MetricBasis::StackReport),
    }
}

/// Compare two optional metric values: both must be present (callers gate missing).
fn cmp_metric(a: Option<u64>, b: Option<u64>) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.cmp(&y),
        _ => Ordering::Equal,
    }
}

fn compare_candidates(
    a: &CandidateMetrics,
    b: &CandidateMetrics,
    objective: &Objective,
) -> Ordering {
    let primary = cmp_metric(
        metric_value(a, objective.primary),
        metric_value(b, objective.primary),
    );
    if primary != Ordering::Equal {
        return primary;
    }
    for metric in &objective.secondary {
        // Secondary only ranks when both sides have the metric; never invent ties from missing.
        match (metric_value(a, *metric), metric_value(b, *metric)) {
            (Some(x), Some(y)) if x != y => return x.cmp(&y),
            (Some(_), Some(_)) => {}
            _ => {}
        }
    }
    // Stable: lower index wins on full tie.
    a.index.cmp(&b.index)
}

/// Rank sealed candidates; never selects violated/incomplete/failed.
///
/// Requires ≥1 accepted/verified-class candidate with the primary metric present.
/// Fast checks are not considered (only sealed candidates with evidence).
pub fn rank_candidates(
    objective: &Objective,
    candidates: &[CandidateMetrics],
    allow_under_preconditions: bool,
) -> Result<SelectionEvidence, OptimizeError> {
    validate_objective(objective)?;

    let mut rejected = Vec::new();
    let mut eligible = Vec::new();

    for c in candidates.iter().take(objective.max_candidates as usize) {
        if !is_accepted(c.correctness_status, allow_under_preconditions) {
            rejected.push(RejectedCandidate {
                index: c.index,
                reason: rejection_reason(c.correctness_status).to_owned(),
            });
            continue;
        }
        if metric_value(c, objective.primary).is_none() {
            rejected.push(RejectedCandidate {
                index: c.index,
                reason: "missing_primary_metric".into(),
            });
            continue;
        }
        eligible.push(c.clone());
    }

    // Candidates beyond max_candidates are rejected explicitly when present.
    for c in candidates.iter().skip(objective.max_candidates as usize) {
        rejected.push(RejectedCandidate {
            index: c.index,
            reason: "beyond_max_candidates".into(),
        });
    }

    if eligible.is_empty() {
        return Err(OptimizeError::msg(
            "selection incomplete: no accepted candidate with primary metric \
             (object_file_bytes is never inferred from source)",
        ));
    }

    // All eligible primaries must share the same metric basis.
    let first_basis = primary_basis(&eligible[0], objective.primary)
        .ok_or_else(|| OptimizeError::msg("internal: eligible missing primary basis"))?;
    for c in eligible.iter().skip(1) {
        let basis = primary_basis(c, objective.primary)
            .ok_or_else(|| OptimizeError::msg("internal: eligible missing primary basis"))?;
        if basis != first_basis {
            return Err(OptimizeError::msg(format!(
                "selection incomplete: mixed primary metric bases (`{}` vs `{}`); \
                 refuse to compare unlike quantities",
                first_basis.as_str(),
                basis.as_str()
            )));
        }
    }

    eligible.sort_by(|a, b| compare_candidates(a, b, objective));
    let winner = &eligible[0];
    let med = metric_evidence_digest(&eligible, objective.primary);

    // Record other accepted candidates that lost the ranking.
    for c in eligible.iter().skip(1) {
        rejected.push(RejectedCandidate {
            index: c.index,
            reason: "not_selected".into(),
        });
    }
    rejected.sort_by_key(|r| r.index);

    Ok(SelectionEvidence {
        schema_version: OBJECTIVE_SCHEMA_VERSION.to_owned(),
        selected_candidate: winner.index,
        correctness_status: status_label(winner.correctness_status).to_owned(),
        objective: SelectedObjectiveView {
            object_file_bytes: winner.object_file_bytes,
            source_text_bytes: winner.source_text_bytes,
            instruction_count: winner.instruction_count,
            stack_bytes: winner.stack_bytes,
            metric_basis: winner.metric_basis.clone(),
            tool_version: tool_version(),
        },
        rejected_candidates: rejected,
        objective_digest: objective_digest(objective),
        metric_evidence_digest: med,
        seal_digest: winner.seal_digest.clone(),
    })
}

/// Scan a run directory for sealed candidates and compute deterministic metrics.
pub fn collect_candidate_metrics(run_dir: &RunDir) -> Result<Vec<CandidateMetrics>, OptimizeError> {
    let cursor = run_dir.resume_cursor()?;
    let mut out = Vec::with_capacity(cursor.next_candidate_index as usize);
    for index in 0..cursor.next_candidate_index {
        let dir = run_dir.candidate_dir(index)?;
        out.push(metrics_for_candidate_dir(index, &dir)?);
    }
    Ok(out)
}

/// Compute metrics for one sealed candidate directory.
pub fn metrics_for_candidate_dir(
    index: u32,
    candidate_dir: &Path,
) -> Result<CandidateMetrics, OptimizeError> {
    let seal_path = candidate_dir.join(BUNDLE_SEAL);
    let seal_raw = fs::read_to_string(&seal_path).map_err(|source| OptimizeError::Io {
        path: seal_path.clone(),
        source,
    })?;
    let seal: SealEnvelope = serde_json::from_str(&seal_raw)
        .map_err(|e| OptimizeError::msg(format!("candidate {index:04} seal JSON: {e}")))?;

    // Cross-check evidence.json final_status when present (full verification evidence).
    let evidence_path = candidate_dir.join(BUNDLE_EVIDENCE);
    if evidence_path.is_file() {
        let evidence_raw =
            fs::read_to_string(&evidence_path).map_err(|source| OptimizeError::Io {
                path: evidence_path.clone(),
                source,
            })?;
        let evidence: Value = serde_json::from_str(&evidence_raw)
            .map_err(|e| OptimizeError::msg(format!("candidate {index:04} evidence JSON: {e}")))?;
        if let Some(status) = evidence.get("final_status") {
            let parsed = parse_status_value(status).ok_or_else(|| {
                OptimizeError::msg(format!(
                    "candidate {index:04}: unrecognized evidence.final_status"
                ))
            })?;
            if parsed != seal.acceptance.final_status {
                return Err(OptimizeError::msg(format!(
                    "candidate {index:04}: evidence.final_status != seal.acceptance.final_status"
                )));
            }
        }
    }

    let object_file_bytes = resolve_object_file_bytes(candidate_dir)?;
    let source_text_bytes = resolve_source_text_bytes(candidate_dir)?;
    let report = load_report_json(candidate_dir);
    let instruction_count = report.as_ref().and_then(extract_instruction_count);
    let stack_bytes = report.as_ref().and_then(extract_stack_bytes);

    let metric_basis = if object_file_bytes.is_some() {
        Some(MetricBasis::ObjectFile)
    } else if instruction_count.is_some() {
        Some(MetricBasis::InstructionReport)
    } else if stack_bytes.is_some() {
        Some(MetricBasis::StackReport)
    } else if source_text_bytes.is_some() {
        Some(MetricBasis::SourceText)
    } else {
        None
    };

    Ok(CandidateMetrics {
        index,
        correctness_status: seal.acceptance.final_status,
        object_file_bytes,
        source_text_bytes,
        instruction_count,
        stack_bytes,
        metric_basis,
        seal_digest: seal.envelope_digest,
    })
}

fn parse_status_value(value: &Value) -> Option<EvidenceStatus> {
    let s = value.as_str()?;
    match s {
        "Verified" | "verified" => Some(EvidenceStatus::Verified),
        "VerifiedUnderPreconditions" | "verified_under_preconditions" => {
            Some(EvidenceStatus::VerifiedUnderPreconditions)
        }
        "Violated" | "violated" => Some(EvidenceStatus::Violated),
        "Incomplete" | "incomplete" => Some(EvidenceStatus::Incomplete),
        "Failed" | "failed" => Some(EvidenceStatus::Failed),
        _ => None,
    }
}

/// Object file size only — never falls back to source text.
fn resolve_object_file_bytes(candidate_dir: &Path) -> Result<Option<u64>, OptimizeError> {
    for name in ["candidate.o", "candidate.obj", "object.o"] {
        let path = candidate_dir.join(name);
        if path.is_file() {
            let meta = fs::metadata(&path).map_err(|source| OptimizeError::Io {
                path: path.clone(),
                source,
            })?;
            return Ok(Some(meta.len()));
        }
    }
    Ok(None)
}

fn resolve_source_text_bytes(candidate_dir: &Path) -> Result<Option<u64>, OptimizeError> {
    let source_path = if candidate_dir.join(BUNDLE_SOURCE).is_file() {
        candidate_dir.join(BUNDLE_SOURCE)
    } else if candidate_dir.join(BUNDLE_SOURCE_GAS).is_file() {
        candidate_dir.join(BUNDLE_SOURCE_GAS)
    } else {
        return Ok(None);
    };
    let meta = fs::metadata(&source_path).map_err(|source| OptimizeError::Io {
        path: source_path,
        source,
    })?;
    Ok(Some(meta.len()))
}

fn load_report_json(candidate_dir: &Path) -> Option<Value> {
    let path = candidate_dir.join(BUNDLE_REPORT);
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Best-effort instruction count from SemASM report / decode coverage.
fn extract_instruction_count(report: &Value) -> Option<u64> {
    for path in [
        &["instruction_count"][..],
        &["metrics", "instruction_count"],
        &["semantic", "instruction_count"],
        &["semantic", "decode", "total"],
        &["semantic", "decode", "modeled"],
        &["decode", "total"],
    ] {
        if let Some(n) = dig_u64(report, path) {
            return Some(n);
        }
    }
    None
}

/// Best-effort stack bytes from SemASM report when present.
fn extract_stack_bytes(report: &Value) -> Option<u64> {
    for path in [
        &["stack_bytes"][..],
        &["metrics", "stack_bytes"],
        &["semantic", "stack_bytes"],
        &["semantic", "max_stack_bytes"],
        &["memory", "stack_bytes"],
    ] {
        if let Some(n) = dig_u64(report, path) {
            return Some(n);
        }
    }
    None
}

fn dig_u64(root: &Value, path: &[&str]) -> Option<u64> {
    let mut cur = root;
    for key in path {
        cur = cur.get(*key)?;
    }
    if let Some(n) = cur.as_u64() {
        return Some(n);
    }
    if let Some(n) = cur.as_i64() {
        if n >= 0 {
            return Some(n as u64);
        }
    }
    None
}

/// Rank sealed candidates in `run_dir` and write `selection-evidence.json`.
pub fn rank_run_dir(
    run_dir_path: &Path,
    objective: &Objective,
    allow_under_preconditions: bool,
) -> Result<(SelectionEvidence, PathBuf), OptimizeError> {
    validate_objective(objective)?;
    let run_dir = RunDir::open(run_dir_path)?;
    let candidates = collect_candidate_metrics(&run_dir)?;
    if candidates.is_empty() {
        return Err(OptimizeError::msg(
            "no sealed candidates in run directory (optimize requires ≥1 accepted candidate)",
        ));
    }
    let evidence = rank_candidates(objective, &candidates, allow_under_preconditions)?;
    let out_path = run_dir.root().join(SELECTION_EVIDENCE_FILE);
    let body = serde_json::to_vec_pretty(&evidence)
        .map_err(|e| OptimizeError::msg(format!("serialize selection evidence: {e}")))?;
    fs::write(&out_path, body).map_err(|source| OptimizeError::Io {
        path: out_path.clone(),
        source,
    })?;
    Ok((evidence, out_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn objective_object_file_bytes() -> Objective {
        Objective {
            schema_version: OBJECTIVE_SCHEMA_VERSION.to_owned(),
            primary: ObjectiveMetric::ObjectFileBytes,
            secondary: vec![
                ObjectiveMetric::InstructionCount,
                ObjectiveMetric::StackBytes,
            ],
            must_preserve_status: true,
            max_candidates: 4,
        }
    }

    fn cand(
        index: u32,
        status: EvidenceStatus,
        object_file_bytes: Option<u64>,
        insn: Option<u64>,
    ) -> CandidateMetrics {
        CandidateMetrics {
            index,
            correctness_status: status,
            object_file_bytes,
            source_text_bytes: None,
            instruction_count: insn,
            stack_bytes: None,
            metric_basis: object_file_bytes.map(|_| MetricBasis::ObjectFile),
            seal_digest: format!("sha256:seal{index}"),
        }
    }

    #[test]
    fn parse_fixture_objective() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/optimize/objective.object_bytes.toml");
        let obj = load_objective(&path).expect("fixture objective");
        assert_eq!(obj.primary, ObjectiveMetric::ObjectFileBytes);
        assert!(obj.must_preserve_status);
        assert_eq!(obj.max_candidates, 4);
        assert_eq!(obj.schema_version, OBJECTIVE_SCHEMA_VERSION);
    }

    #[test]
    fn accepts_legacy_object_bytes_alias() {
        let raw = r#"
schema_version = "0.1"
primary = "object_bytes"
secondary = []
must_preserve_status = true
max_candidates = 2
"#;
        let obj = parse_objective_toml(raw).expect("legacy");
        assert_eq!(obj.primary, ObjectiveMetric::ObjectFileBytes);
        assert_eq!(obj.schema_version, OBJECTIVE_SCHEMA_VERSION);
    }

    #[test]
    fn rejects_must_preserve_status_false() {
        let raw = r#"
schema_version = "0.2"
primary = "object_file_bytes"
secondary = []
must_preserve_status = false
max_candidates = 2
"#;
        let err = parse_objective_toml(raw).expect_err("must reject");
        assert!(err.to_string().contains("must_preserve_status"));
    }

    #[test]
    fn violated_smaller_loses_to_larger_verified() {
        let objective = objective_object_file_bytes();
        let candidates = vec![
            cand(1, EvidenceStatus::Violated, Some(10), Some(1)),
            cand(2, EvidenceStatus::Verified, Some(99), Some(50)),
        ];
        let sel = rank_candidates(&objective, &candidates, false).expect("rank");
        assert_eq!(sel.selected_candidate, 2);
        assert_eq!(sel.correctness_status, "verified");
        assert_eq!(sel.objective.object_file_bytes, Some(99));
        assert!(sel
            .rejected_candidates
            .iter()
            .any(|r| r.index == 1 && r.reason == "behavior_failed"));
        assert!(!sel.metric_evidence_digest.is_empty());
    }

    #[test]
    fn no_accepted_returns_error() {
        let objective = objective_object_file_bytes();
        let candidates = vec![
            cand(0, EvidenceStatus::Violated, Some(5), None),
            cand(1, EvidenceStatus::Failed, Some(3), None),
            cand(2, EvidenceStatus::Incomplete, Some(1), None),
        ];
        let err = rank_candidates(&objective, &candidates, false).expect_err("no accepted");
        assert!(err.to_string().contains("selection incomplete"));
    }

    #[test]
    fn missing_object_file_bytes_not_eligible() {
        let objective = objective_object_file_bytes();
        let candidates = vec![
            cand(0, EvidenceStatus::Verified, None, Some(1)), // source-only / no object
            cand(1, EvidenceStatus::Verified, Some(90), Some(2)),
        ];
        let sel = rank_candidates(&objective, &candidates, false).expect("rank");
        assert_eq!(sel.selected_candidate, 1);
        assert!(sel
            .rejected_candidates
            .iter()
            .any(|r| r.index == 0 && r.reason == "missing_primary_metric"));
    }

    #[test]
    fn all_missing_primary_is_incomplete() {
        let objective = objective_object_file_bytes();
        let candidates = vec![
            cand(0, EvidenceStatus::Verified, None, None),
            cand(1, EvidenceStatus::Verified, None, Some(3)),
        ];
        let err = rank_candidates(&objective, &candidates, false).expect_err("incomplete");
        assert!(err.to_string().contains("selection incomplete"));
    }

    #[test]
    fn source_text_primary_uses_source_metric() {
        let objective = Objective {
            schema_version: OBJECTIVE_SCHEMA_VERSION.to_owned(),
            primary: ObjectiveMetric::SourceTextBytes,
            secondary: vec![],
            must_preserve_status: true,
            max_candidates: 4,
        };
        let candidates = vec![
            CandidateMetrics {
                index: 0,
                correctness_status: EvidenceStatus::Verified,
                object_file_bytes: Some(90),
                source_text_bytes: Some(70),
                instruction_count: None,
                stack_bytes: None,
                metric_basis: Some(MetricBasis::SourceText),
                seal_digest: "sha256:a".into(),
            },
            CandidateMetrics {
                index: 1,
                correctness_status: EvidenceStatus::Verified,
                object_file_bytes: Some(40),
                source_text_bytes: Some(100),
                instruction_count: None,
                stack_bytes: None,
                metric_basis: Some(MetricBasis::SourceText),
                seal_digest: "sha256:b".into(),
            },
        ];
        let sel = rank_candidates(&objective, &candidates, false).expect("rank");
        assert_eq!(sel.selected_candidate, 0);
        assert_eq!(sel.objective.source_text_bytes, Some(70));
    }

    #[test]
    fn primary_object_file_bytes_ranking() {
        let objective = objective_object_file_bytes();
        let candidates = vec![
            cand(0, EvidenceStatus::Verified, Some(80), Some(10)),
            cand(1, EvidenceStatus::Verified, Some(40), Some(20)),
            cand(2, EvidenceStatus::Verified, Some(60), Some(5)),
        ];
        let sel = rank_candidates(&objective, &candidates, false).expect("rank");
        assert_eq!(sel.selected_candidate, 1);
        assert_eq!(sel.objective.object_file_bytes, Some(40));
    }

    #[test]
    fn vup_requires_explicit_flag() {
        let objective = objective_object_file_bytes();
        let candidates = vec![cand(
            0,
            EvidenceStatus::VerifiedUnderPreconditions,
            Some(10),
            None,
        )];
        assert!(rank_candidates(&objective, &candidates, false).is_err());
        let sel = rank_candidates(&objective, &candidates, true).expect("allow VUP");
        assert_eq!(sel.selected_candidate, 0);
        assert_eq!(sel.correctness_status, "verified_under_preconditions");
    }

    #[test]
    fn secondary_breaks_primary_tie() {
        let objective = Objective {
            schema_version: OBJECTIVE_SCHEMA_VERSION.to_owned(),
            primary: ObjectiveMetric::ObjectFileBytes,
            secondary: vec![ObjectiveMetric::InstructionCount],
            must_preserve_status: true,
            max_candidates: 4,
        };
        let candidates = vec![
            cand(0, EvidenceStatus::Verified, Some(40), Some(30)),
            cand(1, EvidenceStatus::Verified, Some(40), Some(10)),
        ];
        let sel = rank_candidates(&objective, &candidates, false).expect("rank");
        assert_eq!(sel.selected_candidate, 1);
    }

    #[test]
    fn extract_decode_coverage_as_instruction_count() {
        let report = serde_json::json!({
            "semantic": { "decode": { "total": 10, "modeled": 10 } }
        });
        assert_eq!(extract_instruction_count(&report), Some(10));
        assert_eq!(extract_stack_bytes(&report), None);
    }
}
