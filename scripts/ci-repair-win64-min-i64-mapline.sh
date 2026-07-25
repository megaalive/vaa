#!/usr/bin/env bash
# Win64 live-repair path: map-line join → generator_source.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

fixture="fixtures/repair/hlax64-min-i64-win64-live"
out_dir="target/repair-win64-mapline"
mkdir -p "$out_dir"
packet="$out_dir/repair-packet.json"
asm="$fixture/candidate.asm"
digest="sha256:$(sha256sum "$asm" | awk '{print $1}')"

cargo run -q -- repair export \
  --spec integrations/hlax64/generator.spec.toml \
  --task-id min-i64-win64-live-v1 \
  --status BehaviorFailed \
  --message "SemASM behavior_failed: signed min_i64 returned max (locked wrong GreaterThan branch / jg)" \
  --diagnostic-code BEHAVIOR_VECTOR_MISMATCH_001 \
  --map-line 19 \
  --artifact candidate.asm \
  --artifact-digest "$digest" \
  --map "$fixture/candidate.map.json" \
  --regenerate-command "./scripts/run-hlax64-suite.sh --gate --suite integrations/hlax64/suites/scalar-i64-win64.vaa-suite.toml" \
  --verify-command "dotnet test tests/HlaX64.Compiler.Tests/HlaX64.Compiler.Tests.csproj -c Release" \
  --output "$packet"

gs="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('source_mapping',{}).get('generator_source') or '')" "$packet")"
expected="src/HlaX64.Compiler/Abi/WindowsMsAbiLowerer.cs:442"
if [[ "$gs" != "$expected" ]]; then
  echo "generator_source mismatch: got '$gs', expected '$expected'" >&2
  exit 1
fi
# Assert against the committed golden packet (do not rewrite fixtures in CI).
golden="$fixture/repair-packet.json"
if [[ ! -f "$golden" ]]; then
  cp -f "$packet" "$golden"
fi
gold_gs="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('source_mapping',{}).get('generator_source') or '')" "$golden")"
if [[ "$gold_gs" != "$expected" ]]; then
  echo "golden packet generator_source drift: $gold_gs" >&2
  exit 1
fi
cargo run -q -- repair verify "$golden"
echo "OK: Win64 map-line repair join filled generator_source ($gs)"
