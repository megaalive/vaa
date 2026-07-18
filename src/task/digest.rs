//! Canonical serialization and content digests for locked tasks.

use sha2::{Digest, Sha256};

use crate::task::model::Task;

/// Hex-encoded SHA-256 digest of the canonical task encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDigest {
    /// Lowercase hex SHA-256.
    pub hex: String,
}

impl TaskDigest {
    /// Prefixed form `sha256:<hex>`.
    #[must_use]
    pub fn prefixed(&self) -> String {
        format!("sha256:{}", self.hex)
    }
}

/// Compute the immutable task digest.
///
/// The digest is SHA-256 over the canonical JSON encoding of the task. Canonical
/// JSON sorts object keys lexicographically at every level and uses compact
/// separators with no insignificant whitespace. Arrays keep author order.
#[must_use]
pub fn task_digest(task: &Task) -> TaskDigest {
    let canonical = canonical_task_bytes(task);
    let hash = Sha256::digest(canonical);
    TaskDigest {
        hex: hex_encode(&hash),
    }
}

/// Canonical JSON bytes used for digests and mutation tests.
#[must_use]
pub fn canonical_task_bytes(task: &Task) -> Vec<u8> {
    let value = serde_json::to_value(task).expect("task serializes to JSON value");
    let canonical = sort_value(value);
    serde_json::to_vec(&canonical).expect("canonical JSON serializes")
}

fn sort_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort_unstable();
            let mut out = serde_json::Map::new();
            for key in keys {
                let child = map.get(&key).cloned().expect("key exists");
                out.insert(key, sort_value(child));
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(sort_value).collect())
        }
        other => other,
    }
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
    use crate::task::model::{
        ArtifactKind, Behavior, Budgets, Capabilities, Delivery, Entry, InstructionPolicy,
        MemoryPolicy, TaskTest, TomlValue, ValueKind, VerificationRequirements,
    };
    use std::collections::BTreeMap;

    fn sample_task() -> Task {
        Task {
            schema_version: "0.1".to_owned(),
            task_id: "sum-i64-v1".to_owned(),
            artifact_kind: ArtifactKind::CallableFunction,
            target: "x86_64-unknown-linux-gnu".to_owned(),
            entry: Entry {
                symbol: "sum_i64".to_owned(),
                abi: "sysv64".to_owned(),
            },
            inputs: BTreeMap::new(),
            output: ValueKind {
                kind: "i64".to_owned(),
            },
            behavior: Behavior {
                summary: "sum".to_owned(),
                integer_overflow: Some("wrap".to_owned()),
                empty_input_result: Some(TomlValue::Integer(0)),
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
                require_behavioral_tests: true,
                require_reproducible_build: true,
            },
            budgets: Budgets {
                max_candidates: 4,
                max_repairs_per_candidate: 2,
                max_wall_time_seconds: 300,
                max_model_tokens: 24000,
                max_no_progress_iterations: 1,
            },
            delivery: Delivery {
                include_source: true,
                include_object: true,
                include_binary: false,
                include_evidence: true,
            },
            tests: vec![TaskTest {
                name: "empty".to_owned(),
                input: BTreeMap::from([
                    ("values".to_owned(), TomlValue::Array(vec![])),
                    ("length".to_owned(), TomlValue::Integer(0)),
                ]),
                expected: TomlValue::Integer(0),
            }],
        }
    }

    #[test]
    fn digest_is_stable_for_same_task() {
        let task = sample_task();
        let a = task_digest(&task);
        let b = task_digest(&task);
        assert_eq!(a, b);
        assert_eq!(a.hex.len(), 64);
        assert!(a.prefixed().starts_with("sha256:"));
    }

    #[test]
    fn digest_changes_when_policy_mutates() {
        let task = sample_task();
        let original = task_digest(&task);
        let mut mutated = task;
        mutated.budgets.max_candidates = 99;
        let after = task_digest(&mutated);
        assert_ne!(original, after);
    }

    #[test]
    fn digest_changes_when_test_mutates() {
        let task = sample_task();
        let original = task_digest(&task);
        let mut mutated = task;
        mutated.tests[0].expected = TomlValue::Integer(1);
        assert_ne!(original, task_digest(&mutated));
    }
}
