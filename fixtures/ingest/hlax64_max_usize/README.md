# HlaX64 → VAA ingest (`max_usize`)

Generator-agnostic ingest of NASM produced by HlaX64 for the SemASM
`max_usize` leaf (oracle `builtin.pure_int.binary_usize`, claim `max`).

Authoring source (sibling repo):
`../hlax64/examples/interop/semasm-vaa/max_usize/max_usize.hla64`

```bash
cargo run -q -- ingest fixtures/ingest/hlax64_max_usize/max_usize.vaa.toml \
  --contract fixtures/ingest/hlax64_max_usize/max_usize.sem.toml \
  --source fixtures/ingest/hlax64_max_usize/candidate.asm \
  --generator hlax64 \
  --run-dir target/vaa-runs \
  --format json

cargo run -q -- evidence verify-chain target/vaa-runs/<run-id>
```

Regenerate committed NASM (requires `hla64` on PATH or `dotnet run` via script):

```powershell
./scripts/regen-hlax64-max_usize.ps1
```

Honesty: Gate-1 without `--allow-execution` → Incomplete. Gate-2 with
`--allow-execution` → Verified. HlaX64 `-Wverify` ≠ SemASM `verified`. `search --ingest` ≠ CryptOpt; SoftHSM ≠ hardware HSM.
Pure-integer leaf: no buffer/pointer arguments, no memory effects.
