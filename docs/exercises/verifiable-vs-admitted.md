# Verifiable vs admitted (author inventory)

SemASM can synthesize oracles for **recognized name/type shapes**. The skill
path only accepts leaves in the frozen admission snapshot
(`vaa admit` / [`fixtures/semasm/capabilities-snapshot.json`](../../fixtures/semasm/capabilities-snapshot.json)).

**Rule:** SemASM `verified` without `vaa admit` → **decline** on the skill path
(see E01, [`HONESTY.md`](../HONESTY.md)).

## Admitted on Win64 NASM (snapshot; use `vaa admit --list`)

Plain **verified** (strict):

- `max_i64`

**VUP** (`verified_under_preconditions` — report as VUP, not plain verified):

- `sum_i64`
- `count_byte`
- `find_first_byte` / `find_last_byte`
- `memcmp` / `memcpy` / `memset`
- `replace_byte`

(Other targets/assemblers may appear in the same snapshot; always query
`vaa admit --list --target …`.)

## Common SemASM-recognizable shapes that are *not* skill-admitted

These names often **agent-verify** when types match, but **`vaa admit` declines**
unless listed above. Useful for clinics and hosted composition — not for
`loop-direct` skill seals.

| Shape family | Example names | Notes |
|---|---|---|
| Binary pure-int | `min_i64`, `add_*`, `sub_*` | `max_i64` is admitted; siblings usually are not |
| Unary pure-int | `abs_*`, `inc_*` / `increment*`, `identity_*`, `return_*`, `scale_*` / `double_*`, `countdown_*`, `sum_range_*`, `add_base_*` | E02 `increment_i64` clinic |
| Hosted / I/O | `mainCRTStartup`, REPL, `WriteFile` | Never leaf shapes |

Exact recognition rules live in SemASM (`UNSUPPORTED_SHAPE` message lists
tokens). Prefer renaming to a recognized token for local verify; prefer an
**admitted** leaf for the skill harness.

## Author checklist

1. `vaa admit --leaf NAME --target TARGET` — if `admitted: false`, stop for skill work.
2. Optional local check: `semasm agent verify` may still pass for clinic learning.
3. Do not write “sealed” / “skill verified” from step 2 alone.
