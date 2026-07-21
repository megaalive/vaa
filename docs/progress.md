# Implementation progress

Tracks evidence gates from `VAA_REVIEWED_AND_HARDENED_ARCHITECTURE_PLAN.md` §26–§27.

Honesty rule: **Done** means the listed acceptance exists as code. Levels of evidence are
called out separately (`unit-tested` / `integration-tested` / `verified-in-CI`).

| Gate | Status | Evidence level | Notes |
|---|---|---|---|
| Phase 0 — SemASM readiness | **Done** | docs | `docs/implementation-baseline.md` |
| PR-001 — Repository bootstrap | **Done** | CI | Rust crate, licenses, CI, dependency policy |
| PR-002 — Task schema v0.1 | **Done** | unit+CI | Typed model, strict parse, fixtures |
| PR-003 — Policy and immutable task digest | **Done** | unit+CI | Canonical JSON + SHA-256 |
| PR-004 — Run directory and event log | **Done** | unit | RunId, RunDir, EventLog (restart resume = R2) |
| PR-005 — SemASM doctor | **Done** | unit | ProcessRunner; missing schema → Degraded |
| PR-006 — SemASM capabilities adapter | **Done** | unit | Embedded snapshot (not live SemASM) |
| PR-007 — SemASM verification adapter | **Done** | unit | stdout-only VerificationReport **0.4** |
| PR-007b — Controller handshake | **Done** | unit | `--contract`, digests, golden fixture |
| PR-008 — Evidence aggregator | **Done** | unit | fail-closed + identity cross-checks |
| Phase 1 exit (`vaa verify`) | **Done** | unit | Offline report; live SemASM smoke ignored |
| PR-009 — Process runner | **Partial** | unit | Timeout/env/output post-check; **no** streaming cap / PG leader yet |
| PR-010 — Build sandbox backend | **Scaffold** | unit | Docker argv wrapper; not hardened isolation |
| PR-011 — NASM/linker pipeline | **Done** | unit | Needs toolchain on PATH for live use |
| PR-012 — Artifact inspection | **Done** | unit | `object` crate |
| PR-013 — Harness templates | **Done** | unit | sysv64/win64 |
| PR-014 — Execution sandbox | **Done** | unit | Opt-in |
| PR-015 — Candidate protocol | **Done** | unit | Target match, digest map, attempt budget |
| PR-016 — Fixture model adapter | **Done** | unit | Queued wrong→repair + generation ids |
| PR-017 — Orchestrator state machine | **Done** | unit | Edges for repair |
| **R1 — `vaa run` wired** | **Done** | unit | Fixture loop + SemASM verify; live SemASM not in CI |
| **R2 — Seal + ingest** | **Done** | unit | integrity envelope; `vaa ingest`; `check-seal` |
| **R2b — Seal hardening** | **Done** | unit | acceptance/envelope; atomic publish; per-candidate chain; `verify-bundle` |
| **R2c — verify-chain + append-only** | **Done** | unit | `verify-chain`; full check details; exclusive candidate dirs; canonical vectors |
| Phase 2–4 “vertical slice” claims | **Components + R1/R2 wiring** | — | Not a CI-proven VAA→SemASM→toolchain golden yet |

## Current executable acceptance

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -q -- status
cargo run -q -- validate fixtures/tasks/sum_i64.vaa.toml
```

Fixture-driven end-to-end (requires `semasm` on PATH):

```bash
cargo run -q -- run fixtures/run/count_byte/count_byte.vaa.toml \
  --contract fixtures/run/count_byte/count_byte.sem.toml \
  --wrong fixtures/run/count_byte/01_wrong.asm \
  --repaired fixtures/run/count_byte/02_repaired.asm \
  --run-dir target/vaa-runs \
  --format json
```

Generator-agnostic ingest (any external `.asm`; no model):

```bash
cargo run -q -- ingest fixtures/ingest/count_byte/count_byte.vaa.toml \
  --contract fixtures/ingest/count_byte/count_byte.sem.toml \
  --source fixtures/ingest/count_byte/candidate.asm \
  --generator external-agent \
  --run-dir target/vaa-runs \
  --format json

cargo run -q -- evidence check-seal \
  target/vaa-runs/<run-id>/candidates/0000/evidence.json \
  target/vaa-runs/<run-id>/candidates/0000/evidence.seal.json

cargo run -q -- evidence verify-bundle \
  target/vaa-runs/<run-id>/candidates/0000

cargo run -q -- evidence verify-chain \
  target/vaa-runs/<run-id>
```

### SemASM VerificationReport 0.4 handshake

- Parse **stdout only**.
- Schema pin: `>=0.4,<0.5`.
- Status map: `verified`→Verified; gate failures→Violated; `execution_denied`→Incomplete; missing report→Failed.
- Evidence identity: target + source/contract digests + tool identity must match.

### Seal + generator-agnostic ingest (R2 / R2b / R2c)

- Per-candidate bundle under `candidates/NNNN/` plus `evidence/final.json` + `evidence/final.seal.json`.
- Seal schema **0.2**: `acceptance_digest` (technical truth) vs `envelope_digest` (includes provenance / chain).
- `check-seal` = evidence/seal JSON drift (full `checks` including `details`).
- `verify-bundle` = re-hash one candidate's artifacts.
- `verify-chain` = contiguous hash chain + final seal; deleting a predecessor fails verification.
- Append-only storage: exclusive candidate dirs + `create_new` writes.
- Integrity ≠ authenticity: SHA-256 envelope detects drift; it does **not** prove a trusted VAA publisher (no signing yet). See [`docs/seal.md`](seal.md).
- Canonicalization: [`docs/vaa-canonical-json-v1.md`](vaa-canonical-json-v1.md) + [`fixtures/canonical-json/`](../fixtures/canonical-json/).
- Atomic publication with seal commit marker (not claimed fully crash-durable on all FS).
- Positioning (honest): CryptOpt-like / Proof-Loop idea = candidates return to SemASM; acceptance digests sealed. Not a search engine or formal proof system.

### Still out of scope (R3+)

- Streaming output caps / process-group kill
- CI job with live SemASM + toolchain + transparency log of digests
- Hardened ContainerBackend / generator FS isolation
- Digital signature (Ed25519) authenticity
- Fully crash-durable transactional pair (directory fsync everywhere)
- Live model adapter
- CryptOpt randomized search engine
- Full `sum_i64` SemASM golden (needs SemASM contract)

## Documentation map

| Document | Role |
|---|---|
| `VAA_REVIEWED_AND_HARDENED_ARCHITECTURE_PLAN.md` | Architecture baseline |
| `docs/implementation-baseline.md` | Phase 0 SemASM reality check |
| `docs/task-schema.md` | Task schema 0.1 |
| `docs/progress.md` | This file |
| `docs/seal.md` | Integrity vs authenticity; seal schema 0.2; verify-chain |
| `docs/vaa-canonical-json-v1.md` | Named canonical JSON profile |
| `fixtures/canonical-json/` | Cross-language conformance vectors |
| `fixtures/run/count_byte/README.md` | R1 golden run |
| `fixtures/ingest/count_byte/README.md` | R2 generator-agnostic ingest |
| `fixtures/semasm/README.md` | Handshake fixtures |

## Honesty constraints

Do not claim formal proof, production readiness, hardened sandbox isolation, or
CI-proven SemASM vertical slices until the corresponding evidence exists.
