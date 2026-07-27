# Friction log — T04 — hexdump (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Everyday **hexdump** (cicil-2): read argv/`sample.bin`/stdin, emit lowercase hex
pairs with spaces, **runtime width** `w` (`hexdump.exe [path] [w]`, default 16,
allow 1–32) via local leaf **`nibble_to_hex`**. Do not claim SemASM/skill seal
for the leaf or the tool.

Scratch (gitignored): `.vaa-exercises/t04-hexdump/`.

## CLAIM

| Surface | Claim |
|---|---|
| `nibble_to_hex` | **Not admitted**; SemASM **`UNSUPPORTED_SHAPE`** (i64→i64 but name not a recognized unary op) |
| Hosted `hexdump` | **Not admitted**; not sealed |
| Runtime hex lines | Integration evidence only |

## Commands run

```text
vaa admit --leaf nibble_to_hex|hexdump …
vaa validate nibble_to_hex.vaa.toml ; vaa validate main.vaa.toml
semasm agent verify nibble_to_hex.asm …   # UNSUPPORTED_SHAPE
vaa build nibble_to_hex.asm … --object-only
vaa build main.asm … --extra-object out/nibble_to_hex.o --linker-arg …
hexdump.exe sample.bin
# → 00 01 0a ff 48 69 0d 0a 01 02 03 04 05 06 07 08 / 09 0a 0b 0c
hexdump.exe sample.bin 8
# → three lines of 8 / 8 / 4 pairs
```

## What helped

- T01 patterns: argv/`GetCommandLineA`, `--object-only`, `--extra-object`, stack align.
- Hosted validate → `suggested_linker_args`.
- Clear SemASM hint listing recognized unary tokens (rename path is honest).

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `nibble_to_hex` → `UNSUPPORTED_SHAPE` despite i64→i64 | honesty / SemASM gap | Encoding/nibble not a harness oracle yet |
| 2 | `admit` declines leaf + tool | honesty | Expected |
| 3 | Task `[[tests]]` exist but SemASM never runs them (shape fail first) | honesty | VAA validate ≠ SemASM verify |
| 4 | Width flags (`-w`) not implemented | resolved cicil-2 | argv w 1..32 |
| 5 | ASCII sidebar / offsets not implemented | deferred | Hex pairs only |

## Outcomes

- Tool **runs** (correct hex for `sample.bin`).
- Leaf clinic: **decline + UNSUPPORTED_SHAPE** (valuable SemASM signal).
- Seal claim: **none**.

## Follow-ups

- [ ] SemASM (optional epic): recognize `nibble_to_hex` / hex-digit unary + vectors — **only with oracle**, then consider admission separately
- [x] T04b: width flag / bytes-per-line
- [ ] T04c: offset column + ASCII gutter
- [ ] Non-goal: do not admit `hexdump` / `nibble_to_hex` just because the tool works
- [x] Roadmap status → `friction_logged` (cicil-2 runtime width)
