# VAA `v0.1.0` release checklist (R-prep)

This is a **prep checklist**, not a release ceremony. Do **not** treat this file as
authorization to `git tag v0.1.0` until every required row is explicitly signed off.

## Required before tagging

| Check | Status / pointer |
|---|---|
| CI `check` (fmt/clippy/test) green on tip | Gate jobs in `.github/workflows/ci.yml` |
| Gate-1 Incomplete + signed ingest + transparency | `semasm-gate1` |
| Gate-2 Verified smoke | `semasm-gate2` |
| HlaX64 emit matches fixture + Gate ingest | `hlax64-bridge` |
| SemASM pin is a SHA (not floating `main`) | `docs/progress.md` N1 |
| HlaX64 pin is a SHA | `docs/progress.md` P0 |
| Seal schema 0.2 documented | `docs/seal.md` |
| Integrity vs authenticity honesty | `docs/seal.md` (A0/A1 practice key ≠ trust root) |
| Transparency: CI artifact ≠ remote immutable log | T0/T1 |
| Container: Scaffold only (C0/C1/C2) | `docs/progress.md` PR-010 |
| Generator write barrier is logical (G0), not OS ACL | `docs/seal.md` |
| `cargo deny check` in CI | `.github/workflows/ci.yml` `check` job |
| Resume from sealed chain (**E1**) | `src/run/resume.rs` + `vaa run --resume` |
| External argv → staging (**G1**) | `vaa generate --command` (not OS FS isolation) |
| E1 resume smoke in Gate-1 CI (**E1b**) | `tests/semasm_gate1_smoke.rs` |
| Linux Gate Incomplete + Verified (**L1/L2**) | `semasm-gate1-linux` / `semasm-gate2-linux` |
| Next post-prep wave | **R-notes** / R-tag ceremony (tag deferred) |

## Explicit non-goals for `v0.1.0`

- Live model adapter / provider SDKs
- CryptOpt-style search engine
- Remote append-only transparency (Rekor / Git notes automation)
- HSM / hardware keys / Sigstore
- Full hardened sandbox (seccomp profile, verified rootless, OS-level FS isolation)
- Claiming Gate-1 Incomplete as a “Verified vertical slice”

## Tag ceremony (deferred)

When the table above is done and maintainers agree:

1. Confirm tip SHA and CI run URL.
2. Update `CHANGELOG` / release notes (if introduced).
3. `git tag -a v0.1.0` + signed tag policy of the org.
4. GitHub Release linking Gate artifact digests as **illustrative**, not as a trust root.

Until then, crate version may remain `0.1.0` in `Cargo.toml` while the **git tag** stays uncut.
