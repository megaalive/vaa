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
| `nasm` | Supported for x86_64 Win64 / SysV | `candidate.asm` |
| `gas` | Supported for AArch64 / RISC-V Linux | `candidate.S` |

`gas` on x86_64 stays **fail-closed** (SemASM+VAA remain NASM/Intel there).
VAA object-inspect / build / reproducible twin-assemble select `nasm` vs
`aarch64-linux-gnu-as` / `riscv64-linux-gnu-as` from the flavor+target pair.
CI job **Agent harness gates (SemASM tip, GAS AArch64)** seals a live
wrong→repaired `.S` loop with SemASM tip + qemu-aarch64.

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

Deterministic loop (no LLM) — prepare, then apply candidates in order until
accepted / budget / policy:

```text
python scripts/agent_harness_adapter.py loop-direct \
  --task fixtures/run/count_byte/count_byte.vaa.toml \
  --contract fixtures/run/count_byte/count_byte.sem.toml \
  --workspace .vaa/harness/demo --run-base .vaa/runs \
  --allow-execution --allow-under-preconditions \
  --candidate fixtures/run/count_byte/01_wrong.asm \
  --candidate fixtures/run/count_byte/02_repaired.asm
```

Generator-repair rehearsal (policy block → accepted patch evidence):

```text
python scripts/agent_harness_adapter.py loop-generator \
  --repair-packet fixtures/repair/hlax64-stack-balance-win64-live-worktree/repair-packet.json \
  --workspace .vaa/harness/gen-demo \
  --suite-evidence fixtures/repair/echoasm-passthrough/suite-evidence.accepted.json
```

## Protocol freeze

- SemASM: `docs/CONTROLLER_PROTOCOL.md`, `docs/CLI_COMPATIBILITY.md`,
  VerificationReport `>=0.4,<0.6`, early `agent_failure` schema `0.1`.
- VAA: `schemas/agent-envelope.schema.json`, `schemas/repair-packet.schema.json`,
  `schemas/harness-submit-result.schema.json`, submit result schema `0.1`.
  Golden fixtures live under `schemas/fixtures/`; `tests/protocol_freeze_gates.rs`
  fails on field/schema drift.
