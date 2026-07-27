# Honesty charter (VAA + SemASM agent surface)

Canonical claim boundary for any agent, skill, or controller that drives VAA's
harness. If a prompt, README, or skill contradicts this file, **this file wins**.
The goal is narrow and durable: never let VAA/SemASM overclaim.

## Non-negotiable claims

1. **Agents are proposers only; SemASM is the verifier.** An agent edits a
   candidate; acceptance is decided by SemASM + VAA evidence, never by the
   agent's own reasoning or by matching stderr text.
2. **Controllers parse stdout JSON only.** stderr is human noise. Never derive a
   status, class, or seal from stderr, log lines, or exit-code guessing when a
   JSON envelope is available.
3. **Allowed shapes = admitted leaves.** Gate via
   `vaa admit --leaf … --target …` / [`admit_leaf`](../src/semasm/admission.rs)
   against the frozen SemASM snapshot
   ([`fixtures/semasm/capabilities-snapshot.json`](../fixtures/semasm/capabilities-snapshot.json)).
   [`schemas/agent-leaf-allowlist.json`](../schemas/agent-leaf-allowlist.json)
   is a discovery/freeze **mirror** of those `leaf_names` — not a second skill
   list to memorize. Anything outside admission — a new shape, free-form `.S`,
   "fix my assembly" — is **declined**, not attempted.
   **SemASM `agent verify` success on a recognized name is not admission**
   (e.g. clinic `min_i64` may verify while `vaa admit` declines — see
   [`exercises/e01-leaf-clinic.md`](exercises/e01-leaf-clinic.md)).
4. **`verified_under_preconditions` (VUP) is not unconditional `verified`.**
   Report it as VUP. Only leaves with `strict_verified_ok: true` may be called
   plainly "verified".
5. **Stub / `VAA_BIN` dry-runs are not evidence.** The hermetic adapter dry-run
   (`scripts/tests/stub_vaa.py`) proves the *contract shape*, not that any
   assembly was verified. Never present a dry-run as a real seal.
6. **Pack pin ≠ tip pin.** The HlaX64 / EchoAsm pack SemASM pin and
   `SEMASM_TIP_SHA` are different things. Do not conflate them in agent prompts
   or claim pack behavior from a tip result (or vice versa).

## What is actually proven (as of this charter)

| Surface | Reality |
|---|---|
| x86_64 Win64 / SysV via NASM | Admitted leaves are CI-proven (`agent-harness-gates`, corpus sweep); inventory via `vaa admit --list`. |
| AArch64 via GAS | CI-proven end-to-end (`agent-harness-gates-gas-aarch64`, qemu). |
| RISC-V64 via GAS | **Dialect/flavor only.** Capability = `Unknown` / fail-closed. No agent-verify claim. |
| Generator-repair mode | Exists and tested, but **not** the skill v1 path. |
| Live model (`--live`) | Out of scope for the agent surface. |

## Forbidden phrases (in skills, prompts, docs)

Do not write or imply any of:

- "proven safe" / "formally verified memory safety"
- "verifies any assembly" / "handles arbitrary asm"
- "SemASM guarantees the program is correct"
- "AI-generated and verified secure"
- calling VUP results plainly "verified"
- presenting a dry-run / stub run as sealed evidence

## Not an MCP product

The agent surface is a **Cursor/Codex project skill** plus the reference adapter
[`scripts/agent_harness_adapter.py`](../scripts/agent_harness_adapter.py). There
is intentionally **no MCP server** and **no HTTP API** for the harness. See
[`docs/agent-playbook.md`](agent-playbook.md) for the happy path and the decline
path.

## Fluent surface roadmap

Product sequencing (capability registry → work packet → target profile →
repair feedback → later stdio/author/optimize) lives in
[`docs/fluent-agent-surface.md`](fluent-agent-surface.md). That roadmap must
not contradict this charter.
