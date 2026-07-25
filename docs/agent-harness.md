# Agent harness (VAA + SemASM)

Thin CLI loop for agents that either edit NASM directly or repair a generator.
SemASM remains the verifier; VAA owns task lock, budgets, path policy, seals,
and session resume. Agents are proposers only.

## Modes

| Mode | Agent edits | Authority packet | Success |
|---|---|---|---|
| `direct-nasm` | `candidate.asm` only | SemASM `TaskPacket` (+ VAA task/contract) | SemASM `verified` (or explicit under-preconditions) |
| `generator-repair` | Allowed generator source paths | VAA `RepairPacket` | Regenerate → suite Accepted + patch evidence; never edit generated `.asm` |

Never promote `incomplete` / `execution_denied` to success. Fb9c arbitrary loop
invariants stay locked.

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
- `prompt.md` — bounded human/agent brief
- `candidate.asm` — writable in direct mode
- `semasm-packet.json` — best-effort SemASM packet when SemASM is on PATH
- `repair-packet.json` / `.md` — generator mode only

## Commands

```text
vaa harness prepare --mode direct-nasm \
  --task task.vaa.toml --contract contract.sem.toml \
  --workspace .vaa/harness/case --seed seed.asm [--allow-execution]

vaa harness prepare --mode generator-repair \
  --repair-packet repair-packet.json --workspace .vaa/harness/repair \
  --target x86_64-pc-windows-msvc

vaa harness submit --task … --contract … --source candidate.asm \
  [--allow-execution] [--allow-under-preconditions] [--timeout 120] \
  [--run-dir …] [--idempotency-key …]

vaa harness resume --run-dir <run>
vaa harness status --run-dir <run>
```

Stdout is one JSON document (default `--format json`). Stderr is human noise —
controllers must parse stdout alone.

### Submit outcome classes

| `class` | Meaning | Auto-retry? |
|---|---|---|
| `accepted` | Verified (or allowed under-preconditions) | no |
| `violated_repairable` | Semantic/behavior violation — edit candidate/generator | no |
| `incomplete_coverage` | Gate-1 only / incomplete — do not promote | no |
| `toolchain_retryable` | Missing tool / I/O / timeout | yes (tooling only) |
| `policy_blocked` | Forbidden path / security | never |
| `failed` | Hard failure | no |

## Windows vs WSL (SysV)

- **Win64** (`x86_64-pc-windows-msvc`): native Windows `semasm` + NASM + MSVC/link toolchain.
- **SysV** (`x86_64-unknown-linux-gnu`): use WSL with **Linux** `dotnet` (if regenerating),
  Linux SemASM/NASM/GCC. Do not point WSL at a Windows SDK `dotnet`.

Pin toolchain digests in `stack.lock.toml` / case manifests; do not invent
cross-OS assembly claims.

## Reference adapter

See [`../scripts/agent_harness_adapter.py`](../scripts/agent_harness_adapter.py):
spawn `vaa harness …`, parse stdout JSON, never scrape stderr for decisions.

## Protocol freeze

- SemASM: `docs/CONTROLLER_PROTOCOL.md`, `docs/CLI_COMPATIBILITY.md`,
  VerificationReport `>=0.4,<0.6`, early `agent_failure` schema `0.1`.
- VAA: `schemas/agent-envelope.schema.json`, `schemas/repair-packet.schema.json`,
  submit result schema `0.1`.

No HTTP/MCP surface until this CLI+JSON loop is stable in CI.
