# VAA — Verifiable Assembly Agent

**Status:** experimental · **Fluent agent surface:** delivered (see
[`docs/fluent-agent-surface.md`](docs/fluent-agent-surface.md))  
**Language:** Rust · **Form:** local CLI (+ library)

VAA is a fail-closed controller: constrained tasks → assembly candidates →
SemASM evidence → optional sealed selection. Agents propose; SemASM verifies.

> **Honesty charter.** Read [`docs/HONESTY.md`](docs/HONESTY.md) before citing
> what VAA "proves". Controllers parse **stdout JSON only**; work is limited to
> **admitted** leaves (`vaa admit`); `verified_under_preconditions` ≠
> `verified` (use admit `claim`, not tier alone). Dry-runs ≠ evidence. VAA is a
> local CLI + project skill — **not an MCP product**.

## Happy path (fluent surface)

Primary commands for agent authoring workflows:

| Command | Role |
|---|---|
| `vaa author` | Case / seed scaffolding |
| `vaa admit` | Capability admission lookup (`claim` + `tier` + `obligations`) |
| `vaa agent serve --stdio --case <dir>` | NDJSON session (assembler from target; resume explicit) |
| `vaa optimize` | Correctness-preserving ranking (`object_file_bytes` never from source) |
| `vaa evidence` | Seal / bundle / chain checks |

Older commands (`harness`, `verify`, `run`, `ingest`, …) remain for
compatibility. Detail + release status:
[`docs/fluent-agent-surface.md`](docs/fluent-agent-surface.md).

Refresh the frozen SemASM snapshot (no auto-commit):

```text
vaa semasm capability-sync --semasm <bin> [--apply]
```

## For agents

Drive VAA through the project skill
[`.cursor/skills/vaa-harness/SKILL.md`](.cursor/skills/vaa-harness/SKILL.md)
(Codex: see [`AGENTS.md`](AGENTS.md)), backed by the reference adapter
[`scripts/agent_harness_adapter.py`](scripts/agent_harness_adapter.py). The skill
operates **only** on admitted leaves (`vaa admit`) and declines anything else. Copy-paste a
happy path and a decline path from [`docs/agent-playbook.md`](docs/agent-playbook.md).
Bounds are fixed by [`docs/HONESTY.md`](docs/HONESTY.md); there is no MCP server.

## What works today

| Capability | Status |
|---|---|
| `vaa version` / `vaa status` | Available |
| `vaa validate <task.vaa.toml>` | Available (schema **0.1**) |
| Task content digest (`sha256:…`) | Available after successful validate |
| `vaa doctor` | Available — SemASM version & schema compat |
| `vaa capabilities --target <triple>` | Available — machine-readable JSON |
| `vaa admit` / `vaa semasm capability-sync` | Available — frozen snapshot + refresh |
| `vaa author` / `vaa agent` / `vaa optimize` | Available — fluent surface |
| `vaa verify <task> --source <asm> --contract <sem.toml>` | Available — SemASM report 0.4, identity-bound evidence |
| `vaa run <task> --contract … --wrong … --repaired …` | Available — fixture wrong→repair loop (no live LLM); writes sealed evidence |
| `vaa ingest <task> --contract … --source …` | Available — generator-agnostic candidate deposit (no model) |
| `vaa evidence check-seal …` | Available — evidence/seal JSON integrity (not artifact rehash) |
| `vaa evidence verify-bundle <dir>` | Available — re-hash task/contract/source/report vs seal |
| `vaa evidence verify-chain <run-dir>` | Available — full candidate hash chain + final seal |
| `vaa generate <task> --output <file.asm>` | Available — fixture model adapter |
| `vaa build <source.asm> [--target elf64] [--sandbox container]` | Available — NASM + linker; container = Scaffold |
| `vaa inspect <artifact>` | Available — ELF/PE/MachO analysis |
| `vaa sandbox status` | Available via `vaa status` |
| `vaa repair …` | Available — repair packet export/verify/rules |
| `vaa harness prepare\|submit\|resume\|status` | Available — agent façade (direct + generator-repair); NASM (x86_64 Win64/SysV) + GAS (AArch64) CI-proven, RISC-V64 GAS dialect-only/fail-closed, GAS-on-x86_64 fail-closed; submit can seal (`--run-base`/`--run-dir`) |
| Model generation / repair | **Fixture adapter** + opt-in **`--live`** (`live-model` feature) |
| Assemble / link / sandbox execute | **Via toolchain on PATH** |
| SemASM discovery | `SEMASM_BIN` (file or dir; fail-closed if invalid), else PATH scan |

This project does **not** claim safety, formal proof, zero overhead, or production readiness.
HSM scaffold ≠ hardware HSM; `search --ingest` ≠ CryptOpt; local transparency
artifact ≠ remote append-only log; Incomplete ≠ Verified.

## Build

Requirements: a recent stable Rust toolchain with `rustfmt` and `clippy` (see `rust-toolchain.toml`).

```bash
cargo build
cargo test
cargo run -q -- status
cargo run -q -- validate fixtures/tasks/sum_i64.vaa.toml
cargo run -q -- validate fixtures/tasks/sum_i64.vaa.toml --format json
```

## Task files

Authoritative contract format: `*.vaa.toml` (schema `0.1`).

- Guide: [`docs/task-schema.md`](docs/task-schema.md)
- JSON Schema: [`schemas/task.vaa.schema.json`](schemas/task.vaa.schema.json)
- Example: [`fixtures/tasks/sum_i64.vaa.toml`](fixtures/tasks/sum_i64.vaa.toml)

Unknown fields are rejected. Authoritative tests and budgets are included in the locked task digest so a repair loop cannot silently weaken the contract.

## Design baseline

Read before contributing functional code:

1. [`VAA_REVIEWED_AND_HARDENED_ARCHITECTURE_PLAN.md`](VAA_REVIEWED_AND_HARDENED_ARCHITECTURE_PLAN.md)
2. [`docs/implementation-baseline.md`](docs/implementation-baseline.md) — SemASM reality check
3. [`docs/progress.md`](docs/progress.md) — PR / phase status
4. [`docs/task-schema.md`](docs/task-schema.md)
5. [`DEPENDENCIES.md`](DEPENDENCIES.md)

Non-negotiable direction:

- one binary crate with internal modules;
- immutable task / policy / tests / budgets after lock;
- four evidence outcomes: `verified`, `violated`, `incomplete`, `failed`;
- never promote unsupported, missing, or incomplete analysis to success;
- SemASM integration via versioned process/JSON protocol (`VerificationReport` schema **0.4**, stdout-only; identity digests bound into evidence);
- dynamic execution disabled by default (`vaa verify` / `vaa run` do not pass `--allow-execution`);
- SemASM contract path is explicit: `--contract <*.sem.toml>` (distinct from the locked `*.vaa.toml` task);
- `vaa run` wires the orchestrator with a **fixture** model queue (wrong→repair); live providers are out of scope;
- `vaa ingest` accepts any external `.asm` (fixture, human, CryptOpt-like search, LLM dump) and always returns to SemASM verify + sealed evidence — generators do not move acceptance;
- seals are **content integrity** envelopes (`acceptance_digest` / `envelope_digest`); opt-in Ed25519 authenticity via `VAA_SEAL_SIGNING_KEY` (practice keys ≠ trust root); see [`docs/seal.md`](docs/seal.md);
- `vaa build --sandbox container` wraps assemble/link via Docker/Podman (**Scaffold**, not hardened isolation); default image `ubuntu:24.04` (`VAA_CONTAINER_IMAGE`).

## Exit codes (partial)

| Code | Meaning |
|---:|---|
| 0 | Success |
| 2 | Invalid user input or task schema |
| 3 | SemASM binary not found or version mismatch |
| 4 | Verification produced violations or failures |
| 7 | Task budget exhausted |

Full table: architecture plan §19.3.

## What's next

`main` is past tagged **`v0.1.1`** (portable Win/Linux binaries + `SHA256SUMS` —
see [Releases](https://github.com/megaalive/vaa/releases)). See
[`CHANGELOG.md`](CHANGELOG.md) **[Unreleased]** for the architectural summary
since that tag (Thin bridges, write-shape Gate, isolation, Horizon honesty,
Dx-era SemASM pin). Alpha **`v0.1.0`** remains source-archive-only historically.

Post-alpha harden notes: [`docs/post-alpha-harden.md`](docs/post-alpha-harden.md).
Known limits: container ≠ absolute isolation; Rekor/Sigstore/SoftHSM ≠ SemASM
Verified or hardware HSM; `search --ingest` ≠ CryptOpt; local transparency
artifact ≠ remote append-only log. Next release should narrate the stack leap,
not only commit lists — no rush to tag until CHANGELOG + checklist agree.

Hardening milestone (optimizer metrics, session identity, multi-dialect): see
companion living doc in the cli-repl workspace when present; do not expand
surface with a “Release E” feature dump.

## License

Licensed under either of:

- Apache License, Version 2.0 (`LICENSE-APACHE`)
- MIT license (`LICENSE-MIT`)

at your option.
