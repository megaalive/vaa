# Friction log — T03 — `uniq` consecutive (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Everyday **`uniq`** (cicil-2 + D4 streaming): drop **adjacent** duplicate lines;
treat `\r\n` like `\n` for compare/emit (strip CR before LF). **`CHUNK=64`** +
carry for incomplete lines; **prev-line** survives across chunks. Compose admitted
**`find_first_byte`** + **`memcmp`** + **`memcpy`**. Probe denser leaf
**`equal_run`**. Tool not sealed.

Scratch (gitignored): `.vaa-exercises/t03-uniq/`.

## CLAIM

| Surface | Claim |
|---|---|
| `find_first_byte`, `memcmp`, `memcpy` | **Admitted** + SemASM **VUP** |
| `equal_run` | **Not admitted**; SemASM **`UNSUPPORTED_SHAPE`** |
| Hosted `uniq` | **Not admitted**; not sealed |
| Runtime multi-chunk collapse → `ppp…\ndup\na\nb\nc\nd\n` | Integration evidence only |

## Commands run

```text
vaa admit --leaf find_first_byte|memcmp|memcpy|equal_run|uniq …
vaa validate … ; semasm agent verify memcmp|find_first_byte|memcpy  # VUP
semasm agent verify equal_run.asm …   # UNSUPPORTED_SHAPE
vaa build leaves --object-only ; vaa build main … --extra-object ×3 --linker-arg …
uniq.exe sample.txt   # 83B sample, CHUNK=64; dup pair across chunk boundary
# → p×60\ndup\na\nb\nc\nd\n (adjacent dups collapsed; prev survived chunk)
```

## What helped

- Three admitted buffer leaves cover line split / compare / prev-line stash.
- Carry incomplete line; only complete lines enter normalize→memcmp→emit.
- `equal_run` (ptr+len → run length) is useful but not a recognized scan+needle.

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `equal_run` → `UNSUPPORTED_SHAPE` | honesty / SemASM gap | Not scan/memcmp oracle |
| 2 | `admit uniq|equal_run` decline | honesty | Expected |
| 3 | Adjacent-only; no global sort/uniq | deferred | Cicil-1 |
| 4 | Single 4KiB buffer | **resolved D4** | `CHUNK=64` + carry + prev spill |
| 5 | CR stripped only if part of compared line bytes | resolved cicil-2 | normalize before memcmp/emit |

## Outcomes

- Working leaves: **admitted VUP**.
- Denser leaf: **decline + UNSUPPORTED_SHAPE**.
- Tool **runs** (multi-chunk); seal: **none**.

## Follow-ups

- [x] T03b: ignore CR before LF
- [x] T03c / D4: streaming across chunk boundaries (prev-line spill)
- [ ] Non-goal: do not admit `uniq` / `equal_run` as skill leaves
- [x] Roadmap → `friction_logged`
