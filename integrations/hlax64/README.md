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

`repository.path` is relative to this pack directory. For sibling checkouts
use `../../../hlax64` from `integrations/hlax64/` or pass `--repo <path>`.

P0 commands: `validate-lock`, `validate-spec`, `check-repo`, `identity`,
`generate`, and top-level `vaa generator-run` (alias `compiler-run`).
Suite/patch chips are P1+.
