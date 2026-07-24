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

## Non-goals

- New leaf families / formal ensures
- Production trust root / hardware HSM / operated remote log
- Gate-2 run Verified for write-shape (search-ingest Gate-2 already covers allow-exec)
