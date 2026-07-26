# Authoring cases (`vaa author`)

Release C lifecycle for bounded SemASM/VAA cases. Agents may **propose** draft
edits; **humans** lock acceptance authority. See
[`HONESTY.md`](HONESTY.md) and
[`fluent-agent-surface.md`](fluent-agent-surface.md).

## Lifecycle

```text
draft ──(vaa author init)──► review ──(vaa author review)──► lock ──(human)──► agent start
                                                                      │
                                                              vaa author lock
                                                              (CLI only)
```

1. **`vaa author init`** — copy/fill a catalog template into
   `<out>/<name>/` (default `.vaa/author/<name>`). Writes
   `AUTHOR_STATE.toml` with `state=draft`. Does **not** lock or admit seal.
2. **`vaa author review`** — validate `task.vaa.toml`, print task/contract
   digests, capability snapshot digest, `admit_leaf` lookup, and issues.
3. **`vaa author lock`** — human only. Fail-closed if review issues remain.
   Leaf×target must be admitted (frozen capability snapshot) **or** pass
   `--experimental` (sets acceptance to `authoring_only`; never
   `sealed_acceptance`). Writes `LOCKED` with digests; further `init`
   mutation is refused.
4. **Agent start** — after lock, drive the case with `vaa harness` /
   `scripts/agent_harness_adapter.py` like any other locked case kit.

## Templates

See [`schemas/author-templates/README.md`](../schemas/author-templates/README.md).
Templates are **drafts until lock**. Known-CI shapes
(`pure-int-binary`, `buffer-read`, `buffer-write`, `dual-buffer`) set
`experimental=false` at init; unary/ternary sketches stay experimental.

## Honesty

- Agents must **decline** `vaa author lock` (skill path). Point humans here.
- `authoring_only` ≠ agent-verify; experimental locks do not grant seal.
- Admission is checked **before** lock.
- Parse stdout JSON only when using `--format json`.
