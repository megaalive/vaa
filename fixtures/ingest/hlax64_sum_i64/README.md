# HlaX64 → VAA ingest (`sum_i64`)

Generator-agnostic ingest of NASM produced by HlaX64 for the SemASM
`wrapping_sum_i64` leaf.

Authoring source (sibling repo):
`../hlax64/examples/interop/semasm-vaa/sum_i64/sum_i64.hla64`

```bash
cargo run -q -- ingest fixtures/ingest/hlax64_sum_i64/sum_i64.vaa.toml \
  --contract fixtures/ingest/hlax64_sum_i64/sum_i64.sem.toml \
  --source fixtures/ingest/hlax64_sum_i64/candidate.asm \
  --generator hlax64 \
  --run-dir target/vaa-runs \
  --format json

cargo run -q -- evidence verify-chain target/vaa-runs/<run-id>
```

Regenerate committed NASM (requires `hla64` on PATH or `dotnet run` via script):

```powershell
./scripts/regen-hlax64-sum_i64.ps1
```

Honesty: Gate-1 without `--allow-execution` → Incomplete. Gate-2 with
`--allow-execution` → Verified. HlaX64 `-Wverify` ≠ SemASM `verified`. `search --ingest` ≠ CryptOpt; SoftHSM ≠ hardware HSM.
