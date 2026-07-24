# Semantic Evidence Integrity (VAA pointer)

Canonical program plan (SemASM + VAA):

[SemASM `docs/SEMANTIC_EVIDENCE_INTEGRITY_PLAN.md`](https://github.com/megaalive/semasm/blob/main/docs/SEMANTIC_EVIDENCE_INTEGRITY_PLAN.md)
(sibling checkout: `../semasm/docs/SEMANTIC_EVIDENCE_INTEGRITY_PLAN.md`).

Milestone internal: **Semantic Evidence Integrity**.

VAA slices in that plan:

1. **P1** — Evidence Requirement Profiles (`SemanticEvidenceSummary`, task
   `verification.semantic_evidence.*`, built-in profiles, sealed checks).
2. **`verified_under_preconditions`** — do not promote to `verified` without
   explicit policy.
3. Gate CI for missing / incomplete / mismatched evidence after SemASM P0 pin.

Do not implement VAA slices until the matching SemASM tip is pinned. Isolation
(P3) and trust-root (P4) stay behind semantic evidence stability unless
public-untrusted execution forces them earlier.
