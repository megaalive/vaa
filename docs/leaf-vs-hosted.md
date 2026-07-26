# Scope notes: leaf verify vs hosted programs (REPL/I-O)

Feedback from agents building a Windows REPL with VAA + SemASM often mixes
three different jobs. Keep them separate:

## 1. Leaf verification (what SemASM agent-verify is for)

SemASM synthesizes behavioral vectors for **recognized leaf shapes**:

- pure-int unary `i64 → i64` when the routine name matches a known op
  (`abs_*`, `inc_*` / `*increment*`, `return_*` / `identity_*`, …)
- pure-int binary `i64,i64 → i64` (`add_*`, `sub_*`, `min_*`, `max_*`)
- buffer scans / memcmp / memcpy / memset / replace_byte (ptr + length …)

`UNSUPPORTED_SHAPE` means the **name/type combo was not recognized**, not that
`i64 → i64` is impossible. Prefer admitted leaves (`vaa admit`) or rename to a
recognized token. Full REPL loops with `GetStdHandle` / `ReadFile` are **not**
an admitted leaf.

## 2. Hosted / REPL programs (out of agent-skill scope)

A Windows console REPL that imports `kernel32` is a `hosted-program` task with
explicit capabilities/imports. That is useful for design-by-contract and
`vaa validate`, but it is **not** the Cursor/Codex leaf-repair skill path and
does not get behavioral seals from the allowlisted admission snapshot.

## 3. `vaa build` on Windows

- Target `x86_64-pc-windows-msvc` / `win64` selects NASM `-f win64` and linker
  `lld-link` (not ELF `ld`).
- Hosted programs that call Win32 APIs need extra linker args such as
  `/DEFAULTLIB:kernel32.lib` (pass via toolchain extras / manual link today).
- Leaf object inspection for SemASM often only needs assemble + object; full
  PE link is optional for verify.

## 4. Task TOML trial-and-error

Schema 0.1 is fail-closed (`deny_unknown_fields`):

- `artifact_kind`: `callable-function` | `hosted-program` | `freestanding-image`
  (not `standalone-executable`)
- No free-form `description` key; use `behavior.summary`
- `inputs` follow `InputSpec` / `ValueKind` — not `kind = "string"` + `max_length`

`vaa validate` now appends hints for these common mistakes. See
[`task-schema.md`](task-schema.md) and [`schemas/task.vaa.schema.json`](../schemas/task.vaa.schema.json).
