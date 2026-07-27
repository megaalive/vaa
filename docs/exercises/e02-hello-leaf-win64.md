# Friction log — Exercise E02 — Hello leaf + thin main (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Verify a pure-int `increment_i64` leaf under SemASM, keep skill admission honest,
and link a thin hosted `mainCRTStartup` that calls the leaf then
`ExitProcess` — proving the leaf/hosted boundary without claiming a REPL seal.

Scratch tree (gitignored): `.vaa-exercises/e02-hello-leaf-win64/`.

## Commands run

```text
vaa admit --leaf increment_i64 --target x86_64-pc-windows-msvc --format json
vaa validate .vaa-exercises/e02-hello-leaf-win64/increment_i64.vaa.toml --format json
vaa validate .vaa-exercises/e02-hello-leaf-win64/main.vaa.toml --format json
semasm agent verify …/increment_i64.asm …/increment_i64.sem.toml \
  --target x86_64-pc-windows-msvc --format json --allow-execution
vaa build …/increment_i64.asm --target x86_64-pc-windows-msvc --output-dir …/out --format json
nasm -f win64 increment_i64.asm -o out/increment_i64.o
nasm -f win64 main.asm -o out/main.o
lld-link /subsystem:console /entry:mainCRTStartup /OUT:out/hello.exe \
  out/main.o out/increment_i64.o /DEFAULTLIB:kernel32.lib
out/hello.exe   # process exit 42
```

## What helped

- `artifact_kind = "callable-function"` vs `"hosted-program"` validated cleanly.
- After a fresh SemASM rebuild (`85d0a7a`), `increment_i64` matched the unary
  inc oracle and returned `status: "verified"` with 5 vectors.
- Manual NASM + `lld-link` with `/subsystem:console`, `/entry:mainCRTStartup`,
  and `/DEFAULTLIB:kernel32.lib` produced a PE whose exit code was `42`
  (`increment_i64(41)` → `ExitProcess`).

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `vaa admit` → `admitted: false`, `next_action: decline` for `increment_i64` | honesty | Snapshot has no such leaf; contrast `max_i64` admitted |
| 2 | Stale `semasm.exe` first returned `UNSUPPORTED_SHAPE` for the same sources that verify after rebuild | agent-mistake / tooling | Rebuild required; do not treat dry/stale bins as evidence |
| 3 | `vaa build` on the leaf alone: `lld-link: error: subsystem must be defined` | tooling | Default PE argv lacks `/subsystem:…`; leaf-only object work does not need a PE |
| 4 | Hosted link needs explicit `kernel32` + entry/subsystem; not inferred from task imports today | tooling | Manual `lld-link` succeeded; `vaa build` extras undocumented for this path |
| 5 | PowerShell `Tee-Object` wrote UTF-16 logs that look “spaced” | agent-mistake | Prefer `Out-File -Encoding utf8` for friction raw logs |

## Outcomes

- Leaf verify: **SemASM `verified`** (unary inc oracle) after rebuild — **not** an
  admitted skill leaf.
- `vaa validate`: both leaf and hosted tasks **ok**.
- `vaa build` / link: leaf-only PE link **failed** (subsystem); thin main via
  manual `lld-link` **ok**, exit **42**.
- Seal / admission claim made: **none**. Skill path remains decline for
  `increment_i64`. Runtime exit code is not a SemASM seal for the hosted main.

## Follow-ups

- [x] Docs: this friction log + exercises scaffold under `docs/exercises/`
- [x] Tooling: clearer Win64 hint when `subsystem must be defined`
- [x] Optional later: `vaa build --object-only` (or skip link for callable leaves)
- [ ] Non-goal: do not admit `increment_i64` into the skill snapshot just to pass E02
- [x] E03: WriteFile / `kernel32` hosted stub (imports + capabilities) → see [e03-writefile-win64.md](e03-writefile-win64.md)
