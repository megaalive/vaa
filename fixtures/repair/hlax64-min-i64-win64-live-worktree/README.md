# HlaX64 live repair evidence — Win64 signed compare (worktree)

Third constrained repair path (after SysV unsigned + Win64 map-line fixture):

- evidence branch: `evidence/vaa-live-repair-signed-win64`
- controlled broken revision: `hlax64@6bd1489` (`LessThanSigned` → `jg`)
- repaired revision: `hlax64@83af744` (restore `jl` + regression test)
- suite: `scalar-i64-win64` — broken **Rejected** (`min_i64`/`abs_i64`
  Violated); repaired **Accepted** (6 Verified)
- diagnostic: `BEHAVIOR_VECTOR_MISMATCH_001`
- repair packet joined via `--map-line 19` → `WindowsMsAbiLowerer.cs:438`
- patch evidence: zero forbidden-path changes

Honesty: controlled evidence worktree ≠ production tip mutation until
the regression test is landed on `main`; practice seal ≠ trust root.

## Verify

```text
vaa repair verify fixtures/repair/hlax64-min-i64-win64-live-worktree/repair-packet.json
vaa patch evidence-verify fixtures/repair/hlax64-min-i64-win64-live-worktree/patch-evidence.json
```
