# HlaX64 live repair evidence — SysV signed greater-than (worktree)

Constrained repair path:

- evidence branch: `evidence/vaa-live-repair-max-signed-sysv`
- broken revision: `hlax64@94a01b2` (`GreaterThanSigned` → `jl`)
- repaired revision: `hlax64@938f6bb` (restore `jg` + regression test)
- suite: `scalar-i64-sysv` — broken **Rejected** (`max_i64` Violated);
  repaired **Accepted** (6/6 Verified)
- diagnostic: `BEHAVIOR_VECTOR_MISMATCH_001`
- patch evidence: zero forbidden-path changes

Gate ran under WSL with native Linux SemASM/.NET. Controlled exercise ≠
naturally occurring incident; practice seal ≠ trust root.

## Verify

```text
vaa repair verify fixtures/repair/hlax64-max-i64-sysv-live-worktree/repair-packet.json
vaa patch evidence-verify fixtures/repair/hlax64-max-i64-sysv-live-worktree/patch-evidence.json
```
