# find_first_byte fixture-driven `vaa run` / `vaa search` (Tranche Q1 + Z)

```bash
cargo run -q -- run fixtures/run/find_first_byte/find_first_byte.vaa.toml \
  --contract fixtures/run/find_first_byte/find_first_byte.sem.toml \
  --wrong fixtures/run/find_first_byte/01_wrong.asm \
  --repaired fixtures/run/find_first_byte/02_repaired.asm \
  --run-dir target/vaa-runs \
  --format json

cargo run -q -- search fixtures/run/find_first_byte/find_first_byte.vaa.toml \
  fixtures/run/find_first_byte/02_repaired.asm \
  --ingest --mutator nop-before-ret --budget 3 \
  --contract fixtures/run/find_first_byte/find_first_byte.sem.toml \
  --run-dir target/vaa-search-find-first
```

Requires `semasm` on PATH and Win64 assemble/link tools. Without `--allow-execution`,
a successful repair ends as `Incomplete` (`execution_denied`). Honesty: Gate-1
Incomplete ≠ Verified; `search --ingest` ≠ CryptOpt; Gate-2 Verified needs
`--allow-execution` (not default CI).

