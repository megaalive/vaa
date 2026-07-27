# Friction log — T05 — `xor` filter (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Everyday **XOR filter** (cicil-2): each input byte XOR argv key
(`xor_filter.exe [path] [key]`, key = two hex digits, default `5A`) via leaf
**`xor_u8`** (binary i64). Also probe buffer leaf **`xor_bytes`**. **D5 / T05b:**
probe **`crc32`** (ptr+len → i64 rolling stub) for SemASM shape pressure.
Demonstrate double-XOR restores plaintext. Tool not sealed.

Scratch (gitignored): `.vaa-exercises/t05-xor/`.

## CLAIM

| Surface | Claim |
|---|---|
| `xor_u8` (i64,i64→i64) | **Not admitted**; SemASM **`UNSUPPORTED_SHAPE`** (binary XOR not in add/sub/min/max tokens) |
| `xor_bytes` (buf+key) | **Not admitted**; SemASM **`UNSUPPORTED_SHAPE`** |
| `crc32` (buf+len→i64) | **Not admitted**; SemASM **`UNSUPPORTED_SHAPE`** (checksum ≠ scan+needle / count) |
| `replace_byte` | **Admitted VUP** (related buffer mutate; not used by cicil-1 tool) |
| Hosted filter | **Not admitted**; not sealed |
| Runtime `Hello` ↔ xor ↔ `Hello` | Integration evidence only |

## Commands run

```text
vaa admit --leaf xor_u8|xor_bytes|xor_i64|replace_byte|crc32 …
vaa validate xor_u8|xor_bytes|crc32|main.vaa.toml
semasm agent verify xor_u8.asm …     # UNSUPPORTED_SHAPE
semasm agent verify xor_bytes.asm …  # UNSUPPORTED_SHAPE
semasm agent verify crc32.asm …      # UNSUPPORTED_SHAPE (D5)
vaa build xor_u8.asm --object-only
vaa build main.asm … --extra-object xor_u8.o --linker-arg …
xor_filter.exe sample.bin      # → 12 3f 36 36 35 (key 5A default)
xor_filter.exe sample.bin 5A   # same
xor_filter.exe sample.bin FF   # then again → Hello round-trip
xor_filter.exe xor1-in.bin     # → Hello
```

### D5 admit / verify evidence (stdout JSON)

```text
vaa admit --leaf crc32 --target x86_64-pc-windows-msvc
→ {"admitted":false,"leaf":"crc32","next_action":"decline",…}

semasm agent verify crc32.asm crc32.sem.toml --target x86_64-pc-windows-msvc --format json
→ {"kind":"agent_failure","code":"UNSUPPORTED_SHAPE",…,
   "detail":"Observed parameters=[Ptr …, Usize] returns=[Int { bits: 64 }]. … Buffer scans need ptr+length (+ optional needle)."}
```

Honesty: tool/leaf candidate may assemble and run; **not admitted**, **not verified**.

## What helped

- Same T04 pattern: leaf links and runs even when SemASM declines shape.
- Clear SemASM hint: binary i64 names are `add_*/sub_*/min_*/max_*` only today.
- `crc32` pressure confirms checksum folds are outside current buffer oracles.

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `xor_u8` arity matches binary pure-int but name rejected | honesty / SemASM gap | Need `xor_*` token or stay declined |
| 2 | `xor_bytes` mutate+key not a buffer oracle | honesty / SemASM gap | Closest admitted: `replace_byte` |
| 3 | Fixed KEY; no argv key | resolved cicil-2 | argv 2 hex digits |
| 4 | Per-byte call overhead (not vectorized) | deferred | Depth vs perf |
| 5 | `crc32` not attempted | **resolved D5** | admit decline + `UNSUPPORTED_SHAPE` |

## Outcomes

- Leaves used/probed (`xor_*`, `crc32`): **not admitted**; **UNSUPPORTED_SHAPE**.
- Tool **runs** (round-trip OK); seal: **none**.
- No VAA `src/` change — compose / decline path sufficient.

## Follow-ups

- [x] T05b / D5: `crc32` / rolling checksum leaf pressure
- [x] T05c: argv key byte; stdin pipeline (key done; stdin still via CreateFile fail path)
- [ ] Optional SemASM: recognize `xor_*` binary i64 (only if repeated demand)
- [ ] Non-goal: do not widen admission just for this filter / crc32
- [x] Roadmap → `friction_logged`
