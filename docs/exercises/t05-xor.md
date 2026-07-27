# Friction log — T05 — `xor` filter (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Everyday **XOR filter** (cicil-2): each input byte XOR argv key
(`xor_filter.exe [path] [key]`, key = two hex digits, default `5A`) via leaf
**`xor_u8`** (binary i64). Also probe buffer leaf **`xor_bytes`**. Demonstrate
double-XOR restores plaintext. Tool not sealed.

Scratch (gitignored): `.vaa-exercises/t05-xor/`.

## CLAIM

| Surface | Claim |
|---|---|
| `xor_u8` (i64,i64→i64) | **Not admitted**; SemASM **`UNSUPPORTED_SHAPE`** (binary XOR not in add/sub/min/max tokens) |
| `xor_bytes` (buf+key) | **Not admitted**; SemASM **`UNSUPPORTED_SHAPE`** |
| `replace_byte` | **Admitted VUP** (related buffer mutate; not used by cicil-1 tool) |
| Hosted filter | **Not admitted**; not sealed |
| Runtime `Hello` ↔ xor ↔ `Hello` | Integration evidence only |

## Commands run

```text
vaa admit --leaf xor_u8|xor_bytes|xor_i64|replace_byte …
vaa validate xor_u8|xor_bytes|main.vaa.toml
semasm agent verify xor_u8.asm …     # UNSUPPORTED_SHAPE
semasm agent verify xor_bytes.asm …  # UNSUPPORTED_SHAPE
vaa build xor_u8.asm --object-only
vaa build main.asm … --extra-object xor_u8.o --linker-arg …
xor_filter.exe sample.bin      # → 12 3f 36 36 35 (key 5A default)
xor_filter.exe sample.bin 5A   # same
xor_filter.exe sample.bin FF   # then again → Hello round-trip
xor_filter.exe xor1-in.bin     # → Hello
```

## What helped

- Same T04 pattern: leaf links and runs even when SemASM declines shape.
- Clear SemASM hint: binary i64 names are `add_*/sub_*/min_*/max_*` only today.

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `xor_u8` arity matches binary pure-int but name rejected | honesty / SemASM gap | Need `xor_*` token or stay declined |
| 2 | `xor_bytes` mutate+key not a buffer oracle | honesty / SemASM gap | Closest admitted: `replace_byte` |
| 3 | Fixed KEY; no argv key | resolved cicil-2 | argv 2 hex digits |
| 4 | Per-byte call overhead (not vectorized) | deferred | Depth vs perf |
| 5 | `crc32` not attempted | deferred | T05b |

## Outcomes

- Leaves used/probed: **not admitted**; **UNSUPPORTED_SHAPE**.
- Tool **runs** (round-trip OK); seal: **none**.
- No VAA `src/` change — compose / decline path sufficient.

## Follow-ups

- [ ] T05b: `crc32` / rolling checksum leaf pressure
- [x] T05c: argv key byte; stdin pipeline (key done; stdin still via CreateFile fail path)
- [ ] Optional SemASM: recognize `xor_*` binary i64 (only if repeated demand)
- [ ] Non-goal: do not widen admission just for this filter
- [x] Roadmap → `friction_logged`
