#!/usr/bin/env bash
# R6: after SemASM is on PATH, assert `vaa doctor` is Available and Gate
# golden live_probe compares are aligned (bash twin of ci-doctor-live-probe.ps1).
set -euo pipefail

cargo build -q
VAA="$(pwd)/target/debug/vaa"
if [[ ! -x "$VAA" ]]; then
  echo "vaa binary missing after cargo build" >&2
  exit 1
fi

RAW="$("$VAA" doctor --format json)"
STATUS="$(printf '%s' "$RAW" | python3 -c 'import json,sys; print(json.load(sys.stdin)["status"])')"
if [[ "$STATUS" != "Available" ]]; then
  echo "expected doctor Available, got $STATUS; raw=$RAW" >&2
  exit 1
fi

python3 - "$RAW" <<'PY'
import json, sys
doc = json.loads(sys.argv[1])
probe = doc.get("live_probe")
if not probe:
    raise SystemExit("doctor JSON missing live_probe")
if probe.get("capability_schema") != "0.1":
    raise SystemExit(f"expected capability_schema 0.1, got {probe.get('capability_schema')}")
compares = {c["target_id"]: c for c in probe.get("compares", [])}
for target in ("x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu"):
    cmp = compares.get(target)
    if cmp is None:
        raise SystemExit(f"expected one live_probe compare for {target}")
    if cmp.get("outcome") != "aligned":
        raise SystemExit(f"{target} compare={cmp.get('outcome')} axes={cmp.get('axes')}")
print("R6 doctor live_probe OK (Available + Gate goldens aligned)")
PY
