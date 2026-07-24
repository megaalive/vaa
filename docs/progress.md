# Implementation progress

Tracks evidence gates from `VAA_REVIEWED_AND_HARDENED_ARCHITECTURE_PLAN.md` §26–§27.

Honesty rule: **Done** means the listed acceptance exists as code. Levels of evidence are
called out separately (`unit-tested` / `integration-tested` / `verified-in-CI`).

**SemASM Region/Alias Evidence v1 (ADR 0006):** SemASM owns the analysis
(`function.memory` + `VerificationReport.alias_analysis` / `region-affine-v1`).
VAA consumes the report as evidence only — it does not re-implement alias
reasoning. Claim is selected affine relations for supported leaves; **not**
general alias analysis or formal memory safety. See SemASM
`docs/STABILIZATION_PROGRESS.md` (Ra0–Ra6 done).

**SemASM Contract Expression Semantics v1 (ADR 0007 / G2):** **done** (Ce0–Ce5).
SemASM evaluates a documented expression subset against living region/relation
evidence (`contract-expr-v1` / `VerificationReport.contract_expressions`).
VAA still only consumes `VerificationReport` — no expression engine in VAA.
G3 (A64/RV memory-effect parity) is **done** (SemASM ADR 0008 Me0–Me5 at
`51e8a96`: collectors + verify wire + ± fixtures; A64/RV `decode`/`lower`
stay `partial`). G4 (isolation ops proof) is **done** (Io0–Io5 at `c040828`:
[`docs/ISOLATION_OPS_PROOF_PLAN.md`](ISOLATION_OPS_PROOF_PLAN.md)). G5 (trust
**ops** proof) is **done** (Tr0–Tr5:
[`docs/TRUST_ROOT_OPS_PROOF_PLAN.md`](TRUST_ROOT_OPS_PROOF_PLAN.md) —
`signer_kind` labels + claim matrix). **Production** trust root / hardware HSM
/ operated remote log as Gate default remain Horizon-locked. Authenticity ≠
semantic truth.

| Gate | Status | Evidence level | Notes |
|---|---|---|---|
| Phase 0 — SemASM readiness | **Done** | docs | `docs/implementation-baseline.md` |
| PR-001 — Repository bootstrap | **Done** | CI | Rust crate, licenses, CI, dependency policy |
| PR-002 — Task schema v0.1 | **Done** | unit+CI | Typed model, strict parse, fixtures |
| PR-003 — Policy and immutable task digest | **Done** | unit+CI | Canonical JSON + SHA-256 |
| PR-004 — Run directory and event log | **Done** | unit | RunId, RunDir, EventLog (restart resume = R2) |
| PR-005 — SemASM doctor | **Done** | unit | ProcessRunner; version JSON + live status probe (R4+R5) |
| PR-006 — SemASM capabilities adapter | **Done** | unit | Embedded snapshot + optional live_probe compare |
| PR-007 — SemASM verification adapter | **Done** | unit | stdout-only VerificationReport **0.4** |
| PR-007b — Controller handshake | **Done** | unit | `--contract`, digests, golden fixture |
| PR-008 — Evidence aggregator | **Done** | unit | fail-closed + identity cross-checks |
| Phase 1 exit (`vaa verify`) | **Done** | unit | Offline report; live SemASM smoke ignored |
| PR-009 — Process runner | **Done** (streaming + tree kill) | unit | Byte cap; Win stdin EOF; Unix process group + Win Job Object (R3) |

| PR-010 — Build sandbox backend | **Scaffold** + B0/C0/C1 | unit | Docker argv: network/caps/no-new-privs/nobody/read-only/tmpfs; C1 optional host bind `/work`+`/input`. **Not** hardened isolation Done. |
| PR-011 — NASM/linker pipeline | **Done** | unit | Needs toolchain on PATH for live use |
| PR-012 — Artifact inspection | **Done** | unit | `object` crate |
| PR-013 — Harness templates | **Done** | unit | sysv64/win64 |
| PR-014 — Execution sandbox | **Done** | unit | Opt-in |
| PR-015 — Candidate protocol | **Done** | unit | Target match, digest map, attempt budget |
| PR-016 — Fixture model adapter | **Done** | unit | Queued wrong→repair + generation ids |
| PR-017 — Orchestrator state machine | **Done** | unit | Edges for repair |
| **R1 — `vaa run` wired** | **Done** | unit | Fixture loop + SemASM verify; live SemASM not in CI |
| **R2 — Seal + ingest** | **Done** | unit | integrity envelope; `vaa ingest`; `check-seal` |
| **R2b — Seal hardening** | **Done** | unit | acceptance/envelope; durable atomic publish; per-candidate chain; `verify-bundle` |
| **R2c — verify-chain + append-only** | **Done** | unit | `verify-chain`; full check details; exclusive candidate dirs; canonical vectors |
| **S0 — Slice lock** | **Done** | docs | Next CI slice = `count_byte` Gate-1 Incomplete; `sum_i64` = SemASM epic |
| **S2 — Gate-1 CI** | **Done** | CI | Windows job: live SemASM Incomplete + ingest `verify-chain` |
| **S3 — Gate-2 allow-execution** | **Done** | CI | `--allow-execution` plumbing + Win64 Verified smoke |
| **S4 — sum_i64 fixtures** | **Done** | CI | SemASM `wrapping_sum_i64` + VAA Win64 fixtures in Gate-1/2 |
| **H0 — HlaX64 bridge lock** | **Done** | docs | Roles: HlaX64 emit → SemASM verify → VAA seal |
| **H1–H3 — HlaX64 bridge** | **Done** | CI | ingest fixture + regen scripts + `hlax64-bridge` job |
| Phase 2–4 “vertical slice” claims | **Gate-1 Incomplete + Gate-2 Verified in CI** | CI | `count_byte` + `sum_i64` Win64 |

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
- `verify-chain` = contiguous hash chain + final seal + chain-wide identity (task/run/target/contract); deleting a predecessor fails verification.
- Append-only storage: exclusive candidate dirs + `create_new` writes.
- Integrity ≠ authenticity by default: SHA-256 envelope detects drift; opt-in Ed25519
  binds `acceptance_digest` when `VAA_SEAL_SIGNING_KEY` is set. See [`docs/seal.md`](seal.md).
- Canonicalization: [`docs/vaa-canonical-json-v1.md`](vaa-canonical-json-v1.md) + [`fixtures/canonical-json/`](../fixtures/canonical-json/).
- Atomic publication: temp `sync_all`, seal-last rename, post-rename file
  `sync_all`, Unix parent-dir sync (Windows directory sync best-effort).
- Positioning (honest): CryptOpt-like / Proof-Loop idea = candidates return to SemASM; acceptance digests sealed. Not a search engine or formal proof system.

### Still out of scope / later waves

- Disposable VM higher-assurance mode (§15.3)
- Hardware HSM (SoftHSM PKCS#11 Linux smoke is Done under P8-K — not a trust root)
- Embedding CryptOpt engine upstream

### Post-alpha harden (P7) — see [`docs/post-alpha-harden.md`](post-alpha-harden.md)

| Wave | Focus | Status |
|---|---|---|
| **P7-S** | Container C1 binds + seccomp + rootless probe + generator jail | **Done** (not absolute isolation) |
| **P7-D** | Durability probe + multi-file seal-last publish | **Done** (labels, not formal FS proof) |
| **P7-A** | SealSigner trait + DSSE + HSM scaffold | **Done** (practice keys ≠ trust root) |
| **P7-T** | Rekor publish/verify (mock + `--features rekor`) | **Done** (opt-in; Gate offline) |
| **P7-C** | `vaa search` nop-slide / mutator staging | **Done** (not formal superopt) |
| **P8-H** | CI Actions Node 24 pins | **Done** |
| **P8-F** | Fuller fuzz smoke (`fuzz/` + CI) | **Done** (not a formal audit) |
| **P8-K** | SoftHSM PKCS#11 live signer | **Done** (Linux smoke; SoftHSM ≠ hardware) |
| **P8-I** | Fulcio keyless DSSE opt-in | **Done** (manual workflow; Gate offline; ≠ SemASM Verified) |

### Planned vertical-slice waves (after R2c)

| Wave | Focus | Claim when done |
|---|---|---|
| **S2 Gate-1** | CI installs SemASM + toolchain; `vaa verify`/`ingest` + `verify-chain` on `count_byte` Win64 **without** `--allow-execution` | **Done** (Incomplete smoke) |
| **S3 Gate-2** | VAA forwards `--allow-execution`; CI assert `Verified` | **Done** (opt-in) |
| **S4** | SemASM ships `sum_i64` contract/oracle; VAA fixtures + CI | **Done** (`builtin.buffer.wrapping_sum_i64`) |
| **M4** | SemASM Tranche M handoff: `min_usize` Gate-1/2 | **Done** (`builtin.pure_int.binary_usize`) |
| **N2** | SemASM Tranche N handoff: `max_usize` Gate-1/2 | **Done** (pure-int claim `max`) |
| **P2** | SemASM Tranche P handoff: `find_first_byte` Gate-1/2 | **Done** (`builtin.buffer.find_first_u8`) |

### Next waves (Q0–Q2 + X0) — repair/search loop + x86 depth

| Wave | Focus | Status |
|---|---|---|
| **Q0** | Docs honesty: next = Tranche Q + further x86 depth | **Done** |
| **Q1** | `find_first_byte` multi-candidate `vaa run` wrong→repair Gate smoke | **Done** (Gate-1 ignored + CI) |
| **Q2** | `vaa search` nop-slide staging Gate smoke (offline; ≠ CryptOpt/Verified) | **Done** (Gate-1 ignored + CI) |
| **X0** | SemASM Win64 W+X object-policy twin (parity SysV) | **Done** (SemASM tip) |

Honesty: search/repair staging ≠ SemASM Verified; live-model stays opt-in/manual.

### Next waves (R0–R1 + X1) — search→ingest + object-policy depth

| Wave | Focus | Status |
|---|---|---|
| **R0** | Docs honesty: next = search→ingest + Win64 import/noexport | **Done** |
| **X1** | SemASM Win64 import + noexport object-policy twins | **Done** (SemASM tip) |
| **R1** | `vaa search` staging → `ingest` Gate smoke + verify-chain | **Done** (Gate-1 ignored + CI) |

Honesty: staged mutator output ≠ Verified until SemASM ingest; import/noexport ≠ execution proof.

### Next waves (X2 + S + T) — Win64 depth, find_last_byte, search-ingest

| Wave | Focus | Status |
|---|---|---|
| **X2a** | SemASM Win64 syscall + stack_imbalance twins | **Done** (SemASM tip) |
| **X2b** | VAA mutator `nop-before-ret` | **Done** (`9a490d3`) |
| **S0–S1** | SemASM `find_last_byte` oracle + Gate pack | **Done** |
| **S2** | VAA pin + Gate-1/2 + run wrong→repair | **Done** (`dcbc536`) |
| **T0** | Docs honesty: search `--ingest` loop; LLM opt-in; CI offline | **Done** |
| **T1** | `vaa search --ingest` skip Violated, stop on Incomplete | **Done** |
| **T2** | Gate smoke + closeout pin tip | **Done** (`1ad5d0e`) |

Honesty: Gate-1 Incomplete ≠ Verified. `search --ingest` ≠ CryptOpt. SoftHSM/Fulcio ≠ SemASM Verified.

### Next waves (X3 + U + V) — Win64 depth, memcmp Gate, search allow-exec

SemASM pin (Gate-1 / Gate-2 / `hlax64-bridge`):
`ca959f39924a34a3bca2a5effe71e96e63238250`
(U1 tip: X3 Win64 callee_saved + `memcmp` Gate pack).

| Wave | Focus | Status |
|---|---|---|
| **X3** | SemASM Win64 `count_byte` callee_saved + caps write/indirect sync | **Done** (SemASM `b9a7079`) |
| **U0–U1** | SemASM `memcmp` dual-buffer oracle + asm/e2e/CI | **Done** (SemASM `ca959f3`) |
| **V0** | Docs honesty: memcmp Gate + search allow-execution | **Done** |
| **V1** | Pin SemASM tip + `memcmp` Gate-1/2 + run wrong→repair | **Done** (`a9f926d`) |
| **V2** | Gate-2 `search --ingest --allow-execution` on `find_last` | **Done** (`a9f926d`) |
| **V3** | Closeout docs + pin tip both repos | **Done** |

Honesty: Gate-1 Incomplete ≠ Verified. Gate-2 search Verified is a SemASM
`--allow-execution` path, not CryptOpt. `memcmp` oracle ≠ formal `ensures`.
Default CI remains Gate-1 fail-closed (no `--allow-execution` on search).

Closeout tips: SemASM `ca959f39924a34a3bca2a5effe71e96e63238250` (Gate pin);
VAA Gate handoff `a9f926d` / V3 docs `789f7ad`.

### Next waves (X4 + H4 + Y) — memcmp fail-closed, find_last bridge, search parity

SemASM pin (Gate-1 / Gate-2 / `hlax64-bridge`):
`0c12bf732c0a1ad6ba0a7acaf15d1f84b3a4e620`
(X4 tip: A64/RV memcmp harness fail-closed).

HlaX64 pin (`hlax64-bridge`):
`3641428a6af4f22b6e7cd12febfeca65ac0d8d1b`
(`find_last_byte` SemASM/VAA bridge leaf).

| Wave | Focus | Status |
|---|---|---|
| **X4** | SemASM MemCmp harness fail-closed on AArch64/RISC-V | **Done** (`0c12bf7`) |
| **H4** | HlaX64 `find_last_byte` emit + VAA ingest/Gate bridge | **Done** (`e105ea0`) |
| **Y0** | Docs honesty: bridge + memcmp search-ingest parity | **Done** |
| **Y1** | `memcmp` `00_write_broken` + search Gate-1/2 smokes | **Done** (`1c43236`) |
| **Y2** | Closeout docs + pin tip | **Done** |

Honesty: Gate-1 Incomplete ≠ Verified. Gate-2 search Verified is SemASM
`--allow-execution` only (≠ CryptOpt). HlaX64 `-Wverify` ≠ SemASM Verified.
MemCmp harness remains x86-only; A64/RV fail closed.

Closeout tips: SemASM `0c12bf732c0a1ad6ba0a7acaf15d1f84b3a4e620`;
HlaX64 `3641428a6af4f22b6e7cd12febfeca65ac0d8d1b`;
VAA Gate handoff `1c432364be6d11bf31cd4bd99466258676e89270`.

### Next waves (X5 + H5 + Z) — caps sync, memcmp bridge, find_first search

SemASM pin (Gate-1 / Gate-2 / `hlax64-bridge`):
`03058461651629d880f3a2b08f92d2e101b6a450`
(X5 tip: SysV write/indirect + A64/RV evidence sync).

HlaX64 pin (`hlax64-bridge`):
`eeac3ba0c9f40f02b3e7fef487c29554df0c6573`
(`memcmp` SemASM/VAA bridge leaf).

| Wave | Focus | Status |
|---|---|---|
| **X5** | SemASM caps SysV write/indirect + A64/RV evidence | **Done** (`0305846`) |
| **H5** | HlaX64 `memcmp` emit + VAA ingest/Gate bridge | **Done** (`85a2dba`) |
| **Z0** | Docs honesty: memcmp bridge + find_first search parity | **Done** |
| **Z1** | `find_first` `00_write_broken` + search Gate-1/2 smokes | **Done** (`9c2203e`) |
| **Z2** | Closeout docs + pin tip | **Done** |

Honesty: Gate-1 Incomplete ≠ Verified. Gate-2 search Verified is SemASM
`--allow-execution` only (≠ CryptOpt). HlaX64 `-Wverify` ≠ SemASM Verified.
MemCmp harness remains x86-only; A64/RV fail closed.

Closeout tips: SemASM `03058461651629d880f3a2b08f92d2e101b6a450`;
HlaX64 `eeac3ba0c9f40f02b3e7fef487c29554df0c6573`;
VAA Gate handoff `9c2203e3fe8df8fccf760717ec6779b0dacb1e82`.

### Maturity inflection (D0–D2) — design only; leaf treadmill paused

After X5 + H5 + Z the leaf/Gate/bridge treadmill is saturated. **Pause** new
oracle leaves, HlaX64 bridges, and search-parity waves except bugfix / pin
tip. Next investment is design (D0–D2), then a separate **W\*** write-shape
implementation plan only after SemASM ADR 0003 is Accepted.

| Wave | Focus | Status |
|---|---|---|
| **D0** | Freeze + inventory honesty (this section) | **Done** |
| **D1** | SemASM ADR write-shape (`replace_byte` v1) | **Done** (Proposed in SemASM `adr/0003`) |
| **D2** | Pipeline maturity checklist + Gate-2 isolation criteria | **Done** (notes below) |

#### Inventory (D0)

| Leaf | VAA Gate | search `--ingest` | HlaX64 bridge |
|---|---|---|---|
| `count_byte` | yes | yes | yes (Th1) |
| `find_first_byte` | yes | yes (Z) | yes (Th2) |
| `find_last_byte` | yes | yes | yes (H4) |
| `memcmp` | yes | yes (Y) | yes (H5) |
| `sum_i64` | yes | — | yes (H1) |
| `min_usize` / `max_usize` | yes | — | yes (Th8) |
| `replace_byte` | yes (W3) | — | yes (W4) |
| `memset` | yes (Wm3) | — | yes (Th5) |
| `memcpy` | yes (Wc) | yes (Th6) | yes (Th7) |

**Intentionally not continued now:** A64/RV MemCmp/replace harness;
CryptOpt embed; formal ensures. (Pure-int (`min_usize`/`max_usize`) HlaX64
bridges landed as **Th8** — residual Thin is now closed; see the Th8
section below.)

Honesty: Incomplete ≠ Verified. Gate-2 Verified is SemASM `--allow-execution`
only. HlaX64 `-Wverify` ≠ SemASM Verified. D* does not bump SemASM pipeline
maturity and does not wire `ExecutionSandbox` into Gate.

#### Gate-2 isolation honesty (D2 companion)

See also `docs/post-alpha-harden.md` § Gate-2 isolation + G4 ops proof.

- Gate-2 CI may claim SemASM Verified under `--allow-execution` only.
- Opt-in `--execution-sandbox` wires `ExecutionSandbox` (LocalBackend);
  `execution_isolation=sandbox` + `execution_sandbox_backend=local` (G4).
  LocalBackend ≠ container; C-012.
- Do not equate “Verified” with container/seccomp isolation (C-012).
- Pipeline maturity bumps remain SemASM-owned (`capabilities.toml` + owner job).

#### After ADR 0003 Accept — W\* (separate plan)

Outline only: W0 contract/oracle → W1 harness/memory gate → W2 x86 e2e →
W3 VAA Gate → W4 optional HlaX64. Not started in this tranche.

### Maturity follow-up (M0–M1) — deepen then SemASM pipeline bump — closed

After D0–D2: deepen criteria, then SemASM bumped x86 pipeline maturity.
I1/I2 and G4 isolation ops proof landed. Write-shape ADR Accepted;
SemASM/VAA `replace_byte` Gate in tranche W*.

| Wave | Focus | Status |
|---|---|---|
| **M0** | Deepen Gate-2 I0–I2 phases + point at SemASM ownership map | **Done** |
| **M1** | SemASM bind `ci_jobs` + bump assemble/link/execute/pipeline_verify | **Done** (SemASM tip after M1) |

SemASM M1 closed: x86 pipeline `verified_in_ci` with owner e2e jobs. Sandbox
**I1/I2** landed on VAA (`execution_isolation` + `--execution-sandbox`).

### Write-shape v1 (W0–W3) — `replace_byte`

SemASM pin (Gate-1 / Gate-2 / `hlax64-bridge`):
`e2bbed6cc5b0eb1792cf8a6db3f80d729032cb4b`

| Wave | Focus | Status |
|---|---|---|
| **W0–W2** | ADR Accept + SemASM replace_byte oracle/harness/asm | **Done** (SemASM `e2bbed6`) |
| **W3** | VAA Gate fixtures + pin | **Done** |

Honesty: not all buffer leaves are read-only (`replace_byte` declares
`memory_write`). Incomplete ≠ Verified. Region-precise store proof deferred.
HlaX64 replace bridge landed in **W4** (see below) — no longer deferred.

Gate-2 isolation phases (see `post-alpha-harden.md`):

| Phase | Status |
|---|---|
| **I0** host `--allow-execution` | current |
| **I1** `execution_isolation` evidence + Gate-2 assert | **landed** |
| **I2** `--execution-sandbox` + sandbox CI claim | **landed** (LocalBackend ≠ container) |

### Write-shape v2 (Wm3) — `memset`

SemASM pin (Gate-1 / Gate-2 / `hlax64-bridge`):
`0b5d115d2838c7243d6ae93373c8f55e5b27dff8`

| Wave | Focus | Status |
|---|---|---|
| **Wm0–Wm2** | SemASM `memset` contract/oracle + `HarnessShape::Memset` + x86 asm/e2e/caps | **Done** (SemASM, upstream) |
| **Wm3** | VAA Gate fixtures + pin | **Done** |

Oracle: `builtin.buffer.memset` (v1). Harness verifies the always-`0` return
**and** that every `buffer[0..length]` byte equals `value` after the call.
`memset` vectors are layout-identical to the read-only `BufferScan` shape;
SemASM's `resolve_harness_shape` disambiguates from the recognized contract
oracle, not vector layout, so VAA does not need any special-casing here.
I1/I2 (`execution_isolation` evidence + `--execution-sandbox`) already landed
on VAA in the M0–M1 tranche; this wave does not retouch that path.

Honesty: Gate-1 Incomplete ≠ Verified. `memset` oracle/vectors ≠ formal
`ensures`/region-precise store proof. HlaX64 `memset` bridge remains
deferred (W4 landed the `replace_byte` bridge only; see below).

### Write-shape v3 (Wc) — `memcpy`

SemASM pin (Gate-1 / Gate-2 / `hlax64-bridge`):
`247b01586dd626e3c4261e170cb73565e7b7b54c`

| Wave | Focus | Status |
|---|---|---|
| **Wc0–Wc2** | SemASM `memcpy` contract/oracle + `HarnessShape::Memcpy` + x86 asm/e2e/caps | **Done** (SemASM, upstream) |
| **Wc3** | VAA Gate fixtures + pin | **Done** |

Oracle: `builtin.buffer.memcpy` (v1). Harness verifies the always-`0` return
**and** that every `dst[0..length]` byte equals `src[0..length]` after the
call; `src` is unchanged input and is never echoed back. `dst`/`src` wire
layout is indistinguishable from the dual-buffer `MemCmp` shape by design;
SemASM's `resolve_harness_shape` disambiguates from the recognized contract
oracle, not vector layout. Overlapping/aliasing `dst`/`src` is out of scope
(SemASM ADR 0003, "overlap fail-closed"): VAA fixtures use distinct,
non-overlapping buffers only. I1/I2 sandbox evidence already landed on VAA
in the M0–M1 tranche; this wave does not retouch that path.

Honesty: Gate-1 Incomplete ≠ Verified. `memcpy` oracle/vectors ≠ formal
`ensures`/region-precise store proof. HlaX64 `memcpy` bridge remains
deferred (W4 landed the `replace_byte` bridge only; see below).

### Next maturity program

Multi-tranche + Thin + **Horizon Closeout (H0-H6)** are **closed**.

Landed in Horizon: guard-byte Rmem (H2), A64/RV MemCmp harness (H3), Dx deepen
without maturity bump (H4), multi-ISA ADR 0005 (H1), remote-transparency honesty
(H5). **Post-Horizon priority landed:** A64/RV write-shape harness
(`replace_byte`/`memset`/`memcpy`). **Post-Horizon:** x86-64 `decode`/`lower`
→ `verified_in_ci` (Dx owner sign-off; A64/RV stay `partial`).
**Horizon-locked deferred:** formal ensures, full symbolic alias, CryptOpt
embed, live-model Gate CI, hardware HSM.

Honesty: SoftHSM ≠ HSM; search ≠ CryptOpt; Incomplete ≠ Verified; HlaX64 ≠
SemASM Verified; local transparency artifact ≠ remote log.

| Wave | Focus | Status |
|---|---|---|
| **H0–H6** | Horizon Closeout program | **Done** |

SemASM pin: `0bb8b813fa1c749ba0a5a760ccb674f6b017e2c3`

### W4 — HlaX64 `replace_byte` bridge

SemASM pin (Gate-1 / Gate-2 / `hlax64-bridge`):
`f20d9791683becf50fc7ac6846fc06666697ad92`
(Dx tip: decode/lower checklist + adversarial wave; caps stay `partial`).

HlaX64 pin (`hlax64-bridge`):
`909553cbad0b5c357a97e21a030153ef9d5648d8`
(`examples: add replace_byte SemASM/VAA bridge leaf`).

| Wave | Focus | Status |
|---|---|---|
| **W4** | HlaX64 `replace_byte.hla64` emit + VAA `fixtures/ingest/hlax64_replace_byte/` + Gate-1/2 + CI | **Done** |

Fourth HlaX64 leaf (after `sum_i64` H1, `find_last_byte` H4, `memcmp` H5).
`replace_byte` writes to `buffer` (not read-only, unlike the three prior
bridged leaves) — same oracle as the write-shape v1 `replace_byte` fixtures
under `fixtures/semasm/replace_byte/` (`builtin.buffer.replace_byte`).

Honesty: HlaX64 emit / `-Wverify` ≠ SemASM `verified`. Gate-1 Incomplete
(no `--allow-execution`) ≠ Gate-2 Verified. `replace_byte` oracle/vectors ≠
formal `ensures`/region-precise store proof (that is Rmem's job, on SemASM,
not this bridge). Update the D0 inventory table: `replace_byte` HlaX64
bridge moves from "paused" to "yes (W4)".

### Dx — decode/lower depth (SemASM)

SemASM tip `f20d979` documents the Dx bump checklist and lands an adversarial
wave (`cpuid` unknown-insn + `find_first_byte` trailing-bytes twins).
`decode`/`lower` remain **`partial`** — no maturity bump.

### Thin (Th1–Th2) — HlaX64 `count_byte` + `find_first_byte` bridges

SemASM pin (Gate-1 / Gate-2 / `hlax64-bridge`):
`f20d9791683becf50fc7ac6846fc06666697ad92`
(Dx tip: decode/lower checklist + adversarial wave; caps stay `partial`;
unchanged from W4 — Thin lands no new SemASM work).

HlaX64 pin (`hlax64-bridge`):
`5e29a4442b7db01c4bb396aea363e01d8067b750`
(`examples: add count_byte and find_first_byte SemASM/VAA bridges`).

| Wave | Focus | Status |
|---|---|---|
| **Th1** | HlaX64 `count_byte.hla64` emit + VAA `fixtures/ingest/hlax64_count_byte/` + Gate-1/2 + CI | **Done** |
| **Th2** | HlaX64 `find_first_byte.hla64` emit + VAA `fixtures/ingest/hlax64_find_first_byte/` + Gate-1/2 + CI | **Done** |

Fifth and sixth HlaX64 leaves (after `sum_i64` H1, `find_last_byte` H4,
`memcmp` H5, `replace_byte` W4). Both `count_byte` and `find_first_byte`
are read-only buffer scans (like `memcmp`/`find_last_byte`) — same oracles
as the existing `fixtures/semasm/count_byte/` (`builtin.buffer.count_equal_u8`
v2) and `fixtures/semasm/find_first_byte/` (`builtin.buffer.find_first_u8`
v1) fixtures.

Honesty: HlaX64 emit / `-Wverify` ≠ SemASM `verified`. Gate-1 Incomplete
(no `--allow-execution`) ≠ Gate-2 Verified. `count_byte`/`find_first_byte`
oracle/vectors ≠ formal `ensures`/region-precise proof. Update the D0
inventory table: `count_byte` HlaX64 bridge moves from "paused" to
"yes (Th1)"; `find_first_byte` HlaX64 bridge moves from "paused" to
"yes (Th2)".

### Th5 — HlaX64 `memset` bridge

SemASM pin (Gate-1 / Gate-2 / `hlax64-bridge`):
`fb2dac5d6e32c86beac15e13f1f64ceb78fd5ab6`
(docs-only sync marking `count_byte`/`find_first_byte` HlaX64 bridges landed
on VAA; functional tip unchanged from Dx `f20d979`).

HlaX64 pin (`hlax64-bridge`):
`b0ef50ffc9de06f4cb8695d861980ea5286d9e92`
(`examples: add memset SemASM/VAA bridge leaf`).

| Wave | Focus | Status |
|---|---|---|
| **Th5** | HlaX64 `memset.hla64` emit + VAA `fixtures/ingest/hlax64_memset/` + Gate-1/2 + CI | **Done** |

Seventh HlaX64 leaf (after `sum_i64` H1, `find_last_byte` H4, `memcmp` H5,
`replace_byte` W4, `count_byte`/`find_first_byte` Th1/Th2). `memset` writes to
`buffer` (not read-only, like `replace_byte`) — same oracle as the
write-shape v2 `memset` fixtures already Gate-1/2'd under
`fixtures/semasm/memset/` (`builtin.buffer.memset`, landed in **Wm3**); this
wave only adds the HlaX64 emit bridge (`fixtures/ingest/hlax64_memset/` +
`scripts/regen-hlax64-memset.{ps1,sh}` + `hlax64-bridge` CI wiring), mirroring
**W4**'s `replace_byte` bridge shape exactly.

`memset.hla64` sets the constant status return (`rax = 0`) only once, after
the fill loop — the same bug class W4 called out for `replace_byte`: the
byte-store lowering (`mov(value, [r10].byte)`) routes the stored value
through `rax`/`al` as scratch (`mov rax, [rbp-24]` / `mov byte [r10], al`
in the emitted NASM), so initializing the status accumulator early or
inside the loop would silently clobber it.

Honesty: HlaX64 emit / `-Wverify` ≠ SemASM `verified`. Gate-1 Incomplete
(no `--allow-execution`) ≠ Gate-2 Verified. `memset` oracle/vectors ≠ formal
`ensures`/region-precise store proof (that remains Rmem's honesty statement,
on SemASM, not this bridge). Update the D0 inventory table: `memset` HlaX64
bridge moves from "—" to "yes (Th5)".

### Th7 — HlaX64 `memcpy` bridge

SemASM pin (Gate-1 / Gate-2 / `hlax64-bridge`):
`fb2dac5d6e32c86beac15e13f1f64ceb78fd5ab6`
(unchanged from Th5/Th6 — current `megaalive/semasm` tip; no new SemASM
behavioral change is required for this bridge).

HlaX64 pin (`hlax64-bridge`):
`209ac5b13b954c771fca8f2257cde7486873a846`
(`examples: add memcpy SemASM/VAA bridge leaf`).

| Wave | Focus | Status |
|---|---|---|
| **Th7** | HlaX64 `memcpy.hla64` emit + VAA `fixtures/ingest/hlax64_memcpy/` + Gate-1/2 + CI | **Done** |

Eighth HlaX64 leaf (after `sum_i64` H1, `find_last_byte` H4, `memcmp` H5,
`replace_byte` W4, `count_byte`/`find_first_byte` Th1/Th2, `memset` Th5).
`memcpy` writes to `dst` and reads `src` — dual-pointer like `memcmp`, write
like `replace_byte`/`memset` — same oracle as the write-shape v3 `memcpy`
fixtures already Gate-1/2'd under `fixtures/semasm/memcpy/`
(`builtin.buffer.memcpy`, landed in **Wc**; search-ingest parity landed in
**Th6**); this wave only adds the HlaX64 emit bridge
(`fixtures/ingest/hlax64_memcpy/` + `scripts/regen-hlax64-memcpy.{ps1,sh}` +
`hlax64-bridge` CI wiring), mirroring **Th5**'s `memset` bridge shape
exactly.

`memcpy.hla64` loads each `src` byte into a scratch register (`r8`) and
stores it straight into `dst`. Unlike `memset`/`replace_byte` (whose stored
value is a stack-spilled parameter, so the byte-store lowering routes it
through `rax`/`al` as scratch), the loaded `src` byte here is already
register-resident, so the emitted store lowers directly
(`movzx r8, byte [r11]` / `mov byte [r10], r8b`) with no `rax`/`al` scratch
routing at all. The constant status return (`rax = 0`) is still set only
once, after the copy loop, for the same defense-in-depth reason as **Th5**.

Honesty: HlaX64 emit / `-Wverify` ≠ SemASM `verified`. Gate-1 Incomplete
(no `--allow-execution`) ≠ Gate-2 Verified. `memcpy` oracle/vectors ≠ formal
`ensures`/region-precise store proof (that remains Rmem's honesty statement,
on SemASM, not this bridge). Non-overlapping `dst`/`src` assumed (SemASM ADR
0003, "overlap fail-closed"). Update the D0 inventory table: `memcpy` HlaX64
bridge moves from "—" to "yes (Th7)".

### Th8 — HlaX64 `min_usize`/`max_usize` bridges (residual Thin closed)

SemASM pin (Gate-1 / Gate-2 / `hlax64-bridge`):
`eb79db5411ae1159efdda688746639bc8dbecb95`
(`docs(Thin): mark min_usize/max_usize HlaX64 bridges landed`; docs-only tip,
no behavioral change to `builtin.pure_int.binary_usize` or any other oracle).

HlaX64 pin (`hlax64-bridge`):
`90cbf3b24faef253467a43ef270656dc6de49f0b`
(`examples: add min_usize and max_usize SemASM/VAA bridges`).

| Wave | Focus | Status |
|---|---|---|
| **Th8** | HlaX64 `min_usize.hla64`/`max_usize.hla64` emit + VAA `fixtures/ingest/hlax64_min_usize/` + `fixtures/ingest/hlax64_max_usize/` + Gate-1/2 + CI | **Done** |

Ninth/tenth HlaX64 leaves (after `sum_i64` H1, `find_last_byte` H4, `memcmp`
H5, `replace_byte` W4, `count_byte`/`find_first_byte` Th1/Th2, `memset` Th5,
`memcpy` Th7). `min_usize`/`max_usize` are pure-integer leaves — no
buffer/pointer arguments, no memory effects — same oracle as the
`fixtures/semasm/min_usize/` / `fixtures/semasm/max_usize/` fixtures already
Gate-1/2'd (`builtin.pure_int.binary_usize` v2, landed in **M2–M4**/**N0–N2**);
this wave only adds the HlaX64 emit bridges
(`fixtures/ingest/hlax64_min_usize/` + `fixtures/ingest/hlax64_max_usize/` +
`scripts/regen-hlax64-min_usize.{ps1,sh}` +
`scripts/regen-hlax64-max_usize.{ps1,sh}` + `hlax64-bridge` CI wiring),
mirroring **Th1**/**Th2**'s read-only-shape bridge pattern (no write-shape
concerns here — pure integers, not buffers).

`min_usize.hla64`/`max_usize.hla64` load both parameters into scratch
registers (`r8`/`r9`) before comparing with the unsigned HlaX64 operators
(`<?`/`>?`, matching the `usize` contract type) and branching to `mov`
whichever register holds the result into `rax`. This is a deliberate
work-around for an HlaX64 code-generation gap found while authoring this
bridge: comparing two *stack-spilled* parameters directly (e.g. `if(a <? b)`
with `a`/`b` left as `[rbp-N]` operands) lowers to `cmp qword [rbp-8], qword
[rbp-16]` — an invalid x86 mem-mem operand pair that NASM rejects outright.
Loading each operand into a register first (`mov(a, r8); mov(b, r9); if(r8
<? r9)`) avoids the gap and lowers to a valid `cmp r8, r9`. The committed
`candidate.asm` files were confirmed to assemble (`nasm -f win64`) and to
fully **Verify** end-to-end via a local `semasm agent verify --allow-execution`
run (6/6 oracle vectors passed for both `min` and `max`) before being frozen
as fixtures; the `hlax64_min_usize_candidate_is_framed_win64` /
`hlax64_max_usize_candidate_is_framed_win64` tests in
`tests/semasm_gate1_smoke.rs` assert the exported symbol, the
register-loaded compare, and the Win64 frame shape.

Honesty: HlaX64 emit / `-Wverify` ≠ SemASM `verified`. Gate-1 Incomplete
(no `--allow-execution`) ≠ Gate-2 Verified. `min_usize`/`max_usize`
oracle/vectors ≠ formal `ensures`/general theorem proving. Update the D0
inventory table: `min_usize`/`max_usize` HlaX64 bridge moves from "—" to
"yes (Th8)". **Residual Thin is now closed** — Thin Th1–Th8 are all
**Done**. Horizon Closeout (H0–H6) is also **closed**; x86-64 `decode`/`lower`
→ `verified_in_ci` landed under Dx owner sign-off (A64/RV stay `partial`).
**Horizon-locked deferred:** formal ensures, CryptOpt embed, hardware HSM,
live-model Gate CI — see Next maturity program above.

### HlaX64 → SemASM → VAA bridge (after S4)

Roles (do not conflate):

| Layer | Owns | Does not own |
|---|---|---|
| **HlaX64** | Authoring `.hla64` → NASM (`hla64 emit-nasm`) | Verification status / seals |
| **SemASM** | Contract + behavioral oracle + `VerificationReport` 0.4 | Task policy / evidence chain |
| **VAA** | Task lock, `ingest`/`verify`, seal chain | Generating assembly |

First leaf: `sum_i64` (Win64). Second leaf: `find_last_byte` (Win64, H4).
Third leaf: `memcmp` (Win64, H5). Fourth leaf: `replace_byte` (Win64, W4).
Fifth/sixth leaves: `count_byte` / `find_first_byte` (Win64, Th1/Th2).
Seventh leaf: `memset` (Win64, Th5). Eighth leaf: `memcpy` (Win64, Th7).
Ninth/tenth leaves: `min_usize` / `max_usize` (Win64, Th8).
Generator label: `--generator hlax64`.

| Wave | Focus | Claim when done |
|---|---|---|
| **H0** | Lock roles in docs | **Done** |
| **H1** | HlaX64 example + frozen NASM ingest fixture + Gate smoke | **Done** |
| **H2** | `scripts/regen-hlax64-sum_i64` | **Done** |
| **H3** | CI checkout HlaX64 + emit-nasm + verify | **Done** (`hlax64-bridge` job) |
| **H4** | `find_last_byte` example + ingest freeze + Gate | **Done** |
| **H5** | `memcmp` example + ingest freeze + Gate | **Done** |

Honesty: HlaX64 `-Wverify` ≠ SemASM `verified`. Gate-1 Incomplete ≠ Verified.

Do **not** call Gate-1 a “verified vertical slice”.

### Next waves (N0–N4) — SemASM tip pin + framed smoke

SemASM pin (Gate-1 / Gate-2 / `hlax64-bridge`):
`2683cf090b8c182c3db13b955a1a4daa870da7f8`
(U1 tip: X3 `memcmp` + U Gate pack).

HlaX64 pin (`hlax64-bridge` only):
`4c797893e0714f64faf1ae2f67ddf26c44f06d91`
(`examples: add sum_i64 SemASM/VAA bridge leaf`).

| Wave | Focus | Status |
|---|---|---|
| **N0** | Push SemASM tip + CI green for T0–T6 | **Done** |
| **N1** | Pin SemASM SHA in VAA CI (not floating `main`) | **Done** |
| **N2** | Refresh `sum_i64` consumer goldens (oracle v2) | **Done** |
| **N3** | Framed `sum_i64` Gate smoke + fixture shape lock | **Done** |
| **N4** | Honesty docs (this file + SemASM progress) | **Done** |

### Next waves (P0–P2) — stack integrity

| Wave | Focus | Status |
|---|---|---|
| **P0** | Pin HlaX64 SHA + refresh SemASM pin to tip | **Done** |
| **P1** | Honesty docs sync (baseline / ROADMAP / progress) | **Done** |
| **P2** | Capability claim bind (`source=vaa_embedded_agent_verify_snapshot`) | **Done** |

Honesty: VAA `capabilities` JSON is an **embedded agent-verify snapshot**, not a
live read of SemASM `capabilities.toml` (pipeline maturity on SemASM x86 is
`verified_in_ci` after M1; VAA embedded snapshot may lag).

### Next waves (R0–R6) — runner + SemASM JSON + process tree + live probe

| Wave | Focus | Status |
|---|---|---|
| **R0** | Honesty docs: P* closed; next = R* | **Done** |
| **R1** | ProcessRunner streaming byte cap + Win stdin EOF (PR-009b) | **Done** |
| **R2** | SemASM `version`/`status --format json` | **Done** |
| **R3** | Process-group / Job Object at spawn (PR-009c) | **Done** |
| **R4+R5** | Live status compare + doctor version JSON (merged) | **Done** |
| **R6** | Gate CI doctor Available + Gate golden live_probe aligned | **Done** |

Later (not this tranche): Ed25519 seal authenticity if seals cross a trust
boundary.

### Next waves (D0) — seal durability

| Wave | Focus | Status |
|---|---|---|
| **D0** | Post-rename file `sync_all` + Unix dir sync (Win dir best-effort) | **Done** |

### Next waves (L0–L1 + B0) — local seal log + container argv

| Wave | Focus | Status |
|---|---|---|
| **L0** | Append-only `evidence/seal-log.jsonl` on candidate seal | **Done** |
| **L1** | `verify-chain` checks seal-log when present | **Done** |
| **B0** | ContainerBackend fail-closed network/caps + digest image ref | **Done** |

### Next waves (T0 + A0 + C0) — transparency export, Ed25519, deeper container argv

| Wave | Focus | Status |
|---|---|---|
| **T0** | `vaa-transparency-v1` export/verify + Gate CI artifact upload | **Done** |
| **A0** | Opt-in Ed25519 over `acceptance_digest` + `keygen-seal` | **Done** |
| **C0** | Deeper ContainerBackend argv (read-only/tmpfs/user/no-new-privs) | **Done** (still Scaffold) |

Honesty: CI transparency artifact is **not** a remote immutable log. Container C0 is **not** “hardened isolation Done”.

### Next waves (A1 + T1) — CI signed seals + verify-transparency

| Wave | Focus | Status |
|---|---|---|
| **A1** | Ephemeral CI `keygen-seal` + `VAA_REQUIRE_SEAL_SIGNATURE` on Gate ingest | **Done** |
| **T1** | `verify-transparency` after export; artifact includes practice public key | **Done** |

Honesty: ephemeral CI signing key is **not** a trust root. `verify-transparency` does **not** make the artifact a remote immutable log.

### Next waves (G0) — logical evidence write barrier

| Wave | Focus | Status |
|---|---|---|
| **G0** | `RunDir` ProtectedZone + `staging/`; `vaa generate --run-dir` | **Done** (logical API barrier ≠ OS isolation) |

### Next waves (C1 + D1 + R-prep)

| Wave | Focus | Status |
|---|---|---|
| **C1** | ContainerBackend optional host bind mounts for `/work` + `/input` | **Done** (still Scaffold) |
| **D1** | Doctor JSON/terminal `evidence_policy` (G0 honesty) | **Done** |
| **R-prep** | [`docs/release-v0.1-checklist.md`](release-v0.1-checklist.md) | **Done** (no git tag) |

Later: remote transparency service, HSM, full PR-010 hardened sandbox, live model, CryptOpt, `v0.1.0` **tag ceremony**.

### Next waves (I0 + E0 + B2) — policy truth

| Wave | Focus | Status |
|---|---|---|
| **I0** | Honor `require_object_inspection` via `ArtifactInspector` on verify/seal | **Done** (unit; not all object formats claimed) |
| **E0** | Persist `events.jsonl` on `vaa run` / `ingest` lifecycle | **Done** (unit; crash resume = E1) |
| **B2** | Enforce task `Budgets` (candidates / wall / no-progress) → exit 7 | **Done** (unit; not token/cost accounting) |

### Next waves (R7 + A2) — Gate parity

| Wave | Focus | Status |
|---|---|---|
| **R7** | Multi-candidate `vaa run` Gate smoke (`count_byte` wrong→repair) | **Done** (Gate-1 ignored + CI) |
| **A2** | Gate-2 signed ingest/seal + transparency parity | **Done** (CI; practice key ≠ trust root) |

### Next waves (C2…E1)

| Wave | Focus | Status |
|---|---|---|
| **C2/C2b** | Wire `ContainerBackend` + honor `cpu_quota`/`pids_limit` | **Done** (Scaffold) |
| **B1** | Build tool digests in manifest | **Done** (unit; not bit-identical cross-host) |
| **N5** | `cargo deny` + light negatives | **Done** |
| **D2** | Doc sync honesty | **Done** |
| **E1** | Resume from sealed chain + events | **Done** (unit; not multi-host) |

### Next waves (G1…R-tag)

| Wave | Focus | Status |
|---|---|---|
| **D3** | Doc/checklist sync pasca-E1 | **Done** |
| **N6** | Negative corpus under `fixtures/negative/` | **Done** (not full fuzz) |
| **G1** | External argv generator → `staging/` | **Done** (Scaffold; not OS FS isolation) |
| **E1b** | Resume smoke in Gate-1 CI | **Done** |
| **L1/L2** | Linux fixtures + ubuntu Gate jobs | **Done** (VAA+SemASM pin smoke; qemu for Linux Verified; not SemASM upstream Linux CI claim) |
| **R-notes** | CHANGELOG + checklist closeout (tag deferred) | **Done** |

### Next waves (Phase 5+) — architecture plan §26–§27

| Wave | Focus | Status |
|---|---|---|
| **PR-019** | Live OpenAI-compatible adapter (`live-model` + `--live`) | **Done** (opt-in; CI stays offline-deterministic) |
| **PR-020** | Content-addressed cache | **Done** (local `.vaa/cache`; `--cache` opt-in; not remote log) |
| **PR-021** | Reproducibility report | **Done** (same-host twin assemble/build; not cross-host) |
| **PR-022** | Negative corpus + fuzz entry points | **Done** (N5/N6 + cache negatives + `fuzz/` smoke CI; not a formal audit) |
| **PR-023 / R-tag** | Alpha release gate + git tag | **Done** (`v0.1.0` annotated tag + GitHub Release, 2026-07-23) |
| **P7-S…C** | Post-alpha harden / trust / search | **Done** (see Post-alpha harden table; honesty in `docs/post-alpha-harden.md`) |

### Next waves (Th1…Th4, Th6) — Thin tranche: write-shape search-ingest Gate parity

Thin tranche closes the remaining `search --ingest` Gate parity gap for the
multi-candidate leaves that already carry Gate-1/2 fixtures: `memcmp`
(Th1, landed as **Y1**), `find_first_byte` (Th2, landed as **Z1**),
`replace_byte` (Th3), `memset` (Th4), and `memcpy` (Th6, this wave).
(Th5 landed the HlaX64 `memset` bridge separately — see the Th5/Th7 sections
above, not this search-ingest table.)

| Wave | Focus | Status |
|---|---|---|
| **Th1** | `memcmp` `00_write_broken` + search Gate-1/2 smokes | **Done** (see **Y1**) |
| **Th2** | `find_first_byte` `00_write_broken` + search Gate-1/2 smokes | **Done** (see **Z1**) |
| **Th3** | `replace_byte` `fixtures/run/replace_byte/` + search Gate-1/2 smokes | **Done** |
| **Th4** | `memset` `fixtures/run/memset/` + search Gate-1/2 smokes | **Done** |
| **Th6** | `memcpy` `fixtures/run/memcpy/` + search Gate-1/2 smokes | **Done** |

**Th3** adds `fixtures/run/replace_byte/` (`replace_byte.vaa.toml` +
`replace_byte.sem.toml` copied from `fixtures/semasm/replace_byte/`,
`00_write_broken.asm`, `01_wrong.asm`, `02_repaired.asm`, `README.md`) plus
`gate1_search_ingest_replace_byte_nop_before_ret_stops_on_incomplete`,
`gate1_search_ingest_replace_byte_skips_violated_budget`, and
`gate2_search_ingest_replace_byte_allow_execution_verified` in the existing
Gate-1/2 smoke suites. No CI workflow edits were needed: `semasm-gate1` /
`semasm-gate2` already run their whole ignored suite (`-- --ignored`), and no
workflow invokes a `gate1_search_ingest_*` / `gate2_search_ingest_*` test by
name.

Honesty: `replace_byte` writes to `buffer` (not read-only, unlike `memcmp` /
`find_first_byte`) — `01_wrong.asm` counts matches correctly but never stores
the replacement, and `00_write_broken.asm` writes one byte past
`buffer[0..length]` regardless of match state; both still resolve through the
same SemASM `verified` / `behavior_failed` / `execution_denied` raw statuses,
so Gate-1 Incomplete ≠ Verified and Gate-2 Verified is SemASM
`--allow-execution` only (≠ CryptOpt), same as **Th1**/**Th2**.

**Th4** mirrors **Th3** exactly for `memset`: adds `fixtures/run/memset/`
(`memset.vaa.toml` + `memset.sem.toml` copied from `fixtures/semasm/memset/`,
`00_write_broken.asm`, `01_wrong.asm` copied from the SemASM
`memset_wrong_win64.asm` shape, `02_repaired.asm` copied from
`fixtures/semasm/memset/memset_win64.asm`, `README.md`) plus
`gate1_search_ingest_memset_nop_before_ret_stops_on_incomplete`,
`gate1_search_ingest_memset_skips_violated_budget`, and
`gate2_search_ingest_memset_allow_execution_verified` in the existing Gate-1/2
smoke suites. No new CI *jobs* were needed for the same reason as **Th3**
(`semasm-gate1` / `semasm-gate2` already run their whole ignored suite via
`-- --ignored`); the SemASM pin in `ci.yml` was bumped to the current
`megaalive/semasm` `HEAD` (docs-only commit ahead of the prior pin — see
below).

Honesty: `memset` writes to `buffer` (not read-only) — `01_wrong.asm` returns
`0` (the status the contract requires) without ever storing to the buffer,
and `00_write_broken.asm` writes one byte past `buffer[0..length]` regardless
of `value`; both still resolve through the same SemASM `verified` /
`behavior_failed` / `execution_denied` raw statuses, so Gate-1 Incomplete ≠
Verified and Gate-2 Verified is SemASM `--allow-execution` only (≠ CryptOpt),
same as **Th1**/**Th2**/**Th3**. SemASM pin bumped to
`fb2dac5d6e32c86beac15e13f1f64ceb78fd5ab6` (docs-only tip; no behavioral
change to `builtin.buffer.memset` or any other oracle).

**Th6** mirrors **Th3**/**Th4** for `memcpy`: adds `fixtures/run/memcpy/`
(`memcpy.vaa.toml` + `memcpy.sem.toml` copied from `fixtures/semasm/memcpy/`,
`00_write_broken.asm`, `01_wrong.asm` copied from the SemASM
`memcpy_wrong_win64.asm` shape, `02_repaired.asm` copied from
`fixtures/semasm/memcpy/memcpy_win64.asm`, `README.md`) plus
`gate1_search_ingest_memcpy_nop_before_ret_stops_on_incomplete`,
`gate1_search_ingest_memcpy_skips_violated_budget`, and
`gate2_search_ingest_memcpy_allow_execution_verified` in the existing
Gate-1/2 smoke suites. No CI workflow edits were needed: `semasm-gate1` /
`semasm-gate2` already run their whole ignored suite (`-- --ignored`), and no
workflow invokes a `gate1_search_ingest_*` / `gate2_search_ingest_*` test by
name; the SemASM pin in `ci.yml` (`fb2dac5d6e32c86beac15e13f1f64ceb78fd5ab6`)
was already current tip, so no bump was needed for this wave.

Honesty: `memcpy` writes to `dst` and reads `src` (not read-only, unlike
`memcmp` / `find_first_byte`) — `01_wrong.asm` returns `0` (the status the
contract requires) without ever copying `src` into `dst`, and
`00_write_broken.asm` writes one byte past `dst[0..length]` regardless of
`src`; both still resolve through the same SemASM `verified` /
`behavior_failed` / `execution_denied` raw statuses, so Gate-1 Incomplete ≠
Verified and Gate-2 Verified is SemASM `--allow-execution` only (≠ CryptOpt),
same as **Th1**/**Th2**/**Th3**/**Th4**. `dst`/`src` stay distinct,
non-overlapping buffers only (SemASM ADR 0003, "overlap fail-closed").

Honesty: `--live` never runs in Gate CI; API keys are env-only and never sealed.
Practice seals and Gate CI artifacts remain illustrative, not a trust root.

| Document | Role |
|---|---|
| `VAA_REVIEWED_AND_HARDENED_ARCHITECTURE_PLAN.md` | Architecture baseline |
| `docs/implementation-baseline.md` | Phase 0 SemASM reality check |
| `docs/task-schema.md` | Task schema 0.1 |
| `docs/progress.md` | This file |
| `docs/seal.md` | Integrity vs authenticity; seal schema 0.2; verify-chain |
| `docs/release-v0.1-checklist.md` | Alpha checklist + R-tag record |
| `CHANGELOG.md` | Release notes (`[0.1.0]` dated) |
| `scripts/release-prep-check.*` | Local fmt/clippy/test prep (never tags) |
| `docs/cache.md` | PR-020 local cache layout + honesty |
| `docs/vaa-canonical-json-v1.md` | Named canonical JSON profile |
| `fixtures/canonical-json/` | Cross-language conformance vectors |
| `fixtures/run/count_byte/README.md` | R1 golden run |
| `fixtures/run/find_first_byte/README.md` | Q1 multi-candidate wrong→repair |
| `fixtures/run/find_last_byte/README.md` | S2 multi-candidate + T search-ingest |
| `fixtures/run/replace_byte/README.md` | W3 multi-candidate + Th3 search-ingest |
| `fixtures/run/memset/README.md` | Wm3 multi-candidate + Th4 search-ingest |
| `fixtures/run/memcpy/README.md` | Wc multi-candidate + Th6 search-ingest |
| `fixtures/ingest/count_byte/README.md` | R2 generator-agnostic ingest |
| `fixtures/ingest/hlax64_sum_i64/README.md` | HlaX64 → VAA ingest bridge (`sum_i64`) |
| `fixtures/ingest/hlax64_memset/README.md` | HlaX64 → VAA ingest bridge (`memset`, Th5) |
| `fixtures/ingest/hlax64_memcpy/README.md` | HlaX64 → VAA ingest bridge (`memcpy`, Th7) |
| `fixtures/ingest/hlax64_min_usize/README.md` | HlaX64 → VAA ingest bridge (`min_usize`, Th8) |
| `fixtures/ingest/hlax64_max_usize/README.md` | HlaX64 → VAA ingest bridge (`max_usize`, Th8) |
| `fixtures/semasm/README.md` | Handshake fixtures |
| `fixtures/negative/` | N6 fail-closed validate/transparency vectors |

## Honesty constraints

Do not claim formal proof, production readiness, hardened sandbox isolation, or
CI-proven SemASM vertical slices until the corresponding evidence exists.
