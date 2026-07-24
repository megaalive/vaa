# Isolation Ops Proof — execution plan (Gelombang 4 / G4)

Owner: **VAA**. Prerequisite: SemASM Region/Alias + ContractExpr + A64/RV
memory-effect parity (G1–G3) **done**. Scope: what I0–I2 and container
scaffolding actually prove — **not** public-untrusted readiness, absolute
isolation, or a trust root.

Related: [`post-alpha-harden.md`](post-alpha-harden.md) (I0–I2), architecture
C-012, SemASM ADR 0006 roadmap item 3.

## Claim

Allowed: VAA documents and unit-tests isolation **ops** (argv/env evidence
labels) for Gate execution and build/container scaffolding — network/credential
denial on `ContainerBackend`, coarse `execution_isolation` plus backend id.

Forbidden: “hardened isolation Done” / “safe for public untrusted execution” /
Firecracker/VM / SoftHSM as trust root / flipping PR-010 Scaffold → Done.

Honesty: LocalBackend ≠ container; Docker/Podman+seccomp ≠ absolute boundary;
Verified ≠ isolation; practice keys ≠ trust root.

## Escalation bar (out of this wave)

If/when VAA offers **public untrusted execution**, escalate beyond ops proof:
disposable VM / stronger multi-tenant boundary, credential vault policy, and a
product ADR. That bar does **not** block starting G4 honesty work.

## Steps (Io0–Io5)

| Step | Focus | Status |
|---|---|---|
| **Io0** | This plan + progress unlock + claim matrix | **done** |
| **Io1** | Fix stale doctor/status (`ExecutionSandbox` is wired, not library-only) | **done** |
| **Io2** | Isolation claim matrix in `post-alpha-harden.md` | **done** |
| **Io3** | Evidence granularity: `execution_sandbox_backend` (`local` today) | **done** |
| **Io4** | Network+credential argv/env checklist tests (ContainerBackend) | **done** |
| **Io5** | CHANGELOG / SemASM progress pin; G5 stays last | **done** |

### Io1 — Doctor honesty

`vaa doctor` / status must not say ExecutionSandbox is library-only. Truth:

- Default Gate-2: `execution_isolation=semasm_host`
- Opt-in `--execution-sandbox`: LocalBackend wrapper → `sandbox` + backend `local`

### Io3 — Evidence field

Keep `execution_isolation: semasm_host | sandbox` (I1/I2). Add optional
`execution_sandbox_backend` when sandbox (`local`; `container` reserved — Gate
path still hardcodes LocalBackend).

### Io4 — Ops checklist (unit)

Prove ContainerBackend argv always includes `--network none`, `--cap-drop ALL`,
no Docker socket mount; default `allowed_env` does not forward credential-shaped
keys. Not a runtime penetration test.

## Non-goals

- Wire Gate `--execution-sandbox=container` (optional later escalate / I3).
- Hardware HSM / remote append-only trust root (G5).
- CryptOpt embed; live-model Gate CI.

## Push order

1. Io0–Io2 docs + Io1 doctor fix → commit.
2. Io3–Io4 code + tests → commit (or same tip if small).
3. Io5 hygiene + SemASM tip pin.
