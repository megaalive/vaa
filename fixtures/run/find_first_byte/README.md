# find_first_byte fixture-driven `vaa run` (Tranche Q1)

```bash
cargo run -q -- run fixtures/run/find_first_byte/find_first_byte.vaa.toml \
  --contract fixtures/run/find_first_byte/find_first_byte.sem.toml \
  --wrong fixtures/run/find_first_byte/01_wrong.asm \
  --repaired fixtures/run/find_first_byte/02_repaired.asm \
  --run-dir target/vaa-runs \
  --format json
```

Requires `semasm` on PATH and Win64 assemble/link tools. Without `--allow-execution`,
a successful repair ends as `Incomplete` (`execution_denied`). Extends R7
(`count_byte`) multi-candidate wrong→repair Gate smoke to the buffer index-of leaf.
