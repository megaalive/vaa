# Canonical JSON conformance vectors (`vaa-canonical-json-v1`)

Cross-language verifiers should digest these cases identically.

| File | Role |
|---|---|
| `vectors.json` | Input JSON values |
| `expected-canonical.jsonl` | `name<TAB>canonical-utf8` |
| `expected-sha256.txt` | `name<TAB>sha256:<hex>` |
| `gen_expected.py` | Helper to regenerate expected files (Rust test is authoritative) |

Run: `cargo test canonical_json::tests::conformance_vectors_match_fixtures`
