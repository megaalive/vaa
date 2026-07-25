#!/usr/bin/env bash
# Milestone 6 pack CI (Linux twin of ci-generator-packs.ps1).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

vaa() { cargo run -q -- "$@"; }

echo "== Gate 1: validate pack locks + specs =="
vaa generator validate-lock integrations/hlax64/stack.lock.toml
vaa generator validate-spec integrations/hlax64/generator.spec.toml
vaa generator validate-spec integrations/hlax64/generator.sysv.spec.toml
vaa generator validate-lock integrations/echoasm/stack.lock.toml
vaa generator validate-spec integrations/echoasm/generator.spec.toml

echo "== Gate 1b: validate suites =="
for s in \
  integrations/hlax64/suites/smoke.vaa-suite.toml \
  integrations/hlax64/suites/scalar-win64.vaa-suite.toml \
  integrations/hlax64/suites/scalar-i64-win64.vaa-suite.toml \
  integrations/hlax64/suites/scalar-sysv.vaa-suite.toml \
  integrations/hlax64/suites/loop-win64.vaa-suite.toml \
  integrations/hlax64/suites/loop-stack-win64.vaa-suite.toml \
  integrations/hlax64/suites/scalar-i64-sysv.vaa-suite.toml \
  integrations/hlax64/suites/loop-stack-sysv.vaa-suite.toml \
  integrations/hlax64/suites/memory-read-win64.vaa-suite.toml \
  integrations/hlax64/suites/memory-write-win64.vaa-suite.toml \
  integrations/hlax64/suites/calls-data-win64.vaa-suite.toml \
  integrations/hlax64/suites/negative-reject-win64.vaa-suite.toml \
  integrations/hlax64/suites/backend-win64.vaa-suite.toml \
  integrations/echoasm/suites/smoke.vaa-suite.toml
do
  vaa suite validate "$s"
done

echo "== Gate 1c: target/ABI parity =="
vaa suite check-parity integrations/hlax64/suites/scalar-win64.vaa-suite.toml
vaa suite check-parity integrations/hlax64/suites/scalar-i64-win64.vaa-suite.toml
vaa suite check-parity integrations/hlax64/suites/scalar-sysv.vaa-suite.toml
vaa suite check-parity integrations/hlax64/suites/loop-stack-win64.vaa-suite.toml
vaa suite check-parity integrations/hlax64/suites/scalar-i64-sysv.vaa-suite.toml
vaa suite check-parity integrations/hlax64/suites/loop-stack-sysv.vaa-suite.toml
vaa suite check-parity integrations/hlax64/suites/calls-data-win64.vaa-suite.toml
vaa suite check-parity integrations/hlax64/suites/negative-reject-win64.vaa-suite.toml
vaa suite check-parity integrations/hlax64/suites/backend-win64.vaa-suite.toml
vaa suite check-parity integrations/echoasm/suites/smoke.vaa-suite.toml

echo "== Gate 3: EchoAsm deterministic generation =="
mkdir -p target/ci-echoasm
# On Linux use the shell twin as the generator binary for identity + run.
chmod +x integrations/echoasm/tools/echoasm.sh
GEN="$(realpath integrations/echoasm/tools/echoasm.sh)"
IN="$(realpath integrations/echoasm/cases/passthrough/input.asm)"
OUT="$(realpath -m target/ci-echoasm/candidate.asm)"
# Temporarily point generation at the sh twin via explicit --generator.
vaa generator generate integrations/echoasm/generator.spec.toml \
  --generator "$GEN" \
  --input "$IN" \
  --output "$OUT" \
  --target x86_64-unknown-linux-gnu \
  --check-deterministic
h1=$(sha256sum "$OUT" | awk '{print $1}')
h2=$(sha256sum "$IN" | awk '{print $1}')
if [[ "$h1" != "$h2" ]]; then
  echo "EchoAsm output digest mismatch" >&2
  exit 1
fi
echo "EchoAsm digest match OK"

echo "== Gate 7: patch evidence fixtures =="
vaa patch evidence-verify fixtures/repair/echoasm-passthrough/patch-evidence.json
vaa repair verify fixtures/repair/hlax64-min-usize-sysv-live/repair-packet.json
vaa patch evidence-verify fixtures/repair/hlax64-min-usize-sysv-live/patch-evidence.json
json=$(vaa patch evidence-verify fixtures/repair/echoasm-passthrough/patch-evidence.forbidden-failed.json --format json)
echo "$json" | grep -qi '"status"[[:space:]]*:[[:space:]]*"failed"' \
  || { echo "forbidden fixture must be Failed: $json" >&2; exit 1; }
echo "forbidden fixture correctly Failed"

echo "== isolation audit =="
vaa generator isolation-check

echo "OK: generator pack matrix passed"
