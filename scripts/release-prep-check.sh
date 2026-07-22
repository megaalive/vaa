#!/usr/bin/env bash
# Release prep check — fmt/clippy/test + deny pointer. Does NOT create a git tag.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== release-prep-check: cargo fmt =="
cargo fmt --all -- --check

echo "== release-prep-check: cargo clippy =="
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "== release-prep-check: cargo test =="
cargo test --workspace --all-features

if command -v cargo-deny >/dev/null 2>&1; then
  echo "== release-prep-check: cargo deny =="
  cargo deny check
else
  echo "skip cargo-deny (not on PATH); CI check job runs EmbarkStudios/cargo-deny-action"
fi

echo
echo "Local prep checks OK."
echo "Before any v0.1.0 tag, confirm:"
echo "  - tip SHA CI is green (including Gate Win64 + Linux jobs)"
echo "  - docs/release-v0.1-checklist.md rows are signed off"
echo "  - CHANGELOG.md Unreleased notes are accurate"
echo "Tag ceremony is maintainer-only; this script never tags."
