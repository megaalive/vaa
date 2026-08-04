# Task schemas v0.1 and v0.2

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
| `schema_version` | `"0.1"` (intent-only tests) or `"0.2"` (SemASM-bound tests) |
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
| `tests` | Optional author-supplied cases; **required** when `require_behavioral_tests = true` |

Unknown fields are **rejected** (`deny_unknown_fields`).

## Fail-closed rules

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

Author-supplied tests are included. Changing a test expectation or a budget changes the digest.

### Behavioral-test evidence boundary

Schema 0.1 locks `[[tests]]` into the task digest, but `vaa verify` does **not**
pass those cases to SemASM. SemASM executes the synthesized vectors for its
recognized, versioned builtin oracle. A matching case name or value is not
evidence that the task case ran.

Consequently, schema 0.1 provides integrity for the author's test intent, not
task-case execution provenance. Reports and documentation must not describe
`[[tests]]` as executed unless a later protocol explicitly binds those cases
into SemASM evidence.

Schema 0.2 is that explicit binding for recognized scalar integer oracles.
VAA writes an input-only external-vector document containing the locked case
IDs and named inputs. SemASM rejects caller-supplied expected values, retains
its builtin vectors, and computes expected results using its builtin oracle.
VAA then requires the report's canonical document digest, external case count,
case IDs/origins, oracle-derived expected strings, and passing case results to
match the locked task. Any missing or mismatched field fails closed.

In schema 0.2 every case must name exactly the declared task inputs and its
expected value must currently be an integer. Pointer/region cases remain on
schema 0.1 until SemASM explicitly admits their external representation.

## Example

See [`fixtures/tasks/sum_i64.vaa.toml`](../fixtures/tasks/sum_i64.vaa.toml) (architecture plan §9.2 / §9.3).
For schema 0.2 scalar vector binding, see
[`fixtures/tasks/sum_range_win64_0_2.vaa.toml`](../fixtures/tasks/sum_range_win64_0_2.vaa.toml).

## What is not in schemas 0.1/0.2

- Natural-language `vaa plan` compilation
- SemASM `.sem.toml` embedding (may be linked later as a side document)
- External task vectors for pointer/region oracle shapes
- Live model fields or provider secrets
- Sandbox backend selection (policy may grow in a later schema minor)

## Versioning

- **Major** changes that break existing tasks require a new `schema_version` and a deliberate VAA acceptance range update.
- This build accepts `0.1` and `0.2`; new task-case execution claims require
  `0.2` evidence binding.
