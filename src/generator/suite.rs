//! Suite manifest load/validate and suite runner aggregation.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::canonical_json::canonical_json_bytes;
use crate::generator::error::GeneratorError;
use crate::generator::run::{run_generator_case, GeneratorRunConfig, GeneratorRunError};
use crate::generator::stack_lock::{load_stack_lock, stack_lock_digest};

/// Accepted suite schema version.
pub const SUITE_SCHEMA_VERSION: &str = "0.1";

/// Suite-level status (plan §9.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuiteStatus {
    Accepted,
    Rejected,
    Incomplete,
    Failed,
}

/// Policy for aggregating case outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuitePolicy {
    #[serde(default = "default_true")]
    pub require_all_cases: bool,
    #[serde(default)]
    pub allow_verified_under_preconditions: bool,
    #[serde(default)]
    pub allow_incomplete: bool,
    #[serde(default)]
    pub stop_on_first_failure: bool,
    #[serde(default = "default_parallel")]
    pub max_parallel_cases: u32,
}

fn default_true() -> bool {
    true
}

fn default_parallel() -> u32 {
    1
}

impl Default for SuitePolicy {
    fn default() -> Self {
        Self {
            require_all_cases: true,
            allow_verified_under_preconditions: false,
            allow_incomplete: false,
            stop_on_first_failure: false,
            max_parallel_cases: 1,
        }
    }
}

/// Generator reference inside a suite manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuiteGeneratorRef {
    /// Path to `ExternalGeneratorSpec` (relative to suite file).
    pub spec: String,
}

/// Typed suite manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuiteManifest {
    pub schema_version: String,
    pub suite_id: String,
    pub target: String,
    /// Calling convention / ABI label (`win64`, `sysv`, …). Optional for
    /// backward compatibility with early manifests; when set, case tasks
    /// must match (plan §17 / P3.16 target/ABI parity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abi: Option<String>,
    pub generator: SuiteGeneratorRef,
    #[serde(default)]
    pub policy: SuitePolicy,
    /// Case directories relative to the suite file.
    pub required_cases: Vec<String>,
    /// Optional stack lock path relative to the suite file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack_lock: Option<String>,
}

/// Per-case files discovered under a case directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CasePaths {
    pub case_dir: PathBuf,
    pub task: PathBuf,
    pub contract: PathBuf,
    pub input: PathBuf,
}

/// One case outcome inside a suite run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuiteCaseResult {
    pub case_id: String,
    pub case_dir: PathBuf,
    /// Normalized status used for aggregation (`accepted`, `rejected`, …).
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Suite evidence digest bindings (plan §9.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuiteEvidence {
    pub schema_version: String,
    pub suite_id: String,
    pub suite_digest: String,
    pub status: SuiteStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack_lock_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator_binary_digest: Option<String>,
    pub cases: Vec<SuiteCaseResult>,
}

/// Full suite run report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteRunReport {
    pub evidence: SuiteEvidence,
    pub manifest_path: PathBuf,
}

/// Load suite manifest and validate.
pub fn load_suite_manifest(path: impl AsRef<Path>) -> Result<SuiteManifest, GeneratorError> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path).map_err(|source| GeneratorError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let suite = parse_suite_manifest(path, &text)?;
    let diagnostics = validate_suite_manifest(&suite);
    if !diagnostics.is_empty() {
        return Err(GeneratorError::from_diagnostics(&diagnostics));
    }
    Ok(suite)
}

/// Parse suite TOML.
pub fn parse_suite_manifest(path: &Path, text: &str) -> Result<SuiteManifest, GeneratorError> {
    toml::from_str::<SuiteManifest>(text).map_err(|error| GeneratorError::Parse {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

/// Validate suite schema and required fields.
#[must_use]
pub fn validate_suite_manifest(suite: &SuiteManifest) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if suite.schema_version != SUITE_SCHEMA_VERSION {
        diagnostics.push(format!(
            "unsupported schema_version `{}` (accepts only `{SUITE_SCHEMA_VERSION}`)",
            suite.schema_version
        ));
    }
    if suite.suite_id.trim().is_empty() {
        diagnostics.push("suite_id must not be empty".to_owned());
    }
    if suite.target.trim().is_empty() {
        diagnostics.push("target must not be empty".to_owned());
    }
    if let Some(abi) = &suite.abi {
        if abi.trim().is_empty() {
            diagnostics.push("abi must not be empty when present".to_owned());
        }
    }
    if suite.generator.spec.trim().is_empty() {
        diagnostics.push("generator.spec must not be empty".to_owned());
    }
    if suite.required_cases.is_empty() {
        diagnostics.push("required_cases must contain at least one case".to_owned());
    }
    for (i, case) in suite.required_cases.iter().enumerate() {
        if case.trim().is_empty() {
            diagnostics.push(format!("required_cases[{i}] is empty"));
        }
    }
    if suite.policy.max_parallel_cases == 0 {
        diagnostics.push("policy.max_parallel_cases must be > 0".to_owned());
    }
    diagnostics
}

/// Digest over canonical JSON of the suite manifest.
#[must_use]
pub fn suite_manifest_digest(suite: &SuiteManifest) -> String {
    let hash = Sha256::digest(canonical_json_bytes(suite));
    format!("sha256:{}", hex_encode(&hash))
}

/// Resolve case directory layout (defaults + optional `case.toml` overrides).
pub fn resolve_case_paths(case_dir: &Path) -> Result<CasePaths, GeneratorError> {
    if !case_dir.is_dir() {
        return Err(GeneratorError::Validation(format!(
            "case directory not found: {}",
            case_dir.display()
        )));
    }

    let mut task = case_dir.join("task.vaa.toml");
    let mut contract = case_dir.join("contract.sem.toml");
    let mut input = find_default_input(case_dir);

    let case_meta = case_dir.join("case.toml");
    if case_meta.is_file() {
        let text = std::fs::read_to_string(&case_meta).map_err(|source| GeneratorError::Io {
            path: case_meta.clone(),
            source,
        })?;
        let meta: CaseMetaFile = toml::from_str(&text).map_err(|error| GeneratorError::Parse {
            path: case_meta,
            message: error.to_string(),
        })?;
        if let Some(t) = meta.task {
            task = case_dir.join(t);
        }
        if let Some(c) = meta.contract {
            contract = case_dir.join(c);
        }
        if let Some(i) = meta.input {
            input = Some(case_dir.join(i));
        }
    }

    let input = input.ok_or_else(|| {
        GeneratorError::Validation(format!(
            "no input file found in case `{}` (expected input.* or case.toml input=)",
            case_dir.display()
        ))
    })?;

    for (label, path) in [("task", &task), ("contract", &contract), ("input", &input)] {
        if !path.is_file() {
            return Err(GeneratorError::Validation(format!(
                "case {}: missing {label} at {}",
                case_dir.display(),
                path.display()
            )));
        }
    }

    Ok(CasePaths {
        case_dir: case_dir.to_path_buf(),
        task,
        contract,
        input,
    })
}

#[derive(Debug, Deserialize)]
struct CaseMetaFile {
    #[serde(default)]
    task: Option<String>,
    #[serde(default)]
    contract: Option<String>,
    #[serde(default)]
    input: Option<String>,
}

fn find_default_input(case_dir: &Path) -> Option<PathBuf> {
    const CANDIDATES: &[&str] = &[
        "input.hla64",
        "input.hlx",
        "input.s",
        "input.asm",
        "input.txt",
        "program.ir",
    ];
    for name in CANDIDATES {
        let p = case_dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

/// Minimal task fields needed for target/ABI parity checks.
#[derive(Debug, Deserialize)]
struct TaskParitySlice {
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    entry: Option<TaskEntrySlice>,
}

#[derive(Debug, Deserialize)]
struct TaskEntrySlice {
    #[serde(default)]
    abi: Option<String>,
}

/// One case parity observation (suite vs task).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseParityReport {
    pub case_id: String,
    pub case_dir: PathBuf,
    pub task_target: Option<String>,
    pub task_abi: Option<String>,
    pub ok: bool,
    pub diagnostics: Vec<String>,
}

/// Full suite parity report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuiteParityReport {
    pub suite_id: String,
    pub suite_target: String,
    pub suite_abi: Option<String>,
    pub cases: Vec<CaseParityReport>,
    pub ok: bool,
}

/// Known first-cut target/ABI profiles for pack docs and validation hints.
#[must_use]
pub fn known_target_abi_profiles() -> &'static [(&'static str, &'static str)] {
    &[
        ("x86_64-pc-windows-msvc", "win64"),
        ("x86_64-unknown-linux-gnu", "sysv"),
    ]
}

/// Check that every required case task matches the suite `target` / `abi`.
///
/// When suite `abi` is unset, only target is checked (when the task declares
/// one). Missing task files surface as diagnostics — never silently skipped.
pub fn check_suite_target_abi_parity(
    suite: &SuiteManifest,
    suite_dir: &Path,
) -> Result<SuiteParityReport, GeneratorError> {
    let mut cases = Vec::with_capacity(suite.required_cases.len());
    for rel in &suite.required_cases {
        let case_dir = suite_dir.join(rel);
        let case_id = case_id_from_path(rel);
        let paths = match resolve_case_paths(&case_dir) {
            Ok(p) => p,
            Err(error) => {
                cases.push(CaseParityReport {
                    case_id,
                    case_dir,
                    task_target: None,
                    task_abi: None,
                    ok: false,
                    diagnostics: vec![error.to_string()],
                });
                continue;
            }
        };
        let text = std::fs::read_to_string(&paths.task).map_err(|source| GeneratorError::Io {
            path: paths.task.clone(),
            source,
        })?;
        let task: TaskParitySlice =
            toml::from_str(&text).map_err(|error| GeneratorError::Parse {
                path: paths.task.clone(),
                message: error.to_string(),
            })?;
        let task_target = task.target.filter(|t| !t.trim().is_empty());
        let task_abi = task
            .entry
            .and_then(|e| e.abi)
            .filter(|a| !a.trim().is_empty());
        let mut diagnostics = Vec::new();
        if let Some(ref tt) = task_target {
            if tt != &suite.target {
                diagnostics.push(format!(
                    "task target `{tt}` != suite target `{}`",
                    suite.target
                ));
            }
        } else {
            diagnostics.push("task does not declare `target`".to_owned());
        }
        match (&suite.abi, &task_abi) {
            (Some(suite_abi), Some(ta)) if ta != suite_abi => {
                diagnostics.push(format!("task entry.abi `{ta}` != suite abi `{suite_abi}`"));
            }
            (Some(suite_abi), None) => {
                diagnostics.push(format!(
                    "suite declares abi `{suite_abi}` but task has no `[entry].abi`"
                ));
            }
            _ => {}
        }
        cases.push(CaseParityReport {
            case_id,
            case_dir,
            task_target,
            task_abi,
            ok: diagnostics.is_empty(),
            diagnostics,
        });
    }
    let ok = cases.iter().all(|c| c.ok);
    Ok(SuiteParityReport {
        suite_id: suite.suite_id.clone(),
        suite_target: suite.target.clone(),
        suite_abi: suite.abi.clone(),
        cases,
        ok,
    })
}

/// Load suite + check target/ABI parity against resolved case tasks.
pub fn check_suite_parity_file(
    suite_path: impl AsRef<Path>,
) -> Result<SuiteParityReport, GeneratorError> {
    let suite_path = suite_path.as_ref();
    let suite = load_suite_manifest(suite_path)?;
    let suite_dir = suite_path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    check_suite_target_abi_parity(&suite, &suite_dir)
}

/// Aggregate case results into a suite status (pure; unit-tested).
#[must_use]
pub fn aggregate_suite_status(cases: &[SuiteCaseResult], policy: &SuitePolicy) -> SuiteStatus {
    if cases.is_empty() {
        return SuiteStatus::Failed;
    }

    let mut saw_incomplete = false;
    let mut saw_rejected = false;
    let mut saw_failed = false;

    for case in cases {
        match classify_case_bucket(&case.status, policy) {
            CaseBucket::Accepted => {}
            CaseBucket::Rejected => saw_rejected = true,
            CaseBucket::Incomplete => saw_incomplete = true,
            CaseBucket::Failed => saw_failed = true,
        }
    }

    if saw_failed {
        return SuiteStatus::Failed;
    }
    if saw_rejected {
        return SuiteStatus::Rejected;
    }
    if saw_incomplete {
        if policy.allow_incomplete {
            // Still incomplete unless every case is accepted-like.
            return SuiteStatus::Incomplete;
        }
        return SuiteStatus::Incomplete;
    }
    SuiteStatus::Accepted
}

#[derive(Debug, Clone, Copy)]
enum CaseBucket {
    Accepted,
    Rejected,
    Incomplete,
    Failed,
}

fn classify_case_bucket(status: &str, policy: &SuitePolicy) -> CaseBucket {
    let normalized = status
        .trim()
        .trim_matches('"')
        .replace(['_', ' '], "")
        .to_ascii_lowercase();

    if normalized.contains("toolchain")
        || normalized.contains("identitymismatch")
        || normalized.contains("schemamismatch")
        || normalized == "failed"
    {
        return CaseBucket::Failed;
    }

    if normalized.contains("violat")
        || normalized == "rejected"
        || normalized.contains("behaviorfailed")
    {
        return CaseBucket::Rejected;
    }

    if normalized.contains("incomplete")
        || normalized.contains("missing")
        || normalized == "generated"
        || normalized.contains("skipped")
    {
        return CaseBucket::Incomplete;
    }

    if normalized.contains("verifiedunderpreconditions") {
        if policy.allow_verified_under_preconditions {
            return CaseBucket::Accepted;
        }
        return CaseBucket::Incomplete;
    }

    if normalized.contains("verified") || normalized == "accepted" || normalized.contains("pass") {
        return CaseBucket::Accepted;
    }

    // Unknown status → fail closed as incomplete.
    CaseBucket::Incomplete
}

/// Options for executing a suite.
#[derive(Debug, Clone)]
pub struct SuiteRunConfig {
    pub suite_path: PathBuf,
    pub repo_override: Option<PathBuf>,
    pub run_base: PathBuf,
    pub skip_repo_guard: bool,
    pub skip_build: bool,
    pub skip_verify: bool,
    pub allow_execution: bool,
    pub check_deterministic: bool,
}

/// Run all required cases and build suite evidence.
pub fn run_suite(config: &SuiteRunConfig) -> Result<SuiteRunReport, GeneratorRunError> {
    let suite = load_suite_manifest(&config.suite_path)?;
    let suite_dir = config.suite_path.parent().unwrap_or_else(|| Path::new("."));
    let spec_path = suite_dir.join(&suite.generator.spec);
    let lock_path = suite
        .stack_lock
        .as_ref()
        .map(|p| suite_dir.join(p))
        .filter(|p| p.is_file());

    let mut stack_lock_digest_value = None;
    if let Some(ref lock) = lock_path {
        let lock = load_stack_lock(lock)?;
        stack_lock_digest_value = Some(stack_lock_digest(&lock).prefixed());
    }

    let mut cases = Vec::new();
    let mut generator_binary_digest = None;
    let mut stop = false;

    for rel in &suite.required_cases {
        if stop {
            cases.push(SuiteCaseResult {
                case_id: case_id_from_path(rel),
                case_dir: suite_dir.join(rel),
                status: "skipped".to_owned(),
                acceptance_digest: None,
                candidate_digest: None,
                error: Some("stopped_on_first_failure".to_owned()),
            });
            continue;
        }

        let case_dir = suite_dir.join(rel);
        let paths = match resolve_case_paths(&case_dir) {
            Ok(p) => p,
            Err(e) => {
                let result = SuiteCaseResult {
                    case_id: case_id_from_path(rel),
                    case_dir: case_dir.clone(),
                    status: "failed".to_owned(),
                    acceptance_digest: None,
                    candidate_digest: None,
                    error: Some(e.to_string()),
                };
                if suite.policy.stop_on_first_failure {
                    stop = true;
                }
                cases.push(result);
                continue;
            }
        };

        let output = config
            .run_base
            .join("suite-out")
            .join(case_id_from_path(rel))
            .join("candidate.asm");
        if let Some(parent) = output.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let run_cfg = GeneratorRunConfig {
            spec_path: spec_path.clone(),
            lock_path: lock_path.clone(),
            task_path: paths.task,
            contract_path: paths.contract,
            input_path: paths.input,
            output_path: output,
            run_base: config
                .run_base
                .join("suite-runs")
                .join(case_id_from_path(rel)),
            repo_override: config.repo_override.clone(),
            skip_repo_guard: config.skip_repo_guard,
            skip_build: config.skip_build || !cases.is_empty(),
            skip_verify: config.skip_verify,
            allow_execution: config.allow_execution,
            check_deterministic: config.check_deterministic,
            target_override: Some(suite.target.clone()),
        };

        match run_generator_case(&run_cfg) {
            Ok(outcome) => {
                generator_binary_digest = Some(outcome.identity.digest.clone());
                let (status, acceptance) = if let Some(v) = &outcome.verify {
                    (v.final_status.clone(), Some(v.acceptance_digest.clone()))
                } else {
                    ("incomplete".to_owned(), None)
                };
                let result = SuiteCaseResult {
                    case_id: case_id_from_path(rel),
                    case_dir,
                    status,
                    acceptance_digest: acceptance,
                    candidate_digest: Some(outcome.generation.candidate_digest),
                    error: None,
                };
                if suite.policy.stop_on_first_failure
                    && !matches!(
                        classify_case_bucket(&result.status, &suite.policy),
                        CaseBucket::Accepted
                    )
                {
                    stop = true;
                }
                cases.push(result);
            }
            Err(e) => {
                let status = match &e {
                    GeneratorRunError::Verify(_) => "incomplete".to_owned(),
                    GeneratorRunError::Generator(_) | GeneratorRunError::Task(_) => {
                        "failed".to_owned()
                    }
                    GeneratorRunError::RunDir(_) => "failed".to_owned(),
                };
                let result = SuiteCaseResult {
                    case_id: case_id_from_path(rel),
                    case_dir,
                    status,
                    acceptance_digest: None,
                    candidate_digest: None,
                    error: Some(e.to_string()),
                };
                if suite.policy.stop_on_first_failure {
                    stop = true;
                }
                cases.push(result);
            }
        }
    }

    let status = aggregate_suite_status(&cases, &suite.policy);
    let evidence = SuiteEvidence {
        schema_version: SUITE_SCHEMA_VERSION.to_owned(),
        suite_id: suite.suite_id.clone(),
        suite_digest: suite_manifest_digest(&suite),
        status,
        stack_lock_digest: stack_lock_digest_value,
        generator_binary_digest,
        cases,
    };

    Ok(SuiteRunReport {
        evidence,
        manifest_path: config.suite_path.clone(),
    })
}

fn case_id_from_path(rel: &str) -> String {
    Path::new(rel).file_name().map_or_else(
        || rel.replace(['/', '\\'], "_"),
        |s| s.to_string_lossy().into_owned(),
    )
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_accepted_when_all_verified() {
        let policy = SuitePolicy::default();
        let cases = vec![
            SuiteCaseResult {
                case_id: "a".into(),
                case_dir: PathBuf::from("a"),
                status: "Verified".into(),
                acceptance_digest: Some("sha256:aa".into()),
                candidate_digest: None,
                error: None,
            },
            SuiteCaseResult {
                case_id: "b".into(),
                case_dir: PathBuf::from("b"),
                status: "accepted".into(),
                acceptance_digest: None,
                candidate_digest: None,
                error: None,
            },
        ];
        assert_eq!(
            aggregate_suite_status(&cases, &policy),
            SuiteStatus::Accepted
        );
    }

    #[test]
    fn aggregate_rejected_on_violation() {
        let policy = SuitePolicy::default();
        let cases = vec![SuiteCaseResult {
            case_id: "a".into(),
            case_dir: PathBuf::from("a"),
            status: "BehaviorFailed".into(),
            acceptance_digest: None,
            candidate_digest: None,
            error: None,
        }];
        assert_eq!(
            aggregate_suite_status(&cases, &policy),
            SuiteStatus::Rejected
        );
    }

    #[test]
    fn under_preconditions_incomplete_by_default() {
        let policy = SuitePolicy::default();
        let cases = vec![SuiteCaseResult {
            case_id: "a".into(),
            case_dir: PathBuf::from("a"),
            status: "VerifiedUnderPreconditions".into(),
            acceptance_digest: None,
            candidate_digest: None,
            error: None,
        }];
        assert_eq!(
            aggregate_suite_status(&cases, &policy),
            SuiteStatus::Incomplete
        );
        let mut allow = policy;
        allow.allow_verified_under_preconditions = true;
        assert_eq!(
            aggregate_suite_status(&cases, &allow),
            SuiteStatus::Accepted
        );
    }

    #[test]
    fn validate_rejects_empty_cases() {
        let suite = SuiteManifest {
            schema_version: SUITE_SCHEMA_VERSION.into(),
            suite_id: "t".into(),
            target: "x86_64".into(),
            abi: None,
            generator: SuiteGeneratorRef {
                spec: "g.toml".into(),
            },
            policy: SuitePolicy::default(),
            required_cases: vec![],
            stack_lock: None,
        };
        assert!(!validate_suite_manifest(&suite).is_empty());
    }

    #[test]
    fn parity_detects_target_and_abi_mismatch() {
        let dir = std::env::temp_dir().join(format!(
            "vaa-parity-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let case = dir.join("case_a");
        std::fs::create_dir_all(&case).unwrap();
        std::fs::write(
            case.join("task.vaa.toml"),
            r#"
schema_version = "0.1"
task_id = "t"
target = "x86_64-pc-windows-msvc"
[entry]
symbol = "f"
abi = "win64"
"#,
        )
        .unwrap();
        std::fs::write(
            case.join("contract.sem.toml"),
            "contract_version = \"0.1\"\n",
        )
        .unwrap();
        std::fs::write(case.join("input.hla64"), "program f;\n").unwrap();

        let suite = SuiteManifest {
            schema_version: SUITE_SCHEMA_VERSION.into(),
            suite_id: "parity.test".into(),
            target: "x86_64-unknown-linux-gnu".into(),
            abi: Some("sysv".into()),
            generator: SuiteGeneratorRef {
                spec: "g.toml".into(),
            },
            policy: SuitePolicy::default(),
            required_cases: vec!["case_a".into()],
            stack_lock: None,
        };
        let report = check_suite_target_abi_parity(&suite, &dir).unwrap();
        assert!(!report.ok);
        let joined = report.cases[0].diagnostics.join("; ");
        assert!(joined.contains("task target"), "{joined}");
        assert!(joined.contains("entry.abi"), "{joined}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parity_accepts_matching_win64() {
        let dir = std::env::temp_dir().join(format!(
            "vaa-parity-ok-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let case = dir.join("case_a");
        std::fs::create_dir_all(&case).unwrap();
        std::fs::write(
            case.join("task.vaa.toml"),
            r#"
schema_version = "0.1"
task_id = "t"
target = "x86_64-pc-windows-msvc"
[entry]
symbol = "f"
abi = "win64"
"#,
        )
        .unwrap();
        std::fs::write(
            case.join("contract.sem.toml"),
            "contract_version = \"0.1\"\n",
        )
        .unwrap();
        std::fs::write(case.join("input.hla64"), "program f;\n").unwrap();

        let suite = SuiteManifest {
            schema_version: SUITE_SCHEMA_VERSION.into(),
            suite_id: "parity.ok".into(),
            target: "x86_64-pc-windows-msvc".into(),
            abi: Some("win64".into()),
            generator: SuiteGeneratorRef {
                spec: "g.toml".into(),
            },
            policy: SuitePolicy::default(),
            required_cases: vec!["case_a".into()],
            stack_lock: None,
        };
        let report = check_suite_target_abi_parity(&suite, &dir).unwrap();
        assert!(report.ok, "{:?}", report.cases);
        assert_eq!(known_target_abi_profiles()[0].1, "win64");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
