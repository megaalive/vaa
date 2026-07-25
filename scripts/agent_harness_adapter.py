#!/usr/bin/env python3
"""Minimal reference adapter: spawn `vaa harness` and parse stdout JSON only.

This is intentionally not an SDK. Controllers should treat stderr as noise.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from typing import Any


def run_vaa(args: list[str], timeout: float | None = None) -> dict[str, Any]:
    cmd = ["vaa", *args]
    env = os.environ.copy()
    proc = subprocess.run(
        cmd,
        check=False,
        capture_output=True,
        text=True,
        timeout=timeout,
        env=env,
    )
    stdout = proc.stdout.strip()
    if not stdout:
        raise RuntimeError(
            f"empty stdout from {' '.join(cmd)}; exit={proc.returncode}; stderr={proc.stderr[:500]!r}"
        )
    try:
        payload = json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise RuntimeError(
            f"stdout was not JSON (do not concatenate stderr): {exc}; head={stdout[:200]!r}"
        ) from exc
    payload["_vaa_exit_code"] = proc.returncode
    return payload


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_prep = sub.add_parser("prepare-direct")
    p_prep.add_argument("--task", required=True)
    p_prep.add_argument("--contract", required=True)
    p_prep.add_argument("--workspace", required=True)
    p_prep.add_argument("--seed")
    p_prep.add_argument("--allow-execution", action="store_true")

    p_gen = sub.add_parser("prepare-generator")
    p_gen.add_argument("--repair-packet", required=True)
    p_gen.add_argument("--workspace", required=True)
    p_gen.add_argument("--target", default="x86_64-pc-windows-msvc")

    p_sub = sub.add_parser("submit")
    p_sub.add_argument("--task", required=True)
    p_sub.add_argument("--contract", required=True)
    p_sub.add_argument("--source", required=True)
    p_sub.add_argument("--allow-execution", action="store_true")
    p_sub.add_argument("--allow-under-preconditions", action="store_true")
    p_sub.add_argument("--timeout", type=int, default=120)
    p_sub.add_argument("--run-dir")
    p_sub.add_argument("--idempotency-key")

    p_st = sub.add_parser("status")
    p_st.add_argument("--run-dir", required=True)

    ns = parser.parse_args()
    if ns.cmd == "prepare-direct":
        args = [
            "harness",
            "prepare",
            "--mode",
            "direct-nasm",
            "--task",
            ns.task,
            "--contract",
            ns.contract,
            "--workspace",
            ns.workspace,
            "--format",
            "json",
        ]
        if ns.seed:
            args.extend(["--seed", ns.seed])
        if ns.allow_execution:
            args.append("--allow-execution")
        payload = run_vaa(args)
    elif ns.cmd == "prepare-generator":
        payload = run_vaa(
            [
                "harness",
                "prepare",
                "--mode",
                "generator-repair",
                "--repair-packet",
                ns.repair_packet,
                "--workspace",
                ns.workspace,
                "--target",
                ns.target,
                "--format",
                "json",
            ]
        )
    elif ns.cmd == "submit":
        args = [
            "harness",
            "submit",
            "--task",
            ns.task,
            "--contract",
            ns.contract,
            "--source",
            ns.source,
            "--timeout",
            str(ns.timeout),
            "--format",
            "json",
        ]
        if ns.allow_execution:
            args.append("--allow-execution")
        if ns.allow_under_preconditions:
            args.append("--allow-under-preconditions")
        if ns.run_dir:
            args.extend(["--run-dir", ns.run_dir])
        if ns.idempotency_key:
            args.extend(["--idempotency-key", ns.idempotency_key])
        payload = run_vaa(args, timeout=float(ns.timeout) + 30.0)
    else:
        payload = run_vaa(
            ["harness", "status", "--run-dir", ns.run_dir, "--format", "json"]
        )

    json.dump(payload, sys.stdout, indent=2)
    sys.stdout.write("\n")
    return int(payload.get("_vaa_exit_code", 0))


if __name__ == "__main__":
    raise SystemExit(main())
