# max_i64 corpus leaf (pure scalar, cmov select, no memory)

First leaf with no pointer inputs: two i64 scalars, signed compare, `cmovg`
select. The wrong candidate uses `cmovl` (returns the minimum). SemASM tip
`>= 0ab8004` models `cmov*` as `OpKind::Select`; earlier tips failed
`require_complete_lowering` on this shape.

```bash
python scripts/corpus_sweep.py --leaf max_i64
python scripts/corpus_sweep.py --strict-verified --leaf max_i64
```

Requires `semasm` on PATH and Win64 assemble/link tools.
