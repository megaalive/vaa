# VAA Implementation Baseline

**Document status:** Phase 0 readiness gate  
**Captured:** 2026-07-19  
**SemASM path inspected:** `D:\_2025\Gits\megaalive\semasm`  
**SemASM package version reported by CLI:** `0.1.0`  
**Capability schema (from `semasm status`):** `0.1`

This document records what is actually available in SemASM today versus what the VAA architecture plan assumes. It is the Phase 0 gate before functional VAA work.

---

## 1. Host inspection environment

| Item | Observed |
|---|---|
| Host OS | Windows |
| Rustc / Cargo | 1.97.0 (local) |
| SemASM binary | Built from workspace (`semasm-cli` package, binary name `semasm`) |
| NASM | Present (`NASM version 3.01`) |
| `ld.lld` / `ld.bfd` | Not found on PATH (Linux ELF link unavailable on this host) |
| `llvm-objdump` / `objdump` | Present |
| `qemu-x86_64` | Not found on PATH |

Implication: full Linux assemble/link/run of `x86_64-unknown-linux-gnu` fixtures cannot be demonstrated on this Windows host without WSL, a Linux CI image, or additional toolchain packages. Controller-side unit tests and schema work remain valid.

---

## 2. SemASM CLI surface (actual)

Top-level commands observed via `semasm --help`:

| Command | Role | Machine-readable JSON |
|---|---|---|
| `version` | Print version string | **No** (`--format` rejected) |
| `status` | Human project/target summary | **No** (terminal only) |
| `explain` / `--explain` | Diagnostic code help | Terminal |
| `target doctor <TARGET>` | Toolchain discovery | Yes (`--format json`) |
| `build <SOURCE>` | Assemble / link / optional run / report | Yes (`--format json`) |
| `contract check <PATH>` | Validate `*.sem.toml` | Yes (`--format json`) |
| `agent packet <CONTRACT>` | Agent task packet | Yes (default JSON) |
| `agent verify <SOURCE> <CONTRACT>` | Assemble/link + test vectors | Yes (`--format json`); execution gated by `--allow-execution` |
| `obj` | Object inspection | Available |
| `decode` / `cfg` | Capstone-backed inspection | Available (feature-gated in crate) |
| `abi` / `win64-abi` / `aarch64-abi` | ABI analysis on raw function body | Yes (`--format json`) |
| `analyze` | Data-flow analysis | Available |

There is **no** top-level command named:

```text
semasm verify --target ... --contract ... --source ... --format json
```

as sketched in the VAA plan §8.2. Closest equivalents:

- static contract validity: `semasm contract check`
- ABI / incomplete evidence on a binary blob: `semasm abi` / `win64-abi` / `aarch64-abi`
- agent harness: `semasm agent verify`
- full pipeline report: `semasm build`

---

## 3. Capability maturity (from `capabilities.toml` / `semasm status`)

| Target | Decode | Lower | ABI | Assemble | Link | Execute | Verify |
|---|---|---|---|---|---|---|---|
| `x86_64-unknown-linux-gnu` | partial | partial | unit-tested | experimental | experimental | experimental | experimental |
| `x86_64-pc-windows-msvc` | partial | partial | unit-tested | experimental | experimental | experimental | experimental |
| `aarch64-unknown-linux-gnu` | partial | partial | unit-tested | CI-verified | CI-verified | CI-verified | CI-verified |
| `riscv64gc-unknown-linux-gnu` | declared | partial | unit-tested | CI-verified | CI-verified | CI-verified | CI-verified |
| `riscv32imac-unknown-none-elf` | declared | partial | unit-tested | unavailable | unavailable | unavailable | experimental |

VAA plan first vertical slice target: **`x86_64-unknown-linux-gnu` + callable function**.

SemASM itself marks that target’s assemble/link/execute/verify rows as **`experimental`**, not CI-verified. Decode and lower are **`partial`**. VAA must not promote these to “supported” without independent evidence.

Capability field names in SemASM use `abi_analysis` in TOML and “abi” in status output. The VAA plan examples use `abi_check` and maturity values `supported` / `partial`. **These naming and maturity vocabularies do not match one-to-one.** VAA adapters must map explicitly and fail closed on unknowns.

---

## 4. Status / evidence model (actual)

### 4.1 What VAA plan requires

Terminal evidence statuses:

- `verified`
- `violated`
- `incomplete`
- `failed`

### 4.2 What SemASM currently exposes

Observed pieces:

| Surface | Status model |
|---|---|
| `semasm-x86` ABI analysis | `AnalysisStatus::{Verified, Incomplete}` (and related diagnostics). CLI maps these to strings `"verified"` / `"incomplete"`. A dedicated `"violated"` / `"failed"` terminal enum is not universal across all commands. |
| `contract check` JSON | `{ "ok": bool, "diagnostics": [...], "contract": ... }` — boolean validity, not the four-way evidence status. |
| `target doctor` JSON | Tool discovery (`all_found`, per-tool `found`/`version`). Not verification evidence. |
| `build` JSON | Build/pipeline report with deterministic evidence hash fields when a build succeeds; fails hard when linker missing. |
| Execution termination | `TerminationOutcome` includes `Incomplete` among other outcomes in `semasm-build`. |
| Capability levels | `declared`, `partial`, `experimental`, `unit-tested` / `verified_in_unit_tests`, `CI-verified` / `verified_in_ci`, `unavailable` — maturity of *implementation*, not pass/fail of a candidate. |

**Gap:** VAA cannot assume a single SemASM process always returns the four-way status. The VAA evidence aggregator must:

1. classify each adapter outcome into VAA’s four statuses;
2. treat missing schema fields, parse errors, timeouts, and tool absence as `failed` or `incomplete` (never `verified`);
3. never invent `verified` from `ok: true` on contract parse alone.

---

## 5. Expected commands vs actual (compatibility checklist)

| VAA plan expectation (§8.1 / §8.2) | Status |
|---|---|
| Machine-readable capability manifest | **Partial.** File `capabilities.toml` exists and is human/CI source of truth; no stable `semasm capabilities --format json` command observed. |
| Versioned JSON verification report | **Available for agent verify.** SemASM emits `VerificationReport` schema **0.4** on stdout (`semasm agent verify … --format json`). VAA adapter parses stdout-only and maps statuses to the 4-outcome vocabulary. |
| Explicit `verified` / `violated` / `incomplete` / `failed` | **Constructed in VAA.** SemASM statuses (`verified`, `semantic_failed`, `executable_failed`, `behavior_failed`, `execution_denied`) are mapped per `docs/progress.md` handshake notes. |
| Instruction coverage counts | **Unknown / not verified in this baseline pass.** Must be confirmed from concrete ABI/build JSON fixtures before VAA maps coverage fields. |
| Unsupported instruction details | **Present in analysis path** (incomplete evidence / unsupported lowering). Exact JSON field names need fixture capture in PR-007. |
| Target identity in every report | **Partial.** Present in doctor and many target-scoped commands; not proven for every JSON surface. |
| Reliable non-zero process outcomes | **Mostly yes** for failures (e.g. missing linker). Must treat stderr warnings carefully (PowerShell may surface warnings as error streams). |
| Bounded process execution | **Present in SemASM build exec design**; VAA still needs its own hardened runner (PR-009). |
| Deterministic evidence mode | **Present for build reports** (`deterministic_evidence_sha256` used in SemASM CI). |
| Stable schema compatibility rules | **Not fully documented as a negotiated range for external consumers.** VAA must pin accepted SemASM versions and fail on unknown major fields. |
| `semasm --version --format json` | **Missing.** Only plain `semasm --version` / `semasm version` text. |
| `semasm capabilities --target ... --format json` | **Missing** as a dedicated command. Use `capabilities.toml` + `target doctor` + `status` until SemASM adds it. |
| `semasm contract check ... --format json` | **Available.** |
| `semasm verify ...` | **Missing** as named command. Use composition of contract / abi / agent verify / build. |

---

## 6. Fixture anchors useful to VAA

From SemASM `fixtures/`:

| Fixture | Use |
|---|---|
| `fixtures/contracts/write_all.sem.toml` | Contract validation smoke |
| `fixtures/contracts/count_byte.sem.toml` | Callable-function style contract |
| `fixtures/asm/count_byte.asm` | Known-good routine source |
| `fixtures/asm/count_byte_wrong.asm` | Negative behavioral / harness case |
| `fixtures/asm/exit.asm` | Hosted program build path |
| `fixtures/negative/**` | Malformed contracts/objects |

VAA should eventually vendor **copies or digests** of selected fixtures under its own tree for offline tests, rather than requiring a live checkout path to SemASM at runtime.

---

## 7. Integration strategy implications

1. **Process protocol first (confirmed):** Prefer invoking the `semasm` binary with explicit argv and JSON stdout. Do not link SemASM crates into VAA for the first slice.
2. **Adapter split:** Separate adapters for version/status, target doctor, contract check, ABI analysis, agent verify, and build. Do not assume one mega-command.
3. **Capability gate:** Until a JSON capabilities command exists, load pinned capability data from a checked-in snapshot or parse SemASM’s `capabilities.toml` only when an explicit path is configured. Prefer fail-closed when maturity is below policy.
4. **First target risk:** Plan chooses `x86_64-unknown-linux-gnu`, which SemASM marks experimental for assemble/link/execute/verify. VAA README and reports must say **experimental** until CI proves the slice.
5. **Host vs target:** VAA controller may build and unit-test on Windows; generated-target evidence for Linux ELF still requires a Linux toolchain job.
6. **Execution default:** SemASM `agent verify` already gates execution behind `--allow-execution`, aligning with VAA’s verify-only default.

---

## 8. Missing SemASM prerequisites (for upstream or local workarounds)

Required for a clean VAA adapter contract (priority order):

1. **JSON version endpoint** — e.g. `semasm version --format json` with version, report schema version, binary identity fields.
2. **JSON capabilities endpoint** — e.g. `semasm capabilities --target <id> --format json` mirroring `capabilities.toml` maturity enums.
3. **Unified verification report schema** with explicit terminal status in `{verified, violated, incomplete, failed}` and coverage counts.
4. **Documented schema compatibility policy** (accepted major/minor ranges).
5. **Stable non-zero exit codes** per terminal status, documented for automation.
6. **CI-proven maturity** for `x86_64-unknown-linux-gnu` assemble/link/abi/verify if that remains VAA’s first product target — or VAA should reconsider starting on a target with stronger SemASM CI evidence (AArch64/RV64) only after re-evaluating the plan’s first-slice decision.

Until (1)–(3) exist, VAA PR-005 through PR-008 must implement **defensive mapping** and may report `dependency_incomplete` rather than false `verified`.

---

## 9. Phase 0 exit criteria

| Criterion | Result |
|---|---|
| Compatibility checklist written | **Met** (this document) |
| Required SemASM commands documented (actual) | **Met** |
| Fixture anchors listed | **Met** |
| Status model differences recorded | **Met** |
| Missing prerequisites listed | **Met** |
| VAA can *already* reliably distinguish four SemASM terminal statuses via one official verify API | **Not met** — must be constructed in VAA adapters with fail-closed mapping |

**Phase 0 decision:** Proceed to **PR-001 (repository bootstrap)** and subsequent offline controller work. Do **not** claim SemASM readiness for production verification. Treat SemASM as an early, partially machine-readable dependency.

---

## 10. Next implementation step

Per architecture plan §27 and live checklist in `docs/progress.md`:

1. ~~PR-001…PR-008 / R1–R2c / S2–S4 / H0–H3 / N0–N4 / P0–P2 / R0–R3~~ **Done**
   (see `docs/progress.md`).
2. **Next:** integrity **R4** — optional live SemASM `status --format json`
   vs embedded VAA capability snapshot compare (honest mismatch → warn /
   degrade, not silent replace).
3. Later (explicitly deferred): Ed25519 seals, live model adapter, CryptOpt
   search, `v0.1.0` release.

Do not implement live model adapters, Redis, Python services, or multi-crate splits in this phase.

## 11. Related VAA docs (post Phase 0)

| Document | Role |
|---|---|
| `docs/progress.md` | PR/phase checklist |
| `docs/task-schema.md` | Task schema 0.1 guide |
| `schemas/task.vaa.schema.json` | Checked-in JSON Schema |
| `fixtures/tasks/` | Positive and negative task fixtures |
