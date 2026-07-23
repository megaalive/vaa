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

### HlaX64 → SemASM → VAA bridge (after S4)

Roles (do not conflate):

| Layer | Owns | Does not own |
|---|---|---|
| **HlaX64** | Authoring `.hla64` → NASM (`hla64 emit-nasm`) | Verification status / seals |
| **SemASM** | Contract + behavioral oracle + `VerificationReport` 0.4 | Task policy / evidence chain |
| **VAA** | Task lock, `ingest`/`verify`, seal chain | Generating assembly |

First leaf: `sum_i64` (Win64). Generator label: `--generator hlax64`.

| Wave | Focus | Claim when done |
|---|---|---|
| **H0** | Lock roles in docs | **Done** |
| **H1** | HlaX64 example + frozen NASM ingest fixture + Gate smoke | **Done** |
| **H2** | `scripts/regen-hlax64-sum_i64` | **Done** |
| **H3** | CI checkout HlaX64 + emit-nasm + verify | **Done** (`hlax64-bridge` job) |

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
live read of SemASM `capabilities.toml` (pipeline maturity there may still be
`partial` / `experimental` on some axes).

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
| `fixtures/ingest/count_byte/README.md` | R2 generator-agnostic ingest |
| `fixtures/ingest/hlax64_sum_i64/README.md` | HlaX64 → VAA ingest bridge (`sum_i64`) |
| `fixtures/semasm/README.md` | Handshake fixtures |
| `fixtures/negative/` | N6 fail-closed validate/transparency vectors |

## Honesty constraints

Do not claim formal proof, production readiness, hardened sandbox isolation, or
CI-proven SemASM vertical slices until the corresponding evidence exists.
