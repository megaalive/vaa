# Evidence seals (R2+)

## Integrity vs authenticity

The current seal is a **content-integrity envelope**, not a cryptographic
attestation of publisher identity.

| Property | Supported today? | Meaning |
|---|---|---|
| **Content integrity** | Yes | Detect drift or corruption between `evidence.json` and the sealed digests, and (via `verify-bundle` / `verify-chain`) between digests and on-disk artifacts. |
| **Authenticity** | Opt-in | Optional Ed25519 signature over `acceptance_digest` when `VAA_SEAL_SIGNING_KEY` is set. Not a certificate chain / HSM / Sigstore. |

Unsigned seals remain valid unless `VAA_REQUIRE_SEAL_SIGNATURE=1`. Anyone who can
rewrite both evidence and seal without a signing key can still re-hash digests;
the signature binds a known public key to technical acceptance.

### Transparency layers

1. **Filesystem isolation** — generators have no write access to the evidence directory (deferred hardening).
2. **Local seal digest log** — `evidence/seal-log.jsonl` appends each candidate’s
   `envelope_digest` / `acceptance_digest` (L0). Checked by `verify-chain` when
   present (L1). This is **not** an external transparency log.
3. **External transparency export (T0)** — `vaa evidence export-transparency`
   writes a portable `vaa-transparency-v1` JSON document. Gate CI uploads that
   file plus `seal-log.jsonl` as a workflow artifact. **CI artifact ≠ remote
   immutable log** (not Rekor, not an append-only SaaS).
4. **Digital signature (A0)** — VAA may sign `acceptance_digest` with Ed25519
   (seed file via `VAA_SEAL_SIGNING_KEY`; `vaa evidence keygen-seal --out <path>`).

Deferred: remote append-only transparency service, HSM / hardware keys, cosign.

## Seal schema 0.2

`evidence.seal.json` separates technical acceptance from provenance:

- `acceptance_digest` — SHA-256 over canonical JSON of `acceptance` only. Stable across runs when task/source/contract/report/status/checks match.
- `envelope_digest` — SHA-256 over canonical JSON of `{acceptance, provenance}`. Changes when `run_id`, generator attribution, candidate index, or seal-chain link changes.
- `canonicalization` = `vaa-canonical-json-v1`
- `digest_algorithm` = `sha256`
- `signature` (optional) — `{ alg: "ed25519", public_key_b64, sig_b64, signed_over: "acceptance_digest" }`. Not included in either digest hash body.

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

### `vaa evidence export-transparency <run-dir> -o <file.json>`

Builds a `vaa-transparency-v1` document (digests + identity) from a verified run.
`exported_at` is UTC unix epoch seconds as a decimal string (no calendar crate).

### `vaa evidence verify-transparency <file.json> --against <run-dir>`

Re-exports from the live run and fail-closes on digest / identity drift.

### `vaa evidence keygen-seal --out <path>`

Writes a 32-byte hex Ed25519 seed file. Set `VAA_SEAL_SIGNING_KEY` to that path
to sign new seals. Public key hex/b64 are printed for operators.

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
evidence/seal-log.jsonl   # L0 local digest log (one JSON object per candidate)
```

Storage boundary: exclusive `create_dir` per candidate index, `create_new` file
writes, and best-effort read-only permissions after seal. Reusing an index fails
with `CandidateAlreadySealed`.

`seal-log.jsonl` is created on ingest/run seal paths. Older runs without the file
still pass `verify-chain`; when the file exists, digests must match the chain.

## Atomic publication

Writes use:

1. `evidence.*.tmp` / `seal.*.tmp`
2. `sync_all` on temporary files
3. rename evidence → `sync_all` final evidence → parent directory `sync_all`
4. rename seal last (commit marker) → `sync_all` final seal → parent directory `sync_all`

Parent-directory sync is **required on Unix**. On Windows, directory handles are
opened with `FILE_FLAG_BACKUP_SEMANTICS` when possible, but `FlushFileBuffers` on
directories frequently returns Access Denied without backup privilege — that
step stays best-effort there. Final seal *files* are always reopened and
`sync_all`'d after rename on both platforms.

Accurate term: **durable atomic publication with a seal commit marker**
(file-durable everywhere; directory-durable on Unix).

Still not claimed: a formal multi-file transaction on every filesystem (network
FS, dishonest kernels), Windows directory-entry durability in all environments,
or cryptographic authenticity of the publisher.
