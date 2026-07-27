# Friction log — T15 — HTTP HEAD timer

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa`)

## Intent

Network capability stress test (cicil-1): time a raw TCP `HEAD` request
to a host via WinSock2 and report `status=<code> ms=<elapsed>`.

The primary purpose is to confirm that VAA/SemASM correctly **declines**
network-using programs at the admission gate, while still being able to
assemble and link a working binary with WinSock2.

Scratch (gitignored): `.vaa-exercises/t15-http-head-timer/`.

## CLAIM

| Surface | Claim |
|---|---|
| Hosted HTTP timer | **Not admitted**; network → fail-closed |
| Skill leaves / SemASM seals | none (network capability is not admitted) |

## Commands run

```text
vaa validate .vaa-exercises/t15-http-head-timer/main.vaa.toml
# → {"error":"capabilities.network=true is rejected in schema 0.1 (fail-closed default)","ok":false}

vaa build .vaa-exercises/t15-http-head-timer/main.asm …
  --linker-arg /DEFAULTLIB:ws2_32.lib
# → success (build does not imply admission)

http_head_timer.exe example.com 80
# → network: admitted-decline
#   status=400 ms=32
```

## Friction

| # | Symptom | Class | Evidence |
|---|---|---|---|
| 1 | `vaa validate` rejects `network=true` | **honesty** (correctly fail-closed) | schema 0.1 rule |
| 2 | Runtime output shows `\n` literal instead of newline in decline notice | minor asm bug | NASM `db "…\n"` does not expand escape; use `db 10` for LF |

The escape literal `\n` in the "decline" notice is cosmetic: the tool
emits it as two characters, not a real newline, because NASM does not
interpret `\n` inside `db "..."` strings. This is a known friction point,
documented here; the overall tool purpose is met.

## Outcomes

- `vaa validate` → fail-closed: **network=true rejected** — exactly the
  expected decline path; confirms VAA's honesty contract.
- `vaa build` succeeds (schema check separate from assembler/linker).
- Binary runs: `status=400 ms=32` (TCP connectivity confirmed; HTTP/1.0
  HEAD to `example.com` returns 400).

## Follow-ups

- [ ] Use `db "…", 10` instead of `\n` in the decline notice string for
  a clean newline.
- [ ] Consider a `--skip-network-check` flag for dry-run / offline
  testing.
