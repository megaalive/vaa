# Friction log — T08 — env-subst (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Everyday **env-subst** (cicil-1): template with a single `${NAME}`; locate `$`
via admitted leaf **`find_first_byte`**; expand with
`GetEnvironmentVariableA`; write prefix + value + suffix. Tool binary not sealed.

Scratch (gitignored): `.vaa-exercises/t08-env-subst/`.

## CLAIM

| Surface | Claim |
|---|---|
| `find_first_byte` | **Admitted** + SemASM **VUP** |
| Hosted `env_subst` | **Not admitted**; not sealed |
| Runtime `Hello, World!` | Integration evidence only |

## Commands run

```text
vaa admit --leaf find_first_byte|env_subst …
vaa validate find_first_byte.vaa.toml ; vaa validate main.vaa.toml
semasm agent verify find_first_byte.asm … --allow-execution
vaa build find_first_byte.asm … --object-only
vaa build main.asm … --extra-object out/find_first_byte.o --linker-arg …
set T08_NAME=World && env_subst.exe template.txt
# → Hello, World!
```

## What helped

- T01/T04 argv + PE link patterns; Win64 stack align discipline.
- Hosted `environment = true` validates (declarative capability).
- `suggested_linker_args` still surfaces for kernel32 imports.

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `find_first_byte` is VUP | honesty | Expected |
| 2 | `admit env_subst` decline | honesty | Tool ≠ leaf |
| 3 | Only **one** `${NAME}` supported; no nested/multiple | deferred | Cicil-1 scope |
| 4 | `replace_byte` not used yet (roadmap “find/replace”) | deferred | T08b optional compose |
| 5 | PowerShell `Start-Process -Environment` unavailable | agent-mistake | Use `cmd /c set …&&` |
| 6 | Template write escaping in PowerShell easy to get wrong | agent-mistake | Prefer `[IO.File]::WriteAllBytes` |

## Outcomes

- Leaf: **admitted VUP** + verify ok.
- Tool: **runs** (`Hello, World!` with `T08_NAME=World`).
- Seal claim for tool: **none**.

## Follow-ups

- [ ] T08b: multiple `${…}` / missing-env policy (`:`-defaults)
- [ ] T08c: compose `replace_byte` (e.g. normalize CR) alongside find
- [ ] Non-goal: do not admit `env_subst` as a skill leaf
- [x] Roadmap → `friction_logged` for cicil-1
