# EchoAsm — second generator pack (universality smoke)
#
# Proves VAA bridge schemas/CLI are generator-agnostic: a new pack under
# `integrations/<id>/` is enough; no VAA core fork. EchoAsm is **not** a
# production generator — it copies the locked input bytes to `candidate.asm`.
#
# Honesty: smoke wiring ≠ Gate Verified. HlaX64 remains the first real
# instance. EchoAsm ≠ CryptOpt; Incomplete ≠ Verified.

## Layout

| Path | Role |
|---|---|
| `stack.lock.toml` | Pins VAA/SemASM + `generators.echoasm` (this pack tree) |
| `generator.spec.toml` | Build/generation/path policy for the echo tool |
| `tools/echoasm.ps1` | Windows generator (copy input → output) |
| `tools/echoasm.sh` | POSIX twin |
| `cases/passthrough/` | One locked case |
| `suites/smoke.vaa-suite.toml` | Suite smoke |

## Commands

```text
vaa generator validate-spec integrations/echoasm/generator.spec.toml
vaa generator validate-lock integrations/echoasm/stack.lock.toml
vaa suite validate integrations/echoasm/suites/smoke.vaa-suite.toml
vaa suite check-parity integrations/echoasm/suites/smoke.vaa-suite.toml

# Live generate (no SemASM required for universality smoke).
# Prefer absolute --generator/--input/--output on Windows.
vaa generator generate integrations/echoasm/generator.spec.toml \
  --generator <abs>/integrations/echoasm/tools/echoasm.cmd \
  --input <abs>/integrations/echoasm/cases/passthrough/input.asm \
  --output <abs>/target/echoasm-out/candidate.asm \
  --check-deterministic
```

`tools/echoasm.ps1` / `echoasm.sh` are human/agent conveniences; the locked
Windows generator identity is `tools/echoasm.cmd`. Smoke wiring ≠ Gate
Verified.
