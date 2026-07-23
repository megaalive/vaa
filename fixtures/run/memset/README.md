# memset fixture-driven `vaa run` / `vaa search` (Tranche Wm3 + Th4)

```bash
cargo run -q -- run fixtures/run/memset/memset.vaa.toml \
  --contract fixtures/run/memset/memset.sem.toml \
  --wrong fixtures/run/memset/01_wrong.asm \
  --repaired fixtures/run/memset/02_repaired.asm \
  --run-dir target/vaa-run-memset

cargo run -q -- search fixtures/run/memset/memset.vaa.toml \
  fixtures/run/memset/02_repaired.asm \
  --ingest --mutator nop-before-ret --budget 3 \
  --contract fixtures/run/memset/memset.sem.toml \
  --run-dir target/vaa-search-memset
```

Requires `semasm` on PATH and Win64 assemble/link tools. Without `--allow-execution`,
a successful repair ends as `Incomplete` (`execution_denied`). Honesty: Gate-1
Incomplete ≠ Verified; `search --ingest` ≠ CryptOpt; Gate-2 Verified needs
`--allow-execution` (not default CI).

`memset` writes to `buffer` (not read-only, unlike `memcmp` / `find_first_byte`).
`01_wrong.asm` returns 0 (the status the contract requires) without ever
storing to the buffer — the return value alone is not sufficient evidence of
a fill; region-precise store proof stays deferred (see `docs/progress.md`).
`00_write_broken.asm` is a Gate-1 static Violated seed (`jmp rax` → control
gate); write-shape skips the static memory gate, so OOB-store-only seeds are
Incomplete without `--allow-execution` (guard-byte evidence is Gate-2 /
SemASM H2).
