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

### Added

- **External generator bridge (P0 + P1 + P2 + P3 complete)** — suite
  runner, patch evidence, `vaa generator check-paths`, and `vaa generator
  triage` (Incomplete / verified_under_preconditions ≠ generator defect).
  P2.11–14: repair packet, diagnostics, source-map join, agent rules.
  P3.15: HlaX64 pack corpus expansion — 10 cases + phase Win64 suites +
  `CORPUS.md`. P3.16: suite `abi` + `vaa suite check-parity`; Win64 suites
  annotated. P3.17: `integrations/echoasm/` second-pack smoke
  (cmd echo generator; schema/CLI universality; ≠ Gate Verified).
  P3.18: generator subprocess isolation (`docs/generator-isolation.md`,
  `vaa generator isolation-check`; credential env never inherits).
  **Milestone 6:** live HlaX64 scalar suite generate path, EchoAsm repair
  patch-evidence fixtures, CI `generator-packs` matrix + pack suite step
  on `hlax64-bridge`.
  **Phase E (calls / data):** 5 cases (`internal_function_call`,
  `nested_call`, `global_rodata`, `multiple_exports`,
  `small_struct_return`) + `suites/calls-data-win64.vaa-suite.toml`
  (validated, parity, live Win64 generate). `small_struct_return` covers
  aggregate layout with a register-returned scalar.
  **SysV live:** `integrations/hlax64/generator.sysv.spec.toml` emits
  `--target linux-x64-sysv`; `scalar-sysv` generates real System V asm
  (`rdi`/`rsi`, CI-asserted). Emit ≠ SysV SemASM Gate.
  **Pack Gate (Win64 scalar):** live `vaa suite run` without `--skip-verify`
  + `--allow-execution` accepts `scalar-win64` (`min_usize`/`max_usize`
  Verified). SemASM subprocess allowlist now includes `TEMP`/`TMP` (scratch
  dir). `scripts/run-hlax64-suite.ps1 -Gate`. Practice seal ≠ trust root.
  **Pack Gate (SysV scalar, Linux):** `scalar-sysv` Accepted with
  `min_usize_sysv`/`max_usize_sysv` Verified. Depends on SemASM `afaa19d`
  (SysV framed `mov rsp,rbp` epilogue). `scripts/run-hlax64-suite.sh --gate`
  + CI `hlax64-pack-sysv-gate`. Stack lock SemASM pin bumped.
  **Live HlaX64 repair acceptance:** controlled SysV unsigned-compare defect
  (`jb` → `ja`) produced `behavior_failed` (4/6 `min_usize` vectors).
  Repair packet `BEHAVIOR_VECTOR_MISMATCH_001` constrained changes to the
  ABI lowerer/test; deterministic regeneration returned `scalar-sysv`
  Accepted (2/2 Verified). Committed repair packet + Accepted patch evidence
  under `fixtures/repair/hlax64-min-usize-sysv-live/`; generator pack CI
  verifies both. HlaX64 main `5379729` carries regression coverage.
  Controlled exercise ≠ naturally occurring incident; practice seal ≠ trust
  root. Honesty locks unchanged.
  **Phase A named i64:** 6 cases (`return_i64`, `add_i64`, `sub_i64`,
  `min_i64`, `max_i64`, `abs_i64`) + `suites/scalar-i64-win64.vaa-suite.toml`
  replace the usize proxy claim. SemASM `566ca8e` adds
  `builtin.pure_int.binary_i64` v1 (wrapping add/sub, signed min/max) and
  `builtin.pure_int.unary_i64` v1 (identity, wrapping-abs;
  `abs(i64::MIN) == i64::MIN`) with a single-register `PureIntUnary`
  harness on SysV/Win64/AAPCS64/RISC-V and signed two's-complement vector
  marshalling. Win64 pack Gate: suite **Accepted**, 6/6 **Verified** (real
  execution). CI: `hlax64-bridge` gates the i64 suite; pack-matrix
  validates/parity-checks it; SemASM pins for pack-gate jobs bumped to
  `566ca8e` (stack lock updated).   `abs_i64` expresses negation as `0 - x`
  (HlaX64 has no `neg`).
  **Phase B named loops/stack:** 4 cases (`sum_range`, `countdown_loop`,
  `stack_local_i64`, `forced_register_spill`) +
  `suites/loop-stack-win64.vaa-suite.toml`. SemASM `3cae1e1` unary i64 v2
  adds `sum_range` + `countdown` (small vectors) + identity aliases.
  Win64 pack Gate: **Accepted**, 4/4 **Verified**.
  **HlaX64 source-map emission:** `emit-nasm --source-map` writes VAA
  `candidate.map.json` (schema 0.1) beside the asm (`22ef241`); pack specs
  enable `--source-map`; CI map-joins after Phase B Gate.
  **Negative suite reject:** locked `min_i64_wrong` (implements max) +
  `suites/negative-reject-win64.vaa-suite.toml` +
  `scripts/ci-negative-suite-reject.ps1` assert suite **Rejected** with
  **Violated** — fail-closed without live agent edits.
  **Phase C/D/E Gate (Win64):** `memory-read` / `memory-write` suites
  Accepted with `VerifiedUnderPreconditions` (policy + Gate scripts
  honest); `calls-data` Accepted 5/5 Verified via SemASM `ecde423`
  Phase-E pure-int oracles; `small_struct_return` stages fields through
  registers. **SysV named i64 + loop-stack:** `scalar-i64-sysv` /
  `loop-stack-sysv` + Linux CI Gate steps. **Source-map line quality:**
  pin HlaX64 `62c9f22` (distinct IR→NASM lines); CI asserts ≥2 distinct
  `assembly_line` values after Phase B Gate.
  **Memory-leaf depth (concrete cells):** `load_byte0` / `store_byte0` +
  `memory-leaf-concrete-v1` (no caller obligations) + suite
  `memory-concrete-win64`. SemASM `28fb22f`. Gate expects unconditional
  **Verified**; does not promote symbolic-length Phase C/D.
  **Source-map `compiler_source`:** HlaX64 `485fca8` emits backend loci in
  `candidate.map.json`; CI asserts after Phase B Gate.
  **Fb4 Indexed (SemASM `ca11fc7`):** modeled `base+index*scale+disp`;
  under_preconditions ≠ proven_inside without const index.
  **Fb5 constant-index (SemASM `ebd5114`):** `index_const` →
  `proven_inside` on literal regions.
  **Fb6 range-guard (SemASM `5d81be5`):** `cmp`+`jae`/`jge` →
  `index_max_exclusive`.
  **Fb7 post-test induction (SemASM `53f8999`):**
  `xor; access; inc; cmp; jb`.
  **Fb8 countdown induction (SemASM `2351ce0`):**
  `mov N; dec; access; jnz`; Fb9 CFG-sound locked.
  **EchoAsm Gate depth:** concrete + scalar + Phase B loops
  (`sum_range` / `countdown_loop`) second-generator **Verified**.
  **Repair join depth:** offset map-join + Win64 `--map-line` + Win64
  signed + Win64 unsigned worktree live repair (`64d5344`→`9a41cb2`).
### Changed

- **Vd15 / find_* memory-leaf** — SemASM pin `928bd66`. Gate
  `find_first_byte` / `find_last_byte` (+ HlaX64) use
  `memory-leaf-affine-v1` with single-buffer `regions.equal`. Sample ≠
  formal memory safety; `verified_under_preconditions` ≠ unconditional
  `verified`.
- **Vd14 / memcmp memory-leaf** — SemASM pin `d2ce02d`. Gate `memcmp`
  (+ HlaX64) uses `memory-leaf-affine-v1` with dual-buffer regions +
  `regions.disjoint(a, b)`. Sample ≠ formal memory safety;
  `verified_under_preconditions` ≠ unconditional `verified`.
- **Vd13 / replace_byte memory-leaf** — SemASM pin `8924564`. Gate
  `replace_byte` (+ HlaX64) uses `memory-leaf-affine-v1`. Sample ≠ formal
  memory safety; `verified_under_preconditions` ≠ unconditional `verified`.
- **Vd12 / memset memory-leaf** — SemASM pin `0f9cd1e` (`regions.equal`
  atom on memset). Gate `memset` (+ HlaX64) uses `memory-leaf-affine-v1`.
  Sample ≠ formal memory safety; `verified_under_preconditions` ≠
  unconditional `verified`.
- **Vd11 / memory-leaf Gate** — SemASM pin `55f2542` (`region_access`
  `passed_under_preconditions` for symbolic lengths). Gate `memcpy` (+ HlaX64
  ingest/run) uses `memory-leaf-affine-v1` with `[function.memory]` + disjoint
  precondition. Profile accepts caller obligations on alias / region_access /
  contract-expr. Sample ≠ formal memory safety; Incomplete ≠ Verified;
  `verified_under_preconditions` ≠ unconditional `verified`.
- **Vd10** — SemASM pin `671c5e2` (x86 frame-spill affinity); HlaX64
  `count_byte` joins Gate `leaf-pure-v1`. Alias Incomplete from lost spill
  tracking no longer forces `semantic_failed`. Sample ≠ formal memory safety;
  `region_access` Incomplete remains observational.
- **Vd9 / Sei Gate wire** — Gate `count_byte` tasks use
  `verification.profile = leaf-pure-v1` (frozen alias + contract-expr
  requirements on lock). SemASM pin `b3c576e` (Ra tuple CI fix). Sample ≠
  formal memory safety; Incomplete ≠ Verified.
- **Sei P1b** — built-in `verification.profile` expansion (`leaf-pure-v1`,
  `memory-leaf-affine-v1`) into frozen `semantic_evidence` on lock. Profile
  definition drift cannot alter already-expanded digests.
- **Sei P1** — typed `SemanticEvidenceSummary` projection from raw SemASM
  report JSON; opt-in `verification.semantic_evidence.*` task policy; accept
  report schema `0.5`; map `verified_under_preconditions` without promoting to
  `verified` (legacy digests unchanged when policy unset).
- **Vd8 SemASM pin** — Gate workflows track SemASM tip `cf0206e` (Sei P0/Ra
  region-access + alias obligations; sample ≠ formal memory safety).
- **Vd7 SemASM pin** — Gate workflows track SemASM tip `bfd184e` (Tw/Ff/Ab
  post-`v0.2.1`; sample ≠ formal ABI / CFG / store proof).
- **Vd6 SemASM pin** — Gate workflows track SemASM `v0.2.1` tip `22d1543`
  (Co+Mm; sample ≠ CFG/CFI or region-precise proof).
- **Vd5 SemASM pin** — Gate workflows track SemASM Mm tip `e991182`
  (A64/RV `memory` leaf; sample ≠ region-precise proof).

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
- **Isolation ops proof (G4 / Io0–Io5)** — claim matrix
  (`docs/ISOLATION_OPS_PROOF_PLAN.md`); doctor honesty (sandbox wired, not
  library-only); optional `execution_sandbox_backend` (`local`);
  ContainerBackend network/socket/credential-env argv checklist. **Not**
  public-untrusted ready; LocalBackend ≠ container; C-012.
- **Trust ops proof (G5 / Tr0–Tr5)** — claim matrix
  (`docs/TRUST_ROOT_OPS_PROOF_PLAN.md`); `signer_kind` on seal signatures
  (`practice-ed25519` / `sigstore-dsse` / `hsm-pkcs11`); doctor/status
  `trust_policy`. **Production** trust root / hardware HSM / operated remote
  log as Gate default remain locked. Authenticity ≠ semantic truth.
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

- CryptOpt embed / live-model Gate CI
- Operated remote Rekor as Gate default / **hardware** HSM / Sigstore as Gate default
  (opt-in Rekor/Fulcio/SoftHSM clients already land under P7-T / P8-I / P8-K; G5
  labels them — production trust root stays locked)
- Hardened seccomp / verified rootless / OS-level generator FS isolation
- Full cargo-fuzz PR-022 security certification
- Auto `cargo publish`
- Cross-host bit-identical builds / cache as a trust root
