# Friction log — T06 — path basename (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Everyday **path basename / join** (cicil-2 / T06b): ONE argv → last path
component for `\` and `/` via admitted **`find_last_byte`**; TWO argv →
length-bounded **path_join** (`\` inserted if needed, `PATH_CAP`). Probe
**`basename`** leaf for SemASM shape pressure. Tool not sealed.

Scratch (gitignored): `.vaa-exercises/t06-path/` (+ `OUTCOME.txt`).

## CLAIM

| Surface | Claim |
|---|---|
| `find_last_byte` | **Admitted** + SemASM **VUP** |
| `basename` / `path_join` | **Not admitted**; `basename` → **`UNSUPPORTED_SHAPE`** |
| Hosted tool | **Not admitted**; not sealed |
| Runtime basename / join | Integration only — see scratch `OUTCOME.txt` |

## Commands run

```text
vaa admit --leaf find_last_byte|basename …
semasm agent verify find_last_byte.asm …  # VUP
semasm agent verify basename.asm …        # UNSUPPORTED_SHAPE
vaa build find_last_byte.asm --object-only --target x86_64-pc-windows-msvc --output-dir out
vaa build main.asm --target … --extra-object out/find_last_byte.o --linker-arg …
basename.exe                              # → file.txt
basename.exe C:\foo\bar.txt               # → bar.txt
basename.exe C:\foo bar.txt               # → C:\foo\bar.txt  (T06b)
```

## What helped

- `find_last_byte` is the right admitted primitive for Win/POSIX separators.
- Hosted picks the later of `\` vs `/` indices.

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `basename` → `UNSUPPORTED_SHAPE` | honesty / SemASM gap | Path semantics ≠ scan+needle |
| 2 | `path_join` was deferred at cicil-1 | resolved (T06b) | Two-argv join in hosted main |
| 3 | No drive-letter / trailing-sep edge cases | deferred | Trailing `\` handled for join |
| 4 | Leaves VUP not strict `verified` | honesty | Expected |

## Outcomes

- Working leaf: **admitted VUP**. Probe leaf: **decline + UNSUPPORTED_SHAPE**.
- Tool **runs** basename + path_join; seal: **none**. Scratch `OUTCOME.txt`.

## Follow-ups

- [x] T06b: `path_join` length-bounded (`\` insert)
- [ ] T06c: dirname / trailing separators
- [ ] Non-goal: do not admit `basename` as a skill leaf
- [x] Roadmap → `friction_logged`
