#!/usr/bin/env bash
# Run an HlaX64 pack suite through `vaa suite run` (Linux twin of run-hlax64-suite.ps1).
#
# Default: generate + identity only (--skip-verify) -> Incomplete.
# Gate mode: --gate implies verify + --allow-execution -> expect Accepted/Verified.
# Practice seal is not a trust root. Incomplete is not Verified.
set -euo pipefail

SUITE="${SUITE:-integrations/hlax64/suites/scalar-sysv.vaa-suite.toml}"
REPO="${REPO:-}"
RUN_DIR="${RUN_DIR:-target/hlax64-suite-runs}"
OUTPUT="${OUTPUT:-target/hlax64-suite-runs/suite-evidence.json}"
SKIP_VERIFY=1
SKIP_BUILD=0
ALLOW_EXECUTION=0
CHECK_DETERMINISTIC=0
GATE=0

usage() {
  cat <<'EOF'
Usage: run-hlax64-suite.sh [options]

  --suite PATH              Suite manifest (default: scalar-sysv)
  --repo PATH               HlaX64 checkout (or HLAX64_ROOT)
  --run-dir PATH            Suite run base directory
  --output PATH             Suite evidence JSON path
  --skip-verify             Generate only (default)
  --no-skip-verify          Enable SemASM verify
  --allow-execution         Pass --allow-execution to SemASM (Gate-2)
  --skip-build              Skip generator build/identity rebuild
  --check-deterministic     Double-generate digest check
  --gate                    Gate mode: verify + allow-execution (Accepted/Verified)
  -h, --help                Show help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --suite) SUITE="$2"; shift 2 ;;
    --repo) REPO="$2"; shift 2 ;;
    --run-dir) RUN_DIR="$2"; shift 2 ;;
    --output) OUTPUT="$2"; shift 2 ;;
    --skip-verify) SKIP_VERIFY=1; shift ;;
    --no-skip-verify) SKIP_VERIFY=0; shift ;;
    --allow-execution) ALLOW_EXECUTION=1; shift ;;
    --skip-build) SKIP_BUILD=1; shift ;;
    --check-deterministic) CHECK_DETERMINISTIC=1; shift ;;
    --gate) GATE=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown arg: $1" >&2; usage >&2; exit 2 ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ "$GATE" -eq 1 ]]; then
  SKIP_VERIFY=0
  ALLOW_EXECUTION=1
fi

if [[ -z "$REPO" ]]; then
  if [[ -n "${HLAX64_ROOT:-}" ]]; then
    REPO="$HLAX64_ROOT"
  elif [[ -d "$ROOT/hlax64" ]]; then
    REPO="$ROOT/hlax64"
  else
    REPO="$(cd "$ROOT/.." && pwd)/hlax64"
  fi
fi
if [[ ! -d "$REPO" ]]; then
  echo "HlaX64 repo not found at $REPO (set --repo or HLAX64_ROOT)" >&2
  exit 1
fi

if [[ "$SKIP_VERIFY" -eq 0 ]]; then
  if ! command -v semasm >/dev/null 2>&1; then
    echo "semasm not on PATH (required for --gate / verify mode)" >&2
    exit 1
  fi
  if [[ -z "${VAA_SEAL_SIGNING_KEY:-}" ]]; then
    echo "VAA_SEAL_SIGNING_KEY unset - running scripts/ci-gate-sign-setup.sh (practice key)"
    # shellcheck disable=SC1091
    source "$ROOT/scripts/ci-gate-sign-setup.sh"
  fi
fi

mkdir -p "$RUN_DIR" "$(dirname "$OUTPUT")"

ARGS=(
  run -q -- suite run "$SUITE"
  --repo "$REPO"
  --run-dir "$RUN_DIR"
  --output "$OUTPUT"
  --skip-repo-guard
)
if [[ "$SKIP_BUILD" -eq 1 ]]; then
  ARGS+=(--skip-build)
fi
if [[ "$SKIP_VERIFY" -eq 1 ]]; then
  ARGS+=(--skip-verify)
fi
if [[ "$ALLOW_EXECUTION" -eq 1 ]]; then
  ARGS+=(--allow-execution)
fi
if [[ "$CHECK_DETERMINISTIC" -eq 1 ]]; then
  ARGS+=(--check-deterministic)
fi

echo "Running: cargo ${ARGS[*]}"
set +e
cargo "${ARGS[@]}"
CODE=$?
set -e

if [[ "$CODE" -ne 0 ]]; then
  if [[ "$SKIP_VERIFY" -eq 1 && -f "$OUTPUT" ]]; then
    STATUS="$(python3 - <<PY
import json
print(json.load(open("$OUTPUT", encoding="utf-8")).get("status",""))
PY
)"
    if [[ "$STATUS" == "incomplete" || "$STATUS" == "Incomplete" ]]; then
      echo "Suite evidence: $OUTPUT (status=$STATUS; skip-verify Incomplete is not Verified)"
      head -n 40 "$OUTPUT"
      exit 0
    fi
  fi
  echo "vaa suite run failed with exit $CODE" >&2
  exit "$CODE"
fi

if [[ -f "$OUTPUT" ]]; then
  STATUS="$(python3 - <<PY
import json
print(json.load(open("$OUTPUT", encoding="utf-8")).get("status",""))
PY
)"
  echo "Suite evidence: $OUTPUT (status=$STATUS)"
  if [[ "$SKIP_VERIFY" -eq 0 ]]; then
    if [[ "$STATUS" != "accepted" && "$STATUS" != "Accepted" ]]; then
      echo "Gate suite expected Accepted, got status=$STATUS" >&2
      exit 1
    fi
    python3 - "$OUTPUT" <<'PY'
import json, sys
ev = json.load(open(sys.argv[1], encoding="utf-8"))
ok = {"Verified", "verified", "VerifiedUnderPreconditions", "verified_under_preconditions"}
for c in ev.get("cases", []):
    st = str(c.get("status", ""))
    if st not in ok:
        raise SystemExit(f"Gate case {c.get('case_id')} status={st} (expected Verified or VerifiedUnderPreconditions)")
print("Gate pack suite Accepted (Verified / VerifiedUnderPreconditions; practice seal is not a trust root)")
PY
  fi
  head -n 40 "$OUTPUT"
fi
