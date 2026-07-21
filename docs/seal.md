# Evidence seals (R2+)

## Integrity vs authenticity

The current seal is a **content-integrity envelope**, not a cryptographic
attestation of publisher identity.

| Property | Supported today? | Meaning |
|---|---|---|
| **Content integrity** | Yes | Detect drift or corruption between `evidence.json` and the sealed digests, and (via `verify-bundle` / `verify-chain`) between digests and on-disk artifacts. |
| **Authenticity** | No | Does **not** prove a trusted VAA instance issued the acceptance. Anyone who can write both evidence and seal files can rewrite the payload and recompute SHA-256. |

There is no secret key and no digital signature in the seal path today. That is
intentional for early phases: process isolation plus an external transparency
artifact (CI log, Git note, append-only store of `envelope_digest`) is enough.

### Future authenticity options (not implemented)

1. **Filesystem isolation** — generators have no write access to the evidence directory.
2. **External transparency log** — store `envelope_digest` / `acceptance_digest` in CI artifacts, Git notes, or an append-only log.
3. **Digital signature** — VAA signs the acceptance digest (e.g. Ed25519). Signing dependencies are deferred.

## Seal schema 0.2

`evidence.seal.json` separates technical acceptance from provenance:

- `acceptance_digest` — SHA-256 over canonical JSON of `acceptance` only. Stable across runs when task/source/contract/report/status/checks match.
- `envelope_digest` — SHA-256 over canonical JSON of `{acceptance, provenance}`. Changes when `run_id`, generator attribution, candidate index, or seal-chain link changes.
- `canonicalization` = `vaa-canonical-json-v1`
- `digest_algorithm` = `sha256`

Generator metadata lives only under `provenance.generator` and is **not** part of
`acceptance_digest`.

`check-seal` compares the full sorted `checks` vector (including `details`).

## Commands

### `vaa evidence check-seal <evidence.json> <evidence.seal.json>`

Detects **evidence/seal JSON drift** (report vs sealed payload / digests),
including full `CheckOutcome` equality.

Does **not** re-hash `candidate.asm` / contract / task files on disk.

### `vaa evidence verify-bundle <bundle-dir>`

Re-hashes on-disk artifacts and compares them to sealed digests for **one**
candidate (or bundle) directory.

### `vaa evidence verify-chain <run-directory>`

Validates the full Proof Loop history:

1. Contiguous `candidates/0000` … `candidates/NNNN` (no gaps; deleting a predecessor fails).
2. Each candidate passes `verify-bundle`.
3. Candidate `0000` has `previous_seal_digest = null`.
4. Candidate `i` has `previous_seal_digest == envelope_digest` of `i-1`.
5. All candidates share chain identity: `task_id`, `task_digest`, `run_id`, `target`, `contract_digest`.
6. `evidence/final.seal.json` matches the last candidate's `envelope_digest` (and the same identity).

`verify-bundle` also binds `task.task_id` / `task.target` from the on-disk task file to the seal
(so a single valid bundle already proves those labels come from the locked task).

Allowed to change across candidates: `source_digest`, SemASM report digest, `final_status`,
`checks`, generator attribution, `candidate_index`, `previous_seal_digest`.

## Layout (append-only)

```text
candidates/0000/
  candidate.asm
  task.vaa.toml
  contract.sem.toml
  semasm-report.json
  evidence.json
  evidence.seal.json
candidates/0001/
  …
evidence/final.json
evidence/final.seal.json
```

Storage boundary: exclusive `create_dir` per candidate index, `create_new` file
writes, and best-effort read-only permissions after seal. Reusing an index fails
with `CandidateAlreadySealed`.

## Atomic publication

Writes use:

1. `evidence.*.tmp` / `seal.*.tmp`
2. `sync_all` on temporary files
3. rename evidence, then rename seal (seal rename = commit marker)
4. best-effort parent-directory fsync

Accurate term: **atomic publication with a seal commit marker**.

Not claimed: fully **crash-durable transactional pair** on every filesystem
(especially where directory fsync is unavailable, e.g. typical Windows directory handles).
