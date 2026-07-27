# Friction log — T01 — wc-lite (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Everyday **byte + LF line counter** (`wc`-lite): hosted opens **argv path**, else
`sample.txt`, else stdin; reads ≤4096-byte chunks; uses admitted leaf
**`count_byte`** for LF counts; prints `bytes` / `lines`. Tool binary is **not**
sealed.

Scratch (gitignored): `.vaa-exercises/t01-wc-lite/`.

## CLAIM

| Surface | Claim |
|---|---|
| `count_byte` | **Admitted** + SemASM **`verified_under_preconditions` (VUP)** — report as VUP |
| Hosted `wc_lite` | **Not admitted**; not SemASM-sealed |
| Runtime counts | Integration evidence only |

## Commands run

```text
vaa admit --leaf count_byte|wc_lite …
vaa validate count_byte.vaa.toml ; vaa validate main.vaa.toml
semasm agent verify count_byte.asm … --allow-execution
vaa build count_byte.asm … --object-only
vaa build main.asm … --extra-object out/count_byte.o --linker-arg …
wc_lite.exe other.txt          # → bytes 9 / lines 3 (any cwd)
wc_lite.exe                    # cwd=scratch → sample.txt → bytes 14 / lines 3
```

## What helped

- E03–E04 tooling: `--object-only`, `--extra-object`, `--linker-arg`, SDK env.
- Hosted validate → `suggested_linker_args` (+ `GetCommandLineA` in imports).
- Chunk size 4096 matches `count_byte` precondition `length <= 4096`.

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `count_byte` is VUP, not strict `verified` | honesty | admit JSON |
| 2 | `admit wc_lite` → decline | honesty | Expected |
| 3 | Relative `sample.txt` needs correct cwd when no argv | agent-mistake | Documented |
| 4 | T01b: odd `push` count + `sub rsp` → AV (`0xC0000005`) | tooling / agent-mistake | Win64 need 16-byte align before `call`; fixed `sub rsp, 80` |
| 5 | Argv parse is ANSI `GetCommandLineA` only | deferred | UTF-16 / `CommandLineToArgvW` optional |
| 6 | Easy to verify hosted asm with leaf contract | agent-mistake | Skip / separate contract |

## Outcomes

- Cicil-1: hardcoded/`sample.txt`+stdin — **ok**.
- Cicil-2 (T01b): argv path — **ok** (`other.txt` → 9/3; default → 14/3).
- Seal claim for tool: **none**.

## Follow-ups

- [x] T01b: parse argv path via `GetCommandLineA`
- [ ] Optional: UTF-16 `CreateFileW` + `CommandLineToArgvW`
- [ ] Optional: docs note / lint for Win64 stack alignment in hosted templates
- [ ] Non-goal: do not admit `wc_lite` as a skill leaf
- [x] Roadmap → `done_enough` for T01 (argv+file+stdin slice)
