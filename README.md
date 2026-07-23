# VAA — Verifiable Assembly Agent

**Status:** experimental  
**Language:** Rust  
**Form:** local CLI, single binary crate with library modules  

VAA is a small, fail-closed controller that will turn a constrained task specification into assembly candidates, collect evidence from [SemASM](https://github.com/megaalive/semasm) and the native toolchain, and return an evidence bundle.

## What works today

| Capability | Status |
|---|---|---|
| `vaa version` / `vaa status` | Available |
| `vaa validate <task.vaa.toml>` | Available (schema **0.1**) |
| Task content digest (`sha256:…`) | Available after successful validate |
| `vaa doctor` | Available — SemASM version & schema compat |
| `vaa capabilities --target <triple>` | Available — machine-readable JSON |
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
| Model generation / repair | **Fixture adapter** + opt-in **`--live`** (`live-model` feature) |
| Assemble / link / sandbox execute | **Via toolchain on PATH** |

This project does **not** claim safety, formal proof, zero overhead, or production readiness.

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

**`v0.1.1`** is the first tagged release with portable Win/Linux binaries + `SHA256SUMS`
(see [Releases](https://github.com/megaalive/vaa/releases)). Alpha **`v0.1.0`** remains
source-archive-only historically.
Post-alpha P7 is Done — [`docs/post-alpha-harden.md`](docs/post-alpha-harden.md).
Known limits: container ≠ absolute isolation; Rekor/Sigstore ≠ SemASM Verified;
HSM is scaffold; search is staging-only (not CryptOpt embed).
Later: disposable VM mode, Fulcio keyless, live PKCS#11, fuller fuzz.

## License

Licensed under either of:

- Apache License, Version 2.0 (`LICENSE-APACHE`)
- MIT license (`LICENSE-MIT`)

at your option.
