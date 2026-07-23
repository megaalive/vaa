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

## Signing + Rekor (P7-A / P7-T)

- `SealSigner` backends: practice Ed25519, Sigstore-shaped DSSE, HSM PKCS#11 scaffold.
- `vaa evidence publish-rekor --dry-run` builds DSSE + hashedrekord without network.
- Live Rekor HTTP requires `--features rekor` (optional workflow).
- Rekor / Sigstore ≠ SemASM Verified; practice keys ≠ trust root.

## Search (P7-C)

- `vaa search` stages CryptOpt-like mutations (`nop-slide` or `--mutator-command`).
- Does not embed CryptOpt; does not run live search in Gate CI.
