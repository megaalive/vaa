# HlaX64 live repair evidence — SysV unsigned compare

This fixture records the first end-to-end HlaX64 backend repair exercise.

- controlled broken revision: `hlax64@354fabb` on
  `evidence/vaa-live-repair-unsigned-sysv`;
- repaired revision: `hlax64@06d1113` on the same evidence branch;
- production regression test: `hlax64@5379729` (`main`);
- failure: `min_usize_sysv` returned the larger operand for 4/6 vectors;
- diagnostic: `BEHAVIOR_VECTOR_MISMATCH_001`;
- changed paths: only the System V ABI lowerer and its compiler test;
- post-repair suite: `hlax64.backend.scalar.sysv.v0` **Accepted** with
  `min_usize_sysv` / `max_usize_sysv` **Verified**;
- deterministic regeneration was enabled.

The controlled defect inverted `CompareKind.LessThanUnsigned` from `jb` to
`ja`. The repair packet constrained edits to actual HlaX64 backend/test paths
and prohibited edits to generated assembly, tasks, contracts, vectors, and
the stack lock.

Verify the committed artifacts:

```text
vaa repair verify fixtures/repair/hlax64-min-usize-sysv-live/repair-packet.json
vaa patch evidence-verify fixtures/repair/hlax64-min-usize-sysv-live/patch-evidence.json
```

Practice seal authenticity is not a production trust root.
