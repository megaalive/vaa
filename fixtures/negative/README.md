# Negative fixtures (N6 + PR-022 thin)

Fail-closed inputs for validate / transparency / seal / cache parsers.
Not a full security corpus or cargo-fuzz CI job.

| Path | Expected |
|---|---|
| `task_zero_budget.vaa.toml` | Validation error (`max_candidates` must be ≥ 1) |
| `transparency_wrong_schema.json` | `UnsupportedSchema` on `read_transparency_file` |
| `transparency_garbage.json` | `Json` error on parse |
| `cache_verification_garbage.json` | Integrity/parse failure when treated as cache record |
| `cache_verification_missing_blob.json` | Policy/NotFound when blob pointer is absent from store |

```bash
cargo run -q -- validate fixtures/negative/task_zero_budget.vaa.toml
```
