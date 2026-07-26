# Agent playbook (VAA harness)

One happy path, one decline path. Bounds are set by [`HONESTY.md`](HONESTY.md);
the skill is [`.cursor/skills/vaa-harness/SKILL.md`](../.cursor/skills/vaa-harness/SKILL.md)
(Codex: [`AGENTS.md`](../AGENTS.md)). Allowed leaves live in
[`schemas/agent-leaf-allowlist.json`](../schemas/agent-leaf-allowlist.json).
The SemASM admission snapshot
([`fixtures/semasm/capabilities-snapshot.json`](../fixtures/semasm/capabilities-snapshot.json))
must list the same `leaf_names`; admission will eventually replace the skill
allowlist, but until then **both** apply — decline anything missing from either.

Rule of thumb: **parse stdout JSON only**, act on `class`, never claim more than
the JSON says. After prepare, read `work-packet.json` (else
`agent-envelope.json`) and `target-profile.json` for ABI — do not hardcode
registers. After submit, read `feedback.json` when present for Repair Feedback
v1 details.

## Happy path A — `max_i64` (Win64, strict `verified`)

`max_i64` is a pure scalar leaf (`strict_verified_ok: true`), so it reaches
unconditional `verified`.

```bash
python scripts/agent_harness_adapter.py loop-direct \
  --task fixtures/run/max_i64/max_i64.vaa.toml \
  --contract fixtures/run/max_i64/max_i64.sem.toml \
  --workspace .vaa/harness/max_i64 \
  --run-base .vaa/runs/max_i64 \
  --candidate fixtures/run/max_i64/01_wrong.asm \
  --candidate fixtures/run/max_i64/02_repaired.asm \
  --allow-execution
```

Expected `final` in the stdout JSON:

```json
{ "class": "accepted", "evidence_status": "verified", "seal_digest": "sha256:…" }
```

Report: "max_i64 accepted — verified, sealed (`seal_digest`)." Nothing more.

## Happy path B — `count_byte` (Win64, `verified_under_preconditions`)

Buffer/loop leaves reach VUP, not plain verified. Pass
`--allow-under-preconditions` and report the result **as VUP**.

```bash
python scripts/agent_harness_adapter.py loop-direct \
  --task fixtures/run/count_byte/count_byte.vaa.toml \
  --contract fixtures/run/count_byte/count_byte.sem.toml \
  --workspace .vaa/harness/count_byte \
  --run-base .vaa/runs/count_byte \
  --candidate fixtures/run/count_byte/01_wrong.asm \
  --candidate fixtures/run/count_byte/02_repaired.asm \
  --allow-execution --allow-under-preconditions
```

Report: "count_byte accepted — **verified_under_preconditions** (not
unconditional verified), sealed." Do **not** drop the "under preconditions".

## `class` → action map

| `class` | Do |
|---|---|
| `accepted` | Stop; report `evidence_status` + `seal_digest`. |
| `violated_repairable` | Edit the candidate assembly only; resubmit. |
| `policy_blocked` | Stop; report the path-policy refusal. |
| `toolchain_retryable` | Stop; report missing/broken toolchain. Do not silently retry. |
| `failed` / `incomplete` / other | Stop; report. Never promote to success. |

## Decline path

Decline (do not attempt) when the request is outside the allowlist. Examples and
the response to give:

- **Unlisted shape** ("verify my `strlen`/`crc32`/… assembly"):
  > That shape isn't in VAA's leaf allowlist (`schemas/agent-leaf-allowlist.json`),
  > so I can't claim SemASM verifies it. I can only drive the listed leaves.
- **Free-form `.S` / "fix my assembly freely"**:
  > This skill only repairs allowlisted corpus leaves toward a locked contract,
  > not arbitrary assembly. SemASM would fail-closed on unmodeled instructions.
- **RISC-V64 verify**:
  > RISC-V64 is GAS dialect-only in VAA (capability `Unknown`, fail-closed). No
  > agent-verify claim exists yet — see `docs/HONESTY.md`.
- **Generator-repair / HlaX64 pack / live model**:
  > Out of scope for the agent skill v1. See `docs/agent-harness.md`; I won't
  > attempt it here.

In every decline: state the boundary, point at `docs/HONESTY.md`, and stop. Do
not partially attempt or invent a result.
