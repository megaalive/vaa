#!/usr/bin/env python3
"""Generate expected canonical JSON fixtures (approximate; Rust test is authoritative)."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parent
vectors = json.loads(root.joinpath("vectors.json").read_text(encoding="utf-8"))


def sort_value(v):
    if isinstance(v, dict):
        return {k: sort_value(v[k]) for k in sorted(v.keys())}
    if isinstance(v, list):
        return [sort_value(x) for x in v]
    return v


canon_lines = []
sha_lines = []
for item in vectors:
    canonical = json.dumps(sort_value(item["value"]), separators=(",", ":"), ensure_ascii=False)
    digest = hashlib.sha256(canonical.encode("utf-8")).hexdigest()
    canon_lines.append(f"{item['name']}\t{canonical}")
    sha_lines.append(f"{item['name']}\tsha256:{digest}")
    print(item["name"], canonical, digest)

root.joinpath("expected-canonical.jsonl").write_text("\n".join(canon_lines) + "\n", encoding="utf-8")
root.joinpath("expected-sha256.txt").write_text("\n".join(sha_lines) + "\n", encoding="utf-8")
print("wrote expected files")
