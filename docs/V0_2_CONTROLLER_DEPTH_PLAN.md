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
| **Vd2** | Docs pin honesty (tip + Incomplete ≠ Verified) | queued |
| **Vd3** | CI Gate jobs green on new pin | queued |

## Non-goals

- New leaf families / SemASM Co pin (follow-up after Co CI green)
- Production trust root / hardware HSM / operated remote log
