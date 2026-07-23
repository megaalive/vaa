# Negative fixtures (N6 + PR-022)

Fail-closed inputs for validate / transparency / seal / cache parsers.
Fuzz entry points + CI smoke: see [`fuzz/`](../../fuzz/) (P8-F). Not a formal
security audit / certification.

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
