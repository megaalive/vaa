# Task schema v0.1

**Status:** implemented in the VAA crate (parse + validate + digest)  
**Authoritative on-disk format:** TOML (`*.vaa.toml`)  
**Checked-in JSON Schema:** [`schemas/task.vaa.schema.json`](../schemas/task.vaa.schema.json)  
**Architecture reference:** plan §9

## Purpose

A task file is the **locked contract** for a VAA run. Natural language may help draft it, but only the structured task (including authoritative tests, budgets, capabilities, and verification requirements) participates in the content digest.

Repair loops and model adapters must not edit a locked task. A changed contract is a new task with a new digest.

## CLI

```bash
vaa validate path/to/task.vaa.toml
vaa validate path/to/task.vaa.toml --format json
```

| Exit code | Meaning |
|---:|---|
| 0 | Task parsed, validated, and locked |
| 2 | Invalid path, TOML, unknown fields, or semantic validation failure |

## Document shape

Required top-level fields:

| Field | Notes |
|---|---|
| `schema_version` | Must be `"0.1"` for this VAA build |
| `task_id` | Stable id: `[A-Za-z][A-Za-z0-9._-]{0,127}` |
| `artifact_kind` | `callable-function` \| `hosted-program` \| `freestanding-image` |
| `target` | Target triple string |
| `entry` | `symbol`, `abi` |
| `output` | `kind` |
| `behavior` | At least `summary` |
| `capabilities` | Fail-closed defaults |
| `memory` | Includes `max_stack_bytes` |
| `instructions` | Feature / mnemonic constraints |
| `verification` | Required evidence layers |
| `budgets` | Candidate / time / token limits |
| `delivery` | What to retain on accept |
| `inputs` | Optional map of named inputs |
| `tests` | Optional array; **required** when `require_behavioral_tests = true` |

Unknown fields are **rejected** (`deny_unknown_fields`).

## Fail-closed rules (schema 0.1)

- `capabilities.network = true` → validation error
- `memory.allow_self_modifying_code = true` → validation error
- `require_behavioral_tests = true` with empty `tests` → validation error
- `hosted-program` / `freestanding-image` with behavioral tests required → validation error (harness not implemented yet)
- `budgets.max_candidates` and `max_wall_time_seconds` must be ≥ 1

## Content digest

After validation, VAA seals the task as a `LockedTask` and computes:

```text
sha256:<hex>
```

over the **canonical JSON** encoding of the full task document:

- object keys sorted lexicographically at every level;
- arrays preserve author order;
- compact JSON (no insignificant whitespace).

Authoritative tests are included. Changing a test expectation or a budget changes the digest.

## Example

See [`fixtures/tasks/sum_i64.vaa.toml`](../fixtures/tasks/sum_i64.vaa.toml) (architecture plan §9.2 / §9.3).

## What is not in schema 0.1

- Natural-language `vaa plan` compilation
- SemASM `.sem.toml` embedding (may be linked later as a side document)
- Live model fields or provider secrets
- Sandbox backend selection (policy may grow in a later schema minor)

## Versioning

- **Major** changes that break existing tasks require a new `schema_version` and a deliberate VAA acceptance range update.
- This build accepts **only** `0.1`.
