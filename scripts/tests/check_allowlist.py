#!/usr/bin/env python3
"""Fail-closed check: allowlist ↔ corpus_sweep ↔ admission snapshot.

Three hermetic invariants (no SemASM / toolchain needed):

  1. Every allowlist leaf is discoverable by scripts/corpus_sweep.py across all
     targets — the allowlist cannot invent a shape that does not exist on disk.
  2. Every allowlist leaf whose target matches the *host* OS appears in the
     host-filtered `corpus_sweep --list`.
  3. Every allowlist leaf×target appears in the frozen SemASM admission
     snapshot (`fixtures/semasm/capabilities-snapshot.json`) leaf_names — the
     skill gate is admission; the allowlist is only a discovery/freeze mirror.

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
ADMISSION = ROOT / "fixtures" / "semasm" / "capabilities-snapshot.json"


def admission_pairs() -> set[tuple[str, str]]:
    snap = json.loads(ADMISSION.read_text(encoding="utf-8"))
    pairs: set[tuple[str, str]] = set()
    for row in snap.get("admission", []):
        for leaf in row.get("leaf_names") or []:
            for target in row.get("targets") or []:
                pairs.add((leaf, target))
    return pairs


def main() -> int:
    allowlist = json.loads(ALLOWLIST.read_text(encoding="utf-8"))
    leaves = allowlist["leaves"]

    discovered = corpus_sweep.discover_leaves()
    discovered_names = {leaf["name"] for leaf in discovered}
    admitted = admission_pairs()

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

    # (3) allowlist ⊆ admission snapshot leaf_names×target.
    for leaf in leaves:
        pair = (leaf["name"], leaf["target"])
        if pair not in admitted:
            errors.append(
                f"{leaf['name']} @ {leaf['target']}: missing from admission snapshot "
                f"({ADMISSION.relative_to(ROOT).as_posix()})"
            )

    if errors:
        print("allowlist/admission check FAILED:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return 1

    print(
        f"allowlist OK: {len(leaves)} leaves, all discoverable + admitted "
        f"(host={sys.platform}, host-relevant={len(host_discovered)}, "
        f"admission_pairs={len(admitted)})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
