#!/usr/bin/env bash
# EchoAsm generator (POSIX twin): copy locked input bytes to candidate output.
set -euo pipefail
if [[ $# -ne 2 ]]; then
  echo "usage: echoasm.sh <input> <output>" >&2
  exit 2
fi
input=$1
output=$2
if [[ ! -f "$input" ]]; then
  echo "echoasm: input not found: $input" >&2
  exit 1
fi
mkdir -p "$(dirname "$output")"
cp -f "$input" "$output"
echo "echoasm: wrote $output"
