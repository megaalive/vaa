# VAA `v0.1.0` release checklist (R-prep → R-tag)

Prep checklist for alpha. **R-tag complete:** annotated `git tag v0.1.0` + GitHub
Release (see ceremony section). Practice seals and Gate artifacts remain
**illustrative**, not a trust root.

## Required before tagging

| Check | Status / pointer |
|---|---|
| CI `check` (fmt/clippy/test) green on tip | `.github/workflows/ci.yml` `check` |
| Gate-1 Incomplete + signed ingest + transparency (Win64) | `semasm-gate1` |
| Gate-2 Verified smoke (Win64) | `semasm-gate2` |
| Gate-1 Incomplete + transparency (Linux) | `semasm-gate1-linux` |
| Gate-2 Verified smoke (Linux, qemu-x86_64) | `semasm-gate2-linux` |
| HlaX64 emit matches fixture + Gate ingest | `hlax64-bridge` |
| SemASM pin is a SHA (not floating `main`) | `docs/progress.md` N1 |
| HlaX64 pin is a SHA | `docs/progress.md` P0 |
| Seal schema 0.2 documented | `docs/seal.md` |
| Integrity vs authenticity honesty | `docs/seal.md` (A0/A1 practice key ≠ trust root) |
| Transparency: CI artifact ≠ remote immutable log | T0/T1 |
| Container: Scaffold only (C0/C1/C2) | `docs/progress.md` PR-010 |
| Generator write barrier is logical (G0/G1), not OS ACL | `docs/seal.md` + `vaa generate --command` |
| `cargo deny check` in CI | `.github/workflows/ci.yml` `check` job |
| Resume from sealed chain (**E1**) | `src/run/resume.rs` + `vaa run --resume` |
| External argv → staging (**G1**) | `vaa generate --command` (not OS FS isolation) |
| E1 resume smoke in Gate-1 CI (**E1b**) | `tests/semasm_gate1_smoke.rs` |
| Linux Gate Incomplete + Verified (**L1/L2**) | `semasm-gate1-linux` / `semasm-gate2-linux` |
| Live model opt-in (**PR-019**) | feature `live-model` + `--live` (Gate CI offline) |
| Local cache (**PR-020**) | [`docs/cache.md`](cache.md); `--cache` opt-in |
| Reproducibility (**PR-021**) | `vaa build --check-reproducible` (same-host) |
| Negative corpus thin (**PR-022**) | `fixtures/negative/` (not full fuzz) |
| CHANGELOG + prep scripts (**R-notes** / **PR-023** docs) | `CHANGELOG.md`, `scripts/release-prep-check.*` |
| **R-tag** | **Done** — annotated `v0.1.0` + GitHub Release (2026-07-23) |

## Explicit non-goals for `v0.1.0`

- CryptOpt-style search engine
- Remote append-only transparency (Rekor / Git notes automation)
- HSM / hardware keys / Sigstore
- Full hardened sandbox (seccomp profile, verified rootless, OS-level FS isolation)
- Claiming Gate-1 Incomplete as a “Verified vertical slice”
- Claiming SemASM Linux assemble/link is upstream CI-verified (VAA proves smoke on pin only)
- Cross-host bit-identical builds / cache as a trust root
- Full cargo-fuzz PR-022 security certification

## Local prep (optional)

```bash
./scripts/release-prep-check.sh
# or
./scripts/release-prep-check.ps1
```

Runs fmt/clippy/test (+ `cargo deny` if installed). Never creates a tag.

## Tag ceremony

1. Confirm tip SHA and CI run URL (all Gate jobs green).
2. Move `CHANGELOG.md` Unreleased notes under a dated `[0.1.0]` section.
3. `git tag -a v0.1.0` (GPG-signed when org signing keys are available).
4. GitHub Release linking Gate artifact digests / CI run as **illustrative**, not as a trust root.

### Record (alpha cut)

| Field | Value |
|---|---|
| Tag | `v0.1.0` |
| Date | 2026-07-23 |
| Tip (R-tag docs) | `64191426aa7808735a45abf379e8f28e75afea29` |
| CI (tag tip green) | https://github.com/megaalive/vaa/actions/runs/29971512238 |
| Release | https://github.com/megaalive/vaa/releases/tag/v0.1.0 |
| Signing | annotated tag (GPG not available on cut host) |
