#!/usr/bin/env python3
"""Reference adapter: spawn `vaa harness` and parse stdout JSON only.

Controllers must treat stderr as noise. Supports:
  - one-shot prepare / submit / status helpers
  - deterministic `loop-direct` that applies a sequence of candidate sources
    (wrong → repaired) until accepted / budget / policy / hard failure
  - deterministic `loop-generator` that rehearses policy-block then accepted
    patch evidence (no LLM, no live generator rebuild)

This is intentionally not an SDK and does not call an LLM.
"""

from __future__ import annotations

import argparse
import json
import os
import shlex
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

TERMINAL_CLASSES = frozenset(
    {
        "accepted",
        "policy_blocked",
        "failed",
        "incomplete_coverage",
    }
)


def load_work_packet(workspace: Path) -> dict[str, Any] | None:
    """Prefer `work-packet.json`, else fall back to `agent-envelope.json`."""
    for name in ("work-packet.json", "agent-envelope.json"):
        path = workspace / name
        if path.is_file():
            return json.loads(path.read_text(encoding="utf-8"))
    return None


def vaa_command() -> list[str]:
    """CLI to spawn: `VAA_BIN` if set, else `vaa` from PATH.

    `VAA_BIN` is parsed shell-style, so paths containing spaces or backslashes
    must be single-quoted (`VAA_BIN="'C:\\tools\\vaa.exe'"`).
    """
    raw = os.environ.get("VAA_BIN", "").strip()
    return shlex.split(raw) if raw else ["vaa"]


def run_vaa(args: list[str], timeout: float | None = None) -> dict[str, Any]:
    cmd = [*vaa_command(), *args]
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
            f"empty stdout from {' '.join(cmd)}; exit={proc.returncode}; "
            f"stderr={proc.stderr[:500]!r}"
        )
    try:
        payload = json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise RuntimeError(
            f"stdout was not JSON (do not concatenate stderr): {exc}; head={stdout[:200]!r}"
        ) from exc
    payload["_vaa_exit_code"] = proc.returncode
    return payload


def emit(payload: dict[str, Any]) -> int:
    json.dump(payload, sys.stdout, indent=2)
    sys.stdout.write("\n")
    return int(payload.get("_vaa_exit_code", 0))


def loop_direct(ns: argparse.Namespace) -> dict[str, Any]:
    workspace = Path(ns.workspace)
    workspace.mkdir(parents=True, exist_ok=True)
    run_base = Path(ns.run_base)
    run_base.mkdir(parents=True, exist_ok=True)

    prep_args = [
        "harness",
        "prepare",
        "--mode",
        "direct-nasm",
        "--task",
        ns.task,
        "--contract",
        ns.contract,
        "--workspace",
        str(workspace),
        "--assembler",
        ns.assembler,
        "--format",
        "json",
    ]
    if ns.seed:
        prep_args.extend(["--seed", ns.seed])
    if ns.allow_execution:
        prep_args.append("--allow-execution")
    envelope = run_vaa(prep_args)
    disk_packet = load_work_packet(workspace)
    if disk_packet is not None:
        disk_packet["_vaa_exit_code"] = envelope.get("_vaa_exit_code", 0)
        envelope = disk_packet

    candidate_name = "candidate.S" if ns.assembler == "gas" else "candidate.asm"
    candidate_path = workspace / candidate_name
    steps: list[dict[str, Any]] = []
    run_dir: str | None = None
    last: dict[str, Any] = envelope

    for index, source in enumerate(ns.candidate):
        shutil.copyfile(source, candidate_path)
        submit_args = [
            "harness",
            "submit",
            "--mode",
            "direct-nasm",
            "--task",
            ns.task,
            "--contract",
            ns.contract,
            "--source",
            str(candidate_path),
            "--assembler",
            ns.assembler,
            "--timeout",
            str(ns.timeout),
            "--format",
            "json",
        ]
        if ns.allow_execution:
            submit_args.append("--allow-execution")
        if ns.allow_under_preconditions:
            submit_args.append("--allow-under-preconditions")
        if getattr(ns, "level", None):
            submit_args.extend(["--level", ns.level])
        if run_dir:
            submit_args.extend(["--run-dir", run_dir])
        else:
            submit_args.extend(["--run-base", str(run_base)])

        result = run_vaa(submit_args, timeout=float(ns.timeout) + 60.0)
        step = {
            "index": index,
            "source": source,
            "class": result.get("class"),
            "next_action": result.get("next_action"),
            "failure_code": result.get("failure_code"),
            "candidate_index": result.get("candidate_index"),
            "exit_code": result.get("_vaa_exit_code"),
        }
        steps.append(step)
        last = result
        if result.get("run_dir"):
            run_dir = result["run_dir"]

        cls = result.get("class")
        if cls == "accepted":
            break
        # incomplete_coverage may continue with --allow-execution already set;
        # violated_repairable is non-terminal by design (agent edits and retries).
        if cls in TERMINAL_CLASSES and cls != "incomplete_coverage":
            break
        if result.get("failure_code") == "BUDGET_EXHAUSTED":
            break
        if cls == "toolchain_retryable" and not result.get("may_auto_retry"):
            break

    status = None
    if run_dir:
        status = run_vaa(
            ["harness", "status", "--run-dir", run_dir, "--format", "json"]
        )

    return {
        "schema_version": "0.1",
        "kind": "agent_harness_loop",
        "mode": "direct",
        "envelope": {
            "task_id": envelope.get("task_id"),
            "target": envelope.get("target"),
            "assembler": envelope.get("assembler"),
            "remaining_attempts": envelope.get("remaining_attempts"),
        },
        "steps": steps,
        "final": {
            "class": last.get("class"),
            "next_action": last.get("next_action"),
            "failure_code": last.get("failure_code"),
            "run_dir": run_dir,
            "seal_digest": last.get("seal_digest"),
            "exit_code": last.get("_vaa_exit_code"),
        },
        "status": status,
        "_vaa_exit_code": last.get("_vaa_exit_code", 0),
    }


def loop_generator(ns: argparse.Namespace) -> dict[str, Any]:
    """Deterministic generator-repair loop: policy reject, then accepted patch."""
    workspace = Path(ns.workspace)
    workspace.mkdir(parents=True, exist_ok=True)

    envelope = run_vaa(
        [
            "harness",
            "prepare",
            "--mode",
            "generator-repair",
            "--repair-packet",
            ns.repair_packet,
            "--workspace",
            str(workspace),
            "--target",
            ns.target,
            "--format",
            "json",
        ]
    )
    disk_packet = load_work_packet(workspace)
    if disk_packet is not None:
        disk_packet["_vaa_exit_code"] = envelope.get("_vaa_exit_code", 0)
        envelope = disk_packet

    steps: list[dict[str, Any]] = []

    # Attempt 1: mutate an authority path → policy_blocked.
    blocked = run_vaa(
        [
            "harness",
            "submit",
            "--mode",
            "generator-repair",
            "--repair-packet",
            ns.repair_packet,
            "--workspace",
            str(workspace),
            "--changed-file",
            ns.forbidden_path,
            "--patched-revision",
            ns.blocked_revision,
            "--suite-evidence",
            ns.suite_evidence,
            "--format",
            "json",
        ]
    )
    steps.append(
        {
            "index": 0,
            "kind": "forbidden_path",
            "changed_file": ns.forbidden_path,
            "class": blocked.get("class"),
            "failure_code": blocked.get("failure_code"),
            "exit_code": blocked.get("_vaa_exit_code"),
        }
    )
    if blocked.get("class") != "policy_blocked":
        return {
            "schema_version": "0.1",
            "kind": "agent_harness_loop",
            "mode": "generator_repair",
            "envelope": {
                "task_id": envelope.get("task_id"),
                "target": envelope.get("target"),
                "mode": envelope.get("mode"),
            },
            "steps": steps,
            "final": {
                "class": blocked.get("class"),
                "failure_code": blocked.get("failure_code"),
                "exit_code": blocked.get("_vaa_exit_code"),
            },
            "_vaa_exit_code": blocked.get("_vaa_exit_code", 1),
        }

    # Attempt 2: allowed generator path + suite evidence → accepted.
    accepted = run_vaa(
        [
            "harness",
            "submit",
            "--mode",
            "generator-repair",
            "--repair-packet",
            ns.repair_packet,
            "--workspace",
            str(workspace),
            "--changed-file",
            ns.allowed_path,
            "--patched-revision",
            ns.accepted_revision,
            "--suite-evidence",
            ns.suite_evidence,
            "--format",
            "json",
        ]
    )
    steps.append(
        {
            "index": 1,
            "kind": "allowed_patch",
            "changed_file": ns.allowed_path,
            "class": accepted.get("class"),
            "failure_code": accepted.get("failure_code"),
            "patch_evidence_path": accepted.get("patch_evidence_path"),
            "exit_code": accepted.get("_vaa_exit_code"),
        }
    )

    return {
        "schema_version": "0.1",
        "kind": "agent_harness_loop",
        "mode": "generator_repair",
        "envelope": {
            "task_id": envelope.get("task_id"),
            "target": envelope.get("target"),
            "mode": envelope.get("mode"),
        },
        "steps": steps,
        "final": {
            "class": accepted.get("class"),
            "next_action": accepted.get("next_action"),
            "failure_code": accepted.get("failure_code"),
            "patch_evidence_path": accepted.get("patch_evidence_path"),
            "exit_code": accepted.get("_vaa_exit_code"),
        },
        "_vaa_exit_code": accepted.get("_vaa_exit_code", 0),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_prep = sub.add_parser("prepare-direct")
    p_prep.add_argument("--task", required=True)
    p_prep.add_argument("--contract", required=True)
    p_prep.add_argument("--workspace", required=True)
    p_prep.add_argument("--seed")
    p_prep.add_argument("--assembler", default="nasm", choices=["nasm", "gas"])
    p_prep.add_argument("--run-dir")
    p_prep.add_argument("--allow-execution", action="store_true")

    p_gen = sub.add_parser("prepare-generator")
    p_gen.add_argument("--repair-packet", required=True)
    p_gen.add_argument("--workspace", required=True)
    p_gen.add_argument("--target", default="x86_64-pc-windows-msvc")

    p_sub = sub.add_parser("submit")
    p_sub.add_argument("--task", required=True)
    p_sub.add_argument("--contract", required=True)
    p_sub.add_argument("--source", required=True)
    p_sub.add_argument("--assembler", default="nasm", choices=["nasm", "gas"])
    p_sub.add_argument("--allow-execution", action="store_true")
    p_sub.add_argument("--allow-under-preconditions", action="store_true")
    p_sub.add_argument("--timeout", type=int, default=120)
    p_sub.add_argument("--run-dir")
    p_sub.add_argument("--run-base")
    p_sub.add_argument("--idempotency-key")
    p_sub.add_argument(
        "--level",
        choices=["fast", "full", "seal"],
        help="Submit verify depth (fast never seals / never enables execution)",
    )

    p_gsub = sub.add_parser("submit-generator")
    p_gsub.add_argument("--repair-packet", required=True)
    p_gsub.add_argument("--workspace", required=True)
    p_gsub.add_argument("--patched-revision", required=True)
    p_gsub.add_argument("--changed-file", action="append", default=[])
    p_gsub.add_argument("--suite")
    p_gsub.add_argument("--suite-evidence")
    p_gsub.add_argument("--base-revision")
    p_gsub.add_argument("--run-base")
    p_gsub.add_argument("--repo")

    p_st = sub.add_parser("status")
    p_st.add_argument("--run-dir", required=True)

    p_loop = sub.add_parser(
        "loop-direct",
        help="Deterministic envelope→edit→submit loop over --candidate sources",
    )
    p_loop.add_argument("--task", required=True)
    p_loop.add_argument("--contract", required=True)
    p_loop.add_argument("--workspace", required=True)
    p_loop.add_argument("--run-base", required=True)
    p_loop.add_argument(
        "--candidate",
        action="append",
        required=True,
        help="Candidate source to copy into the workspace (repeatable, in order)",
    )
    p_loop.add_argument("--seed")
    p_loop.add_argument("--assembler", default="nasm", choices=["nasm", "gas"])
    p_loop.add_argument("--allow-execution", action="store_true")
    p_loop.add_argument("--allow-under-preconditions", action="store_true")
    p_loop.add_argument("--timeout", type=int, default=120)
    p_loop.add_argument(
        "--level",
        choices=["fast", "full", "seal"],
        help="Optional submit --level passthrough (default: seal via --run-base)",
    )

    p_loop_gen = sub.add_parser(
        "loop-generator",
        help="Deterministic generator-repair loop: forbidden path then accepted patch",
    )
    p_loop_gen.add_argument("--repair-packet", required=True)
    p_loop_gen.add_argument("--workspace", required=True)
    p_loop_gen.add_argument("--suite-evidence", required=True)
    p_loop_gen.add_argument(
        "--forbidden-path",
        default="integrations/hlax64/cases/stack_local_i64/task.vaa.toml",
        help="Authority path that must be policy_blocked",
    )
    p_loop_gen.add_argument(
        "--allowed-path",
        default="src/HlaX64.Backend.Nasm/Emit.cs",
        help="Generator path allowed by the repair packet policy",
    )
    p_loop_gen.add_argument("--blocked-revision", default="git:deadbeef")
    p_loop_gen.add_argument("--accepted-revision", default="git:cafebabe")
    p_loop_gen.add_argument("--target", default="x86_64-pc-windows-msvc")

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
            "--assembler",
            ns.assembler,
            "--format",
            "json",
        ]
        if ns.seed:
            args.extend(["--seed", ns.seed])
        if ns.run_dir:
            args.extend(["--run-dir", ns.run_dir])
        if ns.allow_execution:
            args.append("--allow-execution")
        return emit(run_vaa(args))
    if ns.cmd == "prepare-generator":
        return emit(
            run_vaa(
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
        )
    if ns.cmd == "submit":
        args = [
            "harness",
            "submit",
            "--mode",
            "direct-nasm",
            "--task",
            ns.task,
            "--contract",
            ns.contract,
            "--source",
            ns.source,
            "--assembler",
            ns.assembler,
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
        if ns.run_base:
            args.extend(["--run-base", ns.run_base])
        if ns.idempotency_key:
            args.extend(["--idempotency-key", ns.idempotency_key])
        if ns.level:
            args.extend(["--level", ns.level])
        return emit(run_vaa(args, timeout=float(ns.timeout) + 60.0))
    if ns.cmd == "submit-generator":
        args = [
            "harness",
            "submit",
            "--mode",
            "generator-repair",
            "--repair-packet",
            ns.repair_packet,
            "--workspace",
            ns.workspace,
            "--patched-revision",
            ns.patched_revision,
            "--format",
            "json",
        ]
        for path in ns.changed_file:
            args.extend(["--changed-file", path])
        if ns.suite:
            args.extend(["--suite", ns.suite])
        if ns.suite_evidence:
            args.extend(["--suite-evidence", ns.suite_evidence])
        if ns.base_revision:
            args.extend(["--base-revision", ns.base_revision])
        if ns.run_base:
            args.extend(["--run-base", ns.run_base])
        if ns.repo:
            args.extend(["--repo", ns.repo])
        return emit(run_vaa(args))
    if ns.cmd == "loop-direct":
        return emit(loop_direct(ns))
    if ns.cmd == "loop-generator":
        return emit(loop_generator(ns))
    return emit(
        run_vaa(["harness", "status", "--run-dir", ns.run_dir, "--format", "json"])
    )


if __name__ == "__main__":
    raise SystemExit(main())
