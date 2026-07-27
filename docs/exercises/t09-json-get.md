# Friction log — T09 — JSON key extract (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Strict-subset **JSON key extract**: find `"key"` then `:` value via **`memcmp`**
sliding window + **`find_first_byte`**. Values: quoted strings **or** bare
tokens (number/bool) until `,` / `}` / whitespace (**T09c**). Probe **`json_get`**
parse leaf. No minify / RFC JSON. Tool not sealed.

Scratch (gitignored): `.vaa-exercises/t09-json-get/`.

## CLAIM

| Surface | Claim |
|---|---|
| `memcmp`, `find_first_byte` | **Admitted** + SemASM **VUP** |
| `json_get` | **Not admitted**; SemASM **`UNSUPPORTED_SHAPE`** (dual ptr+len) |
| Hosted tool | **Not admitted**; not sealed |
| Runtime `name`→`widget`, `count`→`3`, `ok`→`true` | Integration evidence only |

## Commands run

```text
vaa admit --leaf memcmp|find_first_byte|json_get …
semasm agent verify memcmp|find_first_byte …  # VUP
semasm agent verify json_get.asm …            # UNSUPPORTED_SHAPE
vaa build memcmp.asm|find_first_byte.asm --object-only --target x86_64-pc-windows-msvc
vaa build main.asm --extra-object ×2 --linker-arg /subsystem:console /entry:mainCRTStartup /DEFAULTLIB:kernel32.lib
json_get.exe                    # → widget
json_get.exe sample.json count  # → 3
json_get.exe sample.json ok     # → true
```

## What helped

- Same compose pattern as T07: admitted scans beat inventing a parse leaf.
- T09c bare-value scan is a tiny hosted branch after the existing `:` skip.

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `json_get` → `UNSUPPORTED_SHAPE` | honesty / SemASM gap | Dual-buffer parse |
| 2 | No nested objects / arrays / escapes | deferred | Strict subset |
| 3 | Minify not implemented | deferred | T09b skipped (OUTCOME.txt) |
| 4 | Needle can false-match inside strings | deferred | Honest cicil-1 risk |
| 5 | Single 4KiB buffer | deferred | |

## Outcomes

- Working leaves: **admitted VUP**. Parse candidate: **decline + UNSUPPORTED_SHAPE**.
- Tool **runs** with T09c bare values; seal: **none**.
- T09b minify: **skipped** this pass.

## Follow-ups

- [ ] T09b: minify whitespace-only subset (`minify` argv)
- [x] T09c: numeric / bool values
- [ ] Non-goal: do not weaken `UNSUPPORTED_SHAPE` for JSON
- [x] Roadmap → `friction_logged`
