# count_byte fixture-driven `vaa run`

```bash
cargo run -q -- run fixtures/run/count_byte/count_byte.vaa.toml \
  --contract fixtures/run/count_byte/count_byte.sem.toml \
  --wrong fixtures/run/count_byte/01_wrong.asm \
  --repaired fixtures/run/count_byte/02_repaired.asm \
  --run-dir target/vaa-runs \
  --format json
```

Requires `semasm` on PATH and Win64 assemble/link tools. Without `--allow-execution`,
a successful repair ends as `Incomplete` (`execution_denied`).
