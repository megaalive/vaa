# Evidence seals (R2+)

## Integrity vs authenticity

The current seal is a **content-integrity envelope**, not a cryptographic
attestation of publisher identity.

| Property | Supported today? | Meaning |
|---|---|---|
| **Content integrity** | Yes | Detect drift or corruption between `evidence.json` and the sealed digests, and (via `verify-bundle`) between digests and on-disk artifacts. |
| **Authenticity** | No | Does **not** prove a trusted VAA instance issued the acceptance. Anyone who can write both evidence and seal files can rewrite the payload and recompute SHA-256. |

There is no secret key and no digital signature in the seal path today. That is
intentional for R2: process isolation plus an external transparency artifact
(CI log, Git note, append-only store of `envelope_digest`) is enough for early
phases.

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

## Commands

### `vaa evidence check-seal <evidence.json> <evidence.seal.json>`

Detects **evidence/seal drift** (JSON report vs sealed payload / digests).

Does **not** re-hash `candidate.asm` / contract / task files on disk.

### `vaa evidence verify-bundle <bundle-dir>`

Re-hashes on-disk artifacts and compares them to sealed digests:

| File | Compared to |
|---|---|
| `task.vaa.toml` | locked-task content digest → `acceptance.task_digest` |
| `contract.sem.toml` | SHA-256 of file bytes → `acceptance.contract_digest` |
| `candidate.asm` | SHA-256 of file bytes → `acceptance.source_digest` |
| `semasm-report.json` | SHA-256 of file bytes → `acceptance.semasm_report_digest` (or `none`) |
| `evidence.json` + `evidence.seal.json` | digests + cross-checks |

## Layout (Proof Loop–oriented)

Per candidate (append-only; not overwritten by later repairs):

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

Each candidate seal’s `provenance` may include `previous_seal_digest` (hash chain).
Deleting a failed attempt breaks the chain.

Atomic write: `evidence.json.tmp` / `evidence.seal.json.tmp`, fsync, rename
evidence first, seal last (seal rename is the commit marker).
