//! Named canonical JSON encoding used by task digests and evidence seals.
//!
//! Spec: [`docs/vaa-canonical-json-v1.md`](../docs/vaa-canonical-json-v1.md).

use serde::Serialize;

/// Identifier recorded in seal envelopes so external verifiers know the rules.
pub const CANONICALIZATION_ID: &str = "vaa-canonical-json-v1";

/// Digest algorithm identifier paired with [`CANONICALIZATION_ID`].
pub const DIGEST_ALGORITHM_ID: &str = "sha256";

/// Serialize `value` to canonical JSON bytes (`vaa-canonical-json-v1`).
///
/// Rules (summary):
/// - UTF-8
/// - object keys sorted lexicographically at every level
/// - no insignificant whitespace (compact `serde_json::to_vec`)
/// - array order preserved
/// - recursive application to nested objects/arrays
#[must_use]
pub fn canonical_json_bytes<T: Serialize>(value: &T) -> Vec<u8> {
    let value = serde_json::to_value(value).expect("serialize to JSON value");
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sorts_object_keys_recursively() {
        let value = json!({"b": 1, "a": {"z": 2, "y": 3}});
        let bytes = canonical_json_bytes(&value);
        assert_eq!(bytes, br#"{"a":{"y":3,"z":2},"b":1}"#);
    }

    #[test]
    fn preserves_array_order() {
        let value = json!({"items": [3, 1, 2]});
        let bytes = canonical_json_bytes(&value);
        assert_eq!(bytes, br#"{"items":[3,1,2]}"#);
    }
}
