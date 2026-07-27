# Friction log — T02 — `head` (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Everyday **`head`/`tail`** (cicil-2 + D4 streaming): emit first/last **N**
LF-terminated lines (`head.exe|tail.exe [path] [n]`, n decimal 1–99, default 10)
from argv/`sample.txt`/stdin. **`CHUNK=64`** forces multi-chunk on the 96-byte
sample; partial lines carried across `ReadFile`. Line ends via admitted
**`find_first_byte`**. Probe **`find_nth_lf`** as denser buffer leaf. Tool not
sealed.

Scratch (gitignored): `.vaa-exercises/t02-head/`.

## CLAIM

| Surface | Claim |
|---|---|
| `find_first_byte` | **Admitted** + SemASM **VUP** |
| `find_nth_lf` | **Not admitted**; SemASM **`UNSUPPORTED_SHAPE`** (`ptr+len+usize` ≠ scan+needle) |
| Hosted `head` / `tail` | **Not admitted**; not sealed |
| Runtime N lines (incl. multi-chunk) | Integration evidence only |

## Commands run

```text
vaa admit --leaf find_first_byte|find_nth_lf|head …
vaa validate find_first_byte|find_nth_lf|main.vaa.toml
semasm agent verify find_first_byte.asm … # VUP
semasm agent verify find_nth_lf.asm …     # UNSUPPORTED_SHAPE
vaa build find_first_byte.asm … --object-only
vaa build main.asm|tail.asm … --extra-object find_first_byte.o --linker-arg …
head.exe sample.txt          # → line1..line10 (CHUNK=64 stream)
head.exe sample.txt 3        # → line1..line3 (sample 96B > CHUNK)
tail.exe sample.txt 3        # → line13..line15 (ring of last N)
```

## What helped

- Looping an admitted first-byte scan instead of inventing `find_nth_*`.
- Carry buffer for incomplete line across 64-byte reads; close handle only at EOF.
- Tail: ring of last N complete lines + same carry policy.

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `find_nth_lf` → `UNSUPPORTED_SHAPE` | honesty / SemASM gap | Extra `usize n` not recognized |
| 2 | `admit find_nth_lf|head` decline | honesty | Expected |
| 3 | Fixed `HEAD_N=10`; no `-n` argv | resolved cicil-2 | argv N 1–99 |
| 4 | Single 4KiB buffer (not true stream) | **resolved D4** | `CHUNK=64` + carry |
| 5 | `tail` not implemented | resolved cicil-2 | `tail.asm` + streaming ring |

## Outcomes

- Leaf used by tool: **admitted VUP**.
- Denser leaf candidate: **decline + UNSUPPORTED_SHAPE**.
- Tool **runs** (`head`/`tail` with argv N + multi-chunk); seal: **none**.

## Follow-ups

- [x] T02b: `tail` + argv N
- [x] T02c / D4: streaming multi-chunk head (and tail) without loading whole file
- [ ] Optional SemASM: `find_nth_u8` oracle (only if many tools need it)
- [ ] Non-goal: do not admit `head` / `find_nth_lf` as skill leaves
- [x] Roadmap → `friction_logged` for cicil-1 / cicil-2 / D4
