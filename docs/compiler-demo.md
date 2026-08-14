# Compiler demo (HlaX64 pack Gate)

Operator card for using VAA as a **verified-repair controller around an
external generator**. The first instance is HlaX64. This is not a claim that
the compiler, the generated program, or HlaX64 `examples/tools/**` is SemASM
`verified`.

Canonical claim boundary: [`HONESTY.md`](HONESTY.md). Path policy:
[`../integrations/hlax64/agent-rules.md`](../integrations/hlax64/agent-rules.md).
Agent skill for this path:
[`.cursor/skills/vaa-generator/SKILL.md`](../.cursor/skills/vaa-generator/SKILL.md)
(leaf skill v1 stays on admitted NASM leaves only).

## Allowed claim

> An agent or human edits **HlaX64 generator source** on allowed paths. VAA
> rebuilds the generator, regenerates locked pack inputs, and SemASM verifies
> **pack leaves**. Suite **Accepted** plus `vaa patch evidence-verify` is
> authority. HlaX64 emit / `-Wverify` ≠ SemASM `verified`.
> `verified_under_preconditions` ≠ `verified`. Hosted tools stay runtime-only.

## Three pins (do not conflate)

| Pin | Where | Role |
|---|---|---|
| **Pack** | [`integrations/hlax64/stack.lock.toml`](../integrations/hlax64/stack.lock.toml) `[semasm]` (EchoAsm lock stays aligned) | Live `vaa suite run -Gate` / pack CI. Bump only after C/D memory suites remain **VerifiedUnderPreconditions** (not promoted). |
| **Gate CI tip** | `.github/workflows/ci.yml` `SEMASM_TIP_SHA` | Agent-harness / leaf corpus jobs. Bump **only** via [`semasm-tip-bump.yml`](../.github/workflows/semasm-tip-bump.yml) after owner jobs are green. Never hand-edit as a “sync” with the pack pin. |
| **Release** | `SEMASM_RELEASE_TAG` in release workflows | Packaged VAA smokes against a published SemASM archive (`v0.5.0`). |

Pack may lag tip if a tip promote would flip memory leaves from VUP to a
false unconditional `verified`. Release may lead both.

## Happy-path Gate (Win64)

Requires a sibling HlaX64 checkout (`HLAX64_ROOT` or `../hlax64` from this
repo), `semasm` on `PATH`, NASM, and a practice seal key
(`scripts/ci-gate-sign-setup.ps1` if `VAA_SEAL_SIGNING_KEY` is unset).

Strict **Verified** scalar suite (Phase A named i64):

```powershell
$env:HLAX64_ROOT = "<path-to-hlax64>"
./scripts/run-hlax64-suite.ps1 -Gate `
  -Suite integrations/hlax64/suites/scalar-i64-win64.vaa-suite.toml
```

Expect suite **Accepted** with cases **Verified**. Practice seal ≠ trust root.

Symbolic-length memory suite (**VUP only**):

```powershell
./scripts/run-hlax64-suite.ps1 -Gate `
  -Suite integrations/hlax64/suites/memory-read-win64.vaa-suite.toml
```

A pack-pin bump to SemASM `v0.5.0` (`5888b3a`) is allowed only when this
honesty still holds. Re-checked 2026-08-15 on this host: `memory-read-win64`
and `memory-write-win64` Gate **Accepted** with all cases
**VerifiedUnderPreconditions** against workspace SemASM `v0.5.0`. Pack locks
were bumped. `SEMASM_TIP_SHA` was **not** hand-edited (use
`semasm-tip-bump.yml`).

Repair rehearsal (no live emit required):

```text
vaa repair verify fixtures/repair/hlax64-min-usize-sysv-live/repair-packet.json
vaa patch evidence-verify fixtures/repair/hlax64-min-usize-sysv-live/patch-evidence.json
python scripts/agent_harness_adapter.py loop-generator \
  --repair-packet fixtures/repair/hlax64-min-usize-sysv-live/repair-packet.json \
  --workspace .vaa/harness/gen-demo \
  --suite-evidence fixtures/repair/echoasm-passthrough/suite-evidence.accepted.json
```

`loop-generator` against a stub/`VAA_BIN` dry-run is **not** evidence. See
[`generator-playbook.md`](generator-playbook.md).

## CLAIM template (copy into friction logs)

Fill every dogfood exercise. Never collapse rows.

| Artifact | Status to record | Forbidden upgrade |
|---|---|---|
| Pack leaf (scalar / concrete cell) | `verified` + suite Accepted + patch evidence Accepted | “compiler verified” |
| Pack leaf (symbolic-length memory) | `verified_under_preconditions` | plain `verified` |
| HlaX64 generator binary | identity digest only | SemASM `verified` |
| `examples/tools/**` | runtime / `vaa validate` only | skill seal |
| OS / UEFI / kernel | out of scope | any seal |

Friction log for this demo: [`exercises/g01-hlax64-compiler-demo.md`](exercises/g01-hlax64-compiler-demo.md).

## SemASM chips

Do **not** open Formal Bounds **Fb9c**, new oracles, or Glue-0 ABI program lint
because the compiler demo exists. Open a SemASM chip only when this loop hits a
wall (Gate regression after a pin, `UNSUPPORTED_SHAPE` on an emit you must
seal, or an ABI finding the existing analyzer already knows). Record the wall
in the friction log first.

## Non-goals

- Demo OS / UEFI / kernel modules (deferred in SemASM).
- Sealing HlaX64 `examples/tools/**`.
- Merging leaf skill v1 and generator-repair into one allowlist.
- MCP / HTTP.
- Weakening `UNSUPPORTED_SHAPE` so hosted parsers “pass”.
