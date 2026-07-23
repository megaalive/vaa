# find_last_byte fixture-driven `vaa run` (Tranche S2)

```bash
cargo run -q -- run fixtures/run/find_last_byte/find_last_byte.vaa.toml \
  --contract fixtures/run/find_last_byte/find_last_byte.sem.toml \
  --wrong fixtures/run/find_last_byte/01_wrong.asm \
  --repaired fixtures/run/find_last_byte/02_repaired.asm \
  --run-dir target/vaa-run-flb
```

Honesty: Gate-1 Incomplete != Verified.
