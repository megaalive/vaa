# HlaX64 live repair evidence — Win64 stack-balance (worktree)

Non-compare ABI repair path (`ABI_STACK_BALANCE_001`):

- evidence branch: `evidence/vaa-live-repair-win64-stack-balance`
- broken revision: `hlax64@535136d` (framed Win64 epilogue omitted `pop rbp`)
- repaired revision: `hlax64@0f4e8bf` (restore `pop rbp` + regression test)
- suite: `loop-stack-win64` — broken **Rejected** (4 Violated, SemASM
  `semantic_failed` / ABI `STACK_BALANCE_RET`); repaired **Accepted**
  (4 Verified)
- diagnostic: `ABI_STACK_BALANCE_001`
- patch evidence: zero forbidden-path changes

Controlled exercise != naturally occurring incident; practice seal != trust root.

## Verify

```text
vaa repair verify fixtures/repair/hlax64-stack-balance-win64-live-worktree/repair-packet.json
vaa patch evidence-verify fixtures/repair/hlax64-stack-balance-win64-live-worktree/patch-evidence.json
```
