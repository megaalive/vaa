# HlaX64 → VAA ingest (`count_byte`)

Generator-agnostic ingest of NASM produced by HlaX64 for the SemASM
`count_byte` leaf.

Authoring source (sibling repo):
`../hlax64/examples/interop/semasm-vaa/count_byte/count_byte.hla64`

```bash
cargo run -q -- ingest fixtures/ingest/hlax64_count_byte/count_byte.vaa.toml \
  --contract fixtures/ingest/hlax64_count_byte/count_byte.sem.toml \
  --source fixtures/ingest/hlax64_count_byte/candidate.asm \
  --generator hlax64 \
  --run-dir target/vaa-runs \
  --format json

cargo run -q -- evidence verify-chain target/vaa-runs/<run-id>
```

Regenerate committed NASM (requires `hla64` on PATH or `dotnet run` via script):

```powershell
./scripts/regen-hlax64-count_byte.ps1
```

Honesty: Gate-1 without `--allow-execution` → Incomplete. Gate-2 with
`--allow-execution` → Verified. HlaX64 `-Wverify` ≠ SemASM `verified`.
`count_byte` only reads `buffer` (read-only, like `memcmp`/`find_last_byte`/`find_first_byte`).
