# HlaX64 live repair evidence — Win64 signed greater-than (worktree)

Constrained repair path:

- evidence branch: `evidence/vaa-live-repair-max-signed-win64`
- broken revision: `hlax64@a9bf2ea` (`GreaterThanSigned` → `jl`)
- repaired revision: `hlax64@bd85039` (restore `jg` + regression test)
- suite: `scalar-i64-win64` — broken **Rejected** (`max_i64` Violated);
  repaired **Accepted** (6/6 Verified)
- diagnostic: `BEHAVIOR_VECTOR_MISMATCH_001`
- patch evidence: zero forbidden-path changes

Controlled exercise ≠ naturally occurring incident; practice seal ≠ trust root.

## Verify

```text
vaa repair verify fixtures/repair/hlax64-max-i64-win64-live-worktree/repair-packet.json
vaa patch evidence-verify fixtures/repair/hlax64-max-i64-win64-live-worktree/patch-evidence.json
```
