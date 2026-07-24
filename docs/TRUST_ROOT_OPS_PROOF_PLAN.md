# Trust Root Ops Proof — execution plan (Gelombang 5 / G5)

Owner: **VAA**. Prerequisite: G1–G4 done. Scope: authenticity / transparency
**ops labels** — what practice keys, SoftHSM smoke, and opt-in Rekor/Fulcio
actually prove. **Not** a production trust root.

Related: [`seal.md`](seal.md), [`post-alpha-harden.md`](post-alpha-harden.md)
(H5 remote-transparency bar), SemASM ADR 0006 roadmap item 4.

## Claim

Allowed: VAA documents and unit-tests authenticity surfaces (integrity vs
practice Ed25519 vs SoftHSM smoke vs opt-in Rekor/Fulcio) with explicit
`signer_kind` labels. Authenticity ≠ semantic truth / SemASM Verified.

Forbidden: “trust root Done” / hardware HSM Done / operated remote log as Gate
default / Fulcio = Verified / SoftHSM = hardware HSM / practice key = trust root.

## Steps (Tr0–Tr5)

| Step | Focus | Status |
|---|---|---|
| **Tr0** | This plan + progress unlock | **done** |
| **Tr1** | Fix stale progress claims (I2 / ExecutionSandbox) | **done** |
| **Tr2** | Trust claim matrix in harden + seal docs | **done** |
| **Tr3** | Persist `signer_kind` on `SealSignature` | **done** |
| **Tr4** | Checklist unit tests (practice ≠ trust root) | **done** |
| **Tr5** | CHANGELOG / SemASM pin; production trust root stays locked | **done** |

### Tr3 — Seal field

Additive optional `signer_kind` on seal `signature` block:

- `practice-ed25519` — `VAA_SEAL_SIGNING_KEY` / Gate practice path
- `sigstore-dsse` — DSSE carrier over transparency (still practice key material)
- `hsm-pkcs11` — SoftHSM PKCS#11 smoke (`--features pkcs11`)

Absent / unknown kind must not imply production trust root.

### Tr4 — Ops checklist (unit)

- Unsigned seals OK unless `VAA_REQUIRE_SEAL_SIGNATURE`
- Practice signer emits `signer_kind=practice-ed25519`
- DSSE keyid stays `vaa-practice`
- HSM path without `pkcs11` feature fail-closes
- Status/doctor strings still say “not a trust root”

## Non-goals (Horizon-locked)

- Hardware HSM / vault / FIPS key policy
- Operated Rekor (or other append-only log) as **default** Gate verify
- Fulcio OIDC identity as SemASM Verified
- Cosign as Gate default; committed production signing seeds

## Push order

1. Tr0–Tr2 docs + Tr1 stale fixes
2. Tr3–Tr4 code + tests
3. Tr5 hygiene + SemASM tip pin
