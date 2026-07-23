# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
for **crate** versions. Git tag `v0.1.0` marks the alpha release; see
`docs/release-v0.1-checklist.md`.

## [Unreleased]

## [0.1.0] — 2026-07-23

Alpha release (`git tag v0.1.0`). Gate CI artifacts and practice seals are
**illustrative**, not a trust root.

### Added

- **PR-019** — Opt-in OpenAI-compatible live model adapter behind feature `live-model`
  (`ureq`): `vaa generate … --live` with `VAA_MODEL_API_KEY` (+ optional base URL / model).
  CI remains deterministic without network; API keys never enter seals.
- **PR-020** — Local content-addressed cache (`.vaa/cache` / `VAA_CACHE_DIR`): verification +
  build keys; `vaa cache status`; opt-in `--cache` on verify/build. Not a remote log;
  Incomplete/Failed never promoted to Verified.
- **PR-021** — Same-host reproducibility: `vaa build --check-reproducible` + required
  `reproducible_build` evidence check (twin NASM assemble). Not cross-host bit-identical.
- **PR-022** — Thin negative corpus under `fixtures/negative/` (validate / transparency /
  cache fail-closed vectors; not full fuzz).
- **PR-023** — Release checklist closeout + `docs/cache.md`; alpha tag ceremony.
- **G1** — External argv generator (`vaa generate --run-dir … --command …`) writes
  only under `staging/` (`GeneratorMeta.kind = external-argv`). Logical barrier only;
  not OS ACL / job-object FS isolation.
- **E1b** — Gate-1 CI resume smoke: sealed `0000` → `vaa run --resume` → `0001` with
  `previous_seal_digest` + `verify-chain`.
- **L1/L2** — Linux `count_byte` / `sum_i64` sysv64 fixtures and ubuntu Gate jobs
  (`semasm-gate1-linux`, `semasm-gate2-linux`) on the pinned SemASM SHA. Gate-2 Linux
  uses `qemu-x86_64` (SemASM `__native__` is Windows-only on this pin). VAA+SemASM
  smoke only — not a claim that SemASM Linux assemble/link is upstream CI-verified.
- **N6** — Light negative corpus (fail-closed validate / transparency vectors).
- **R-notes** — Release prep check scripts (`scripts/release-prep-check.sh` / `.ps1`).

### Highlights (I0–E1 and Gate surface)

- Task schema **0.1**, immutable digests, fail-closed evidence outcomes.
- SemASM doctor / capabilities / `VerificationReport` **0.4** handshake.
- Seal schema **0.2** (`acceptance_digest` / `envelope_digest`); opt-in Ed25519
  practice keys ≠ trust root.
- Generator-agnostic `vaa ingest`, `verify-chain`, local seal-log + transparency
  export (CI artifact ≠ remote immutable log).
- Gate-1 Incomplete + Gate-2 Verified on Win64 (`count_byte`, `sum_i64`) + HlaX64 bridge.
- Container build backend remains **Scaffold** (not hardened isolation).
- Resume from sealed runs (`vaa run --resume`) — E1 unit + E1b Gate CI.

### Changed

- **D3** — Docs / checklist point past E1/G1/Linux Gate toward release prep, then alpha tag.

### Explicit non-goals (still deferred)

- CryptOpt search / remote Rekor / HSM / Sigstore
- Hardened seccomp / verified rootless / OS-level generator FS isolation Done
- Full cargo-fuzz PR-022 security certification
- Auto `cargo publish`
- Cross-host bit-identical builds / cache as a trust root
