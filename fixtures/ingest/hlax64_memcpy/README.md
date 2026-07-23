# HlaX64 → VAA ingest (`memcpy`)

Generator-agnostic ingest of NASM produced by HlaX64 for the SemASM
`memcpy` leaf.

Authoring source (sibling repo):
`../hlax64/examples/interop/semasm-vaa/memcpy/memcpy.hla64`

```bash
cargo run -q -- ingest fixtures/ingest/hlax64_memcpy/memcpy.vaa.toml \
  --contract fixtures/ingest/hlax64_memcpy/memcpy.sem.toml \
  --source fixtures/ingest/hlax64_memcpy/candidate.asm \
  --generator hlax64 \
  --run-dir target/vaa-runs \
  --format json

cargo run -q -- evidence verify-chain target/vaa-runs/<run-id>
```

Regenerate committed NASM (requires `hla64` on PATH or `dotnet run` via script):

```powershell
./scripts/regen-hlax64-memcpy.ps1
```

Honesty: Gate-1 without `--allow-execution` → Incomplete. Gate-2 with
`--allow-execution` → Verified. HlaX64 `-Wverify` ≠ SemASM `verified`.
`memcpy` writes to `dst` and reads `src` (not read-only, unlike
`memcmp`/`find_last_byte`); `dst`/`src` are assumed distinct, non-overlapping
buffers (SemASM ADR 0003, "overlap fail-closed").
