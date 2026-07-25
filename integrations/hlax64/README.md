# HlaX64 instance pack

First external-generator integration pack for the VAA verified-repair bridge.

VAA core stays generator-agnostic. This directory owns HlaX64-specific lock,
spec, cases, suites, and agent rules. Adding another generator means a new pack under
`integrations/<id>/`, not a VAA fork.

## Files

| File | Role |
|---|---|
| `stack.lock.toml` | Exact VAA / SemASM / HlaX64 revision pins |
| `generator.spec.toml` | `ExternalGeneratorSpec` for build + generation |
| `agent-rules.md` | Agent/editor repair rules (regenerate via `vaa repair rules`) |
| `CORPUS.md` | Phase A–E corpus map (plan §17) |
| `cases/<id>/` | Locked generator input + task + contract per leaf |
| `suites/*.vaa-suite.toml` | Smoke + phase suites (Win64) |

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
./scripts/run-hlax64-suite.ps1 -Suite integrations/hlax64/suites/scalar-win64.vaa-suite.toml
```

`--skip-verify` (default) ⇒ suite status Incomplete ≠ Verified. Emit ≠
SemASM verified. Pack CI also runs this generate path on `hlax64-bridge`.
