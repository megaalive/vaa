# HlaX64 live repair evidence — SysV callee-saved (worktree)

Non-compare ABI repair path (`ABI_CALLEE_SAVED_001` SysV twin):

- evidence branch: `evidence/vaa-live-repair-sysv-callee-saved`
- broken revision: `hlax64@4461cbd` (prologue `xor rbx, rbx` without save)
- repaired revision: `hlax64@e23b1d9` (drop clobber + regression test)
- suite: `loop-stack-sysv` — broken **Rejected** (4 Violated, SemASM
  `semantic_failed` / ABI callee-saved); repaired **Accepted** (4 Verified)
  via WSL Linux Gate
- diagnostic: `ABI_CALLEE_SAVED_001`
- patch evidence: zero forbidden-path changes

Controlled exercise != naturally occurring incident; practice seal != trust root.

## Verify

```text
vaa repair verify fixtures/repair/hlax64-callee-saved-sysv-live-worktree/repair-packet.json
vaa patch evidence-verify fixtures/repair/hlax64-callee-saved-sysv-live-worktree/patch-evidence.json
```
