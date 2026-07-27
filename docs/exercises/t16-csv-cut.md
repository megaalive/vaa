# Friction log — T16 — CSV column cut (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Everyday **CSV column cut**: emit 0-based column (default **1**, argv **0–99**)
per LF line via **`find_first_byte`** for `,` / LF / `"`. Light quotes (**T16b**):
field starting with `"` takes until closing `"` (no escapes). Probe **`csv_cut`**
leaf. Tool not sealed.

Scratch (gitignored): `.vaa-exercises/t16-csv-cut/`.

## CLAIM

| Surface | Claim |
|---|---|
| `find_first_byte` | **Admitted** + SemASM **VUP** |
| `csv_cut` | **Not admitted**; SemASM **`UNSUPPORTED_SHAPE`** (`ptr+len+col`) |
| Hosted tool | **Not admitted**; not sealed |
| Runtime col1 → `b`/`hello,world`/`y`/`1`; col10 → `10` | Integration evidence only |

## Commands run

```text
vaa admit --leaf find_first_byte|csv_cut …
semasm agent verify find_first_byte.asm …  # VUP
semasm agent verify csv_cut.asm …          # UNSUPPORTED_SHAPE
vaa build find_first_byte.asm --object-only --target x86_64-pc-windows-msvc
vaa build main.asm --extra-object find_first_byte.o --linker-arg …
csv_cut.exe                 # → b\nhello,world\ny\n1\n
csv_cut.exe sample.csv 0    # → a\n1\nx\n0\n
csv_cut.exe sample.csv 10   # → 10\n
```

## What helped

- Delimiter scan is exactly the admitted first-byte leaf in a hosted loop.
- Quoted-field branch reuses `find_first_byte` for closing `"`.

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `csv_cut` → `UNSUPPORTED_SHAPE` | honesty / SemASM gap | ptr+len+col |
| 2 | No escaped quotes (`""`) | deferred | T16b light only |
| 3 | Multi-digit col capped 0–99 (2 digits) | bounded | T16c |
| 4 | Single 4KiB buffer | deferred | |
| 5 | Rows missing the column emit nothing | deferred | |

## Outcomes

- Working leaf: **admitted VUP**. Candidate: **decline + UNSUPPORTED_SHAPE**.
- Tool **runs** with T16c + T16b light; seal: **none**.

## Follow-ups

- [x] T16b: quoted CSV fields (light; no escapes)
- [x] T16c: multi-digit column argv 0–99
- [ ] Escaped quotes / RFC CSV
- [ ] Non-goal: do not admit `csv_cut` as a skill leaf
- [x] Roadmap → `friction_logged`
