# Friction log — T12 — log grepper

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa`)

## Intent

Everyday **log grepper** (cicil-1): count occurrences of a byte
substring (`needle`) inside a log-like input, streamed via `ReadFile` in
chunks.

Requirements:
- must correctly count matches that start in one `ReadFile` chunk and
  finish in the next chunk (carry overlap)
- output is simple: `matches=<n>` (substring occurrences; overlaps allowed
  by sliding window)

Scratch (gitignored): `.vaa-exercises/t12-log-grepper/`.

## CLAIM

| Surface | Claim |
|---|---|
| Hosted grepper tool | **Not admitted**; not sealed |
| Skill leaves / SemASM seals | none (hosted runtime parsing only) |

## Commands run

```text
vaa validate .vaa-exercises/t12-log-grepper/main.vaa.toml
vaa build .vaa-exercises/t12-log-grepper/main.asm …
  --output-dir .vaa-exercises/t12-log-grepper/out
  --linker-arg /subsystem:console
  --linker-arg /entry:mainCRTStartup
  --linker-arg /DEFAULTLIB:kernel32.lib

grepper.exe <abs-path>/sample.log NEEDL
  -> matches=3
```

Sample was constructed so `NEEDL` appears at byte offsets `0`, `60`,
`80` with `CHUNK=64`, ensuring at least one boundary-spanning match.

## What helped

- Reuse of the T10/T11 streaming discipline: always carry the last
  `(needle_len - 1)` bytes into the next `ReadFile` chunk.
- Compare logic uses direct byte indexing (no RIP-relative + index
  tricks).

## Friction

- Running with a *relative* log path failed open and produced `matches=0`
  because `WorkingDirectory` was `out/`. Using an absolute log path fixed it.

## Outcomes

- `grepper.exe` correctly outputs `matches=3` on the boundary-spanning
  sample.

## Follow-ups

- [ ] Decide whether to count overlapping matches explicitly in docs
  (current behavior is overlap-friendly).
- [ ] Add a “count lines containing needle” mode (optional).

