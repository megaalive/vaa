# HlaX64 → VAA ingest (`min_usize`)

Generator-agnostic ingest of NASM produced by HlaX64 for the SemASM
`min_usize` leaf (oracle `builtin.pure_int.binary_usize`, claim `min`).

Authoring source (sibling repo):
`../hlax64/examples/interop/semasm-vaa/min_usize/min_usize.hla64`

```bash
cargo run -q -- ingest fixtures/ingest/hlax64_min_usize/min_usize.vaa.toml \
  --contract fixtures/ingest/hlax64_min_usize/min_usize.sem.toml \
  --source fixtures/ingest/hlax64_min_usize/candidate.asm \
  --generator hlax64 \
  --run-dir target/vaa-runs \
  --format json

cargo run -q -- evidence verify-chain target/vaa-runs/<run-id>
```

Regenerate committed NASM (requires `hla64` on PATH or `dotnet run` via script):

```powershell
./scripts/regen-hlax64-min_usize.ps1
```

Honesty: Gate-1 without `--allow-execution` → Incomplete. Gate-2 with
`--allow-execution` → Verified. HlaX64 `-Wverify` ≠ SemASM `verified`.
Pure-integer leaf: no buffer/pointer arguments, no memory effects.
