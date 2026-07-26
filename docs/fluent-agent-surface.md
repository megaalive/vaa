# Fluent Assembly Agent Surface v2

Durable roadmap for making SemASM + VAA fluent for agent harnesses
(Codex, Cursor, OpenCode, local agents) **without weakening evidence integrity**.

> **Honesty wins.** [`HONESTY.md`](HONESTY.md) remains the claim boundary.
> This document describes product shape and sequencing; it does not expand what
> may be claimed as verified.

## Primary objective

An agent should need to understand one work packet, write one candidate
artifact, submit it through one stable protocol, receive one actionable repair
response, and finish with sealed evidence.

## Locked policy

- Do **not** prioritize more ISAs, MASM, MCP/HTTP, provider SDKs, or arbitrary asm.
- Unknown shapes on the skill / agent-verify path are **declined**.
  `authoring_only` is allowed only on explicit draft / `vaa author` paths.
- Evolve existing surfaces; do not fork parallel schemas:
  - SemASM `capabilities.toml` → admission-capable JSON export
  - VAA `agent-envelope` → canonical work packet
  - VAA `HarnessSubmitResult` → Repair Feedback v1
- RISC-V remains fail-closed until a dedicated gate exists.
- `verified_under_preconditions` ≠ `verified`; dry-runs ≠ evidence.

## Ownership

| Concern | Owner |
|---|---|
| Technical capability maturity + diagnostic codes | SemASM |
| Admission tiers, acceptance claims, seal, session | VAA |
| Target authoring profile generation | SemASM (VAA embeds at prepare) |
| Authority lock | Human / VAA — agents never self-lock |

## Releases

### Release A — Capability-driven authoring (current)

1. Capability Admission Registry (SemASM export + VAA freeze digest)
2. Canonical Agent Work Packet (envelope vNext)
3. Target Authoring Profile
4. Repair Feedback v1 (`feedback.json` + extended submit JSON)

Claim when gates are green:

> Agents discover supported authoring/acceptance level from one capability
> snapshot, receive one work packet + target profile, and repair from one
> structured feedback document — without duplicated leaf/ABI tables in skills.

### Release B — Fluent repair loop

- `vaa agent serve --stdio`
- Submit levels: `fast` → `full` → `seal` (fast never upgrades evidence)
- Small verified idiom catalog (guidance only)

### Release C — Authoring cases

- `vaa author init|review|lock` + bounded template catalog
- Draft vs locked; agents may propose, must not lock

### Release D — Correctness-preserving optimization

- Optimize only after an accepted candidate
- Deterministic metrics; invalid smaller candidate never wins
- Sealed selection evidence

## Non-goals

- Public network service / MCP as primary interface
- Large VAA SDK / provider-specific core adapters
- MASM support
- Automatic trust of agent-authored contracts
- Unrestricted RISC-V agent-verify claims
- Broad microbenchmark acceptance as seal authority

## Related

- [`HONESTY.md`](HONESTY.md) — claim boundary
- [`agent-harness.md`](agent-harness.md) — harness CLI
- [`agent-playbook.md`](agent-playbook.md) — happy / decline paths
- SemASM: `docs/fluent-agent-surface.md` (controller-facing pointer)
