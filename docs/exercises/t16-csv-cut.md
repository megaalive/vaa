# Friction log — T16 — CSV column cut (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Everyday **CSV column cut** (cicil-1): emit 0-based column (default **1**) per
LF line via **`find_first_byte`** for `,` / LF. Probe **`csv_cut`** leaf. No
quotes/escapes. Tool not sealed.

Scratch (gitignored): `.vaa-exercises/t16-csv-cut/`.

## CLAIM

| Surface | Claim |
|---|---|
| `find_first_byte` | **Admitted** + SemASM **VUP** |
| `csv_cut` | **Not admitted**; SemASM **`UNSUPPORTED_SHAPE`** (`ptr+len+col`) |
| Hosted tool | **Not admitted**; not sealed |
| Runtime col1 → `b/2/y`; col0 → `a/1/x` | Integration evidence only |

## Commands run

```text
vaa admit --leaf find_first_byte|csv_cut …
semasm agent verify find_first_byte.asm …  # VUP
semasm agent verify csv_cut.asm …          # UNSUPPORTED_SHAPE
vaa build … --extra-object find_first_byte.o --linker-arg …
csv_cut.exe                 # → b\n2\ny\n
csv_cut.exe sample.csv 0    # → a\n1\nx\n
```

## What helped

- Delimiter scan is exactly the admitted first-byte leaf in a hosted loop.
- Same shape pressure as `find_nth_lf`: extra column index ≠ scan+needle.

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `csv_cut` → `UNSUPPORTED_SHAPE` | honesty / SemASM gap | ptr+len+col |
| 2 | No quoted fields / commas-in-quotes | deferred | Cicil-1 |
| 3 | Single-digit column argv only | deferred | |
| 4 | Single 4KiB buffer | deferred | |

## Outcomes

- Working leaf: **admitted VUP**. Candidate: **decline + UNSUPPORTED_SHAPE**.
- Tool **runs**; seal: **none**. Tier C pick logged.

## Follow-ups

- [ ] T16b: quoted CSV fields
- [ ] T16c: multi-digit `-f` / stdout TSV
- [ ] Non-goal: do not admit `csv_cut` as a skill leaf
- [x] Roadmap → `friction_logged`
