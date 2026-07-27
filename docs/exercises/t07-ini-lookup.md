# Friction log — T07 — INI key lookup (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Everyday **INI `[section]` key lookup** (cicil-2 / T07b + quoted values): defaults
`sample.ini` / `demo` / `name`. Locate `=` / LF / `[` via admitted
**`find_first_byte`**; match section/key with admitted **`memcmp`**. If value after
`=` starts with `"`, emit until closing `"` (no escapes); else until LF.
Empty section argv `""` → flat whole-file. Probe **`ini_lookup`** for SemASM
shape pressure. Tool binary not sealed.

Scratch (gitignored): `.vaa-exercises/t07-ini-lookup/`.

## CLAIM

| Surface | Claim |
|---|---|
| `memcmp`, `find_first_byte` | **Admitted** + SemASM **VUP** |
| `ini_lookup` (parse leaf) | **Not admitted**; SemASM **`UNSUPPORTED_SHAPE`** (two ptr+len buffers) |
| Hosted tool | **Not admitted**; not sealed |
| Runtime `demo`/`name`→`widget`; `demo`/`title`→`hello=world` | Integration only |

## Commands run

```text
vaa admit --leaf memcmp|find_first_byte|ini_lookup|ini_get …
vaa build memcmp.asm|find_first_byte.asm --object-only --target …
vaa build main.asm … --extra-object memcmp.o --extra-object find_first_byte.o --linker-arg …
ini_lookup.exe                          # → widget ([demo]/name)
ini_lookup.exe sample.ini demo count    # → 3
ini_lookup.exe sample.ini demo title    # → hello=world  (quoted; '=' inside)
ini_lookup.exe sample.ini other name    # → gadget
ini_lookup.exe sample.ini "" flat_key   # → orphan (flat mode)
```

## What helped

- Compose two admitted buffer leaves instead of one parse leaf.
- Quoted-value branch is a tiny hosted check after `=` (closing `"` via find).
- Save length **before** `find_first_byte` — leaf clobbers `rdx`.

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `ini_lookup` → `UNSUPPORTED_SHAPE` | honesty / SemASM gap | Dual ptr+len not a harness oracle |
| 2 | `admit ini_lookup|ini_get` decline | honesty | Expected |
| 3 | Leafs are VUP, not strict `verified` | honesty | admit JSON |
| 4 | Default `sample.ini` depends on process CWD | agent-mistake / tool UX | Run from scratch or pass argv path |
| 5 | No `[section]` at cicil-1 | resolved (T07b) | Section window until next `[` |
| 6 | Single 4KiB read | deferred | Matches leaf `length <= 4096` |
| 7 | Quoted values | resolved (D2) | `title="hello=world"` |

## Outcomes

- Working leaves: **admitted VUP**.
- Parse candidate: **decline + UNSUPPORTED_SHAPE** (pressure on SemASM depth).
- Tool **runs** section + quoted values + optional flat (`""`); seal: **none**.

## Follow-ups

- [x] T07b: `[section]` + `key` lookup
- [x] D2: quoted values after `=` (until closing `"`, no escapes)
- [ ] T07c: streaming / multi-chunk files
- [ ] Optional SemASM epic: dual-buffer / “find key then slice” template (only if repeated)
- [ ] Non-goal: do not admit `ini_lookup` just because the tool works
- [x] Roadmap → `friction_logged` for cicil-2
