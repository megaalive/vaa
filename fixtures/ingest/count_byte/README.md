# Generator-agnostic ingest (count_byte)

Any external generator drops an `.asm` file; VAA verifies and seals a per-candidate bundle.

```bash
cargo run -q -- ingest fixtures/ingest/count_byte/count_byte.vaa.toml \
  --contract fixtures/ingest/count_byte/count_byte.sem.toml \
  --source fixtures/ingest/count_byte/candidate.asm \
  --generator external-agent \
  --run-dir target/vaa-runs \
  --format json

# JSON drift only (evidence ↔ seal):
cargo run -q -- evidence check-seal \
  target/vaa-runs/<run-id>/candidates/0000/evidence.json \
  target/vaa-runs/<run-id>/candidates/0000/evidence.seal.json

# Re-hash on-disk artifacts against sealed digests:
cargo run -q -- evidence verify-bundle \
  target/vaa-runs/<run-id>/candidates/0000
```

Requires `semasm` on PATH. Generator never writes `final_status`. The seal is **content integrity**, not a cryptographic attestation of publisher identity — see `docs/seal.md`.
