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

### Release A — Capability-driven authoring (**delivered**)

1. Capability Admission Registry (SemASM export + VAA freeze digest)
2. Canonical Agent Work Packet (envelope vNext)
3. Target Authoring Profile
4. Repair Feedback v1 (`feedback.json` + extended submit JSON)

**Landed:**

- SemASM: `semasm capabilities --format json` + admission entries + digest;
  `semasm target profile <target>`.
- VAA: frozen snapshot
  [`fixtures/semasm/capabilities-snapshot.json`](../fixtures/semasm/capabilities-snapshot.json)
  (`CAPABILITY_SNAPSHOT_DIGEST`); `admit_leaf` / `vaa admit` + admission tiers in
  `src/semasm/admission.rs`. **Skill gate is admission** (`vaa admit`); the JSON
  allowlist is a discovery/freeze mirror of admitted `leaf_names` (freeze gate
  requires matching pairs).
- Prepare writes `agent-envelope.json` and `work-packet.json` (same
  content), plus `target-profile.json` (live SemASM profile or embedded
  fallback) and records digests on the envelope.
- Submit writes `feedback.json` (Repair Feedback v1 on `HarnessSubmitResult`).

**Claim (gates green):**

> Agents discover supported authoring/acceptance level from one capability
> snapshot (`vaa admit`), receive one work packet + target profile, and repair
> from one structured feedback document — without duplicated leaf/ABI tables in
> skills.

### Release B — Fluent repair loop (**delivered**)

- `vaa agent serve --stdio --case <dir>` — NDJSON session
  (`session.start` / `candidate.submit` / `feedback.get` / `session.status` /
  `session.finish`); stdout protocol-only
- Submit levels: `fast` → `full` → `seal` via `--level`
  (**fast never upgrades evidence**; fast success ≠ acceptance)
- Idiom catalog v0 (`vaa agent idioms`, prepare writes `idioms.json`) —
  guidance only, not acceptance authority

**Landed:** `VerifyLevel` on harness submit + adapter `--level`;
`src/harness/stdio_serve.rs`; `src/harness/idioms.rs` +
`schemas/idiom-catalog.json`.

### Release C — Authoring cases (**delivered**)

- `vaa author init|review|lock` + bounded template catalog
- Draft vs locked; agents may propose, must not lock

**Landed:**

- Template catalog under
  [`schemas/author-templates/`](../schemas/author-templates/)
  (`pure-int-unary` / `pure-int-binary` / `pure-int-ternary` /
  `buffer-read` / `buffer-write` / `dual-buffer`) — drafts until lock
- `vaa author init --template … --name … --target …` writes
  `.vaa/author/<name>/` with `task.vaa.toml`, `contract.sem.toml`, and
  `AUTHOR_STATE.toml` (`state=draft`, `experimental=true` unless known-CI
  template)
- `vaa author review` validates task schema, prints digests, admission via
  `admit_leaf`, capability snapshot digest, and issues (JSON or terminal)
- `vaa author lock` is **human CLI only**: fail-closed if review issues remain;
  requires admission for the leaf×target (or `--experimental` →
  `authoring_only`, never `sealed_acceptance`); writes `LOCKED` marker with
  digests; refuses further init mutation
- Skill path **declines** lock — agents propose drafts only

**Claim (gates green):**

> Humans author bounded cases from templates, review admission before lock,
> and only then hand a locked case to the agent harness. Agents never hold
> acceptance / lock authority.
### Release D — Correctness-preserving optimization (**delivered**)

- Optimize only after an accepted candidate
- Deterministic metrics; invalid smaller candidate never wins
- Sealed selection evidence

**Landed:**

- Objective schema
  [`schemas/objective.vaa.schema.json`](../schemas/objective.vaa.schema.json)
  + example
  [`fixtures/optimize/objective.object_bytes.toml`](../fixtures/optimize/objective.object_bytes.toml)
  (separate from `task.vaa.toml`)
- `src/optimize/mod.rs` — parse/validate objective; scan sealed candidates;
  compute `object_bytes` / `instruction_count` / `stack_bytes`;
  `rank_candidates` → `selection-evidence.json`
- CLI: `vaa optimize validate` / `vaa optimize rank --run-dir … --objective …`
  (`--allow-under-preconditions` for VUP; fail-closed if no accepted)

**Claim (gates green):**

> After ≥1 sealed accepted/verified-class candidate exists, VAA ranks by a
> declared objective with deterministic metrics and writes sealed selection
> evidence — without ever selecting violated, incomplete, or failed candidates,
> and without counting fast checks as acceptance.

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
- [`author.md`](author.md) — draft → review → lock lifecycle
- [`optimize.md`](optimize.md) — correctness-preserving selection
- [`agent-playbook.md`](agent-playbook.md) — happy / decline paths
- SemASM: `docs/fluent-agent-surface.md` (controller-facing pointer)
