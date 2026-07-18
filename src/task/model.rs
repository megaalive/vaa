//! Typed `task.vaa.toml` model (schema 0.1).
//!
//! Unknown fields are rejected (`deny_unknown_fields`) so policy drift surfaces
//! as a hard validation error rather than silent ignore.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Root task document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    /// Schema version string, currently `"0.1"`.
    pub schema_version: String,
    /// Stable task identifier chosen by the author.
    pub task_id: String,
    /// Artifact class produced by the pipeline.
    pub artifact_kind: ArtifactKind,
    /// Target triple identity (for example `x86_64-unknown-linux-gnu`).
    pub target: String,
    /// Entry symbol and ABI.
    pub entry: Entry,
    /// Named inputs to the entry routine.
    #[serde(default)]
    pub inputs: BTreeMap<String, InputSpec>,
    /// Primary output description.
    pub output: ValueKind,
    /// Informal behavior notes that still participate in the digest.
    pub behavior: Behavior,
    /// Capability allow-list for the candidate.
    pub capabilities: Capabilities,
    /// Memory policy.
    pub memory: MemoryPolicy,
    /// Instruction selection constraints.
    pub instructions: InstructionPolicy,
    /// Verification requirements.
    pub verification: VerificationRequirements,
    /// Resource budgets for generation and repair.
    pub budgets: Budgets,
    /// Delivery options for accepted artifacts.
    pub delivery: Delivery,
    /// Authoritative behavioral tests (locked into the digest).
    #[serde(default)]
    pub tests: Vec<TaskTest>,
}

/// Supported artifact classes for schema 0.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactKind {
    /// Function conforming to a declared ABI.
    CallableFunction,
    /// Hosted program with an OS entry path.
    HostedProgram,
    /// Freestanding / bare-metal image.
    FreestandingImage,
}

/// Entry point declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Exported symbol name.
    pub symbol: String,
    /// ABI identifier (for example `sysv64`).
    pub abi: String,
}

/// Description of one named input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputSpec {
    /// Value kind for this input.
    pub kind: String,
    /// Element type when `kind` is a pointer/array.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element: Option<String>,
    /// Access mode (`read`, `write`, `read-write`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access: Option<String>,
    /// Name of the length input when this is a pointer buffer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length_from: Option<String>,
    /// Whether a null pointer is permitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,
}

/// Output value kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValueKind {
    /// Type kind string (for example `i64`, `usize`).
    pub kind: String,
}

/// Behavior notes attached to the locked task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Behavior {
    /// Short human summary.
    pub summary: String,
    /// Integer overflow rule (for example `wrap`, `saturate`, `trap`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integer_overflow: Option<String>,
    /// Result for empty input when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub empty_input_result: Option<TomlValue>,
}

/// Capability allow-list. Defaults are fail-closed (all false / empty).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    /// Allowed syscall names or groups.
    #[serde(default)]
    pub syscalls: Vec<String>,
    /// Allowed import symbols.
    #[serde(default)]
    pub imports: Vec<String>,
    /// Heap allocation permitted.
    #[serde(default)]
    pub heap: bool,
    /// Filesystem access permitted.
    #[serde(default)]
    pub filesystem: bool,
    /// Network access permitted.
    #[serde(default)]
    pub network: bool,
    /// Environment variable access permitted.
    #[serde(default)]
    pub environment: bool,
    /// Clock / time access permitted.
    #[serde(default)]
    pub clock: bool,
    /// Randomness sources permitted.
    #[serde(default)]
    pub random: bool,
}

/// Memory limits and memory-safety related policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryPolicy {
    /// Maximum stack usage in bytes.
    pub max_stack_bytes: u64,
    /// Whether writable globals are allowed.
    #[serde(default)]
    pub allow_global_writable: bool,
    /// Whether self-modifying code is allowed.
    #[serde(default)]
    pub allow_self_modifying_code: bool,
}

/// Instruction selection constraints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstructionPolicy {
    /// Required ISA feature tags.
    #[serde(default)]
    pub required_features: Vec<String>,
    /// Forbidden mnemonic names.
    #[serde(default)]
    pub forbidden_mnemonics: Vec<String>,
    /// Whether unknown instruction semantics may still pass.
    #[serde(default)]
    pub allow_unknown_semantics: bool,
}

/// Required verification layers for acceptance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationRequirements {
    /// Require complete SemASM lowering coverage.
    pub require_complete_lowering: bool,
    /// Require ABI check evidence.
    pub require_abi_check: bool,
    /// Require object inspection evidence.
    pub require_object_inspection: bool,
    /// Require authoritative behavioral tests.
    pub require_behavioral_tests: bool,
    /// Require reproducible build evidence.
    pub require_reproducible_build: bool,
}

/// Generation and repair budgets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budgets {
    /// Maximum candidate sources to try.
    pub max_candidates: u32,
    /// Maximum repair attempts per candidate.
    pub max_repairs_per_candidate: u32,
    /// Wall-clock budget for a full run.
    pub max_wall_time_seconds: u64,
    /// Model token budget when a live model is enabled later.
    pub max_model_tokens: u64,
    /// Stop after this many iterations without progress.
    pub max_no_progress_iterations: u32,
}

/// What to keep when a run accepts an artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Delivery {
    /// Keep assembly source.
    pub include_source: bool,
    /// Keep object file.
    pub include_object: bool,
    /// Keep linked binary.
    pub include_binary: bool,
    /// Keep evidence bundle.
    pub include_evidence: bool,
}

/// One authoritative test case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskTest {
    /// Test case name.
    pub name: String,
    /// Input map (names match `inputs` keys or free-form fixture fields).
    #[serde(default)]
    pub input: BTreeMap<String, TomlValue>,
    /// Expected result value.
    pub expected: TomlValue,
}

/// Restricted TOML/JSON value subset used in tests and behavior notes.
///
/// Accepts null, bool, integer, string, and homogeneous arrays of those.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TomlValue {
    /// JSON/TOML null (rare in TOML fixtures).
    Null,
    /// Boolean.
    Bool(bool),
    /// Signed 64-bit integer.
    Integer(i64),
    /// UTF-8 string.
    String(String),
    /// Array of values.
    Array(Vec<TomlValue>),
}

impl TomlValue {
    /// True when this value is a finite integer tree (no floats).
    #[must_use]
    pub fn is_supported_test_value(&self) -> bool {
        match self {
            Self::Null | Self::Bool(_) | Self::Integer(_) | Self::String(_) => true,
            Self::Array(items) => items.iter().all(Self::is_supported_test_value),
        }
    }
}
