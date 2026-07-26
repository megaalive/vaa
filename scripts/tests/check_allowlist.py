#!/usr/bin/env python3
"""Fail-closed check: the agent leaf allowlist agrees with corpus_sweep discovery.

Two invariants, both hermetic (no SemASM / toolchain needed):

  1. Every allowlist leaf is discoverable by scripts/corpus_sweep.py across all
     targets — the allowlist cannot invent a shape that does not exist on disk.
  2. Every allowlist leaf whose target matches the *host* OS appears in the
     host-filtered `corpus_sweep --list`, i.e. what an agent could actually run
     here is a subset of what the sweep would exercise.

Run from CI (adapter-reference job) and locally:

    python scripts/tests/check_allowlist.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import corpus_sweep  # noqa: E402

ALLOWLIST = ROOT / "schemas" / "agent-leaf-allowlist.json"


def main() -> int:
    allowlist = json.loads(ALLOWLIST.read_text(encoding="utf-8"))
    leaves = allowlist["leaves"]

    discovered = corpus_sweep.discover_leaves()
    discovered_names = {leaf["name"] for leaf in discovered}

    errors: list[str] = []

    # (1) allowlist ⊆ all discovered leaves; paths exist.
    for leaf in leaves:
        name = leaf["name"]
        if name not in discovered_names:
            errors.append(f"{name}: not discovered by corpus_sweep (invented leaf?)")
        for key in ("task", "contract"):
            if not (ROOT / leaf[key]).is_file():
                errors.append(f"{name}: {key} path missing: {leaf[key]}")

    # (2) host-relevant allowlist ⊆ host-filtered discovery.
    host = corpus_sweep.host_targets()
    host_discovered = {leaf["name"] for leaf in discovered if leaf["target"] in host}
    for leaf in leaves:
        if leaf["target"] in host and leaf["name"] not in host_discovered:
            errors.append(f"{leaf['name']}: host target {leaf['target']} not in host discovery")

    if errors:
        print("allowlist check FAILED:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return 1

    print(
        f"allowlist OK: {len(leaves)} leaves, all discoverable "
        f"(host={sys.platform}, host-relevant={len(host_discovered)})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
