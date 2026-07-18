//! Immutable locked task view used after validation.

use std::sync::Arc;

use crate::task::digest::{task_digest, TaskDigest};
use crate::task::model::Task;

/// A validated task sealed with its content digest.
///
/// Repair loops and model adapters must not replace this value with a mutated
/// task. Callers that need a different contract must create a new locked task
/// through the normal load/validate path.
#[derive(Debug, Clone, PartialEq)]
pub struct LockedTask {
    task: Arc<Task>,
    digest: TaskDigest,
}

impl LockedTask {
    /// Seal a validated task. Caller must already have run semantic validation.
    #[must_use]
    pub fn lock(task: Task) -> Self {
        let digest = task_digest(&task);
        Self {
            task: Arc::new(task),
            digest,
        }
    }

    /// Borrow the immutable task document.
    #[must_use]
    pub fn task(&self) -> &Task {
        &self.task
    }

    /// Content digest computed at lock time.
    #[must_use]
    pub fn digest(&self) -> &TaskDigest {
        &self.digest
    }

    /// Recompute the digest from the sealed task and compare to the stored one.
    ///
    /// This is a defense-in-depth check against accidental interior mutation if
    /// a future API exposes interior mutability by mistake.
    #[must_use]
    pub fn digest_matches(&self) -> bool {
        task_digest(self.task()) == self.digest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::model::{
        ArtifactKind, Behavior, Budgets, Capabilities, Delivery, Entry, InstructionPolicy,
        MemoryPolicy, ValueKind, VerificationRequirements,
    };
    use std::collections::BTreeMap;

    fn sample() -> Task {
        Task {
            schema_version: "0.1".to_owned(),
            task_id: "lock-demo".to_owned(),
            artifact_kind: ArtifactKind::CallableFunction,
            target: "x86_64-unknown-linux-gnu".to_owned(),
            entry: Entry {
                symbol: "f".to_owned(),
                abi: "sysv64".to_owned(),
            },
            inputs: BTreeMap::new(),
            output: ValueKind {
                kind: "i64".to_owned(),
            },
            behavior: Behavior {
                summary: "demo".to_owned(),
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
                max_stack_bytes: 64,
                allow_global_writable: false,
                allow_self_modifying_code: false,
            },
            instructions: InstructionPolicy {
                required_features: vec![],
                forbidden_mnemonics: vec![],
                allow_unknown_semantics: false,
            },
            verification: VerificationRequirements {
                require_complete_lowering: false,
                require_abi_check: false,
                require_object_inspection: false,
                require_behavioral_tests: false,
                require_reproducible_build: false,
            },
            budgets: Budgets {
                max_candidates: 1,
                max_repairs_per_candidate: 0,
                max_wall_time_seconds: 10,
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
    fn locked_task_preserves_digest() {
        let locked = LockedTask::lock(sample());
        assert!(locked.digest_matches());
        assert_eq!(locked.task().task_id, "lock-demo");
    }

    #[test]
    fn cloning_locked_task_does_not_fork_digest() {
        let locked = LockedTask::lock(sample());
        let clone = locked.clone();
        assert_eq!(locked.digest(), clone.digest());
        assert!(clone.digest_matches());
    }
}
