# Repair E2E — `compiler_source` map-join into repair packet
#
# Proves `vaa repair export --map … --instruction-offset …` fills
# `source_mapping.generator_source` from the HlaX64 `compiler_source`
# alias (plan §13). Assembly-only fallback (§13.3) remains when the
# offset is unjoinable.
#
# Honesty: fixture join ≠ live broken-generator repair; practice seal ≠
# trust root.

## Files

| File | Role |
|---|---|
| `candidate.map.json` | Minimal map with `compiler_source` at offset `0x10` |

## Check

```text
# PowerShell twin: scripts/ci-repair-compiler-source-mapjoin.ps1
./scripts/ci-repair-compiler-source-mapjoin.sh
```

Expected: exported packet has
`source_mapping.generator_source == src/HlaX64.Compiler/Abi/Win64AbiLowerer.cs:214`.
