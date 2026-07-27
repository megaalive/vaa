# Friction log — T10 — diff hunk stats

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa`)

## Intent

Everyday **diff hunk stats** (cicil-1): parse a unified diff text from
argv file path (default `sample.diff`) or stdin.

Counts:
- `hunks`: lines starting with `@@`
- `added`: lines starting with `+` but not `+++`
- `removed`: lines starting with `-` but not `---`

Tool binary not sealed.

Scratch (gitignored): `.vaa-exercises/t10-diff-hunk-stats/`.

## CLAIM

| Surface | Claim |
|---|---|
| Hosted diff tool | **Not admitted**; not sealed |
| Skill leaves / SemASM seals | none (hosted runtime parsing only) |

## Commands run

```text
vaa validate .vaa-exercises/t10-diff-hunk-stats/main.vaa.toml
vaa build .vaa-exercises/t10-diff-hunk-stats/main.asm …
  --output-dir .vaa-exercises/t10-diff-hunk-stats/out
  --linker-arg /subsystem:console
  --linker-arg /entry:mainCRTStartup
  --linker-arg /DEFAULTLIB:kernel32.lib

diff_stats.exe sample.diff
# → added 3
#   removed 2
#   hunks 2
```

## What helped

- E04-style byte scanning + newline-delimited line state (carry state across
  `ReadFile` chunks).
- Simplified `+++` / `---` exclusion via a 2-byte check: count `+` iff
  second byte is not `+`; count `-` iff second byte is not `-`.

## Friction

- Label string length: avoid writing the trailing `\0` terminator to stdout.
- Stream edge cases: last line without `\n` needs explicit finalize.

## Outcomes

- Tool runs and produces correct counts on `sample.diff`.

## Follow-ups

- [ ] Handle CRLF variants explicitly (strip `\r` before prefix checks)
- [ ] Improve exclusion rules for exotic diff header lines
- [ ] Optionally also output per-hunk totals by parsing `@@ -a,b +c,d @@`
  headers

