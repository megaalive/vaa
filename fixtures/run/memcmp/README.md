# memcmp fixture-driven `vaa run` (Tranche V1)

```bash
cargo run -q -- run fixtures/run/memcmp/memcmp.vaa.toml \
  --contract fixtures/run/memcmp/memcmp.sem.toml \
  --wrong fixtures/run/memcmp/01_wrong.asm \
  --repaired fixtures/run/memcmp/02_repaired.asm \
  --run-dir target/vaa-run-memcmp
```

Honesty: Gate-1 Incomplete != Verified.
