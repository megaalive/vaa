#!/usr/bin/env python3
"""Hermetic dry-run of the reference adapter loops (no SemASM, no assembler).

Spawns `agent_harness_adapter.py` against `stub_vaa.py` via `VAA_BIN`, then
checks the emitted JSON against `schemas/harness-loop-result.schema.json` and
the golden fixtures under `schemas/fixtures/`.

Guards the controller-facing stdout contract only. Verification claims are made
by `vaa` + SemASM in the live gates, never here.

    python scripts/tests/harness_adapter_dryrun.py [--update]
"""

from __future__ import annotations

import argparse
import json
import os
import shlex
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

import jsonschema

ROOT = Path(__file__).resolve().parents[2]
ADAPTER = ROOT / "scripts" / "agent_harness_adapter.py"
STUB = Path(__file__).resolve().parent / "stub_vaa.py"
SCHEMA = ROOT / "schemas" / "harness-loop-result.schema.json"
FIXTURES = ROOT / "schemas" / "fixtures"

RUN_DIR = "/tmp/vaa-runs/20260101T000000Z-stub"
SEAL_DIGEST = "sha256:1111111111111111111111111111111111111111111111111111111111111111"

# Each scenario replays one adapter loop: the CLI calls the adapter is expected
# to make, in order, and the stdout the stubbed `vaa` returns for each.
SCENARIOS: dict[str, dict[str, Any]] = {
    "direct_accepted": {
        "golden": "harness-loop-result.direct_accepted.json",
        "expected_exit_code": 0,
        "argv": [
            "loop-direct",
            "--task",
            "fixtures/run/count_byte/count_byte.vaa.toml",
            "--contract",
            "fixtures/run/count_byte/count_byte.sem.toml",
            "--candidate",
            "fixtures/run/count_byte/01_wrong.asm",
            "--candidate",
            "fixtures/run/count_byte/02_repaired.asm",
            "--allow-execution",
        ],
        "responses": [
            {
                "expect_args": ["harness", "prepare", "--mode", "direct-nasm"],
                "stdout_json": {
                    "schema_version": "0.1",
                    "task_id": "count-byte-v1",
                    "target": "x86_64-pc-windows-msvc",
                    "assembler": "nasm",
                    "remaining_attempts": 4,
                    "mode": "direct_nasm",
                },
            },
            {
                "expect_args": ["harness", "submit", "--mode", "direct-nasm"],
                "exit_code": 5,
                "stdout_json": {
                    "schema_version": "0.1",
                    "class": "violated_repairable",
                    "next_action": "edit_candidate",
                    "evidence_status": "violated",
                    "raw_status": "behavior_failed",
                    "exit_code": 5,
                    "message": "sealed candidate 0000 as violated",
                    "candidate_index": 0,
                    "run_dir": RUN_DIR,
                    "seal_digest": SEAL_DIGEST,
                    "assembler": "nasm",
                    "may_auto_retry": False,
                },
            },
            {
                "expect_args": ["harness", "submit", "--mode", "direct-nasm"],
                "stdout_json": {
                    "schema_version": "0.1",
                    "class": "accepted",
                    "next_action": "done",
                    "evidence_status": "verified",
                    "raw_status": "verified",
                    "exit_code": 0,
                    "message": "sealed candidate 0001 as verified",
                    "candidate_index": 1,
                    "run_dir": RUN_DIR,
                    "seal_digest": SEAL_DIGEST,
                    "assembler": "nasm",
                    "may_auto_retry": False,
                },
            },
            {
                "expect_args": ["harness", "status", "--run-dir"],
                "stdout_json": {
                    "run_id": "20260101T000000Z-stub",
                    "next_candidate_index": 2,
                    "sealed_candidates": 2,
                },
            },
        ],
    },
    "generator_repair_accepted": {
        "golden": "harness-loop-result.generator_repair_accepted.json",
        "expected_exit_code": 0,
        "argv": [
            "loop-generator",
            "--repair-packet",
            "schemas/fixtures/repair-packet.golden.json",
            "--suite-evidence",
            "fixtures/repair/echoasm-passthrough/suite-evidence.accepted.json",
        ],
        "responses": [
            {
                "expect_args": ["harness", "prepare", "--mode", "generator-repair"],
                "stdout_json": {
                    "schema_version": "0.1",
                    "task_id": "stack-local-i64-win64-stack-balance-v1",
                    "target": "x86_64-pc-windows-msvc",
                    "mode": "generator_repair",
                },
            },
            {
                "expect_args": ["harness", "submit", "--mode", "generator-repair"],
                "exit_code": 8,
                "stdout_json": {
                    "schema_version": "0.1",
                    "class": "policy_blocked",
                    "next_action": "stop_policy",
                    "evidence_status": "failed",
                    "exit_code": 8,
                    "message": "changed file outside generator authority",
                    "failure_code": "FORBIDDEN_PATH",
                    "may_auto_retry": False,
                },
            },
            {
                "expect_args": ["harness", "submit", "--mode", "generator-repair"],
                "stdout_json": {
                    "schema_version": "0.1",
                    "class": "accepted",
                    "next_action": "done",
                    "evidence_status": "accepted",
                    "exit_code": 0,
                    "message": "patch evidence written",
                    "patch_evidence_path": "/tmp/vaa-gen/patch-evidence.json",
                    "may_auto_retry": False,
                },
            },
        ],
    },
}


def run_scenario(name: str, scenario: dict[str, Any], workdir: Path) -> dict[str, Any]:
    """Drive one adapter loop against the stub and return its parsed stdout."""
    scenario_path = workdir / f"{name}.scenario.json"
    scenario_path.write_text(
        json.dumps({"responses": scenario["responses"]}, indent=2), encoding="utf-8"
    )
    calls_path = workdir / f"{name}.calls.json"

    argv = [*scenario["argv"], "--workspace", str(workdir / name)]
    if scenario["argv"][0] == "loop-direct":
        argv += ["--run-base", str(workdir / f"{name}-runs")]

    env = {
        **os.environ,
        "VAA_BIN": f"{shlex.quote(sys.executable)} {shlex.quote(str(STUB))}",
        "VAA_STUB_SCENARIO": str(scenario_path),
        "VAA_STUB_CALLS": str(calls_path),
    }
    proc = subprocess.run(
        [sys.executable, str(ADAPTER), *argv],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=env,
        check=False,
    )
    if proc.returncode != scenario["expected_exit_code"]:
        raise SystemExit(
            f"{name}: exit {proc.returncode} != {scenario['expected_exit_code']}\n"
            f"stdout={proc.stdout}\nstderr={proc.stderr}"
        )
    try:
        payload = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        raise SystemExit(
            f"{name}: adapter stdout was not JSON ({exc}); stderr={proc.stderr}"
        ) from exc

    calls = json.loads(calls_path.read_text(encoding="utf-8"))
    if len(calls) != len(scenario["responses"]):
        raise SystemExit(
            f"{name}: adapter made {len(calls)} vaa calls, "
            f"scenario declares {len(scenario['responses'])}"
        )
    return payload


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--update",
        action="store_true",
        help="Rewrite golden fixtures from this run instead of comparing",
    )
    ns = parser.parse_args()

    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    validator = jsonschema.Draft202012Validator(schema)
    failures: list[str] = []

    with tempfile.TemporaryDirectory(prefix="vaa-adapter-dryrun-") as tmp:
        workdir = Path(tmp)
        for name, scenario in SCENARIOS.items():
            payload = run_scenario(name, scenario, workdir)
            errors = sorted(validator.iter_errors(payload), key=lambda e: e.path)
            for err in errors:
                path = "/".join(str(p) for p in err.absolute_path) or "<root>"
                failures.append(f"{name}: schema violation at {path}: {err.message}")

            golden_path = FIXTURES / scenario["golden"]
            serialized = json.dumps(payload, indent=2) + "\n"
            if ns.update:
                golden_path.write_text(serialized, encoding="utf-8")
                print(f"{name}: wrote {golden_path.relative_to(ROOT)}")
                continue
            if not golden_path.is_file():
                failures.append(f"{name}: missing golden {golden_path}; run --update")
            elif golden_path.read_text(encoding="utf-8") != serialized:
                failures.append(
                    f"{name}: output drifted from {golden_path.relative_to(ROOT)}; "
                    "re-run with --update if the change is intended"
                )
            else:
                print(f"{name}: ok (schema + golden)")

    for failure in failures:
        print(f"error: {failure}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
