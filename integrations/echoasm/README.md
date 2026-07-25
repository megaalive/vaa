# EchoAsm — second generator pack (universality smoke)
#
# Proves VAA bridge schemas/CLI are generator-agnostic: a new pack under
# `integrations/<id>/` is enough; no VAA core fork. EchoAsm is **not** a
# production generator — it copies the locked input bytes to `candidate.asm`.
#
# Honesty: smoke wiring ≠ Gate Verified. HlaX64 remains the first real
# instance. EchoAsm ≠ CryptOpt; Incomplete ≠ Verified.

## Layout

| File | Role |
|---|---|
| `stack.lock.toml` | Pins VAA/SemASM + `generators.echoasm` (this pack tree) |
| `generator.spec.toml` | Build/generation/path policy for the echo tool |
| `tools/echoasm.ps1` | Windows generator (copy input → output) |
| `tools/echoasm.sh` | POSIX twin |
| `cases/passthrough/` | Universality smoke case (not Gate) |
| `cases/load_byte0_echo/` | Locked `load_byte0` asm — Gate Verified via SemASM |
| `cases/store_byte0_echo/` | Locked `store_byte0` asm — Gate Verified via SemASM |
| `cases/return_i64_echo/` | Locked `return_i64` asm — Gate Verified (scalar depth) |
| `cases/add_i64_echo/` | Locked `add_i64` asm — Gate Verified (scalar depth) |
| `cases/sum_range_echo/` | Locked `sum_range` asm — Gate Verified (Phase B) |
| `cases/countdown_loop_echo/` | Locked `countdown_loop` asm — Gate Verified (Phase B) |
| `cases/stack_local_i64_echo/` | Locked stack-local asm — Gate Verified (Phase B) |
| `cases/forced_register_spill_echo/` | Locked spill-pressure asm — Gate Verified (Phase B) |
| `cases/internal_function_call_echo/` | Locked call helper asm — Gate Verified (Phase E) |
| `cases/global_rodata_echo/` | Locked global constant asm — Gate Verified (Phase E) |
| `suites/smoke.vaa-suite.toml` | Suite smoke |
| `suites/gate-load-byte0-win64.vaa-suite.toml` | Second-generator Gate (load only) |
| `suites/gate-concrete-win64.vaa-suite.toml` | Second-generator Gate (load + store) |
| `suites/gate-scalar-i64-win64.vaa-suite.toml` | Second-generator Gate (return_i64 + add_i64) |
| `suites/gate-phase-b-loops-win64.vaa-suite.toml` | Second-generator Gate (sum_range + countdown) |
| `suites/gate-phase-b-stack-win64.vaa-suite.toml` | Second-generator Gate (stack local + spill) |
| `suites/gate-phase-e-calls-win64.vaa-suite.toml` | Second-generator Gate (call + rodata) |

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

# Gate (SemASM Verified) — concrete cells + scalar via copy-generator.
vaa suite run integrations/echoasm/suites/gate-concrete-win64.vaa-suite.toml \
  --repo . --allow-execution --skip-repo-guard
vaa suite run integrations/echoasm/suites/gate-scalar-i64-win64.vaa-suite.toml \
  --repo . --allow-execution --skip-repo-guard
```

`tools/echoasm.ps1` / `echoasm.sh` are human/agent conveniences; the locked
Windows generator identity is `tools/echoasm.cmd`. Smoke wiring ≠ Gate
Verified; Gate suite claims Verified only with SemASM evidence.
Practice seal ≠ trust root. EchoAsm ≠ CryptOpt.
