# Friction log — T02 — `head` (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Everyday **`head`** (cicil-1): emit the first **10** LF-terminated lines from
argv/`sample.txt`/stdin. Line ends via admitted **`find_first_byte`**. Probe
**`find_nth_lf`** as a denser buffer leaf for SemASM shape pressure. Tool not
sealed.

Scratch (gitignored): `.vaa-exercises/t02-head/`.

## CLAIM

| Surface | Claim |
|---|---|
| `find_first_byte` | **Admitted** + SemASM **VUP** |
| `find_nth_lf` | **Not admitted**; SemASM **`UNSUPPORTED_SHAPE`** (`ptr+len+usize` ≠ scan+needle) |
| Hosted `head` | **Not admitted**; not sealed |
| Runtime 10 lines | Integration evidence only |

## Commands run

```text
vaa admit --leaf find_first_byte|find_nth_lf|head …
vaa validate find_first_byte|find_nth_lf|main.vaa.toml
semasm agent verify find_first_byte.asm … # VUP
semasm agent verify find_nth_lf.asm …     # UNSUPPORTED_SHAPE
vaa build find_first_byte.asm … --object-only
vaa build main.asm … --extra-object find_first_byte.o --linker-arg …
head.exe sample.txt
# → line1..line10 (10 LF bytes)
```

## What helped

- Looping an admitted first-byte scan instead of inventing `find_nth_*`.
- SemASM hint: buffer scans need `ptr+length` (+ optional needle) — nth-count
  is a different shape.

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `find_nth_lf` → `UNSUPPORTED_SHAPE` | honesty / SemASM gap | Extra `usize n` not recognized |
| 2 | `admit find_nth_lf|head` decline | honesty | Expected |
| 3 | Fixed `HEAD_N=10`; no `-n` argv | deferred | Cicil-1 |
| 4 | Single 4KiB buffer (not true stream) | deferred | Same as T01/T07 chunk |
| 5 | `tail` not implemented | deferred | T02b |

## Outcomes

- Leaf used by tool: **admitted VUP**.
- Denser leaf candidate: **decline + UNSUPPORTED_SHAPE**.
- Tool **runs** (exactly 10 LF lines on `sample.txt`); seal: **none**.

## Follow-ups

- [ ] T02b: `tail` + `-n` argv
- [ ] T02c: streaming multi-chunk head without loading whole file
- [ ] Optional SemASM: `find_nth_u8` oracle (only if many tools need it)
- [ ] Non-goal: do not admit `head` / `find_nth_lf` as skill leaves
- [x] Roadmap → `friction_logged` for cicil-1
