# Generator-repair playbook (VAA + HlaX64 instance)

Bounds: [`HONESTY.md`](HONESTY.md), [`compiler-demo.md`](compiler-demo.md).
Skill: [`.cursor/skills/vaa-generator/SKILL.md`](../.cursor/skills/vaa-generator/SKILL.md).
HlaX64 path policy: [`integrations/hlax64/agent-rules.md`](../integrations/hlax64/agent-rules.md).

Leaf NASM repair stays on [`agent-playbook.md`](agent-playbook.md) + skill
`vaa-harness`. Do not use this playbook to admit extra leaf names.

Rule of thumb: **parse stdout JSON only**. Never edit generated assembly.
Acceptance is suite **Accepted** + `vaa patch evidence-verify`, not agent text.

## Happy path — committed SysV unsigned-compare repair

Controlled defect: `CompareKind.LessThanUnsigned` `jb` → `ja` on an evidence
branch; repair restored min. Fixture:
`fixtures/repair/hlax64-min-usize-sysv-live/`.

```bash
vaa repair verify fixtures/repair/hlax64-min-usize-sysv-live/repair-packet.json
vaa patch evidence-verify fixtures/repair/hlax64-min-usize-sysv-live/patch-evidence.json
```

Adapter rehearsal (policy block on an authority path, then allowed generator
path). Against a live `vaa` this exercises harness `generator-repair`; against
`stub_vaa.py` it is **not** evidence:

```bash
python scripts/agent_harness_adapter.py loop-generator \
  --repair-packet fixtures/repair/hlax64-min-usize-sysv-live/repair-packet.json \
  --workspace .vaa/harness/gen-demo \
  --suite-evidence fixtures/repair/echoasm-passthrough/suite-evidence.accepted.json
```

Expected live `vaa` final class after the allowed patch: `accepted` with
`patch_evidence_path`. Expected first step: `policy_blocked` /
`FORBIDDEN_PATH` when `--forbidden-path` is a pack authority file.

Hermetic contract check (no SemASM):

```bash
python scripts/tests/harness_adapter_dryrun.py
```

## Happy path — live pack Gate (Win64)

```powershell
$env:HLAX64_ROOT = "<path-to-hlax64>"
./scripts/run-hlax64-suite.ps1 -Gate `
  -Suite integrations/hlax64/suites/scalar-i64-win64.vaa-suite.toml
./scripts/run-hlax64-suite.ps1 -Gate `
  -Suite integrations/hlax64/suites/memory-read-win64.vaa-suite.toml
```

Report: scalar suite **Verified**; memory-read **VerifiedUnderPreconditions**.
Practice seal ≠ trust root. HlaX64 emit ≠ SemASM `verified`.

## `class` → action map

| `class` | Do |
|---|---|
| `accepted` | Stop. Report patch evidence + suite status. |
| `policy_blocked` | Stop. You edited a forbidden path. |
| `violated_repairable` | Edit **generator** source on allowed paths; regenerate; resubmit. |
| `toolchain_retryable` | Stop; report missing toolchain. |
| `failed` / `incomplete` / other | Stop; report. Run `vaa generator triage` before calling it a compiler defect. |

## Decline path

- **“Fix the emitted `.asm`”**: this skill repairs the generator, not
  `candidate.asm`. Decline.
- **“Seal hexdump / wc / my OS”**: hosted tools and kernels are not pack
  leaves. Decline. See [`compiler-demo.md`](compiler-demo.md) CLAIM template.
- **“Make VUP count as verified”**: honesty lock. Decline.
- **Leaf-only NASM repair**: use `vaa admit` + skill `vaa-harness`, not this
  playbook.

In every decline: state the boundary, point at `HONESTY.md`, and stop.
