# Correctness-preserving optimization (`vaa optimize`)

Selection over **sealed** candidates. Metrics never override correctness: a
smaller violated / incomplete / failed candidate cannot win. See
[`HONESTY.md`](HONESTY.md) and [`fluent-agent-surface.md`](fluent-agent-surface.md).

## When to use

Only after a run directory already has ≥1 accepted / verified-class sealed
candidate. Fast-level harness checks are **not** acceptance for ranking.

## Objective file

Objectives live in a **separate** TOML (not inside `task.vaa.toml`) so task
schema 0.1 fixtures stay stable.

Schema: [`schemas/objective.vaa.schema.json`](../schemas/objective.vaa.schema.json)  
Example: [`fixtures/optimize/objective.object_bytes.toml`](../fixtures/optimize/objective.object_bytes.toml)

```toml
schema_version = "0.2"
primary = "object_file_bytes"   # object_file_bytes | source_text_bytes | instruction_count | stack_bytes
secondary = ["instruction_count", "stack_bytes"]
must_preserve_status = true
max_candidates = 4
```

`must_preserve_status` is always required to be `true`. Schema `0.1` with
legacy `object_bytes` is accepted and mapped to `object_file_bytes`.

## Metrics (H1 integrity)

For each sealed candidate (`candidates/NNNN/` with `evidence.seal.json`):

| Metric | Source |
|---|---|
| `object_file_bytes` | Size of assembled object (`candidate.o` / `.obj`) **only**. Never inferred from source. Missing → candidate not eligible for this primary. |
| `source_text_bytes` | Explicit source text size — separate metric; not interchangeable with object size. |
| `instruction_count` | Best-effort from SemASM report / decode coverage; secondary tie-break only when **both** sides have it |
| `stack_bytes` | From report when present; same both-sides rule |

Rules:

- Primary metric must be present on every eligible candidate.
- Missing primary → `missing_primary_metric` (or selection incomplete if none remain).
- Mixed primary metric bases → refuse comparison (`selection incomplete`).
- Selection evidence includes `metric_evidence_digest` + `tool_version`.

Correctness status comes from seal / evidence (`verified`, optional VUP).

## Commands

```text
vaa optimize validate <objective.toml>
vaa optimize rank --run-dir <dir> --objective <objective.toml> \
  [--allow-under-preconditions] [--format json]
```

- **validate** — parse + fail-closed checks.
- **rank** — filter to status that preserves correctness, sort primary then
  secondary (smaller better), write `selection-evidence.json` into the run dir.
  Fail-closed if no accepted candidate with primary metric.
- `--allow-under-preconditions` — treat `verified_under_preconditions` as
  eligible; never silently promotes VUP to `verified`.

## Selection evidence

`selection-evidence.json` records the winner, rejected candidates with
reasons, objective digest, metric evidence digest, and selected seal digest.
Invalid candidates are listed under `rejected_candidates` (e.g.
`behavior_failed`, `missing_primary_metric`) and never selected.

## Honesty

- Optimize only after accepted evidence exists.
- Every ranked candidate must have full sealed verification evidence.
- Violated / incomplete / failed never win, even if smaller.
- Fast checks ≠ acceptance for ranking.
- Do not compare unlike quantities (object vs source).
