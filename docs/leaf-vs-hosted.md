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

Real-tools pressure (T02/T04/T05/T07): dual-buffer parse (`ini_lookup`), nibble
encode (`nibble_to_hex`), `find_nth_lf` (`ptr+len+n`), and binary `xor_*`
(arity matches pure-int binary but name ∉ `add_*/sub_*/min_*/max_*`) are useful
for tools but currently fail closed at SemASM synthesis — compose admitted scans
(`find_first_byte`, `memcmp`, `count_byte`) in the hosted loop instead of
weakening `UNSUPPORTED_SHAPE`.

## 2. Hosted / REPL programs (out of agent-skill scope)

A Windows console REPL that imports `kernel32` is a `hosted-program` task with
explicit capabilities/imports. That is useful for design-by-contract and
`vaa validate`, but it is **not** the Cursor/Codex leaf-repair skill path and
does not get behavioral seals from the allowlisted admission snapshot.

## 3. `vaa build` on Windows

- Target `x86_64-pc-windows-msvc` / `win64` selects NASM `-f win64` and linker
  `lld-link` (not ELF `ld`).
- Hosted programs that call Win32 APIs need extra linker args such as
  `/DEFAULTLIB:kernel32.lib`. Pass them explicitly:
  `vaa build … --linker-arg /subsystem:console --linker-arg /entry:mainCRTStartup
  --linker-arg /DEFAULTLIB:kernel32.lib` (see
  [exercises/e03-writefile-win64.md](exercises/e03-writefile-win64.md)).
  Local builds forward Windows SDK discovery env (`SystemRoot`,
  `ProgramFiles(x86)`, optional `LIB`) so `lld-link` can find Kits libs.
- Win64 hosted asm: keep **16-byte stack alignment** before `call` (odd number of
  pushes + wrong `sub rsp` → AV). See [exercises/t01-wc-lite.md](exercises/t01-wc-lite.md).
- Multi-object programs (hosted main + leaf `.o`): assemble the leaf first, then
  `vaa build main.asm … --extra-object path/to/leaf.o --linker-arg …` (see
  [exercises/e04-line-loop-win64.md](exercises/e04-line-loop-win64.md)).
- Leaf object only: `vaa build leaf.asm --object-only` (skip PE link; E02).
- `vaa validate` on `hosted-program` with Win32 `imports` may include
  `suggested_linker_args` (`/subsystem:console`, `/entry:…`,
  `/DEFAULTLIB:kernel32.lib`). Imports stay declarative in schema 0.1 — not
  enforced against asm or `filesystem` flags.
- Leaf object inspection for SemASM often only needs assemble + object; full
  PE link is optional for verify. Linking a leaf alone without `/subsystem:…`
  fails under `lld-link` — use `--object-only` or a thin hosted main for PE (see
  [exercises/e02-hello-leaf-win64.md](exercises/e02-hello-leaf-win64.md)).

## 4. Orchestration vs leaf seal

A line loop / REPL that *calls* an admitted leaf does **not** inherit the leaf's
seal. Admit/verify the leaf; treat the hosted loop as integration only (E04/E05).

`vaa validate` rejects `artifact_kind = "callable-function"` when
`capabilities.imports` is non-empty — Win32/I-O imports belong on
`hosted-program` (see [exercises/e05-repl-sketch-win64.md](exercises/e05-repl-sketch-win64.md)).

## 5. Task TOML trial-and-error

Schema 0.1 is fail-closed (`deny_unknown_fields`):

- `artifact_kind`: `callable-function` | `hosted-program` | `freestanding-image`
  (not `standalone-executable`)
- No free-form `description` key; use `behavior.summary`
- `inputs` follow `InputSpec` / `ValueKind` — not `kind = "string"` + `max_length`

`vaa validate` now appends hints for these common mistakes. See
[`task-schema.md`](task-schema.md) and [`schemas/task.vaa.schema.json`](../schemas/task.vaa.schema.json).
