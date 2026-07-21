# Implementation progress

Tracks evidence gates from `VAA_REVIEWED_AND_HARDENED_ARCHITECTURE_PLAN.md` §26–§27.

| Gate | Status | Notes |
|---|---|---|
| Phase 0 — SemASM readiness | **Done** | `docs/implementation-baseline.md` |
| PR-001 — Repository bootstrap | **Done** | Rust crate, licenses, CI, dependency policy |
| PR-002 — Task schema v0.1 | **Done** | Typed model, strict parse, fixtures, JSON Schema, `vaa validate` |
| PR-003 — Policy and immutable task digest | **Done** | Canonical JSON + SHA-256, `LockedTask`, mutation tests |
| PR-004 — Run directory and event log | **Done** | RunId, RunDir, EventLog with atomic writes and bounded records |
| PR-005 — SemASM doctor / version negotiation | **Done** | Graceful if `semasm` not on PATH |
| PR-006 — SemASM capabilities adapter | **Done** | `vaa capabilities --target <triple>` |
| PR-007 — SemASM verification adapter | **Done** | stdout-only VerificationReport **0.4** + status map |
| PR-008 — Final evidence status aggregator | **Done** | preserves SemASM-mapped outcomes (`execution_denied`→Incomplete) |
| Phase 1 exit (`vaa verify …` full offline report) | **Done** | EvidenceAggregator, 4-outcome bundle |
| PR-007b — Controller handshake (SemASM 0.4) | **Done** | `--contract`, digests/`tool_version`, golden + Win64 smoke fixtures |
| PR-009 — Hardened process runner | **Done** | `src/process/runner.rs` — timeout, env allowlist, bounds, tree kill |
| PR-010 — Build sandbox backend | **Done** | `src/sandbox/backend.rs` — trait, LocalBackend, ContainerBackend |
| PR-011 — NASM and linker pipeline | **Done** | `src/build/pipeline.rs` — explicit argv, BuildManifest |
| PR-012 — Artifact inspection gate | **Done** | `src/inspect/artifact.rs` — `object` crate, ELF/PE/MachO |
| PR-013 — Trusted callable-function harness | **Done** | `src/harness/template.rs` — sysv64/win64 templates |
| PR-014 — Execution sandbox | **Done** | `src/sandbox/exec.rs` — opt-in, timeout, result |
| PR-015 — Candidate protocol | **Done** | `src/candidate/protocol.rs` — hash dedup, size limits |
| PR-016 — Fixture model adapter | **Done** | `src/model/adapter.rs` — deterministic scripted responses |
| PR-017 — Orchestrator state machine | **Done** | `src/orchestrate/machine.rs` — legal transitions, invariant tests |
| Phase 2 — Isolated build vertical slice | **Done** | process runner + sandbox + assembler + linker + inspection |
| Phase 3 — Trusted behavioral harness | **Done** | harness template + execution sandbox |
| Phase 4 — Deterministic model adapter | **Done** | candidate protocol + fixture adapter + state machine |

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

Phase 1: doctor, capabilities, verify all available:

```bash
cargo run -q -- doctor
cargo run -q -- capabilities --target x86_64-unknown-linux-gnu
cargo run -q -- verify fixtures/semasm/count_byte/count_byte.vaa.toml \
  --source fixtures/semasm/count_byte/count_byte_win64.asm \
  --contract fixtures/semasm/count_byte/count_byte.sem.toml \
  --format json
```

### SemASM VerificationReport 0.4 handshake

- Parse **stdout only** (never concatenate stderr).
- Status map: `verified`→Verified; `semantic_failed`/`executable_failed`/`behavior_failed`→Violated; `execution_denied`→Incomplete; missing report / parse / binary errors→Failed.
- Fixtures: [`fixtures/semasm/`](../fixtures/semasm/README.md).
- Out of scope still: automatic `.vaa.toml`→`.sem.toml` translation, live model repair loop, SemASM `--allow-execution` policy in VAA.

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
