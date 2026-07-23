# replace_byte fixture-driven `vaa run` / `vaa search` (Tranche W3 + Th3)

```bash
cargo run -q -- run fixtures/run/replace_byte/replace_byte.vaa.toml \
  --contract fixtures/run/replace_byte/replace_byte.sem.toml \
  --wrong fixtures/run/replace_byte/01_wrong.asm \
  --repaired fixtures/run/replace_byte/02_repaired.asm \
  --run-dir target/vaa-run-replace-byte

cargo run -q -- search fixtures/run/replace_byte/replace_byte.vaa.toml \
  fixtures/run/replace_byte/02_repaired.asm \
  --ingest --mutator nop-before-ret --budget 3 \
  --contract fixtures/run/replace_byte/replace_byte.sem.toml \
  --run-dir target/vaa-search-replace-byte
```

Requires `semasm` on PATH and Win64 assemble/link tools. Without `--allow-execution`,
a successful repair ends as `Incomplete` (`execution_denied`). Honesty: Gate-1
Incomplete ≠ Verified; `search --ingest` ≠ CryptOpt; Gate-2 Verified needs
`--allow-execution` (not default CI).

`replace_byte` writes to `buffer` (not read-only, unlike `memcmp` / `find_first_byte`).
`01_wrong.asm` counts matching bytes correctly but never stores the replacement —
the return value alone is not sufficient evidence of a store; region-precise
store proof stays deferred (see `docs/progress.md`). `00_write_broken.asm`
writes one byte past the declared `buffer[0..length]` region regardless of
match state, which fails the SemASM behavioral oracle (`builtin.buffer.replace_byte`)
and is reported `Violated`, mirroring the `memcmp` / `find_first_byte`
adversarial seeds.
