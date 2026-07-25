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
# Phase A scalar (Win64)
./scripts/run-hlax64-suite.ps1 -Suite integrations/hlax64/suites/scalar-win64.vaa-suite.toml
# Phase E calls / data (Win64)
./scripts/run-hlax64-suite.ps1 -Suite integrations/hlax64/suites/calls-data-win64.vaa-suite.toml
# SysV live (System V AMD64 — emits rdi/rsi arg registers)
./scripts/run-hlax64-suite.ps1 -Suite integrations/hlax64/suites/scalar-sysv.vaa-suite.toml
```

`--skip-verify` (default) ⇒ suite status Incomplete ≠ Verified. Emit ≠
SemASM verified. The SysV suite generates real System V assembly via
`generator.sysv.spec.toml`, but SysV **Gate** evidence on Linux (a run
without `--skip-verify`) is not yet claimed. Pack CI also runs the scalar,
Phase E, and SysV generate paths on `hlax64-bridge`.

## Gate pack suite (SemASM Verified)

Win64 scalar pack path can run SemASM Gate (generate → verify → seal):

```powershell
$env:HLAX64_ROOT = "<path-to-hlax64>"
# Requires `semasm` on PATH. Auto-provisions a practice seal key if unset.
./scripts/run-hlax64-suite.ps1 -Gate `
  -Suite integrations/hlax64/suites/scalar-win64.vaa-suite.toml
```

Expect suite status **Accepted** with cases **Verified**. Practice seal ≠
trust root.

## Gate pack suite SysV (Linux)

```bash
export HLAX64_ROOT="<path-to-hlax64>"
# Requires Linux host + semasm on PATH (afaa19d+: SysV framed epilogue).
./scripts/run-hlax64-suite.sh --gate \
  --suite integrations/hlax64/suites/scalar-sysv.vaa-suite.toml
```

CI job `hlax64-pack-sysv-gate` runs this path. SysV emit uses `rdi`/`rsi`;
Gate Accepted with `min_usize_sysv` / `max_usize_sysv` Verified.

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
