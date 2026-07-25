#!/usr/bin/env bash
# T1: export + verify-transparency for a Gate run directory (bash twin of ci-gate-transparency.ps1).
set -euo pipefail

RUN_BASE=""
OUT_DIR=""
PUBLIC_KEY_SRC=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --run-base)
      RUN_BASE="$2"
      shift 2
      ;;
    --out-dir)
      OUT_DIR="$2"
      shift 2
      ;;
    --public-key-src)
      PUBLIC_KEY_SRC="$2"
      shift 2
      ;;
    *)
      echo "unknown arg: $1" >&2
      exit 1
      ;;
  esac
done

if [[ -z "$RUN_BASE" || -z "$OUT_DIR" ]]; then
  echo "usage: $0 --run-base <dir> --out-dir <dir> [--public-key-src <file>]" >&2
  exit 1
fi

if [[ ! -d "$RUN_BASE" ]]; then
  echo "run base missing: $RUN_BASE" >&2
  exit 1
fi

RUN="$(find "$RUN_BASE" -mindepth 1 -maxdepth 1 -type d -printf '%T@ %p\n' | sort -nr | head -n1 | cut -d' ' -f2-)"
if [[ -z "$RUN" ]]; then
  echo "no run under $RUN_BASE" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"

cargo build -q
VAA="$(pwd)/target/debug/vaa"
if [[ ! -x "$VAA" ]]; then
  echo "vaa binary missing after cargo build" >&2
  exit 1
fi

TRANSPARENCY="$OUT_DIR/transparency.json"
"$VAA" evidence export-transparency "$RUN" -o "$TRANSPARENCY"

SEAL_LOG="$RUN/evidence/seal-log.jsonl"
if [[ ! -f "$SEAL_LOG" ]]; then
  echo "seal-log.jsonl missing under $RUN" >&2
  exit 1
fi
cp "$SEAL_LOG" "$OUT_DIR/seal-log.jsonl"

if [[ -n "$PUBLIC_KEY_SRC" && -f "$PUBLIC_KEY_SRC" ]]; then
  DEST_PK="$OUT_DIR/public_key.txt"
  SRC_FULL="$(cd "$(dirname "$PUBLIC_KEY_SRC")" && pwd)/$(basename "$PUBLIC_KEY_SRC")"
  DEST_FULL="$(cd "$OUT_DIR" && pwd)/public_key.txt"
  if [[ "$SRC_FULL" != "$DEST_FULL" ]]; then
    cp "$PUBLIC_KEY_SRC" "$DEST_PK"
  fi
fi

"$VAA" evidence verify-transparency "$TRANSPARENCY" --against "$RUN"

echo "T1 transparency OK from $RUN → $OUT_DIR"
echo "  (CI artifact ≠ remote immutable log)"
