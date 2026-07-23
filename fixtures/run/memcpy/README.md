# memcpy fixture-driven `vaa run` / `vaa search` (Tranche Wc + Th6)

```bash
cargo run -q -- run fixtures/run/memcpy/memcpy.vaa.toml \
  --contract fixtures/run/memcpy/memcpy.sem.toml \
  --wrong fixtures/run/memcpy/01_wrong.asm \
  --repaired fixtures/run/memcpy/02_repaired.asm \
  --run-dir target/vaa-run-memcpy

cargo run -q -- search fixtures/run/memcpy/memcpy.vaa.toml \
  fixtures/run/memcpy/02_repaired.asm \
  --ingest --mutator nop-before-ret --budget 3 \
  --contract fixtures/run/memcpy/memcpy.sem.toml \
  --run-dir target/vaa-search-memcpy
```

Requires `semasm` on PATH and Win64 assemble/link tools. Without `--allow-execution`,
a successful repair ends as `Incomplete` (`execution_denied`). Honesty: Gate-1
Incomplete ≠ Verified; `search --ingest` ≠ CryptOpt; Gate-2 Verified needs
`--allow-execution` (not default CI).

`memcpy` writes to `dst` (not read-only, unlike `memcmp` / `find_first_byte`)
and reads `src` (never mutated). `01_wrong.asm` returns 0 (the status the
contract requires) without ever copying `src` into `dst` — the return value
alone is not sufficient evidence of a copy; region-precise store proof stays
deferred (see `docs/progress.md`). `00_write_broken.asm` is a Gate-1 static
Violated seed (`jmp rax` → control gate); write-shape skips the static memory
gate, so OOB-store-only seeds are Incomplete without `--allow-execution`
(guard-byte evidence is Gate-2 / SemASM H2). `dst`/`src` are distinct,
non-overlapping buffers only (SemASM ADR 0003, "overlap fail-closed").
