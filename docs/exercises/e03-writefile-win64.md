# Friction log — Exercise E03 — I/O stub WriteFile (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Build a hosted console stub that prints one line via `GetStdHandle` +
`WriteFile`, declare imports/capabilities in a `hosted-program` task, and keep
admission / SemASM agent-verify honest (no leaf seal for I/O).

Scratch tree (gitignored): `.vaa-exercises/e03-writefile-win64/`.

## Commands run

```text
vaa admit --leaf writefile --target x86_64-pc-windows-msvc --format json
vaa admit --leaf GetStdHandle --target x86_64-pc-windows-msvc --format json
vaa validate .vaa-exercises/e03-writefile-win64/main.vaa.toml --format json
vaa validate .vaa-exercises/e03-writefile-win64/wrong-as-leaf.vaa.toml --format json
semasm agent verify …/main.asm …/main.sem.toml \
  --target x86_64-pc-windows-msvc --format json --allow-execution
vaa build …/main.asm --target x86_64-pc-windows-msvc --output-dir …/out --format json
nasm -f win64 main.asm -o out/main.o
lld-link /subsystem:console /entry:mainCRTStartup /OUT:out/writefile.exe \
  out/main.o /DEFAULTLIB:kernel32.lib
out/writefile.exe
# after bounded fix:
vaa build …/main.asm --target x86_64-pc-windows-msvc --output-dir …/out \
  --linker-arg /subsystem:console \
  --linker-arg /entry:mainCRTStartup \
  --linker-arg /DEFAULTLIB:kernel32.lib --format json
```

## What helped

- `hosted-program` task with `imports = ["GetStdHandle", "WriteFile", "ExitProcess"]`
  validates cleanly.
- SemASM `UNSUPPORTED_SHAPE` on `mainCRTStartup` includes the recognized-name
  hint (fail-closed, clear).
- Manual NASM + `lld-link` with `/DEFAULTLIB:kernel32.lib` prints
  `E03 hello via WriteFile` and exits `0`.

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `vaa admit` declines `writefile` / `GetStdHandle` | honesty | Snapshot has no I/O leaves |
| 2 | SemASM agent-verify on hosted main → `UNSUPPORTED_SHAPE` | honesty | Not a pure-int / buffer leaf |
| 3 | `vaa build` without extras → `subsystem must be defined` | tooling | Fixed: repeatable `--linker-arg` |
| 4 | With `--linker-arg …kernel32.lib`, still `could not open 'kernel32.lib'` under VAA | tooling | ProcessRunner `env_clear` dropped `SystemRoot`/`ProgramFiles(x86)`; fixed via toolchain allowlist |
| 5 | Task `capabilities.imports` are declarative only; not checked against `extern` | honesty / tooling | Validate does not cross-check asm |
| 6 | `callable-function` + Win32 `imports` + a dummy `[[tests]]` still validates | honesty | Schema gap; skill path still declines on admit/verify |
| 7 | `filesystem = false` with `WriteFile` to stdout still validates | honesty | Capabilities are not I/O-policy enforced yet |

## Outcomes

- Leaf / admission: **decline** for WriteFile-shaped names.
- SemASM verify hosted main: **UNSUPPORTED_SHAPE** (correct).
- `vaa validate` hosted task: **ok**.
- Link / run: **ok** (stdout line, exit 0) via `lld-link` / later `--linker-arg`.
- Seal / admission claim made: **none**. Runtime print ≠ SemASM seal.

## Follow-ups

- [x] Docs: this friction log
- [x] Tooling: `vaa build --linker-arg` for PE subsystem/entry/import libs
- [x] Tooling: forward Windows SDK discovery env into assemble/link subprocesses
- [ ] Optional: validate warn when `callable-function` lists non-empty `imports`
- [ ] Optional: map task imports → suggested linker libs (still explicit)
- [ ] Non-goal: do not admit WriteFile / REPL into the skill snapshot
- [x] E04: line loop (orchestration vs leaf seal) → see [e04-line-loop-win64.md](e04-line-loop-win64.md)
