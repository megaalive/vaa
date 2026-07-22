# Release prep check — fmt/clippy/test + deny pointer. Does NOT create a git tag.
$ErrorActionPreference = "Stop"

Set-Location (Split-Path -Parent $PSScriptRoot)

Write-Host "== release-prep-check: cargo fmt =="
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { throw "cargo fmt failed" }

Write-Host "== release-prep-check: cargo clippy =="
cargo clippy --workspace --all-targets --all-features -- -D warnings
if ($LASTEXITCODE -ne 0) { throw "cargo clippy failed" }

Write-Host "== release-prep-check: cargo test =="
cargo test --workspace --all-features
if ($LASTEXITCODE -ne 0) { throw "cargo test failed" }

if (Get-Command cargo-deny -ErrorAction SilentlyContinue) {
    Write-Host "== release-prep-check: cargo deny =="
    cargo deny check
    if ($LASTEXITCODE -ne 0) { throw "cargo deny failed" }
} else {
    Write-Host "skip cargo-deny (not on PATH); CI check job runs EmbarkStudios/cargo-deny-action"
}

Write-Host ""
Write-Host "Local prep checks OK."
Write-Host "Before any v0.1.0 tag, confirm:"
Write-Host "  - tip SHA CI is green (including Gate Win64 + Linux jobs)"
Write-Host "  - docs/release-v0.1-checklist.md rows are signed off"
Write-Host "  - CHANGELOG.md Unreleased notes are accurate"
Write-Host "Tag ceremony is maintainer-only; this script never tags."
