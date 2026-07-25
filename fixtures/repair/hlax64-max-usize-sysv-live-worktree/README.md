# HlaX64 live repair evidence — SysV unsigned greater-than (worktree)

Final constrained compare-surface twin for greater-than:

- evidence branch: `evidence/vaa-live-repair-max-unsigned-sysv`
- broken revision: `hlax64@6b4d96a` (`GreaterThanUnsigned` → `jb`)
- repaired revision: `hlax64@8045551` (restore `ja` + regression test)
- suite: `scalar-sysv` — broken **Rejected** (`max_usize` Violated);
  repaired **Accepted** (2/2 Verified)
- diagnostic: `BEHAVIOR_VECTOR_MISMATCH_001`
- patch evidence: zero forbidden-path changes

Gate ran under WSL. Controlled exercise ≠ naturally occurring incident;
practice seal ≠ trust root.

## Verify

```text
vaa repair verify fixtures/repair/hlax64-max-usize-sysv-live-worktree/repair-packet.json
vaa patch evidence-verify fixtures/repair/hlax64-max-usize-sysv-live-worktree/patch-evidence.json
```
