# VAA generator skill (external-generator repair)

You drive VAA **generator-repair**: edit an **external generator** under a
locked path policy, regenerate candidates, and accept only suite + patch
evidence. You are a **proposer**. SemASM verifies pack leaves. The claim
boundary is [`docs/HONESTY.md`](../../../docs/HONESTY.md) and
[`docs/compiler-demo.md`](../../../docs/compiler-demo.md).

This skill is **not** the admitted-leaf skill. For NASM leaf repair use
[`.cursor/skills/vaa-harness/SKILL.md`](../vaa-harness/SKILL.md). Do not merge
the two allowlists.

VAA core stays generator-agnostic. HlaX64 is the first instance pack
([`integrations/hlax64/`](../../../integrations/hlax64/)), not a hardcoded
compiler product. EchoAsm is a second pack. Do not teach `hlax64` as the only
verb.

## Hard rules

1. **Mode is generator-repair.** Prepare/submit with `--mode generator-repair`
   (or the adapter `loop-generator` / `prepare-generator` / `submit-generator`).
   Never edit `candidate.asm` / generated assembly.
2. **Edit only pack-allowed generator paths.** For HlaX64 see
   [`integrations/hlax64/agent-rules.md`](../../../integrations/hlax64/agent-rules.md):
   `src/HlaX64.Compiler/Abi/**`, `Ir/**`, `src/HlaX64.Backend.Nasm/**`,
   `tests/HlaX64.Compiler.Tests/**`. Forbidden: pack `cases/**` authority files,
   `stack.lock.toml`, contracts, vectors, evidence files, SemASM sources.
3. **Parse stdout JSON only.** stderr is noise. After prepare, read
   `repair-packet.json` / workspace packet. After submit, read
   `patch_evidence_path` and `class`.
4. **Triage before blaming the compiler.** `vaa generator triage`. Incomplete /
   `verified_under_preconditions` is **not** a generator defect by itself.
5. **Map `class` → action:**
   - `accepted` → stop. Report suite Accepted + `vaa patch evidence-verify`
     Accepted. That is authority — not agent self-report.
   - `policy_blocked` → stop. You touched a forbidden path. Do not retry by
     editing authority files.
   - `violated_repairable` → edit generator source on allowed paths only;
     rebuild; regenerate; resubmit.
   - `toolchain_retryable` → stop and report. Do not silently retry.
   - `failed` / `incomplete` / other → stop and report. Never promote to success.
6. **VUP ≠ verified.** Memory pack suites that land
   `verified_under_preconditions` must be reported as VUP.
7. **Dry-runs are not evidence.** `scripts/tests/stub_vaa.py` / `VAA_BIN` adapter
   dry-runs prove the JSON contract only.
8. **Never lock.** `vaa author lock` is human CLI only.
9. **Do not seal hosted tools.** HlaX64 `examples/tools/**` is out of this skill.
   Decline OS / UEFI / “verify the compiler”.

## Forbidden phrases

Never write or imply: "compiler verified", "proven safe", "formally verified
memory", "HlaX64 is SemASM verified", "verifies any assembly", or calling a VUP
result plainly "verified".

## Happy path

See [`docs/generator-playbook.md`](../../../docs/generator-playbook.md).
Short form (committed SysV unsigned-compare repair evidence):

```bash
vaa repair verify fixtures/repair/hlax64-min-usize-sysv-live/repair-packet.json
vaa patch evidence-verify fixtures/repair/hlax64-min-usize-sysv-live/patch-evidence.json

python scripts/agent_harness_adapter.py loop-generator \
  --repair-packet fixtures/repair/hlax64-min-usize-sysv-live/repair-packet.json \
  --workspace .vaa/harness/gen-demo \
  --suite-evidence fixtures/repair/echoasm-passthrough/suite-evidence.accepted.json
```

Live Gate (toolchain required; not a dry-run):

```powershell
$env:HLAX64_ROOT = "<path-to-hlax64>"
./scripts/run-hlax64-suite.ps1 -Gate `
  -Suite integrations/hlax64/suites/scalar-i64-win64.vaa-suite.toml
```

## Decline path

Decline (do not attempt) when the user asks to:

- edit emitted `candidate.asm` instead of generator source;
- repair arbitrary / unadmitted leaf assembly (send them to `vaa-harness` +
  `vaa admit`);
- seal `examples/tools/**` or a hosted program because a leaf verified;
- weaken contracts, vectors, or `stack.lock.toml` to make a suite pass;
- treat Incomplete/VUP as a compiler bug without `vaa generator triage`.
