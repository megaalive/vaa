# HlaX64 backend corpus (plan §17)

Pack cases under `cases/` are the **locked generator inputs** for the
verified-repair bridge. Sources are copied from the sibling HlaX64
`examples/interop/semasm-vaa/` surface; tasks/contracts match the VAA
`fixtures/ingest/hlax64_*` authority files.

Honesty: packing a case here ≠ Gate Verified. Live generation + SemASM
still required. HlaX64 emit / `-Wverify` ≠ SemASM `verified`.
`verified_under_preconditions` ≠ unconditional `verified`.

## Phase status

| Phase | Plan cases | Pack status |
|---|---|---|
| A — scalar leaf | `return_i64`, `add_i64`, `sub_i64`, `min_i64`, `max_i64`, `abs_i64` | **done** (named i64 pack wired; Win64 Gate Accepted, 6 Verified via SemASM `builtin.pure_int.binary_i64` / `unary_i64`, SemASM 566ca8e+). `min_usize` / `max_usize` retained as the usize-oracle pair. |
| B — loops / stack | `sum_range`, `countdown_loop`, `stack_local_i64`, `forced_register_spill` | **done** (named pack wired; Win64 Gate Accepted, 4 Verified via SemASM unary i64 v2 — `sum_range` / `countdown` / identity aliases). `sum_i64` retained as buffer-sum oracle proxy. |
| C — memory reads | `count_byte`, `find_first_byte`, `find_last_byte`, `memcmp` | **done** (pack wired; Win64 Gate Accepted with VerifiedUnderPreconditions — honesty: under_preconditions ≠ unconditional verified). |
| C/D depth — concrete cells | `load_byte0`, `store_byte0` | **done** (literal region length; Win64 Gate Accepted, 2 Verified via SemASM `28fb22f` + profile `memory-leaf-concrete-v1`). Does not promote symbolic-length C/D. |
| D — memory writes | `replace_byte`, `memset`, `memcpy` | **done** (pack wired; Win64 Gate Accepted with VerifiedUnderPreconditions). |
| E — calls / data | `internal_function_call`, `nested_call`, `global_rodata`, `multiple_exports`, `small_struct_return` | **done** (pack wired; Win64 Gate Accepted, 5 Verified via SemASM Phase-E pure-int oracles, SemASM ecde423+). |

## Suites

| Suite | Cases |
|---|---|
| `suites/smoke.vaa-suite.toml` | `_placeholder` (wiring only) |
| `suites/scalar-win64.vaa-suite.toml` | Phase A usize pair (Win64) |
| `suites/scalar-i64-win64.vaa-suite.toml` | Phase A named i64 (Win64) — Gate needs SemASM 566ca8e+ |
| `suites/scalar-i64-sysv.vaa-suite.toml` | Phase A named i64 (SysV) — Gate on Linux |
| `suites/scalar-sysv.vaa-suite.toml` | Phase A SysV usize pair — **live** via `generator.sysv.spec.toml` |
| `suites/loop-win64.vaa-suite.toml` | Phase B proxy (`sum_i64`) |
| `suites/loop-stack-win64.vaa-suite.toml` | Phase B named loops/stack (Win64) — Gate needs SemASM 3cae1e1+ |
| `suites/loop-stack-sysv.vaa-suite.toml` | Phase B named loops/stack (SysV) — Gate on Linux |
| `suites/negative-reject-win64.vaa-suite.toml` | Locked wrong `min_i64` (must Reject / Violated) |
| `suites/memory-read-win64.vaa-suite.toml` | Phase C |
| `suites/memory-write-win64.vaa-suite.toml` | Phase D |
| `suites/memory-concrete-win64.vaa-suite.toml` | Concrete cells (`load_byte0`/`store_byte0`) — Gate needs SemASM 28fb22f+ |
| `suites/calls-data-win64.vaa-suite.toml` | Phase E (calls / data) |
| `suites/backend-win64.vaa-suite.toml` | A–D pack union |

## Target / ABI parity

Suites declare `target` + optional `abi`. Check with:

```text
vaa suite check-parity integrations/hlax64/suites/scalar-win64.vaa-suite.toml
vaa suite check-parity integrations/hlax64/suites/scalar-sysv.vaa-suite.toml
```

Known first-cut profiles: `x86_64-pc-windows-msvc`/`win64`,
`x86_64-unknown-linux-gnu`/`sysv`.

## SysV live generation + Gate

SysV suites generate real System V AMD64 assembly via a dedicated
spec that targets `linux-x64-sysv`:

```text
# Generate only (Incomplete)
vaa suite run integrations/hlax64/suites/scalar-sysv.vaa-suite.toml \
  --repo ../hlax64 --skip-verify

# Gate on Linux (Accepted / Verified) — needs SemASM afaa19d+ (framed epilogue)
./scripts/run-hlax64-suite.sh --gate \
  --suite integrations/hlax64/suites/scalar-sysv.vaa-suite.toml
```

The emitted `candidate.asm` uses System V argument registers (`rdi`,
`rsi`, ...) — CI asserts this. Pack Gate on Linux is claimed by the
`hlax64-pack-sysv-gate` job: `scalar-sysv` (usize pair),
`scalar-i64-sysv` (6 named i64), and `loop-stack-sysv` (4) Verified.
Practice seal is not a trust root. SemASM SysV ABI now accepts HlaX64
`mov rsp, rbp` framed epilogues (parity with Win64).

## Case layout

```text
cases/<id>/
  case.toml          # task/contract/input overrides
  input.hla64        # locked generator input
  task.vaa.toml      # VAA task (authority)
  contract.sem.toml  # SemASM contract (authority)
```

Do not hand-edit authority files as part of generator repair
(see `agent-rules.md` / `patch_policy`).

## Live repair fixtures

Constrained evidence packets live under `fixtures/repair/hlax64-*-live*/`
(compare flips + non-compare ABI stack-balance / callee-saved, Win64 and
SysV). Pack CI Gates **7*** run `vaa repair verify` +
`vaa patch evidence-verify` on each. Index and hashes: see
`integrations/hlax64/README.md` § Live repair evidence. SysV Gate twins
were closed via WSL / `hlax64-pack-sysv-gate`. Practice seal ≠ trust root.
