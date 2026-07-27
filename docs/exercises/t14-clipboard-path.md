# Friction log — T14 — clipboard / path helper

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa`)

## Intent

Everyday **clipboard / path helper** (cicil-1): two modes via `argv1`:

- `cwd`  — print current working directory via `GetCurrentDirectoryA`
- `clip` — read clipboard text via `OpenClipboard` / `GetClipboardData` /
  `GlobalLock`

Both are hosted-only; clipboard and CWD are not admitted or sealed.

Scratch (gitignored): `.vaa-exercises/t14-clipboard-path/`.

## CLAIM

| Surface | Claim |
|---|---|
| Hosted clipboard/path tool | **Not admitted**; not sealed |
| Skill leaves / SemASM seals | none (system capability not admitted) |

## Commands run

```text
vaa validate .vaa-exercises/t14-clipboard-path/main.vaa.toml
# → ok=True  (no network, no other rejected capability)

vaa build .vaa-exercises/t14-clipboard-path/main.asm …
  --linker-arg /DEFAULTLIB:kernel32.lib
  --linker-arg /DEFAULTLIB:user32.lib

clip_path.exe cwd
# → cwd: D:\...\t14-clipboard-path\out

clip_path.exe clip
# → clipboard: PgBouncer
```

## What helped

- `GetCurrentDirectoryA(DWORD nBufLen, LPSTR lpBuf)` — correct arg order
  after initial AV debug (rcx=nBufLen, rdx=lpBuf).
- `OpenClipboard(NULL)` → `GetClipboardData(CF_TEXT)` → `GlobalLock`
  pattern works with no hwnd when the executable runs as a console tool.

## Friction

| # | Symptom | Class | Evidence |
|---|---|---|---|
| 1 | `cwd` mode AV | agent-mistake | `GetCurrentDirectoryA` args swapped (`xchg ecx,edx`); fixed to `mov ecx,512 / lea rdx,[cwdbuf]` |

## Outcomes

- Both modes run and produce correct output.

## Follow-ups

- [ ] `set` mode: write text into clipboard via `SetClipboardData`
- [ ] Support absolute path normalization (optional)
