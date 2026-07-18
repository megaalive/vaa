# Task fixtures

These fixtures exercise `task.vaa.toml` schema **0.1**.

| File | Expected result |
|---|---|
| `sum_i64.vaa.toml` | Valid callable-function task with three authoritative tests |
| `invalid_unknown_field.vaa.toml` | Parse error (`deny_unknown_fields`) |
| `invalid_schema_version.vaa.toml` | Validation error (unsupported `schema_version`) |
| `invalid_missing_tests.vaa.toml` | Validation error (behavioral tests required but empty) |

Validate the good fixture:

```bash
cargo run -q -- validate fixtures/tasks/sum_i64.vaa.toml
cargo run -q -- validate fixtures/tasks/sum_i64.vaa.toml --format json
```
