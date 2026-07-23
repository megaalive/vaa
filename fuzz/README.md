# Fuzz targets (P8-F / PR-022 closeout)

Fail-closed parser smoke via `cargo-fuzz` / libFuzzer. **Not** a security certification.

## Targets

| Target | Input |
|---|---|
| `task_toml` | task schema TOML |
| `seal_envelope` | `evidence.seal.json` |
| `transparency_doc` | `vaa-transparency-v1` JSON |
| `cache_verification_record` | verification cache index JSON |

## Local (Linux/macOS; needs nightly + clang)

```bash
cargo install cargo-fuzz
cargo +nightly fuzz run task_toml -- -max_total_time=30
```

Seed corpora live under `fuzz/corpus/<target>/` (copied from `fixtures/negative/` where useful).

Windows hosts: run fuzz in Ubuntu CI / WSL — libFuzzer is not supported on MSVC.
