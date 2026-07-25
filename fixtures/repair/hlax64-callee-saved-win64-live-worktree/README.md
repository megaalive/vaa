# HlaX64 live repair evidence — Win64 callee-saved (worktree)

Non-compare ABI repair path (`ABI_CALLEE_SAVED_001`):

- evidence branch: `evidence/vaa-live-repair-win64-callee-saved`
- broken revision: `hlax64@9bf1e7b` (prologue `xor rbx, rbx` without save)
- repaired revision: `hlax64@8e9d582` (drop clobber + regression test)
- suite: `loop-stack-win64` — broken **Rejected** (4 Violated, SemASM
  `semantic_failed` / ABI callee-saved); repaired **Accepted** (4 Verified)
- diagnostic: `ABI_CALLEE_SAVED_001`
- patch evidence: zero forbidden-path changes

Controlled exercise != naturally occurring incident; practice seal != trust root.

## Verify

```text
vaa repair verify fixtures/repair/hlax64-callee-saved-win64-live-worktree/repair-packet.json
vaa patch evidence-verify fixtures/repair/hlax64-callee-saved-win64-live-worktree/patch-evidence.json
```
