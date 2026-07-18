# Implementation progress

Tracks evidence gates from `VAA_REVIEWED_AND_HARDENED_ARCHITECTURE_PLAN.md` §26–§27.

| Gate | Status | Notes |
|---|---|---|
| Phase 0 — SemASM readiness | **Done** | `docs/implementation-baseline.md` |
| PR-001 — Repository bootstrap | **Done** | Rust crate, licenses, CI, dependency policy |
| PR-002 — Task schema v0.1 | **Done** | Typed model, strict parse, fixtures, JSON Schema, `vaa validate` |
| PR-003 — Policy and immutable task digest | **Done** | Canonical JSON + SHA-256, `LockedTask`, mutation tests |
| PR-004 — Run directory and event log | **Done** | RunId, RunDir, EventLog with atomic writes and bounded records |
| PR-005 — SemASM doctor / version negotiation | Pending | Blocked on gaps listed in baseline |
| PR-006 — SemASM capabilities adapter | Pending | |
| PR-007 — SemASM verification adapter | Pending | |
| PR-008 — Final evidence status aggregator | Pending | |
| Phase 1 exit (`vaa verify …` full offline report) | Pending | Needs PR-004+ |

## Current executable acceptance

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -q -- status
cargo run -q -- validate fixtures/tasks/sum_i64.vaa.toml
cargo run -q -- validate fixtures/tasks/sum_i64.vaa.toml --format json
```

Negative fixtures must fail with exit code 2:

```bash
cargo run -q -- validate fixtures/tasks/invalid_unknown_field.vaa.toml; echo $?
cargo run -q -- validate fixtures/tasks/invalid_schema_version.vaa.toml; echo $?
cargo run -q -- validate fixtures/tasks/invalid_missing_tests.vaa.toml; echo $?
```

## Documentation map

| Document | Role |
|---|---|
| `VAA_REVIEWED_AND_HARDENED_ARCHITECTURE_PLAN.md` | Architecture baseline |
| `docs/implementation-baseline.md` | Phase 0 SemASM reality check |
| `docs/task-schema.md` | Task schema 0.1 operator/developer guide |
| `docs/progress.md` | This file — PR/phase status |
| `schemas/task.vaa.schema.json` | Checked-in JSON Schema for task 0.1 |
| `DEPENDENCIES.md` | Dependency policy |
| `README.md` | Truthful project entry point |
| `fixtures/tasks/README.md` | Fixture catalogue |

## Honesty constraints

Do not claim:

- assembly verification;
- model-assisted generation;
- sandbox execution;
- production readiness;

until the corresponding PR exit criteria have executable evidence.
