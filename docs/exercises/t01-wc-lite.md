# Friction log — T01 — wc-lite (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Everyday **byte + LF line counter** (`wc`-lite): hosted opens `sample.txt` (else
stdin), reads ≤4096-byte chunks, uses admitted leaf **`count_byte`** for LF
counts, prints `bytes` / `lines`. Do **not** claim the tool binary is sealed.

Scratch (gitignored): `.vaa-exercises/t01-wc-lite/`.

## CLAIM

| Surface | Claim |
|---|---|
| `count_byte` | **Admitted** + SemASM **`verified_under_preconditions` (VUP)** — report as VUP, not plain verified |
| Hosted `wc_lite` / `mainCRTStartup` | **Not admitted**; not SemASM-sealed |
| Runtime `bytes 14` / `lines 3` | Integration evidence only |

## Commands run

```text
vaa admit --leaf count_byte --target x86_64-pc-windows-msvc --format json
vaa admit --leaf wc_lite --target x86_64-pc-windows-msvc --format json
vaa validate …/count_byte.vaa.toml ; vaa validate …/main.vaa.toml
semasm agent verify count_byte.asm count_byte.sem.toml … --allow-execution
vaa build count_byte.asm … --object-only
vaa build main.asm … --extra-object out/count_byte.o --linker-arg …
# run wc_lite.exe with cwd = scratch (sample.txt)
```

## What helped

- E03–E04 tooling: `--object-only`, `--extra-object`, `--linker-arg`, SDK env.
- Hosted validate returned `suggested_linker_args` for kernel32 imports.
- `count_byte` chunk size 4096 matches fixture precondition `length <= 4096`.
- Tool output matched expectation: `bytes 14` / `lines 3` on `hello\\nworld\\nx\\n`.

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `count_byte` is VUP, not strict `verified` | honesty | `acceptance_level: verified_under_preconditions` |
| 2 | `admit wc_lite` → decline | honesty | Expected; tool ≠ leaf |
| 3 | Relative `sample.txt` requires correct process cwd | tooling / agent-mistake | Run with `-WorkingDirectory` scratch |
| 4 | Argv path not implemented yet (roadmap “argv path”) | deferred | Hardcoded `sample.txt` / stdin fallback |
| 5 | Easy to point SemASM verify at hosted asm + leaf contract | agent-mistake | Use separate hosted contract or skip verify |

## Outcomes

- Leaf: **admitted VUP** + SemASM VUP verify (7/7).
- Hosted: validate **ok**; admit **decline**; PE **runs** correctly.
- Seal claim for tool: **none**.

## Follow-ups

- [ ] T01b: parse argv path via `GetCommandLineA` / `CommandLineToArgvW`
- [ ] Optional: UTF-16 `CreateFileW` + wide path
- [ ] Non-goal: do not admit `wc_lite` as a skill leaf
- [x] Roadmap status → `friction_logged` / cicil-1 `done_enough` for stdin/file-name slice
