# Friction log — T08 — env-subst (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Everyday **env-subst** (cicil-2 / T08b): template with **multiple** `${NAME}`;
locate `$` via admitted leaf **`find_first_byte`** in a loop; expand with
`GetEnvironmentVariableA`; continue on suffix. Missing env → empty OK.
Tool binary not sealed.

Scratch (gitignored): `.vaa-exercises/t08-env-subst/` (+ `OUTCOME.txt`).

## CLAIM

| Surface | Claim |
|---|---|
| `find_first_byte` | **Admitted** + SemASM **VUP** |
| Hosted `env_subst` | **Not admitted**; not sealed |
| Runtime `Hello, X and Y!` | Integration only — see scratch `OUTCOME.txt` |

## Commands run

```text
vaa admit --leaf find_first_byte|env_subst …
vaa validate find_first_byte.vaa.toml ; vaa validate main.vaa.toml
semasm agent verify find_first_byte.asm … --allow-execution
vaa build find_first_byte.asm --object-only --target x86_64-pc-windows-msvc --output-dir out
vaa build main.asm … --extra-object out/find_first_byte.o --linker-arg …
set A=X&& set B=Y&& env_subst.exe template.txt
# → Hello, X and Y!
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
| 3 | Only **one** `${NAME}` at cicil-1 | resolved (T08b) | Loop find/expand/continue |
| 4 | `replace_byte` not used yet (roadmap “find/replace”) | deferred | T08b optional compose |
| 5 | PowerShell `Start-Process -Environment` unavailable | agent-mistake | Use `cmd /c set …&&` |
| 6 | Template write escaping in PowerShell easy to get wrong | agent-mistake | Prefer `[IO.File]::WriteAllBytes` |

## Outcomes

- Leaf: **admitted VUP** + verify ok.
- Tool: **runs** multi-subst (`Hello, X and Y!` with `A=X` `B=Y`).
- Seal claim for tool: **none**. Scratch `OUTCOME.txt`.

## Follow-ups

- [x] T08b: multiple `${…}` / missing-env → empty
- [ ] T08c: compose `replace_byte` (e.g. normalize CR) alongside find
- [ ] Non-goal: do not admit `env_subst` as a skill leaf
- [x] Roadmap → `friction_logged` for cicil-2
