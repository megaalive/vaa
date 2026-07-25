# HlaX64 live repair evidence — Win64 signed compare (map-line join)
#
# Second constrained repair path (after SysV unsigned `min_usize`):
# Win64 `min_i64` defect surface + `vaa repair export --map --map-line`
# filling `generator_source` from `compiler_source`.
#
# - locked wrong input: `integrations/hlax64/cases/min_i64_wrong`
#   (signed `>` instead of `<` → MAX not MIN; negative suite Reject);
# - generated `candidate.asm` shows `jg` at the compare site;
# - map entry at assembly line 19 joins
#   `WindowsMsAbiLowerer.cs` GreaterThanSigned locus;
# - diagnostic: `BEHAVIOR_VECTOR_MISMATCH_001`.
#
# Honesty: recorded map-line join ≠ full ABI worktree mutation like the
# SysV evidence branch; practice seal ≠ trust root; locked wrong case ≠
# production generator tip.

## Verify

```text
./scripts/ci-repair-win64-min-i64-mapline.ps1
vaa repair verify fixtures/repair/hlax64-min-i64-win64-live/repair-packet.json
```
