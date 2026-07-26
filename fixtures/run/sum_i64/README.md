# sum_i64 corpus leaf (i64-element buffer, arithmetic accumulation)

First non-u8 element type in the run corpus: reads `values[0..length]` as i64
and returns the wrapping sum. The wrong candidate subtracts instead of adding.

```bash
python scripts/corpus_sweep.py --leaf sum_i64
```

Requires `semasm` on PATH and Win64 assemble/link tools.
