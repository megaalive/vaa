# AGENTS.md

Guidance for coding agents (Codex, Claude, Cursor, …) working in this repo.

## Agent harness (assembly leaf repair)

If asked to repair/verify a VAA/SemASM assembly leaf, follow the project skill
[`.cursor/skills/vaa-harness/SKILL.md`](.cursor/skills/vaa-harness/SKILL.md).
The single source of truth for what you may claim is
[`docs/HONESTY.md`](docs/HONESTY.md); the allowed shapes are
[`schemas/agent-leaf-allowlist.json`](schemas/agent-leaf-allowlist.json).

Non-negotiables:

- **Propose only.** SemASM verifies; you edit candidates.
- **Parse stdout JSON only.** stderr is noise.
- **Allowlist only.** Anything else → decline (see
  [`docs/agent-playbook.md`](docs/agent-playbook.md) decline path).
- **`verified_under_preconditions` ≠ `verified`.** Only `max_i64` is strict.
- **Dry-runs / stub `VAA_BIN` runs are not evidence.**
- Prefer `python scripts/agent_harness_adapter.py loop-direct …` over ad-hoc
  `vaa harness` pipelines.

This is a local CLI + project skill. There is **no MCP server** and no HTTP API.

## General repo work

For non-harness changes, keep VAA's fail-closed contract intact: never promote
`incomplete` / `failed` / VUP to success, and keep schemas, golden fixtures, and
docs in sync (protocol-freeze gates enforce this). See [`README.md`](README.md).
