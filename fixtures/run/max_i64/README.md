# max_i64 corpus leaf (pure scalar, signed branch, no memory)

First leaf with no pointer inputs at all: two i64 scalars in registers, signed
compare, branch select (SemASM's oracle recognizes this as
`builtin.pure_int.binary_i64`). The wrong candidate flips the branch and
returns the minimum. Note: `cmov` is not modeled by SemASM's semantic lowering
yet, so the repaired candidate deliberately uses `cmp`/`jcc`/`mov`.

```bash
python scripts/corpus_sweep.py --leaf max_i64
```

Requires `semasm` on PATH and Win64 assemble/link tools.
