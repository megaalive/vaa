# Post-alpha hardening notes (P7)

Honesty constraints for waves after `v0.1.0`.

## Sandbox (P7-S)

- `vaa build --sandbox container` now defaults to **C1 binds** (`/input` ro + `/work` rw) with argv path remap.
- `--seccomp` writes the bundled profile from `assets/seccomp/vaa-default.json`.
- `--require-rootless` fail-closes unless `docker|podman info` looks rootless.
- `--generator-jail` wraps external generators in a container with only `staging/` at `/work`.
- Gate-2 still uses SemASM `--allow-execution`; `ExecutionSandbox` remains a library API.
- Docker/Podman + seccomp ≠ absolute isolation (architecture C-012).

## Seal durability (P7-D)

- `vaa evidence durability-probe` classifies `local-durable` / `best-effort` / `refuse-verified`.
- `VAA_REQUIRE_LOCAL_DURABLE=1` promotes best-effort to refuse-verified.
- Not a formal FS correctness proof; network/lying FS remain fail-closed labels.

## Signing + Rekor (P7-A / P7-T) + SoftHSM (P8-K) + Fulcio (P8-I)

- `SealSigner` backends: practice Ed25519, Sigstore-shaped DSSE, SoftHSM PKCS#11
  (`--features pkcs11`; Linux smoke CI job `pkcs11-softhsm`).
- `vaa evidence publish-rekor --dry-run` builds DSSE + hashedrekord without network.
- Live Rekor HTTP requires `--features rekor` (optional workflow).
- Fulcio keyless: `vaa evidence fulcio-sign` + `--features fulcio` + manual
  `fulcio-sign.yml` (OIDC). Default `--dry-run` uses mock transport.
- Rekor / Sigstore / SoftHSM / Fulcio ≠ SemASM Verified; practice keys ≠ trust root;
  SoftHSM ≠ hardware HSM; Fulcio identity attest ≠ behavioral proof.

## Search (P7-C) + Tranche Q / R / T

- `vaa search` stages CryptOpt-like mutations (`nop-slide`, `nop-before-ret`, or `--mutator-command`).
- Does not embed CryptOpt; live LLM search stays opt-in/manual.
- Tranche **Q1** extends multi-candidate repair Gate smoke beyond `count_byte`
  (`find_first_byte` wrong→repair).
- Tranche **Q2** adds offline nop-slide staging Gate smoke (≠ Verified; ≠ formal
  superopt). Staging ≠ SemASM ingest; `verified=false` is intentional.
- Tranche **R1** wires one staged candidate into `vaa ingest` + `verify-chain`
  (still ≠ CryptOpt; Gate Incomplete without `--allow-execution`).
- Mutator **`nop-before-ret`** (X2b) inserts NOPs before the last `ret` so SemASM
  does not see trailing-after-ret Violated (unlike post-ret `nop-slide`).
- Tranche **T**: `vaa search --ingest --contract` runs bounded mutate→verify/seal
  internally (no shell recursion). Skips Violated/Failed; stops on first
  Incomplete (`stopped_reason=incomplete_accepted`, `verified=false`). Optional
  `--allow-execution` may reach Verified (Gate-2; not default CI). Incomplete ≠
  Verified; mutator output ≠ CryptOpt.
- Tranche **V** adds an ignored Gate-2
  `search --ingest --allow-execution` smoke that must stop on SemASM Verified
  (`stopped_reason=verified`). Default CI remains Gate-1 fail-closed:
  Incomplete ≠ Verified. Any Verified wording identifies the SemASM path only;
  it is not a CryptOpt claim. SoftHSM/Fulcio ≠ SemASM Verified.
- Tranche **Y** extends search-ingest Gate parity to `memcmp` (write-broken
  Violated skip + Incomplete / allow-exec Verified). HlaX64 `find_last_byte`
  bridge (H4) is a second emit leaf; HlaX64 `-Wverify` ≠ SemASM Verified.
  LLM search stays opt-in/manual.
- Tranche **Z** extends search-ingest Gate parity to `find_first_byte`. HlaX64
  `memcmp` bridge (H5) is a third emit leaf (dual-buffer `-1/0/1`). Default CI
  remains Gate-1 fail-closed.