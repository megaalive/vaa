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
| E — calls / data | `internal_function_call`, `nested_call`, `global_rodata`, `multiple_exports`, `small_struct_return` | **open** |

## Suites

| Suite | Cases |
|---|---|
| `suites/smoke.vaa-suite.toml` | `_placeholder` (wiring only) |
| `suites/scalar-win64.vaa-suite.toml` | Phase A proxy (Win64) |
| `suites/scalar-sysv.vaa-suite.toml` | Phase A SysV scaffold (`*_sysv`) |
| `suites/loop-win64.vaa-suite.toml` | Phase B proxy |
| `suites/memory-read-win64.vaa-suite.toml` | Phase C |
| `suites/memory-write-win64.vaa-suite.toml` | Phase D |
| `suites/backend-win64.vaa-suite.toml` | A–D pack union |

## Target / ABI parity

Suites declare `target` + optional `abi`. Check with:

```text
vaa suite check-parity integrations/hlax64/suites/scalar-win64.vaa-suite.toml
vaa suite check-parity integrations/hlax64/suites/scalar-sysv.vaa-suite.toml
```

Known first-cut profiles: `x86_64-pc-windows-msvc`/`win64`,
`x86_64-unknown-linux-gnu`/`sysv`. SysV cases are scaffolds — packing ≠
live SysV Gate evidence.

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
