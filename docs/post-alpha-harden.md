# Post-alpha hardening notes (P7)

Honesty constraints for waves after `v0.1.0`.

## Sandbox (P7-S)

- `vaa build --sandbox container` now defaults to **C1 binds** (`/input` ro + `/work` rw) with argv path remap.
- `--seccomp` writes the bundled profile from `assets/seccomp/vaa-default.json`.
- `--require-rootless` fail-closes unless `docker|podman info` looks rootless.
- `--generator-jail` wraps external generators in a container with only `staging/` at `/work`.
- Gate-2 still uses SemASM `--allow-execution` by default (`execution_isolation=semasm_host`).
- Opt-in `vaa verify --execution-sandbox` runs SemASM via `ExecutionSandbox`
  (LocalBackend process wrapper) and sets `execution_isolation=sandbox`.
- Docker/Podman + seccomp ≠ absolute isolation (architecture C-012).
  LocalBackend ≠ container isolation.

### Gate-2 isolation honesty (maturity inflection D2 + M0 phases)

Criteria before any doc or CI label may say Gate-2 uses process isolation:

1. **Today (I0):** Gate-2 = SemASM `agent verify --allow-execution` (host process).
   Verified means SemASM behavioral proof under that flag — not sandbox proof.
2. **`ExecutionSandbox` on Gate path** only when:
   - Gate / `vaa verify` (or named CI job) invokes the sandbox API for the
     candidate under test;
   - CI asserts sandbox was used (evidence field or argv), not merely that the
     library compiles;
   - failure without sandbox is fail-closed for jobs that claim isolation.
3. Until (2), wording must keep: Gate-2 Verified ≠ isolated execution;
   container build sandbox ≠ Gate execution sandbox.
4. SoftHSM / Fulcio / practice seals still ≠ SemASM Verified.

| Phase | Meaning | Status |
|---|---|---|
| **I0** | Gate-2 = SemASM `--allow-execution` on host; Verified ≠ isolation | **current** |
| **I1** | Evidence field `execution_isolation: semasm_host \| sandbox`; Gate-2 CI asserts `semasm_host` by default | **landed** |
| **I2** | Opt-in `--execution-sandbox` on `vaa verify` wires `ExecutionSandbox` (LocalBackend); CI asserts `execution_isolation=sandbox`; fail-closed if sandbox cannot run | **landed** (LocalBackend scaffold ≠ absolute isolation; C-012) |

I1 does not change default Gate behavior (still `semasm_host`). I2 is opt-in;
sandbox claim ≠ container isolation; SoftHSM/Fulcio ≠ Verified.

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
- **Maturity inflection:** leaf/search/HlaX64 treadmill paused after Z for thin
  bridges. Write-shape + Thin + Horizon Closeout (H0–H6) followed; see
  `docs/progress.md` Horizon map.

## Remote transparency honesty (H5)

Local and opt-in paths today:

- Gate CI uploads `vaa-transparency-v1` export artifacts (T0/T1).
- Opt-in Rekor publish/verify (`--features rekor`) and Fulcio keyless
  (`--features fulcio`, manual workflow).

**Local export + CI artifact ≠ operated remote append-only log.** Practice keys
≠ trust root. Opt-in Rekor/Fulcio ≠ default Gate remote transparency.

"Remote transparency" may only be claimed when all hold:

1. An operated append-only remote log (or always-on production Rekor) is the
   default verify path for Gate jobs that claim remote transparency.
2. Verify-from-remote fails closed when the log is unreachable.
3. Docs name the log endpoint and key policy (not practice-only).

Until then, wording must keep: CI artifact / dry-run Rekor / Fulcio identity
attest ≠ remote transparency service; SoftHSM ≠ hardware HSM; Fulcio ≠ SemASM
Verified.