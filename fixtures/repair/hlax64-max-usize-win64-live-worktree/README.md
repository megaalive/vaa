# HlaX64 live repair evidence — Win64 unsigned greater-than (worktree)

Constrained repair path beyond less-than defects:

- evidence branch: `evidence/vaa-live-repair-max-unsigned-win64`
- broken revision: `hlax64@e68aac7` (`GreaterThanUnsigned` → `jb`)
- repaired revision: `hlax64@ee3b1b2` (restore `ja` + regression test)
- suite: `scalar-win64` — broken **Rejected** (`max_usize` Violated);
  repaired **Accepted** (2 Verified)
- diagnostic: `BEHAVIOR_VECTOR_MISMATCH_001`
- patch evidence: zero forbidden-path changes

Controlled exercise ≠ naturally occurring incident; practice seal ≠ trust root.

## Verify

```text
vaa repair verify fixtures/repair/hlax64-max-usize-win64-live-worktree/repair-packet.json
vaa patch evidence-verify fixtures/repair/hlax64-max-usize-win64-live-worktree/patch-evidence.json
```
