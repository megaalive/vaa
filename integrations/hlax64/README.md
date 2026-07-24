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

Runtime build/generate/suite chips are not wired yet; load + validate only.
