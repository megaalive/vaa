#!/usr/bin/env bash
# A1: ephemeral Ed25519 seal signing key for Gate CI (bash twin of ci-gate-sign-setup.ps1).
# Not a trust root — practice key only; regenerated each job.
#
# Usage (must source or load the env file so cargo test inherits the key):
#   source ./scripts/ci-gate-sign-setup.sh [public_key_out]
#   # or:
#   ./scripts/ci-gate-sign-setup.sh [public_key_out]
#   source "${RUNNER_TEMP:-/tmp}/vaa-ci-seal.env"
set -euo pipefail

PUBLIC_KEY_OUT="${1:-}"

TEMP_ROOT="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
SEED_PATH="${TEMP_ROOT}/vaa-ci-seal.seed"
ENV_FILE="${TEMP_ROOT}/vaa-ci-seal.env"

cargo build -q
VAA="$(pwd)/target/debug/vaa"
if [[ ! -x "$VAA" ]]; then
  echo "vaa binary missing after cargo build" >&2
  return 1 2>/dev/null || exit 1
fi

OUT="$("$VAA" evidence keygen-seal --out "$SEED_PATH" 2>&1)" || {
  echo "keygen-seal failed: $OUT" >&2
  return 1 2>/dev/null || exit 1
}

PK_HEX="$(printf '%s\n' "$OUT" | sed -n 's/^[[:space:]]*public_key_hex:[[:space:]]*\([^[:space:]]*\).*$/\1/p' | head -n1)"
PK_B64="$(printf '%s\n' "$OUT" | sed -n 's/^[[:space:]]*public_key_b64:[[:space:]]*\([^[:space:]]*\).*$/\1/p' | head -n1)"
if [[ -z "$PK_HEX" || -z "$PK_B64" ]]; then
  echo "keygen-seal did not print public_key_hex/b64; output=$OUT" >&2
  return 1 2>/dev/null || exit 1
fi

export VAA_SEAL_SIGNING_KEY="$SEED_PATH"
export VAA_REQUIRE_SEAL_SIGNATURE=1

cat >"$ENV_FILE" <<EOF
export VAA_SEAL_SIGNING_KEY=$SEED_PATH
export VAA_REQUIRE_SEAL_SIGNATURE=1
EOF

if [[ -n "${GITHUB_ENV:-}" ]]; then
  {
    echo "VAA_SEAL_SIGNING_KEY=$SEED_PATH"
    echo "VAA_REQUIRE_SEAL_SIGNATURE=1"
  } >>"$GITHUB_ENV"
fi

if [[ -n "$PUBLIC_KEY_OUT" ]]; then
  mkdir -p "$(dirname "$PUBLIC_KEY_OUT")"
  cat >"$PUBLIC_KEY_OUT" <<EOF
# Ephemeral CI practice key — not a trust root / not Rekor / not HSM.
public_key_hex=$PK_HEX
public_key_b64=$PK_B64
EOF
fi

echo "A1 seal sign setup OK (ephemeral CI practice key, not a trust root)"
echo "  VAA_SEAL_SIGNING_KEY=$SEED_PATH"
echo "  VAA_REQUIRE_SEAL_SIGNATURE=1"
echo "  public_key_hex=$PK_HEX"
echo "  env_file=$ENV_FILE"
