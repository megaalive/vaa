# HlaX64 live repair evidence — SysV signed compare

Constrained repair path:

- evidence branch: `evidence/vaa-live-repair-signed-sysv`
- broken revision: `hlax64@0f0dee7` (`LessThanSigned` → `jg`)
- repaired revision: `hlax64@f1e56b1` (restore `jl` + regression test)
- suite: `scalar-i64-sysv` — broken **Rejected** (`min_i64` and `abs_i64`
  Violated); repaired **Accepted** (6/6 Verified)
- diagnostic: `BEHAVIOR_VECTOR_MISMATCH_001`
- patch evidence: zero forbidden-path changes

The Gate ran under WSL with native Linux SemASM and .NET. Controlled exercise
≠ naturally occurring incident; practice seal ≠ trust root.

## Verify

```text
vaa repair verify fixtures/repair/hlax64-min-i64-sysv-live-worktree/repair-packet.json
vaa patch evidence-verify fixtures/repair/hlax64-min-i64-sysv-live-worktree/patch-evidence.json
```
