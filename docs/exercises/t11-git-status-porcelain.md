# Friction log — T11 — `git status` porcelain summary

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa`)

## Intent

Everyday **`git status` porcelain summary** (cicil-1): parse text output of
`git status --porcelain` (v1 style), and count lines that contain status
letters:

- `hunks` (not used here)
- `M`: any of `(X,Y)` is `'M'`
- `A`: any of `(X,Y)` is `'A'`
- `D`: any of `(X,Y)` is `'D'`

The tool is hosted-only (no skill seal claims). Input is read from argv1
(file path) or stdin. Output is simple:

```text
M=<n>
A=<n>
D=<n>
```

Scratch (gitignored): `.vaa-exercises/t11-git-status-porcelain/`.

## CLAIM

| Surface | Claim |
|---|---|
| Hosted porcelain parser | **Not admitted**; not sealed |
| Skill leaves / SemASM seals | none (hosted runtime parsing only) |

## Commands run

```text
vaa validate .vaa-exercises/t11-git-status-porcelain/main.vaa.toml
vaa build .vaa-exercises/t11-git-status-porcelain/main.asm …
  --output-dir .vaa-exercises/t11-git-status-porcelain/out
  --linker-arg /subsystem:console
  --linker-arg /entry:mainCRTStartup
  --linker-arg /DEFAULTLIB:kernel32.lib

git_status_summary.exe sample_porcelain.txt
```

## Evidence

Using scratch `sample_porcelain.txt`, stdout was:

```text
M=2
A=1
D=1
```

## What helped

- Same streaming newline-delimited state machine pattern as T10.

## Friction

- Porcelain has two status columns `XY`; ensure we capture both bytes even
  when the line start spans `ReadFile` chunk boundaries.

## Outcomes

- Tool runs and counts M/A/D correctly on sample input.

## Follow-ups

- [ ] Verify semantics on real `git status --porcelain` outputs (rename,
  copies, unmerged lines with other letters).
- [ ] Option: output also total line count / other letters if needed.

