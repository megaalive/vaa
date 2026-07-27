# Friction log — T08 — env-subst (Win64)

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

Everyday **env-subst** (cicil-2 / T08b + T08c): template with **multiple** `${NAME}`;
locate `$` via admitted leaf **`find_first_byte`** in a loop; expand with
`GetEnvironmentVariableA`; continue on suffix. Before subst, admitted
**`replace_byte`** normalizes CR→LF. Missing env → empty OK.
Tool binary not sealed.

Scratch (gitignored): `.vaa-exercises/t08-env-subst/` (+ `OUTCOME.txt`).

## CLAIM

| Surface | Claim |
|---|---|
| `find_first_byte` | **Admitted** + SemASM **VUP** |
| `replace_byte` | **Admitted** + SemASM **VUP** (`verified_under_preconditions`) |
| Hosted `env_subst` | **Not admitted**; not sealed |
| Runtime normalize+subst | Integration only — see scratch `OUTCOME.txt` |

## Commands run

```text
vaa admit --leaf find_first_byte|replace_byte|env_subst …
vaa validate find_first_byte.vaa.toml ; vaa validate replace_byte.vaa.toml ; vaa validate main.vaa.toml
vaa build replace_byte.asm|find_first_byte.asm --object-only --target x86_64-pc-windows-msvc --output-dir out
vaa build main.asm … --extra-object out/replace_byte.o --extra-object out/find_first_byte.o --linker-arg …
# template.txt = Hello,<CR>${A} and ${B}!
set A=X&& set B=Y&& env_subst.exe template.txt
# → Hello,<LF>X and Y!   (CR normalized then subst)
```

## What helped

- T01/T04 argv + PE link patterns; Win64 stack align discipline.
- Hosted `environment = true` validates (declarative capability).
- Compose two admitted buffer leaves (`replace_byte` then `find_first_byte`).

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | `find_first_byte` / `replace_byte` are VUP | honesty | Expected |
| 2 | `admit env_subst` decline | honesty | Tool ≠ leaf |
| 3 | Only **one** `${NAME}` at cicil-1 | resolved (T08b) | Loop find/expand/continue |
| 4 | `replace_byte` not used yet | resolved (T08c / D1) | CR→LF before subst |
| 5 | PowerShell `Start-Process -Environment` unavailable | agent-mistake | Use `cmd /c set …&&` |
| 6 | Template write escaping in PowerShell easy to get wrong | agent-mistake | Prefer `[IO.File]::WriteAllBytes` |

## Outcomes

- Leaves: **admitted VUP** (`find_first_byte`, `replace_byte`) + verify ok.
- Tool: **runs** multi-subst after CR→LF (`Hello,\nX and Y!` with `A=X` `B=Y`).
- Seal claim for tool: **none**. Scratch `OUTCOME.txt`.

## Follow-ups

- [x] T08b: multiple `${…}` / missing-env → empty
- [x] T08c / D1: compose `replace_byte` (normalize CR) alongside find
- [ ] Non-goal: do not admit `env_subst` as a skill leaf
- [x] Roadmap → `friction_logged` for cicil-2
