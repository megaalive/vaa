# Semantic Evidence Integrity (VAA pointer)

Canonical program plan (SemASM + VAA):

[SemASM `docs/SEMANTIC_EVIDENCE_INTEGRITY_PLAN.md`](https://github.com/megaalive/semasm/blob/main/docs/SEMANTIC_EVIDENCE_INTEGRITY_PLAN.md)
(sibling checkout: `../semasm/docs/SEMANTIC_EVIDENCE_INTEGRITY_PLAN.md`).

Milestone internal: **Semantic Evidence Integrity**.

VAA slices in that plan:

1. **P1** — Evidence Requirement Profiles (`SemanticEvidenceSummary`, task
   `verification.semantic_evidence.*`, sealed checks) — **landed** (`ed6d961`);
   built-in named profiles (`leaf-pure-v1`, `memory-leaf-affine-v1`) expand
   deterministically into frozen `semantic_evidence` on lock (P1b).
   Gate `count_byte` wired to `leaf-pure-v1` (Vd9). Gate `memcpy` /
   `memset` / `replace_byte` / `memcmp` wired to `memory-leaf-affine-v1`
   (Vd11–Vd14).
2. **`verified_under_preconditions`** — mapped distinctly; not promoted to
   `verified` without explicit policy.
3. Gate CI pin SemASM tip `d2ce02d` (Vd14); Gate memory leaves on
   `memory-leaf-affine-v1`; HlaX64 `count_byte` on `leaf-pure-v1`.

Isolation (P3) and trust-root (P4) stay behind semantic evidence stability unless
public-untrusted execution forces them earlier.
