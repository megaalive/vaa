# VAA — Verifiable Assembly Agent

**Status:** experimental bootstrap  
**Language:** Rust  
**Form:** local CLI, single binary crate  

VAA is intended to be a small, fail-closed controller that turns a constrained task specification into assembly candidates, collects evidence from [SemASM](https://github.com/megaalive/semasm) and the native toolchain, and returns an evidence bundle.

This repository currently contains:

- the reviewed architecture plan (`VAA_REVIEWED_AND_HARDENED_ARCHITECTURE_PLAN.md`);
- Phase 0 readiness notes (`docs/implementation-baseline.md`);
- a bootstrap Rust CLI that prints version/status only.

It does **not** yet:

- verify assembly;
- call a language model;
- assemble, link, or execute candidates;
- claim safety, formal proof, zero overhead, or production readiness.

## Build

Requirements: a recent stable Rust toolchain with `rustfmt` and `clippy` (see `rust-toolchain.toml`).

```bash
cargo build
cargo test
cargo run -- status
cargo run -- version
```

## Design baseline

Read before contributing functional code:

1. `VAA_REVIEWED_AND_HARDENED_ARCHITECTURE_PLAN.md`
2. `docs/implementation-baseline.md`
3. `DEPENDENCIES.md`

Non-negotiable direction for early PRs:

- one binary crate with internal modules;
- immutable task/policy/tests/budgets;
- four evidence outcomes: `verified`, `violated`, `incomplete`, `failed`;
- never promote unsupported, missing, or incomplete analysis to success;
- SemASM integration via versioned process/JSON protocol, not internal forks;
- dynamic execution disabled by default.

## License

Licensed under either of:

- Apache License, Version 2.0 (`LICENSE-APACHE`)
- MIT license (`LICENSE-MIT`)

at your option.
