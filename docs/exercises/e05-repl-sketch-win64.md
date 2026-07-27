# Friction log — Exercise E05 — REPL sketch (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Build a **minimal hosted REPL** (banner, prompt, echo, `q`/`quit`) that may call
an admitted leaf (`max_i64`) for a session metric — and prove that integration
success must **not** be reported as SemASM/skill verification.

Scratch tree (gitignored): `.vaa-exercises/e05-repl-sketch-win64/`.

## Commands run

```text
vaa admit --leaf repl|windows_repl|max_i64 --target x86_64-pc-windows-msvc --format json
vaa validate …/main.vaa.toml --format json
vaa validate …/wrong-as-leaf.vaa.toml --format json   # before fix: ok; after: fail
semasm agent verify …/max_i64.asm …           # verified
semasm agent verify …/main.asm …              # UNSUPPORTED_SHAPE
nasm -f win64 max_i64.asm -o out/max_i64.o
vaa build …/main.asm --extra-object out/max_i64.o \
  --linker-arg /subsystem:console --linker-arg /entry:mainCRTStartup \
  --linker-arg /DEFAULTLIB:kernel32.lib --format json
# scripted: hi / hello / quit → prompts + echoes + bye; ExitProcess(5)
```

## What helped

- Banner text states `hosted; not SemASM-verified` (honest UX in the binary).
- E03/E04 tooling (`--linker-arg`, SDK env, `--extra-object`) composed cleanly.
- Admit/verify correctly split: `max_i64` sealed; `repl` / REPL main declined.
- Scripted session: prompts, `echo: …`, `bye`, exit **5** (session max line len).

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `admit repl` / SemASM verify REPL → decline | honesty | Expected end-state for E05 |
| 2 | Mis-tagged `callable-function` + Win32 `imports` + dummy test still validated | honesty | Fixed: validate rejects imports on callable-function |
| 3 | Agent may treat “REPL runs + leaf verified” as “REPL verified” | honesty | Docs + banner; exit code is not a seal |
| 4 | Full interactive TTY vs piped stdin (CR/LF, no line editing) | tooling / non-goal | Sketch only; no ReadConsole |
| 5 | Harness `loop-direct` is for admitted leaves, not hosted REPL | honesty | Adapter requires admitted leaf shapes |

## Outcomes

- REPL integration: **runs** (prompt/echo/quit).
- Leaf `max_i64`: **admitted** + **verified** (unchanged).
- REPL admit/verify: **decline** / `UNSUPPORTED_SHAPE`.
- Seal claim: **none for the REPL**. Leaf seal does not transfer.

## Follow-ups

- [x] Docs: this friction log; close maturity ladder E02–E05
- [x] Tooling: reject `callable-function` with non-empty `capabilities.imports`
- [ ] Non-goal: do not admit REPL / ReadFile loops into the skill snapshot
- [ ] Optional later: richer console APIs / line editing (out of leaf scope)
