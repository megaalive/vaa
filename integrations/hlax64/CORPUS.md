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
| A — scalar leaf | `return_i64`, `add_i64`, `sub_i64`, `min_i64`, `max_i64`, `abs_i64` | **proxy:** `min_usize`, `max_usize` (usize compare/return). Named i64 leaves remain open. |
| B — loops / stack | `sum_range`, `countdown_loop`, `stack_local_i64`, `forced_register_spill` | **proxy:** `sum_i64` (pointer+length loop). Stack spill cases open. |
| C — memory reads | `count_byte`, `find_first_byte`, `find_last_byte`, `memcmp` | **done** (pack wired) |
| D — memory writes | `replace_byte`, `memset`, `memcpy` | **done** (pack wired) |
| E — calls / data | `internal_function_call`, `nested_call`, `global_rodata`, `multiple_exports`, `small_struct_return` | **done** (pack wired; live Win64 generate). `small_struct_return` exercises aggregate layout/field offsets and returns a scalar (HlaX64 returns via register). |

## Suites

| Suite | Cases |
|---|---|
| `suites/smoke.vaa-suite.toml` | `_placeholder` (wiring only) |
| `suites/scalar-win64.vaa-suite.toml` | Phase A proxy (Win64) |
| `suites/scalar-sysv.vaa-suite.toml` | Phase A SysV — **live** via `generator.sysv.spec.toml` |
| `suites/loop-win64.vaa-suite.toml` | Phase B proxy |
| `suites/memory-read-win64.vaa-suite.toml` | Phase C |
| `suites/memory-write-win64.vaa-suite.toml` | Phase D |
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
`hlax64-pack-sysv-gate` job (`min_usize_sysv` / `max_usize_sysv` Verified).
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
