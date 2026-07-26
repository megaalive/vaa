#!/usr/bin/env python3
"""Canned `vaa` stand-in for adapter dry-runs — no toolchain, no SemASM.

Replays responses from the scenario file named by `VAA_STUB_SCENARIO` in call
order and appends every invocation to `VAA_STUB_CALLS`, so a dry-run can assert
both the adapter's stdout contract and the CLI calls it makes.

This stub deliberately knows nothing about verification: it must never be used
to claim evidence.
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path


def main() -> int:
    scenario_path = os.environ.get("VAA_STUB_SCENARIO")
    if not scenario_path:
        sys.stderr.write("stub_vaa: VAA_STUB_SCENARIO is not set\n")
        return 70

    scenario = json.loads(Path(scenario_path).read_text(encoding="utf-8"))
    responses: list[dict] = scenario["responses"]

    calls_path = Path(os.environ["VAA_STUB_CALLS"])
    calls = (
        json.loads(calls_path.read_text(encoding="utf-8"))
        if calls_path.is_file()
        else []
    )
    index = len(calls)
    calls.append(sys.argv[1:])
    calls_path.write_text(json.dumps(calls, indent=2), encoding="utf-8")

    if index >= len(responses):
        sys.stderr.write(
            f"stub_vaa: unexpected call {index} ({' '.join(sys.argv[1:])}); "
            f"scenario declares {len(responses)}\n"
        )
        return 70

    response = responses[index]
    expect = response.get("expect_args")
    if expect is not None and sys.argv[1 : 1 + len(expect)] != expect:
        sys.stderr.write(
            f"stub_vaa: call {index} expected prefix {expect}, got {sys.argv[1:]}\n"
        )
        return 70

    payload = dict(response["stdout_json"])
    sys.stderr.write("stub_vaa: this line is noise and must be ignored\n")
    json.dump(payload, sys.stdout)
    sys.stdout.write("\n")
    return int(response.get("exit_code", 0))


if __name__ == "__main__":
    raise SystemExit(main())
