# VAA controller / Gate depth on SemASM v0.2.0 — plan

Prerequisite: SemASM Rel-0.2 (`v0.2.0` @ `c5d8458`). Honesty: Gate-1
Incomplete ≠ Verified; search/repair ≠ CryptOpt; SoftHSM ≠ production trust.

## Claim

Allowed: CI Gate-1/2 (+ existing search-ingest smokes) on SemASM `v0.2.0` tip.

Forbidden: claiming Horizon cliffs (HSM, live model, CryptOpt) as Done.

## Steps (Vd0–Vd3)

| Step | Focus | Status |
|---|---|---|
| **Vd0** | This plan + progress note | **done** |
| **Vd1** | Bump workflow SemASM `ref` → `c5d8458` (v0.2.0) | **done** |
| **Vd2** | Docs pin honesty (tip + Incomplete ≠ Verified) | **done** |
| **Vd3** | CI Gate jobs green on new pin | **done** (`2815aa3`) |
| **Vd4** | `vaa run` wrong→repair Gate-1 for replace/memset/memcpy | **done** |
| **Vd5** | Re-pin Gate workflows to SemASM Mm tip `e991182` | **done** (`8d1286f`) |
| **Vd6** | Re-pin Gate workflows to SemASM `v0.2.1` tip `22d1543` | **done** (`20746e5`) |
| **Vd7** | Re-pin Gate workflows to SemASM tip `bfd184e` (Tw/Ff/Ab) | **done** (`6835e89`) |
| **Vd8** | Re-pin Gate workflows to SemASM tip `cf0206e` (Sei P0/Ra, report 0.5) | **done** (`981c3fe` tip; Gate CI green) |
| **Vd9** | Wire `leaf-pure-v1` on Gate `count_byte` + pin SemASM `b3c576e` | **done** |
| **Vd10** | Frame-spill affinity pin `671c5e2` + HlaX64 `count_byte` `leaf-pure-v1` | **done** |
| **Vd11** | Wire `memory-leaf-affine-v1` on Gate `memcpy` + pin SemASM `55f2542` | **done** |
| **Vd12** | Wire `memory-leaf-affine-v1` on Gate `memset` + pin SemASM `0f9cd1e` | **done** |
| **Vd13** | Wire `memory-leaf-affine-v1` on Gate `replace_byte` + pin SemASM `8924564` | **done** |
| **Vd14** | Wire `memory-leaf-affine-v1` on Gate `memcmp` + pin SemASM `d2ce02d` | **done** |
| **Vd15** | Wire `memory-leaf-affine-v1` on Gate `find_first`/`find_last` + pin SemASM `928bd66` | **done** |
| **Vd16** | Concrete cell depth: `memory-leaf-concrete-v1` + `load_byte0`/`store_byte0` Gate Verified + pin SemASM `28fb22f` | **done** |

## Non-goals

- Promoting symbolic-length `verified_under_preconditions` → `verified`
- New leaf families beyond concrete cells / formal ensures
- Production trust root / hardware HSM / operated remote log
- Gate-2 run Verified for write-shape (search-ingest Gate-2 already covers allow-exec)
