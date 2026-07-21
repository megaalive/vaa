# SemASM handshake fixtures

Vendored inputs for the VAA ↔ SemASM controller protocol (VerificationReport
schema **0.4**).

## Layout

| Path | Role |
|---|---|
| `count_byte/count_byte.vaa.toml` | Locked VAA task (Win64 target) |
| `count_byte/count_byte_win64.asm` | Candidate source (copied from SemASM) |
| `count_byte/count_byte.sem.toml` | SemASM contract (copied from SemASM) |
| `reports/verification-report-count_byte.execution_denied.json` | Golden report for unit parse tests |

## Smoke (requires `semasm` on PATH)

```bash
cargo run -q -- verify fixtures/semasm/count_byte/count_byte.vaa.toml \
  --source fixtures/semasm/count_byte/count_byte_win64.asm \
  --contract fixtures/semasm/count_byte/count_byte.sem.toml \
  --format json
```

Without `--allow-execution` on the SemASM side (VAA does not pass it), expect
`final_status: Incomplete` with `verify_report.raw_status: execution_denied`.

Ignored integration test:

```bash
cargo test -p vaa --test semasm_verify_smoke -- --ignored
```
