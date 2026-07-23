# memcmp fixture-driven `vaa run` / `vaa search` (Tranche V1 + Y)

```bash
cargo run -q -- run fixtures/run/memcmp/memcmp.vaa.toml \
  --contract fixtures/run/memcmp/memcmp.sem.toml \
  --wrong fixtures/run/memcmp/01_wrong.asm \
  --repaired fixtures/run/memcmp/02_repaired.asm \
  --run-dir target/vaa-run-memcmp

cargo run -q -- search fixtures/run/memcmp/memcmp.vaa.toml \
  fixtures/run/memcmp/02_repaired.asm \
  --ingest --mutator nop-before-ret --budget 3 \
  --contract fixtures/run/memcmp/memcmp.sem.toml \
  --run-dir target/vaa-search-memcmp
```

Honesty: Gate-1 Incomplete ≠ Verified. `search --ingest` ≠ CryptOpt.
Gate-2 Verified requires `--allow-execution` (not default CI).

