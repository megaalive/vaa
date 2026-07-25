#!/usr/bin/env bash
# Repair E2E: assert repair export joins compiler_source → generator_source.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

out_dir="target/repair-mapjoin"
mkdir -p "$out_dir"
packet="$out_dir/repair-packet.json"

cargo run -q -- repair export \
  --spec integrations/echoasm/generator.spec.toml \
  --task-id compiler-source-mapjoin-v1 \
  --status Violated \
  --message "map-join e2e: join compiler_source into repair packet" \
  --instruction-offset 0x10 \
  --artifact candidate.asm \
  --artifact-digest "sha256:0000000000000000000000000000000000000000000000000000000000000000" \
  --map fixtures/repair/compiler-source-mapjoin/candidate.map.json \
  --regenerate-command "echo regenerate" \
  --verify-command "echo verify" \
  --output "$packet"

gs="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('source_mapping',{}).get('generator_source') or '')" "$packet")"
expected="src/HlaX64.Compiler/Abi/Win64AbiLowerer.cs:214"
if [[ -z "$gs" ]]; then
  echo "expected source_mapping.generator_source from map join; got none" >&2
  exit 1
fi
if [[ "$gs" != "$expected" ]]; then
  echo "generator_source mismatch: got '$gs', expected '$expected'" >&2
  exit 1
fi
echo "OK: repair export map-join filled generator_source ($gs)"
