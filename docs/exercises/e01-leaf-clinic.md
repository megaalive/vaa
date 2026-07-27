# Friction log — Exercise E01 — Leaf clinic from intent

- **Date:** 2026-07-27
- **Host:** Windows
- **Target:** `x86_64-pc-windows-msvc`
- **Agent / human:** agent (local debug `vaa` / `semasm`)

## Intent

From a plain-language intent (**signed min of two i64**), author a leaf **without
copying a fixture**, and walk naming → behavioral feedback → repair → admit.

Scratch tree (gitignored): `.vaa-exercises/e01-leaf-clinic/`.

## Commands run

```text
# 1 wrong synonym name
semasm agent verify smaller_i64.asm smaller_i64.sem.toml …  # UNSUPPORTED_SHAPE
# 2 recognized name, wrong body (cmovg / max)
semasm agent verify min_i64_wrong.asm min_i64.sem.toml …    # behavior_failed
# 3 repaired (cmovl)
semasm agent verify min_i64.asm min_i64.sem.toml …          # verified
vaa admit --leaf min_i64 --target x86_64-pc-windows-msvc    # decline
vaa admit --leaf max_i64 --target x86_64-pc-windows-msvc    # admitted
vaa validate min_i64.vaa.toml
```

## What helped

- SemASM `UNSUPPORTED_SHAPE` lists recognized binary tokens (`min_*` / `max_*`).
- Wrong-body `behavior_failed` cases show expected vs observed (actionable repair).
- Repaired `min_i64` reaches SemASM `verified` (8/8 vectors).
- Contrast with `max_i64` admission makes the dual-gate story concrete.

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | Synonym `smaller_i64` → `UNSUPPORTED_SHAPE` despite correct i64,i64→i64 types | honesty / agent-mistake | Name must carry `min` token |
| 2 | SemASM `verified` for `min_i64` but `vaa admit` → decline | honesty | Snapshot lists `max_i64`, not `min_i64` |
| 3 | Agents may conflate “SemASM verified” with “skill admitted” | honesty | Fixed: decline JSON/terminal `hint` |
| 4 | Harness `loop-direct` is for admitted leaves; clinic leaf stays outside skill path | honesty | Expected |

## Outcomes

- Naming clinic: **pass** (wrong → hint → rename).
- Behavioral clinic: **pass** (wrong body → counterexamples → repair → verified).
- Admission: **`min_i64` decline**; **`max_i64` admit** (contrast).
- Skill seal claim for this clinic leaf: **none** (not in snapshot).

## Follow-ups

- [x] Docs: this friction log; complete ladder E01–E05
- [x] Tooling: `vaa admit` decline hint (SemASM verify ≠ skill admit)
- [ ] Non-goal: do not silently admit every SemASM-verifiable name into the snapshot
- [x] Optional: inventory “verifiable but not admitted” → [verifiable-vs-admitted.md](verifiable-vs-admitted.md)
