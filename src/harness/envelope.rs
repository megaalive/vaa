//! Shared agent envelope discriminating direct assembly vs generator repair.

use serde::{Deserialize, Serialize};

use crate::harness::assembler::AssemblerFlavor;

/// Schema version for [`AgentEnvelope`].
pub const AGENT_ENVELOPE_SCHEMA_VERSION: &str = "0.1";

/// Default direct-mode operations declared on the work packet.
#[must_use]
pub fn default_allowed_operations() -> Vec<String> {
    vec![
        "write_candidate".to_owned(),
        "check_candidate".to_owned(),
        "verify_candidate".to_owned(),
        "finish_session".to_owned(),
    ]
}

fn is_default_allowed_operations(ops: &[String]) -> bool {
    ops == default_allowed_operations()
}

/// Which agent loop this envelope drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    /// Agent edits a candidate assembly file (NASM today; GAS reserved).
    DirectNasm,
    /// Agent edits generator source; must regenerate assembly.
    GeneratorRepair,
}

impl AgentMode {
    /// Stable snake_case label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DirectNasm => "direct_nasm",
            Self::GeneratorRepair => "generator_repair",
        }
    }
}

/// Attempt / wall budgets from the locked task (or defaults).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentBudget {
    pub max_candidates: u32,
    #[serde(default)]
    pub max_repairs_per_candidate: u32,
    pub max_wall_time_seconds: u64,
}

impl Default for AgentBudget {
    fn default() -> Self {
        Self {
            max_candidates: 4,
            max_repairs_per_candidate: 2,
            max_wall_time_seconds: 300,
        }
    }
}

/// Reproduction commands for the harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCommands {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doctor: Option<String>,
    pub verify: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify_gate2: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regenerate: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suite: Option<String>,
}

/// Digests recorded at prepare/submit time.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDigests {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate: Option<String>,
    /// Frozen SemASM capabilities snapshot digest (`CAPABILITY_SNAPSHOT_DIGEST`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_snapshot: Option<String>,
    /// Digest of the embedded/live `target-profile.json` written at prepare.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_profile: Option<String>,
}

/// Agent-facing prepare payload (machine JSON) / work packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentEnvelope {
    pub schema_version: String,
    pub mode: AgentMode,
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abi: Option<String>,
    /// Assembler flavor the candidate must use (`nasm` today; `gas` reserved).
    #[serde(default)]
    pub assembler: AssemblerFlavor,
    pub task_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    pub writable_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub commands: AgentCommands,
    pub budget: AgentBudget,
    /// Remaining candidate attempts when known (budget − sealed count).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remaining_attempts: Option<u32>,
    /// Latest structured failure / submit class for the prompt payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_failure: Option<String>,
    #[serde(default)]
    pub digests: AgentDigests,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semasm_packet_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair_packet_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_markdown_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events_path: Option<String>,
    /// Allowed harness operations for this work packet (direct-mode default).
    #[serde(
        default = "default_allowed_operations",
        skip_serializing_if = "is_default_allowed_operations"
    )]
    pub allowed_operations: Vec<String>,
    /// Workspace-relative or absolute path to `target-profile.json`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_profile_path: Option<String>,
    /// Path where submit will write `feedback.json` (hint for agents).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback_path: Option<String>,
    /// Canonical work-packet path (`work-packet.json`); mirrors envelope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_packet_path: Option<String>,
}

impl AgentEnvelope {
    /// Build a direct-assembly envelope (NASM by default).
    #[must_use]
    pub fn direct_nasm(
        target: impl Into<String>,
        task_id: impl Into<String>,
        commands: AgentCommands,
        budget: AgentBudget,
    ) -> Self {
        Self {
            schema_version: AGENT_ENVELOPE_SCHEMA_VERSION.to_owned(),
            mode: AgentMode::DirectNasm,
            target: target.into(),
            abi: None,
            assembler: AssemblerFlavor::Nasm,
            task_id: task_id.into(),
            run_id: None,
            writable_paths: vec![AssemblerFlavor::Nasm.candidate_filename().to_owned()],
            forbidden_paths: vec![
                "**/*.vaa.toml".to_owned(),
                "**/*.sem.toml".to_owned(),
                "**/stack.lock.toml".to_owned(),
                "**/vectors.json".to_owned(),
            ],
            commands,
            budget,
            remaining_attempts: None,
            latest_failure: None,
            digests: AgentDigests::default(),
            semasm_packet_path: None,
            repair_packet_path: None,
            workspace_dir: None,
            prompt_markdown_path: None,
            events_path: None,
            allowed_operations: default_allowed_operations(),
            target_profile_path: None,
            feedback_path: None,
            work_packet_path: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trip() {
        let env = AgentEnvelope::direct_nasm(
            "x86_64-unknown-linux-gnu",
            "t1",
            AgentCommands {
                doctor: None,
                verify: "semasm agent verify …".into(),
                verify_gate2: None,
                regenerate: None,
                suite: None,
            },
            AgentBudget::default(),
        );
        let json = serde_json::to_string(&env).unwrap();
        let back: AgentEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(back.mode, AgentMode::DirectNasm);
        assert_eq!(back.assembler, AssemblerFlavor::Nasm);
        assert_eq!(back.schema_version, AGENT_ENVELOPE_SCHEMA_VERSION);
        assert_eq!(back.allowed_operations, default_allowed_operations());
    }

    #[test]
    fn golden_direct_envelope_deserializes() {
        let raw = include_str!("../../schemas/fixtures/agent-envelope.direct_nasm.json");
        let env: AgentEnvelope = serde_json::from_str(raw).expect("fixture");
        assert_eq!(env.mode, AgentMode::DirectNasm);
        assert_eq!(env.schema_version, AGENT_ENVELOPE_SCHEMA_VERSION);
        assert_eq!(env.target, "x86_64-pc-windows-msvc");
        assert!(!env.commands.verify.is_empty());
        assert_eq!(env.allowed_operations, default_allowed_operations());
    }

    #[test]
    fn default_ops_omitted_from_minimal_serialize() {
        let env = AgentEnvelope::direct_nasm(
            "x86_64-pc-windows-msvc",
            "t",
            AgentCommands {
                doctor: None,
                verify: "v".into(),
                verify_gate2: None,
                regenerate: None,
                suite: None,
            },
            AgentBudget::default(),
        );
        let v = serde_json::to_value(&env).unwrap();
        assert!(v.get("allowed_operations").is_none());
    }
}
