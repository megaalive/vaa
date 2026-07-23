# HlaX64 → VAA ingest (`memcmp`)

Generator-agnostic ingest of NASM produced by HlaX64 for the SemASM
`memcmp_i8` leaf.

Authoring source (sibling repo):
`../hlax64/examples/interop/semasm-vaa/memcmp/memcmp.hla64`

```bash
cargo run -q -- ingest fixtures/ingest/hlax64_memcmp/memcmp.vaa.toml \
  --contract fixtures/ingest/hlax64_memcmp/memcmp.sem.toml \
  --source fixtures/ingest/hlax64_memcmp/candidate.asm \
  --generator hlax64 \
  --run-dir target/vaa-runs \
  --format json

cargo run -q -- evidence verify-chain target/vaa-runs/<run-id>
```

Regenerate committed NASM (requires `hla64` on PATH or `dotnet run` via script):

```powershell
./scripts/regen-hlax64-memcmp.ps1
```

Honesty: Gate-1 without `--allow-execution` → Incomplete. Gate-2 with
`--allow-execution` → Verified. HlaX64 `-Wverify` ≠ SemASM `verified`. `search --ingest` ≠ CryptOpt; SoftHSM ≠ hardware HSM.
