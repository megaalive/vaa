# Friction log — Exercise E04 — Line loop (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Compose an **admitted leaf** (`max_i64`) with a **hosted line loop** that reads
stdin, echoes bytes, and tracks max line length via the leaf. Keep seals honest:
only the leaf is admit/verify-eligible; the orchestrator is not.

Scratch tree (gitignored): `.vaa-exercises/e04-line-loop-win64/`.

## Commands run

```text
vaa admit --leaf max_i64 --target x86_64-pc-windows-msvc --format json
vaa admit --leaf line_loop --target x86_64-pc-windows-msvc --format json
vaa validate …/max_i64.vaa.toml --format json
vaa validate …/main.vaa.toml --format json
semasm agent verify …/max_i64.asm …/max_i64.sem.toml \
  --target x86_64-pc-windows-msvc --format json --allow-execution
semasm agent verify …/main.asm …/main.sem.toml …   # expect UNSUPPORTED_SHAPE
nasm -f win64 max_i64.asm -o out/max_i64.o
nasm -f win64 main.asm -o out/main.o
lld-link /subsystem:console /entry:mainCRTStartup /OUT:out/line_loop.exe \
  out/main.o out/max_i64.o /DEFAULTLIB:kernel32.lib
# input "hi\nhello\nx\n" → echo + ExitProcess(5)
# after bounded fix:
vaa build …/main.asm --target x86_64-pc-windows-msvc --output-dir …/out \
  --extra-object …/out/max_i64.o \
  --linker-arg /subsystem:console --linker-arg /entry:mainCRTStartup \
  --linker-arg /DEFAULTLIB:kernel32.lib --format json
```

## What helped

- Clear split: `callable-function` leaf vs `hosted-program` orchestrator both
  validate.
- `vaa admit max_i64` → admitted + SemASM `verified` (behavioral oracle).
- Hosted loop correctly **declined** on admit/verify.
- Linked composition: piped `hi` / `hello` / `x` echoed; exit code **5**
  (= `max(2,5,1)` via `max_i64`).

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | Orchestrator admit/verify decline while leaf seals | honesty | Expected; do not promote loop to leaf |
| 2 | `vaa build` is single-source; leaf+main needs a second object | tooling | Fixed: `--extra-object` |
| 3 | Temptation to claim “program verified” because leaf verified + exit 5 | honesty | Runtime exit ≠ hosted seal |
| 4 | PowerShell `Start-Process -RedirectStandardInput` needs the input file to exist first | agent-mistake | Create `input.txt` before spawn |

## Outcomes

- Leaf: **admitted** + SemASM **`verified`**.
- Hosted line loop: validate **ok**; admit/verify **decline** / `UNSUPPORTED_SHAPE`.
- Link / run: echo OK, exit **5**.
- Seal claim: **leaf only**. No seal for the ReadFile/WriteFile loop.

## Follow-ups

- [x] Docs: this friction log
- [x] Tooling: `vaa build --extra-object` for multi-object hosted link
- [x] Optional: `vaa build` multi-source / response-file for larger programs — deferred; `--extra-object` covers E04
- [ ] Non-goal: do not admit line-loop / REPL orchestration into the skill snapshot
- [x] E05: REPL sketch (integration only; no false verify claims) → see [e05-repl-sketch-win64.md](e05-repl-sketch-win64.md)
