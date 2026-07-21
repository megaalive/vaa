# Generator-agnostic ingest (count_byte)

Any external generator drops an `.asm` file; VAA verifies and seals.

```bash
cargo run -q -- ingest fixtures/ingest/count_byte/count_byte.vaa.toml \
  --contract fixtures/ingest/count_byte/count_byte.sem.toml \
  --source fixtures/ingest/count_byte/candidate.asm \
  --generator external-agent \
  --run-dir target/vaa-runs \
  --format json

cargo run -q -- evidence check-seal \
  target/vaa-runs/<run-id>/evidence/evidence.json \
  target/vaa-runs/<run-id>/evidence/evidence.seal.json
```

Requires `semasm` on PATH. Generator never writes `final_status` — only VAA seals acceptance digests.
