# Friction log — T06 — path basename (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Everyday **basename** (cicil-1): last path component for `\` and `/`, via
admitted **`find_last_byte`**. Probe **`basename`** leaf for SemASM shape
pressure. Length-bounded argv path; tool not sealed.

Scratch (gitignored): `.vaa-exercises/t06-path/`.

## CLAIM

| Surface | Claim |
|---|---|
| `find_last_byte` | **Admitted** + SemASM **VUP** |
| `basename` / `path_join` | **Not admitted**; `basename` → **`UNSUPPORTED_SHAPE`** |
| Hosted tool | **Not admitted**; not sealed |
| Runtime `…\file.txt` → `file.txt` | Integration evidence only |

## Commands run

```text
vaa admit --leaf find_last_byte|basename …
semasm agent verify find_last_byte.asm …  # VUP
semasm agent verify basename.asm …        # UNSUPPORTED_SHAPE
vaa build … --extra-object find_last_byte.o --linker-arg …
basename.exe                              # → file.txt
basename.exe D:/foo/bar/baz.qux           # → baz.qux
```

## What helped

- `find_last_byte` is the right admitted primitive for Win/POSIX separators.
- Hosted picks the later of `\` vs `/` indices.

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `basename` → `UNSUPPORTED_SHAPE` | honesty / SemASM gap | Path semantics ≠ scan+needle |
| 2 | `path_join` not implemented in tool | deferred | Cicil-1 basename only |
| 3 | No drive-letter / trailing-sep edge cases | deferred | |
| 4 | Leaves VUP not strict `verified` | honesty | Expected |

## Outcomes

- Working leaf: **admitted VUP**. Probe leaf: **decline + UNSUPPORTED_SHAPE**.
- Tool **runs**; seal: **none**.

## Follow-ups

- [ ] T06b: `path_join` length-bounded (`\` insert)
- [ ] T06c: dirname / trailing separators
- [ ] Non-goal: do not admit `basename` as a skill leaf
- [x] Roadmap → `friction_logged`
