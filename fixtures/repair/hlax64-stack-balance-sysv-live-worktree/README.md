# HlaX64 live repair evidence — SysV stack-balance (worktree)

Non-compare ABI repair path (`ABI_STACK_BALANCE_001` SysV twin):

- evidence branch: `evidence/vaa-live-repair-sysv-stack-balance`
- broken revision: `hlax64@9cec09d` (framed SysV epilogue omitted `pop rbp`)
- repaired revision: `hlax64@0b9dee2` (restore `pop rbp` + regression test)
- suite: `loop-stack-sysv` — broken **Rejected** (4 Violated, SemASM
  `semantic_failed` / ABI `STACK_BALANCE_RET`); repaired **Accepted**
  (4 Verified) via WSL Linux Gate
- diagnostic: `ABI_STACK_BALANCE_001`
- patch evidence: zero forbidden-path changes

Controlled exercise != naturally occurring incident; practice seal != trust root.

## Verify

```text
vaa repair verify fixtures/repair/hlax64-stack-balance-sysv-live-worktree/repair-packet.json
vaa patch evidence-verify fixtures/repair/hlax64-stack-balance-sysv-live-worktree/patch-evidence.json
```
