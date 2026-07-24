# HlaX64 instance pack (stub)

First external-generator integration pack for the VAA verified-repair bridge.

VAA core stays generator-agnostic. This directory owns HlaX64-specific lock,
spec, cases, and scripts. Adding another generator means a new pack under
`integrations/<id>/`, not a VAA fork.

## Files (Milestone 0/1)

| File | Role |
|---|---|
| `stack.lock.toml` | Exact VAA / SemASM / HlaX64 revision pins |
| `generator.spec.toml` | `ExternalGeneratorSpec` for build + generation |
| `agent-rules.md` | Agent/editor repair rules (regenerate via `vaa repair rules`) |
| `suites/smoke.vaa-suite.toml` | Placeholder smoke suite for the suite runner |

`repository.path` is relative to this pack directory. For sibling checkouts
use `../../../hlax64` from `integrations/hlax64/` or pass `--repo <path>`.

P0 commands: `validate-lock`, `validate-spec`, `check-repo`, `identity`,
`generate`, and top-level `vaa generator-run` (alias `compiler-run`).
P1 commands: `vaa suite validate|run`, `vaa patch evidence-build|verify`,
`vaa generator check-paths|triage`.
P2 commands: `vaa repair export|verify|rules`, `vaa generator
diagnostics|map-join`. `agent-rules.md` is generated output — edit the
spec/commands and re-run `vaa repair rules`, do not hand-edit.
