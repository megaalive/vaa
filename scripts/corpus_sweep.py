#!/usr/bin/env python3
"""Sweep every direct leaf fixture through `vaa harness submit` (wrong→repaired).

The pinned gates only exercise `count_byte`. This sweep runs the whole
`fixtures/run/<leaf>/` corpus against a live SemASM so a tip regression that
breaks `memcpy`, `memset`, `find_*`, etc. is caught here instead of leaking to
downstream packs. Intended to run on a schedule with a fresh SemASM tip.

For each leaf it submits `01_wrong.asm` then `02_repaired.asm` into one run
dir, then verifies the seal chain. A leaf passes when:

  * the wrong candidate is NOT accepted (guards against a spuriously weak tip),
  * the repaired candidate reaches `accepted` (VerifiedUnderPreconditions is
    accepted only with --allow-under-preconditions, matching each task profile),
  * `evidence verify-chain` succeeds.

`VAA_BIN` (via the shared adapter helper) selects the CLI to spawn; the
scheduled job points it at the freshly built SemASM tip.

    python scripts/corpus_sweep.py [--leaf memcpy ...] [--strict-verified]
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))
import agent_harness_adapter as adapter

ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "fixtures" / "run"

# Behavioral verification runs the linked artifact, so a leaf is only sweepable
# on a host whose OS matches the task target.
HOST_TARGETS = {
    "win32": {"x86_64-pc-windows-msvc"},
    "linux": {"x86_64-unknown-linux-gnu"},
}


def host_targets() -> set[str]:
    return HOST_TARGETS.get(sys.platform, set())


def discover_leaves() -> list[dict[str, Any]]:
    """Leaf dirs holding a task, contract, wrong, and repaired candidate."""
    leaves: list[dict[str, Any]] = []
    for leaf_dir in sorted(p for p in CORPUS.iterdir() if p.is_dir()):
        tasks = list(leaf_dir.glob("*.vaa.toml"))
        contracts = list(leaf_dir.glob("*.sem.toml"))
        wrong = leaf_dir / "01_wrong.asm"
        repaired = leaf_dir / "02_repaired.asm"
        # Skip budget variants; the sweep uses the canonical task per leaf.
        tasks = [t for t in tasks if "budget" not in t.name]
        if len(tasks) == 1 and len(contracts) == 1 and wrong.is_file() and repaired.is_file():
            target = tomllib.loads(tasks[0].read_text(encoding="utf-8")).get("target", "")
            leaves.append(
                {
                    "name": leaf_dir.name,
                    "task": tasks[0],
                    "contract": contracts[0],
                    "wrong": wrong,
                    "repaired": repaired,
                    "target": target,
                }
            )
    return leaves


def submit(leaf: dict[str, Any], source: Path, run_dir: str | None, run_base: Path,
           allow_vup: bool) -> dict[str, Any]:
    args = [
        "harness",
        "submit",
        "--mode",
        "direct-nasm",
        "--task",
        str(leaf["task"]),
        "--contract",
        str(leaf["contract"]),
        "--source",
        str(source),
        "--allow-execution",
        "--format",
        "json",
    ]
    if allow_vup:
        args.append("--allow-under-preconditions")
    if run_dir:
        args += ["--run-dir", run_dir]
    else:
        args += ["--run-base", str(run_base)]
    return adapter.run_vaa(args, timeout=300.0)


def sweep_leaf(leaf: dict[str, Any], workdir: Path, allow_vup: bool) -> dict[str, Any]:
    run_base = workdir / f"{leaf['name']}-runs"
    run_base.mkdir(parents=True, exist_ok=True)

    wrong = submit(leaf, leaf["wrong"], None, run_base, allow_vup)
    wrong_class = wrong.get("class")
    run_dir = wrong.get("run_dir")

    if wrong_class == "toolchain_retryable":
        return {
            "name": leaf["name"],
            "status": "toolchain_retryable",
            "detail": wrong.get("message"),
        }

    repaired: dict[str, Any] = {}
    chain_ok = False
    if run_dir:
        repaired = submit(leaf, leaf["repaired"], run_dir, run_base, allow_vup)
        # verify-chain prints human text, not JSON; check its exit code directly.
        chain = subprocess.run(
            [*adapter.vaa_command(), "evidence", "verify-chain", run_dir],
            capture_output=True,
            text=True,
            check=False,
        )
        chain_ok = chain.returncode == 0

    repaired_class = repaired.get("class")
    passed = (
        wrong_class != "accepted"
        and repaired_class == "accepted"
        and chain_ok
    )
    return {
        "name": leaf["name"],
        "status": "pass" if passed else "fail",
        "wrong_class": wrong_class,
        "repaired_class": repaired_class,
        "repaired_evidence": repaired.get("evidence_status"),
        "chain_ok": chain_ok,
        "run_dir": run_dir,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--leaf", action="append", help="Restrict to named leaves (repeatable)")
    parser.add_argument(
        "--strict-verified",
        action="store_true",
        help="Reject VerifiedUnderPreconditions (do not pass --allow-under-preconditions)",
    )
    parser.add_argument("--list", action="store_true", help="List discovered leaves and exit")
    parser.add_argument(
        "--all-targets",
        action="store_true",
        help="Do not filter leaves by host OS (may fail for cross targets)",
    )
    ns = parser.parse_args()

    leaves = discover_leaves()
    if not ns.all_targets:
        runnable = host_targets()
        leaves = [leaf for leaf in leaves if leaf["target"] in runnable]
    if ns.leaf:
        wanted = set(ns.leaf)
        leaves = [leaf for leaf in leaves if leaf["name"] in wanted]
        missing = wanted - {leaf["name"] for leaf in leaves}
        if missing:
            print(f"error: unknown leaves: {sorted(missing)}", file=sys.stderr)
            return 2

    if ns.list:
        for leaf in leaves:
            print(leaf["name"])
        return 0

    if not leaves:
        print("error: no leaves discovered under fixtures/run", file=sys.stderr)
        return 2

    results: list[dict[str, Any]] = []
    tmp = Path(tempfile.mkdtemp(prefix="vaa-corpus-sweep-"))
    try:
        for leaf in leaves:
            results.append(sweep_leaf(leaf, tmp, allow_vup=not ns.strict_verified))
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    summary = {
        "schema_version": "0.1",
        "kind": "corpus_sweep",
        "host_platform": sys.platform,
        "strict_verified": ns.strict_verified,
        "total": len(results),
        "passed": sum(1 for r in results if r["status"] == "pass"),
        "results": results,
    }
    json.dump(summary, sys.stdout, indent=2)
    sys.stdout.write("\n")

    failed = [r for r in results if r["status"] != "pass"]
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
