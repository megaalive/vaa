#!/usr/bin/env python3
"""Read, check, or bump the SemASM tip pin shared by the CI workflows.

`SEMASM_TIP_SHA` is duplicated in `.github/workflows/ci.yml` and
`.github/workflows/corpus-sweep.yml` (GitHub Actions cannot share env across
workflow files). This is the single tool that touches both, so the pins cannot
drift and a bump is a reviewable one-liner in each file.

    python scripts/bump_semasm_tip.py --check          # fail if the pins differ
    python scripts/bump_semasm_tip.py --print          # print the current pin
    python scripts/bump_semasm_tip.py --sha <40-hex>   # rewrite both files

The pack pin (SEMASM_PACK_SHA) is intentionally NOT touched: it tracks the
Phase-E-stable SemASM for HlaX64/EchoAsm suites and moves under a separate,
suite-policy-gated decision.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FILES = [
    ROOT / ".github" / "workflows" / "ci.yml",
    ROOT / ".github" / "workflows" / "corpus-sweep.yml",
]
PIN_RE = re.compile(r"^(\s*SEMASM_TIP_SHA:\s*)([0-9a-fA-F]{40})(\s*)$", re.MULTILINE)
SHA_RE = re.compile(r"^[0-9a-f]{40}$")


def read_pins() -> dict[Path, str]:
    pins: dict[Path, str] = {}
    for path in FILES:
        text = path.read_text(encoding="utf-8")
        matches = PIN_RE.findall(text)
        if len(matches) != 1:
            raise SystemExit(
                f"{path}: expected exactly one SEMASM_TIP_SHA line, found {len(matches)}"
            )
        pins[path] = matches[0][1].lower()
    return pins


def check() -> int:
    pins = read_pins()
    unique = set(pins.values())
    if len(unique) != 1:
        for path, sha in pins.items():
            print(f"{path.relative_to(ROOT)}: {sha}", file=sys.stderr)
        print("error: SEMASM_TIP_SHA differs between workflows", file=sys.stderr)
        return 1
    print(next(iter(unique)))
    return 0


def bump(new_sha: str) -> int:
    new_sha = new_sha.lower()
    if not SHA_RE.match(new_sha):
        print(f"error: not a 40-char lowercase hex SHA: {new_sha!r}", file=sys.stderr)
        return 2
    changed = False
    for path in FILES:
        text = path.read_text(encoding="utf-8")
        new_text, n = PIN_RE.subn(rf"\g<1>{new_sha}\g<3>", text)
        if n != 1:
            print(f"error: {path}: expected 1 pin line, rewrote {n}", file=sys.stderr)
            return 2
        if new_text != text:
            path.write_text(new_text, encoding="utf-8", newline="\n")
            changed = True
            print(f"bumped {path.relative_to(ROOT)} -> {new_sha}")
    if not changed:
        print(f"already at {new_sha}; nothing to do")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--check", action="store_true", help="Fail if the two pins differ")
    group.add_argument("--print", action="store_true", help="Print the current pin")
    group.add_argument("--sha", help="Rewrite both workflows to this 40-hex SHA")
    ns = parser.parse_args()

    if ns.check:
        return check()
    if ns.print:
        pins = read_pins()
        print(next(iter(set(pins.values()))))
        return 0
    return bump(ns.sha)


if __name__ == "__main__":
    raise SystemExit(main())
