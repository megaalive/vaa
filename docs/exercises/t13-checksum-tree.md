# Friction log — T13 — checksum tree

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa`)

## Intent

Everyday **checksum tree** (bounded): enumerate files in a directory and
emit a deterministic checksum per file.

This bounded version is **non-recursive** (top-level files only) and uses a
simple checksum: `sum(byte_values)` mod `2^64`, printed as 16 hex digits.

Scratch (gitignored): `.vaa-exercises/t13-checksum-tree/`.

## CLAIM

| Surface | Claim |
|---|---|
| Hosted checksum tool | **Not admitted**; not sealed |
| Skill leaves / SemASM seals | none (hosted runtime hashing only) |

## Commands run

```text
vaa validate .vaa-exercises/t13-checksum-tree/main.vaa.toml
vaa build .vaa-exercises/t13-checksum-tree/main.asm …
  --output-dir .vaa-exercises/t13-checksum-tree/out

checksum_tree.exe <abs-path-to-t13-sample-dir>
```

Sample input (scratch):
- `sample/a.txt` = `"hello"` → expected sum-byte = `0x214`
- `sample/b.txt` = `"world"` → expected sum-byte = `0x228`

Observed stdout:

```text
a.txt 0000000000000214
b.txt 0000000000000228
```

## What helped

- Win64 streaming discipline: `FindFirstFileA` + `ReadFile` chunk loop
  (and a local checksum accumulator).
- `write_cstring` helper: bugfix in argument ordering for `WriteFile`
  length.
- Formatting helper: corrected hex conversion variable-shift to use `CL`
  shift-count.

## Friction

- `write_cstring` initially printed too many bytes (computed length was
  overwritten before `WriteFile`).
- Output formatting had an extra NUL between filename and checksum (space
  length included the terminator).

## Outcomes

- Tool runs and produces correct checksum hex for top-level files.

## Follow-ups

- [ ] Make the traversal recursive (true checksum *tree*)
- [ ] Replace simple sum-byte with a stronger hash (hosted or admitted leaf)

