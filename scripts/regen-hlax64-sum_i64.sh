#!/usr/bin/env bash
# Regenerate fixtures/ingest/hlax64_sum_i64/candidate.asm from HlaX64.
set -euo pipefail

VAA_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HLAX64_ROOT="${HLAX64_ROOT:-$(cd "$VAA_ROOT/../hlax64" && pwd)}"
SOURCE="$HLAX64_ROOT/examples/interop/semasm-vaa/sum_i64/sum_i64.hla64"
OUT_ASM="$VAA_ROOT/fixtures/ingest/hlax64_sum_i64/candidate.asm"
TMP="$(mktemp).asm"

cleanup() { rm -f "$TMP"; }
trap cleanup EXIT

if [[ ! -f "$SOURCE" ]]; then
  echo "HlaX64 source not found: $SOURCE (set HLAX64_ROOT)" >&2
  exit 1
fi

if command -v hla64 >/dev/null 2>&1; then
  hla64 emit-nasm "$SOURCE" --target windows-x64-msabi --output-kind shared-library -o "$TMP"
else
  CLI="$HLAX64_ROOT/src/HlaX64.Cli/HlaX64.Cli.csproj"
  if [[ ! -f "$CLI" ]]; then
    echo "hla64 not on PATH and CLI project missing: $CLI" >&2
    exit 1
  fi
  dotnet run --project "$CLI" --no-launch-profile -- emit-nasm "$SOURCE" \
    --target windows-x64-msabi --output-kind shared-library -o "$TMP"
fi

grep -q "global sum_i64" "$TMP" || { echo "emit missing global sum_i64" >&2; exit 1; }
if grep -qE "global _start|ExitProcess" "$TMP"; then
  echo "emit still contains program entry; use --output-kind shared-library" >&2
  exit 1
fi

mkdir -p "$(dirname "$OUT_ASM")"
cp "$TMP" "$OUT_ASM"
echo "Wrote $OUT_ASM"
