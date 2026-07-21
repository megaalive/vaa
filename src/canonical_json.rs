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
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::path::PathBuf;

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

    #[test]
    fn conformance_vectors_match_fixtures() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/canonical-json");
        let vectors: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(root.join("vectors.json")).unwrap()).unwrap();
        let expected_canon = fs::read_to_string(root.join("expected-canonical.jsonl")).unwrap();
        let expected_sha = fs::read_to_string(root.join("expected-sha256.txt")).unwrap();

        let mut canon_map = std::collections::BTreeMap::new();
        for line in expected_canon.lines().filter(|l| !l.is_empty()) {
            let (name, body) = line.split_once('\t').expect("tab");
            canon_map.insert(name.to_owned(), body.to_owned());
        }
        let mut sha_map = std::collections::BTreeMap::new();
        for line in expected_sha.lines().filter(|l| !l.is_empty()) {
            let (name, body) = line.split_once('\t').expect("tab");
            sha_map.insert(name.to_owned(), body.to_owned());
        }

        for item in vectors.as_array().unwrap() {
            let name = item["name"].as_str().unwrap();
            let value = &item["value"];
            let bytes = canonical_json_bytes(value);
            let canonical = String::from_utf8(bytes.clone()).unwrap();
            let digest = Sha256::digest(&bytes);
            let mut hex = String::with_capacity(64);
            for b in digest {
                use std::fmt::Write as _;
                let _ = write!(hex, "{b:02x}");
            }
            let prefixed = format!("sha256:{hex}");

            assert_eq!(
                canon_map.get(name).map(String::as_str),
                Some(canonical.as_str()),
                "canonical mismatch for {name}"
            );
            assert_eq!(
                sha_map.get(name).map(String::as_str),
                Some(prefixed.as_str()),
                "sha256 mismatch for {name}"
            );
        }
    }
}
