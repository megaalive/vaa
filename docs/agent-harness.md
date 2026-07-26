# Agent harness (VAA + SemASM)

Thin CLI loop for agents that either edit assembly directly or repair a generator.
SemASM remains the verifier; VAA owns task lock, budgets, path policy, seals,
and session resume. Agents are proposers only.

## Modes

| Mode | Agent edits | Authority packet | Success |
|---|---|---|---|
| `direct-nasm` / `direct` | Candidate assembly only | SemASM `TaskPacket` (+ VAA task/contract) | SemASM `verified` (or explicit under-preconditions) + optional seal |
| `generator-repair` | Allowed generator source paths | VAA `RepairPacket` | Suite Accepted + patch evidence Accepted; never edit generated assembly |

Never promote `incomplete` / `execution_denied` to success. Fb9c arbitrary loop
invariants stay locked.

## Assembler flavors

| Flavor | Status | Candidate file |
|---|---|---|
| `nasm` | Supported (Win64 / SysV x86_64) | `candidate.asm` |
| `gas` | **Reserved / fail-closed** | `candidate.S` (when enabled) |

SemASM already has gas dialects for AArch64/RISC-V. VAA's build + object-inspect
path is still NASM-hardcoded for x86_64, so `vaa harness … --assembler gas`
rejects until that wiring lands. Do not invent cross-assembler claims.

## Case kit layout

```text
case/
  task.vaa.toml          # locked task (immutable)
  contract.sem.toml      # SemASM contract (immutable)
  seed.asm               # optional starting candidate
  stack.lock.toml        # optional generator pin
```

After `prepare`, the workspace also contains:

- `agent-envelope.json` — machine payload (`schemas/agent-envelope.schema.json`)
- `prompt.md` — bounded human/agent brief (remaining attempts, assembler, verify)
- `candidate.asm` (or `.S`) — writable in direct mode
- `semasm-packet.json` — best-effort SemASM packet when SemASM is on PATH
- `repair-packet.json` / `.md` — generator mode only

## Commands

```text
vaa harness prepare --mode direct-nasm \
  --task task.vaa.toml --contract contract.sem.toml \
  --workspace .vaa/harness/case --seed seed.asm \
  [--assembler nasm] [--run-dir <existing>] [--allow-execution]

vaa harness prepare --mode generator-repair \
  --repair-packet repair-packet.json --workspace .vaa/harness/repair \
  --target x86_64-pc-windows-msvc

# Verify-only (no seal):
vaa harness submit --mode direct-nasm \
  --task … --contract … --source candidate.asm \
  [--allow-execution] [--allow-under-preconditions] [--timeout 120]

# Seal into a new or existing run:
vaa harness submit --mode direct-nasm \
  --task … --contract … --source candidate.asm \
  --allow-execution --run-base .vaa/runs
# or --run-dir <existing-run> to append the next candidate index

vaa harness submit --mode generator-repair \
  --repair-packet … --workspace … \
  --changed-file src/… --patched-revision <rev> \
  [--suite suite.toml | --suite-evidence suite-evidence.json] \
  [--run-base …] [--repo …]

vaa harness resume --run-dir <run>
vaa harness status --run-dir <run>
```

Stdout is one JSON document (default `--format json`). Stderr is human noise —
controllers must parse stdout alone. `status`/`resume` expose `events.jsonl`,
evidence dir, seal cursor, and recent events — not human logs as decision truth.

### Submit outcome classes

| `class` | Meaning | Auto-retry? |
|---|---|---|
| `accepted` | Verified / patch Accepted | no |
| `violated_repairable` | Semantic/behavior / suite rejected — edit candidate/generator | no |
| `incomplete_coverage` | Gate-1 only / suite missing — do not promote | no |
| `toolchain_retryable` | Missing tool / I/O / timeout | yes (tooling only) |
| `policy_blocked` | Forbidden / authority path mutation | never |
| `failed` | Hard failure | no |

## Windows vs WSL (SysV)

- **Win64** (`x86_64-pc-windows-msvc`): native Windows `semasm` + NASM + MSVC/link toolchain.
- **SysV** (`x86_64-unknown-linux-gnu`): use WSL with **Linux** `dotnet` (if regenerating),
  Linux SemASM/NASM/GCC. Do not point WSL at a Windows SDK `dotnet`.

## Reference adapter

See [`../scripts/agent_harness_adapter.py`](../scripts/agent_harness_adapter.py):
spawn `vaa harness …`, parse stdout JSON, never scrape stderr for decisions.

## Protocol freeze

- SemASM: `docs/CONTROLLER_PROTOCOL.md`, `docs/CLI_COMPATIBILITY.md`,
  VerificationReport `>=0.4,<0.6`, early `agent_failure` schema `0.1`.
- VAA: `schemas/agent-envelope.schema.json`, `schemas/repair-packet.schema.json`,
  submit result schema `0.1`.

No HTTP/MCP surface until this CLI+JSON loop stays stable in CI.
