# VAA — Verifiable Assembly Agent

**Status:** experimental  
**Language:** Rust  
**Form:** local CLI, single binary crate with library modules  

VAA is a small, fail-closed controller that will turn a constrained task specification into assembly candidates, collect evidence from [SemASM](https://github.com/megaalive/semasm) and the native toolchain, and return an evidence bundle.

## What works today

| Capability | Status |
|---|---|
| `vaa version` / `vaa status` | Available |
| `vaa validate <task.vaa.toml>` | Available (schema **0.1**) |
| Task content digest (`sha256:…`) | Available after successful validate |
| `vaa doctor` | Available — SemASM version & schema compat |
| `vaa capabilities --target <triple>` | Available — machine-readable JSON |
| `vaa verify <task> --source <candidate.asm>` | Available — 4-outcome evidence bundle |
| Model generation / repair | **Not implemented** |
| Assemble / link / sandbox execute | **Not implemented** |

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
- SemASM integration via versioned process/JSON protocol;
- dynamic execution disabled by default.

## Exit codes (partial)

| Code | Meaning |
|---:|---|
| 0 | Success |
| 2 | Invalid user input or task schema |
| 3 | SemASM binary not found or version mismatch |
| 4 | Verification produced violations or failures |

Full table: architecture plan §19.3.

## License

Licensed under either of:

- Apache License, Version 2.0 (`LICENSE-APACHE`)
- MIT license (`LICENSE-MIT`)

at your option.
