# Semantic Evidence Integrity (VAA pointer)

Canonical program plan (SemASM + VAA):

[SemASM `docs/SEMANTIC_EVIDENCE_INTEGRITY_PLAN.md`](https://github.com/megaalive/semasm/blob/main/docs/SEMANTIC_EVIDENCE_INTEGRITY_PLAN.md)
(sibling checkout: `../semasm/docs/SEMANTIC_EVIDENCE_INTEGRITY_PLAN.md`).

Milestone internal: **Semantic Evidence Integrity**.

VAA slices in that plan:

1. **P1** — Evidence Requirement Profiles (`SemanticEvidenceSummary`, task
   `verification.semantic_evidence.*`, sealed checks) — **landed** (`ed6d961`);
   built-in named profiles remain optional follow-up.
2. **`verified_under_preconditions`** — mapped distinctly; not promoted to
   `verified` without explicit policy.
3. Gate CI pin SemASM tip `cf0206e` (Vd8) for schema 0.5 / region-access evidence.

Isolation (P3) and trust-root (P4) stay behind semantic evidence stability unless
public-untrusted execution forces them earlier.
