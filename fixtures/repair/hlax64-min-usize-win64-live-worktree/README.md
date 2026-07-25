# HlaX64 live repair evidence — Win64 unsigned compare (worktree)

Fourth constrained repair path (SysV unsigned, Win64 signed, map-line,
then this Win64 unsigned worktree):

- evidence branch: `evidence/vaa-live-repair-unsigned-win64`
- controlled broken revision: `hlax64@64d5344` (`LessThanUnsigned` → `ja`)
- repaired revision: `hlax64@9a41cb2` (restore `jb` + regression test)
- suite: `scalar-win64` — broken **Rejected** (`min_usize` Violated);
  repaired **Accepted** (2 Verified)
- diagnostic: `BEHAVIOR_VECTOR_MISMATCH_001`
- repair packet joined via `--map-line 19` → `WindowsMsAbiLowerer.cs:439`
- patch evidence: zero forbidden-path changes

Honesty: controlled evidence worktree ≠ production tip until the
regression test lands on `main`; practice seal ≠ trust root.

## Verify

```text
vaa repair verify fixtures/repair/hlax64-min-usize-win64-live-worktree/repair-packet.json
vaa patch evidence-verify fixtures/repair/hlax64-min-usize-win64-live-worktree/patch-evidence.json
```
