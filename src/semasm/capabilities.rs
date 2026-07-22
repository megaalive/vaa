use crate::Task;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityLevel {
    Supported,
    Partial,
    Experimental,
    Unavailable,
    Unknown,
}

impl CapabilityLevel {
    #[must_use]
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Supported | Self::Partial | Self::Experimental)
    }

    #[must_use]
    pub fn is_sufficient_for(&self, required: &Self) -> bool {
        matches!(
            (required, self),
            (_, Self::Supported)
                | (Self::Partial, Self::Partial | Self::Experimental)
                | (Self::Experimental, Self::Experimental)
        )
    }
}

#[derive(Debug, Clone)]
pub struct TargetCapabilities {
    pub target_id: String,
    pub decode: CapabilityLevel,
    pub lower: CapabilityLevel,
    pub abi_check: CapabilityLevel,
    pub object_inspect: CapabilityLevel,
    pub assemble: CapabilityLevel,
    pub link: CapabilityLevel,
    pub sandbox_run: CapabilityLevel,
}

impl TargetCapabilities {
    #[must_use]
    pub fn for_target(target: &str) -> Self {
        match target {
            // Agent-verify golden path is exercised in VAA Gate-1/2 CI (Win64)
            // and SemASM e2e (SysV). Snapshot must not under-claim Supported
            // levels required by locked tasks with require_* = true.
            "x86_64-unknown-linux-gnu" | "x86_64-pc-windows-msvc" => Self {
                target_id: target.to_owned(),
                decode: CapabilityLevel::Supported,
                lower: CapabilityLevel::Supported,
                abi_check: CapabilityLevel::Supported,
                object_inspect: CapabilityLevel::Supported,
                assemble: CapabilityLevel::Supported,
                link: CapabilityLevel::Supported,
                sandbox_run: CapabilityLevel::Supported,
            },
            "aarch64-unknown-linux-gnu" => Self {
                target_id: target.to_owned(),
                decode: CapabilityLevel::Partial,
                lower: CapabilityLevel::Partial,
                abi_check: CapabilityLevel::Partial,
                object_inspect: CapabilityLevel::Supported,
                assemble: CapabilityLevel::Supported,
                link: CapabilityLevel::Supported,
                sandbox_run: CapabilityLevel::Supported,
            },
            _ => Self {
                target_id: target.to_owned(),
                decode: CapabilityLevel::Unknown,
                lower: CapabilityLevel::Unknown,
                abi_check: CapabilityLevel::Unknown,
                object_inspect: CapabilityLevel::Unknown,
                assemble: CapabilityLevel::Unknown,
                link: CapabilityLevel::Unknown,
                sandbox_run: CapabilityLevel::Unknown,
            },
        }
    }

    #[must_use]
    pub fn digest(&self) -> String {
        use sha2::{Digest, Sha256};
        let input = format!(
            "{}{:?}{:?}{:?}{:?}{:?}{:?}{:?}",
            self.target_id,
            self.decode,
            self.lower,
            self.abi_check,
            self.object_inspect,
            self.assemble,
            self.link,
            self.sandbox_run
        );
        let hash = Sha256::digest(input.as_bytes());
        {
            use std::fmt::Write;
            hash.iter().fold(String::with_capacity(64), |mut s, b| {
                let _ = write!(s, "{b:02x}");
                s
            })
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CapabilityMatch {
    pub compatible: bool,
    pub missing: Vec<String>,
    pub insufficient: Vec<String>,
}

#[must_use]
pub fn match_task_requirements(task: &Task, caps: &TargetCapabilities) -> CapabilityMatch {
    let mut missing = Vec::new();
    let mut insufficient = Vec::new();

    if task.verification.require_complete_lowering
        && !caps.lower.is_sufficient_for(&CapabilityLevel::Supported)
    {
        insufficient.push(format!("lower: required supported, got {:?}", caps.lower));
    }

    if task.verification.require_abi_check
        && !caps
            .abi_check
            .is_sufficient_for(&CapabilityLevel::Supported)
    {
        insufficient.push(format!(
            "abi_check: required supported, got {:?}",
            caps.abi_check
        ));
    }

    if task.verification.require_object_inspection
        && !caps
            .object_inspect
            .is_sufficient_for(&CapabilityLevel::Supported)
    {
        insufficient.push(format!(
            "object_inspect: required supported, got {:?}",
            caps.object_inspect
        ));
    }

    if task.verification.require_behavioral_tests
        && !caps
            .sandbox_run
            .is_sufficient_for(&CapabilityLevel::Supported)
    {
        insufficient.push(format!(
            "sandbox_run (behavioral tests): required supported, got {:?}",
            caps.sandbox_run
        ));
    }

    if matches!(caps.decode, CapabilityLevel::Unknown) {
        missing.push("target not recognized".to_owned());
    }

    CapabilityMatch {
        compatible: missing.is_empty() && insufficient.is_empty(),
        missing,
        insufficient,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{
        ArtifactKind, Behavior, Budgets, Capabilities, Delivery, Entry, InstructionPolicy,
        MemoryPolicy, ValueKind, VerificationRequirements,
    };
    use std::collections::BTreeMap;

    fn sample_task() -> Task {
        Task {
            schema_version: "0.1".to_owned(),
            task_id: "test-v1".to_owned(),
            artifact_kind: ArtifactKind::CallableFunction,
            target: "x86_64-unknown-linux-gnu".to_owned(),
            entry: Entry {
                symbol: "test".to_owned(),
                abi: "sysv64".to_owned(),
            },
            inputs: BTreeMap::new(),
            output: ValueKind {
                kind: "i64".to_owned(),
            },
            behavior: Behavior {
                summary: "test".to_owned(),
                integer_overflow: None,
                empty_input_result: None,
            },
            capabilities: Capabilities {
                syscalls: vec![],
                imports: vec![],
                heap: false,
                filesystem: false,
                network: false,
                environment: false,
                clock: false,
                random: false,
            },
            memory: MemoryPolicy {
                max_stack_bytes: 128,
                allow_global_writable: false,
                allow_self_modifying_code: false,
            },
            instructions: InstructionPolicy {
                required_features: vec!["x86-64-baseline".to_owned()],
                forbidden_mnemonics: vec![],
                allow_unknown_semantics: false,
            },
            verification: VerificationRequirements {
                require_complete_lowering: true,
                require_abi_check: true,
                require_object_inspection: true,
                require_behavioral_tests: false,
                require_reproducible_build: false,
            },
            budgets: Budgets {
                max_candidates: 1,
                max_repairs_per_candidate: 0,
                max_wall_time_seconds: 60,
                max_model_tokens: 0,
                max_no_progress_iterations: 1,
            },
            delivery: Delivery {
                include_source: true,
                include_object: false,
                include_binary: false,
                include_evidence: true,
            },
            tests: vec![],
        }
    }

    #[test]
    fn x86_64_win64_capabilities_support_gate2_requirements() {
        let caps = TargetCapabilities::for_target("x86_64-pc-windows-msvc");
        assert_eq!(caps.lower, CapabilityLevel::Supported);
        assert_eq!(caps.abi_check, CapabilityLevel::Supported);
        assert_eq!(caps.object_inspect, CapabilityLevel::Supported);
        assert_eq!(caps.sandbox_run, CapabilityLevel::Supported);
    }

    #[test]
    fn x86_64_linux_capabilities_support_agent_verify() {
        let caps = TargetCapabilities::for_target("x86_64-unknown-linux-gnu");
        assert_eq!(caps.decode, CapabilityLevel::Supported);
        assert_eq!(caps.assemble, CapabilityLevel::Supported);
        assert_eq!(caps.sandbox_run, CapabilityLevel::Supported);
    }

    #[test]
    fn unknown_target_returns_unknown_capabilities() {
        let caps = TargetCapabilities::for_target("nonexistent-target");
        assert_eq!(caps.decode, CapabilityLevel::Unknown);
    }

    #[test]
    fn capability_match_rejects_insufficient_lower() {
        let task = sample_task();
        let mut caps = TargetCapabilities::for_target("x86_64-unknown-linux-gnu");
        caps.lower = CapabilityLevel::Partial;
        let result = match_task_requirements(&task, &caps);
        assert!(!result.compatible);
        assert!(result.insufficient.iter().any(|s| s.contains("lower")));
    }

    #[test]
    fn capability_match_accepts_x86_win64_gate_tasks() {
        let mut task = sample_task();
        task.target = "x86_64-pc-windows-msvc".to_owned();
        task.verification.require_behavioral_tests = true;
        task.verification.require_object_inspection = true;
        let caps = TargetCapabilities::for_target("x86_64-pc-windows-msvc");
        let result = match_task_requirements(&task, &caps);
        assert!(
            result.compatible,
            "insufficient={:?} missing={:?}",
            result.insufficient, result.missing
        );
    }

    #[test]
    fn capability_match_accepts_when_requirements_minimal() {
        let mut task = sample_task();
        task.verification.require_complete_lowering = false;
        task.verification.require_abi_check = false;
        task.verification.require_object_inspection = false;
        let mut caps = TargetCapabilities::for_target("x86_64-unknown-linux-gnu");
        caps.lower = CapabilityLevel::Partial;
        caps.abi_check = CapabilityLevel::Partial;
        caps.object_inspect = CapabilityLevel::Experimental;
        let result = match_task_requirements(&task, &caps);
        assert!(result.compatible);
    }

    #[test]
    fn digest_is_stable() {
        let caps = TargetCapabilities::for_target("x86_64-unknown-linux-gnu");
        let a = caps.digest();
        let b = caps.digest();
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn digest_differs_for_different_targets() {
        let a = TargetCapabilities::for_target("x86_64-unknown-linux-gnu");
        let b = TargetCapabilities::for_target("aarch64-unknown-linux-gnu");
        assert_ne!(a.digest(), b.digest());
    }
}
