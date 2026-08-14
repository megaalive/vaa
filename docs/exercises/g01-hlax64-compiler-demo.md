# Friction log — G01 — HlaX64 compiler demo (verified-repair)

- **Date:** 2026-08-15
- **Host:** Windows
- **Target:** pack leaves (`x86_64-unknown-linux-gnu` SysV scalar pair in the
  committed fixture; Win64 Gate commands in the operator card)
- **Agent / human:** agent implementing VAA compiler-dogfood G0–G2

## Intent

Dogfood VAA as a controller around the **existing HlaX64 compiler**, not a new
SemASM leaf and not `examples/tools/**`. Re-verify the landed SysV unsigned
compare repair (generator source only) and record the CLAIM so later compiler
work copies this pattern.

Operator card: [`../compiler-demo.md`](../compiler-demo.md).

## CLAIM

| Artifact | Status | Forbidden upgrade |
|---|---|---|
| `min_usize_sysv` / `max_usize_sysv` pack leaves | SemASM **Verified** on the repaired evidence branch; suite Accepted | “HlaX64 compiler verified” |
| Repair packet + patch evidence (committed) | `vaa repair verify` / `vaa patch evidence-verify` | dry-run / stub as seal |
| Memory pack suites | **VUP** when run (not this fixture) | plain `verified` |
| `examples/tools/**` | not in scope | any seal |
| SemASM Fb9c / new oracle | **not opened** | product-claim expansion |

This exercise does **not** start a new HlaX64 ABI mutation. The real compiler
fix already landed (`hlax64@06d1113` repair, `5379729` main regression). G01
locks the **dogfood loop**, skill hand-off, and honesty so the next mutation
can reuse it.

## Commands run

```text
# Committed evidence (no emit)
cargo run -q -- repair verify fixtures/repair/hlax64-min-usize-sysv-live/repair-packet.json
cargo run -q -- patch evidence-verify fixtures/repair/hlax64-min-usize-sysv-live/patch-evidence.json

# Adapter contract (not evidence)
python scripts/tests/harness_adapter_dryrun.py

# Live Gate (toolchain; optional on this host — see Outcomes)
# $env:HLAX64_ROOT = "<hlax64>"
# ./scripts/run-hlax64-suite.ps1 -Gate -Suite integrations/hlax64/suites/scalar-i64-win64.vaa-suite.toml
# ./scripts/run-hlax64-suite.ps1 -Gate -Suite integrations/hlax64/suites/memory-read-win64.vaa-suite.toml
```

## What helped

- Pack already has Phase A–E suites and live ABI repair fixtures (7k–7n).
- Adapter `loop-generator` + `harness_adapter_dryrun.py` already cover
  policy_blocked → accepted without SemASM.
- Separate `vaa-generator` skill keeps leaf `vaa admit` from collapsing into
  pack Gate.

## Friction

| # | Symptom | Likely class | Evidence |
|---|---|---|---|
| 1 | Leaf skill v1 declined generator-repair, so agents could not dogfood the compiler loop | tooling | `.cursor/skills/vaa-harness` (fixed: hand-off to `vaa-generator`) |
| 2 | Three SemASM pins (pack `67cba2a`, tip `0ab8004`, release `v0.5.0`) were easy to conflate | honesty | `docs/compiler-demo.md` table |
| 3 | Inventing a new ABI bug for theater would not be dogfood | honesty | reuse `hlax64-min-usize-sysv-live` |
| 4 | Live pack Gate vs SemASM `v0.5.0` needs HlaX64 + NASM + `semasm` on PATH | tooling | pin bump only if C/D stay VUP |

## Outcomes

- Leaf verify: N/A (generator path). Pack fixture claims remain as recorded in
  `fixtures/repair/hlax64-min-usize-sysv-live/README.md`.
- `vaa validate`: N/A.
- `vaa build` / link: N/A.
- Seal / admission claim made: **repair packet + patch evidence verified**.
  Pack Gate vs SemASM `v0.5.0`: `memory-read-win64` and `memory-write-win64`
  **Accepted** with all cases **VerifiedUnderPreconditions** (not promoted).
  Pack `[semasm]` locks bumped to `git:5888b3a…`. `SEMASM_TIP_SHA` unchanged
  (workflow-only). Not a compiler-wide seal.

## Follow-ups

- [x] Pack `stack.lock.toml` `[semasm]` → `v0.5.0` SHA after
      `memory-read-win64` / `memory-write-win64` Gate stayed VUP
- [ ] `SEMASM_TIP_SHA` via `semasm-tip-bump.yml` only (not this wave)
- [x] Docs only: compiler-demo + generator skill + this log
- [x] Non-goal: Fb9c, Glue-0, auto-admit `min_i64` on the leaf skill
