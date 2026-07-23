# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
for **crate** versions. See `docs/release-v0.1-checklist.md` for release hygiene.

## [Unreleased]

`main` is **materially past** the `v0.1.1` tag. This section is the
architectural summary for the **next** release notes — not a claim that a tag
has shipped. SoftHSM ≠ hardware HSM; `search --ingest` ≠ CryptOpt; Incomplete ≠
Verified; HlaX64 ≠ SemASM Verified; local transparency artifact ≠ remote log.

### Stack identity (SemASM + VAA)

VAA owns task lock, candidate lifecycle, sandbox profiles, proof/seal chain,
signing, and transparency exports. SemASM owns object policy, decode/lower,
ABI/CFG, capabilities, behavioral oracles, and verification evidence. Generators
(HlaX64, search mutators, humans, LLMs) never decide acceptance.

### Added

- **Thin leaf bridges (Th1–Th8)** — HlaX64 Win64 ingest + Gate for
  `count_byte`, `find_first_byte`, `memset`, `memcpy`, `min_usize`, `max_usize`
  (plus earlier `sum_i64` / `find_last_byte` / `memcmp` / `replace_byte`).
  Emit/`-Wverify` ≠ SemASM Verified.
- **Write-shape Gate parity** — `replace_byte` / `memset` / `memcpy` fixture
  run + `search --ingest` (nop-before-ret); Gate-1 Incomplete without
  `--allow-execution`.
- **Execution isolation (I2)** — `execution_isolation` + `--execution-sandbox`
  on Gate paths (profile ≠ absolute OS isolation).
- **Horizon Closeout (consumer side)** — progress/map honesty; remote-
  transparency non-claim (local export / opt-in Rekor/Fulcio ≠ operated remote
  log); SoftHSM/CryptOpt honesty on leaf READMEs.
- **P8-F / P8-K / P8-I** — cargo-fuzz smoke; SoftHSM PKCS#11 live signer
  (`--features pkcs11`); Fulcio keyless DSSE (`--features fulcio`). Not a
  trust root; Gate stays offline by default.
- Older Unreleased fixtures still in tree: `find_first_byte` / `min_usize` /
  `max_usize` SemASM Gate packs (pins superseded by later tips — see
  `docs/progress.md`).

### Changed

- SemASM pin tracks post-Horizon tip (x86 decode/lower sign-off, A64/RV
  write-shape harness, MemCmp A64/RV, guard-byte Rmem). See SemASM
  `CHANGELOG` Unreleased for producer-side detail.
- GitHub Actions Node 24 pin bump (`checkout@v6`, artifact v6/v7, etc.).
- README “What works today” table markdown fixed; honesty lines tightened.

### Honesty / non-goals (unchanged)

- No CryptOpt embed; no hardware HSM; no live-model Gate CI default; no claim
  that practice seals or SoftHSM are a production trust root.

## [0.1.1] — 2026-07-23

Patch release: first tagged cut with **portable Win/Linux binaries** + `SHA256SUMS`
(via `.github/workflows/release-binaries.yml`). No MSI/Docker/installer.
Practice seals and Gate artifacts remain illustrative, not a trust root.

### Added

- **Release packaging** — `.github/workflows/release-binaries.yml` builds portable
  Windows + Linux archives + `SHA256SUMS` on `v*` tags (no MSI/Docker/installer).
- **P7-S** — Container C1 binds + path remap, bundled seccomp, rootless probe,
  `--generator-jail` for external generators; Gate exec honesty (SemASM path).
- **P7-D** — `vaa evidence durability-probe` + multi-file seal-last helper.
- **P7-A** — `SealSigner` trait, Sigstore-shaped DSSE, HSM PKCS#11 scaffold.
- **P7-T** — Rekor publish/verify with mock transport; `--features rekor` for live HTTP;
  optional `transparency-rekor.yml` workflow_dispatch.
- **P7-C** — `vaa search` nop-slide / external mutator staging loop (no CryptOpt embed).

### Fixed

- CHANGELOG `[0.1.0]` non-goals: remove stray `Done` after OS-level FS isolation line.
- GitHub repository About description + topics for discoverability.
- Twin-assemble `reproducible_build` on Windows: normalize COFF `TimeDateStamp`
  before object digest compare (same-host; not cross-host bit-identical).

## [0.1.0] — 2026-07-23

Alpha release (`git tag v0.1.0`). Gate CI artifacts and practice seals are
**illustrative**, not a trust root. Release assets were **source archives only**
(no portable binaries — added in `v0.1.1`).

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
- Hardened seccomp / verified rootless / OS-level generator FS isolation
- Full cargo-fuzz PR-022 security certification
- Auto `cargo publish`
- Cross-host bit-identical builds / cache as a trust root
