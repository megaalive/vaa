# Negative fixtures (N6)

Fail-closed inputs for validate / transparency / seal parsers.
Not a full security corpus (see PR-022 deferred).

| Path | Expected |
|---|---|
| `task_zero_budget.vaa.toml` | Validation error (`max_candidates` must be ≥ 1) |
| `transparency_wrong_schema.json` | `UnsupportedSchema` on `read_transparency_file` |
| `transparency_garbage.json` | `Json` error on parse |

```bash
cargo run -q -- validate fixtures/negative/task_zero_budget.vaa.toml
```
