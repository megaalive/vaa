# HlaX64 instance pack

First external-generator integration pack for the VAA verified-repair bridge.

VAA core stays generator-agnostic. This directory owns HlaX64-specific lock,
spec, cases, suites, and agent rules. Adding another generator means a new pack under
`integrations/<id>/`, not a VAA fork.

## Files

| File | Role |
|---|---|
| `stack.lock.toml` | Exact VAA / SemASM / HlaX64 revision pins |
| `generator.spec.toml` | `ExternalGeneratorSpec` — Win64 emit (build + generation) |
| `generator.sysv.spec.toml` | `ExternalGeneratorSpec` — SysV emit (`--target linux-x64-sysv`) |
| `agent-rules.md` | Agent/editor repair rules (regenerate via `vaa repair rules`) |
| `CORPUS.md` | Phase A–E corpus map (plan §17) |
| `cases/<id>/` | Locked generator input + task + contract per leaf |
| `suites/*.vaa-suite.toml` | Smoke + phase suites (Win64) + `scalar-sysv` (SysV live) |

`repository.path` is relative to this pack directory. For sibling checkouts
use `../../../hlax64` from `integrations/hlax64/` or pass `--repo <path>`.

## Commands

P0: `validate-lock`, `validate-spec`, `check-repo`, `identity`,
`generate`, `vaa generator-run` (alias `compiler-run`).

P1: `vaa suite validate|run`, `vaa patch evidence-build|verify`,
`vaa generator check-paths|triage`.

P2: `vaa repair export|verify|rules`, `vaa generator diagnostics|map-join`.

P3: expand `cases/` + phase suites per `CORPUS.md`. Pack cases ≠ live Gate
Verified until generation + SemASM evidence exists.

## Validate suites (no live generator required)

```text
vaa suite validate integrations/hlax64/suites/scalar-win64.vaa-suite.toml
vaa suite check-parity integrations/hlax64/suites/scalar-win64.vaa-suite.toml
vaa suite validate integrations/hlax64/suites/backend-win64.vaa-suite.toml
```

## Live generate suite (HlaX64 checkout required)

```powershell
$env:HLAX64_ROOT = "<path-to-hlax64>"
# Phase A scalar usize pair (Win64)
./scripts/run-hlax64-suite.ps1 -Suite integrations/hlax64/suites/scalar-win64.vaa-suite.toml
# Phase A named i64 (Win64): return/add/sub/min/max/abs_i64
./scripts/run-hlax64-suite.ps1 -Suite integrations/hlax64/suites/scalar-i64-win64.vaa-suite.toml
# Phase B named loops/stack (Win64)
./scripts/run-hlax64-suite.ps1 -Suite integrations/hlax64/suites/loop-stack-win64.vaa-suite.toml
# Phase C/D memory leaves (Win64) — Gate → VerifiedUnderPreconditions
./scripts/run-hlax64-suite.ps1 -Suite integrations/hlax64/suites/memory-read-win64.vaa-suite.toml
./scripts/run-hlax64-suite.ps1 -Suite integrations/hlax64/suites/memory-write-win64.vaa-suite.toml
# Phase E calls / data (Win64)
./scripts/run-hlax64-suite.ps1 -Suite integrations/hlax64/suites/calls-data-win64.vaa-suite.toml
# SysV live (System V AMD64 — emits rdi/rsi arg registers)
./scripts/run-hlax64-suite.ps1 -Suite integrations/hlax64/suites/scalar-sysv.vaa-suite.toml
./scripts/run-hlax64-suite.ps1 -Suite integrations/hlax64/suites/scalar-i64-sysv.vaa-suite.toml
./scripts/run-hlax64-suite.ps1 -Suite integrations/hlax64/suites/loop-stack-sysv.vaa-suite.toml
```

`--skip-verify` (default) ⇒ suite status Incomplete ≠ Verified. Emit ≠
SemASM verified. SysV suites generate real System V assembly via
`generator.sysv.spec.toml`; SysV **Gate** evidence is claimed on Linux CI
(`hlax64-pack-sysv-gate`). Pack CI also runs Win64 Gate for Phase A–E.

## Gate pack suite (SemASM Verified)

Win64 scalar pack path can run SemASM Gate (generate → verify → seal):

```powershell
$env:HLAX64_ROOT = "<path-to-hlax64>"
# Requires `semasm` on PATH. Auto-provisions a practice seal key if unset.
./scripts/run-hlax64-suite.ps1 -Gate `
  -Suite integrations/hlax64/suites/scalar-win64.vaa-suite.toml
# Phase A named i64 Gate (6 Verified) — requires SemASM 566ca8e+
# (builtin.pure_int.binary_i64 / unary_i64 oracles).
./scripts/run-hlax64-suite.ps1 -Gate `
  -Suite integrations/hlax64/suites/scalar-i64-win64.vaa-suite.toml
# Phase B loops/stack Gate (4 Verified) — requires SemASM 3cae1e1+
./scripts/run-hlax64-suite.ps1 -Gate `
  -Suite integrations/hlax64/suites/loop-stack-win64.vaa-suite.toml
# Phase C/D memory Gate — VerifiedUnderPreconditions (≠ unconditional Verified)
./scripts/run-hlax64-suite.ps1 -Gate `
  -Suite integrations/hlax64/suites/memory-read-win64.vaa-suite.toml
./scripts/run-hlax64-suite.ps1 -Gate `
  -Suite integrations/hlax64/suites/memory-write-win64.vaa-suite.toml
# Phase E calls/data Gate (5 Verified) — requires SemASM ecde423+
./scripts/run-hlax64-suite.ps1 -Gate `
  -Suite integrations/hlax64/suites/calls-data-win64.vaa-suite.toml
# Negative suite must Reject (locked wrong min_i64)
./scripts/ci-negative-suite-reject.ps1
```

Expect suite status **Accepted** with cases **Verified** (or
`VerifiedUnderPreconditions` for memory leaves). Practice seal ≠
trust root. Generator emit also writes `candidate.map.json` (join with
`vaa generator map-join <map> --line N`; HlaX64 `62c9f22+` maps IR to
distinct NASM opcode lines).

## Gate pack suite SysV (Linux)

```bash
export HLAX64_ROOT="<path-to-hlax64>"
# Requires Linux host + semasm on PATH (afaa19d+: SysV framed epilogue).
./scripts/run-hlax64-suite.sh --gate \
  --suite integrations/hlax64/suites/scalar-sysv.vaa-suite.toml
# Named i64 + loop/stack SysV (SemASM ecde423+)
./scripts/run-hlax64-suite.sh --gate \
  --suite integrations/hlax64/suites/scalar-i64-sysv.vaa-suite.toml
./scripts/run-hlax64-suite.sh --gate \
  --suite integrations/hlax64/suites/loop-stack-sysv.vaa-suite.toml
```

CI job `hlax64-pack-sysv-gate` runs these paths. SysV emit uses `rdi`/`rsi`;
Gate Accepted with Verified cases (usize pair + named i64 + loop/stack).

## Live repair evidence

`fixtures/repair/hlax64-min-usize-sysv-live/` records the constrained
repair exercise for a controlled SysV unsigned-compare defect:

```text
vaa repair verify fixtures/repair/hlax64-min-usize-sysv-live/repair-packet.json
vaa patch evidence-verify fixtures/repair/hlax64-min-usize-sysv-live/patch-evidence.json
```

The broken evidence branch returned max instead of min for 4/6 vectors.
Only actual HlaX64 ABI/test paths were editable; authority files were
untouched. Post-repair `scalar-sysv` was Accepted (2/2 Verified).
