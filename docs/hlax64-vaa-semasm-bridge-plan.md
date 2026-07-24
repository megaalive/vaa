# External Generator Verified Repair Bridge Plan

**(First instance: HlaX64)**

**Status:** Execution plan  
**Primary repositories:** VAA, SemASM, plus one external generator repository  
**First generator instance:** HlaX64  
**Primary agent surfaces:** any coding agent or human editor (examples only: Claude, Codex, Cursor, …)  
**Goal:** Add **generator-agnostic** layers in VAA so any locked external generator can regenerate candidates, obtain SemASM evidence, run regression suites, and receive sealed patch acceptance — with HlaX64 as the first concrete instance, not the only supported generator.

---

## 0. Universality (non-coupling) — hard rules

VAA must remain a **universal acceptance controller**. HlaX64 is the first *integration pack*, not a permanent special case inside VAA core.

### 0.1 What must stay generic in VAA core

| Layer | Generic name | Must NOT hardcode |
|---|---|---|
| Stack lock | multi-generator lock | only `hlax64` keys |
| Generator spec | `ExternalGeneratorSpec` | HlaX64 CLI flags or crate paths in Rust |
| Repo / patch policy | path allow/deny on *any* generator repo | HlaX64 `src/backend/**` in VAA source |
| Build identity | build command + binary digest | `hlax64.exe` as a type |
| Deterministic generation | locked argv template | HlaX64 `compile` subcommand in core |
| Suite / patch evidence | suite + patch schemas | suite_id prefix `hlax64.` required |
| Repair packet | failure + commands + constraints | “Fix HlaX64…” as the only prompt |
| CLI | `vaa generator-run` / `suite` / `patch` / `repair` | `compiler-run` as the only public verb |

### 0.2 What may be HlaX64-specific

Only under an **instance pack**:

```text
integrations/hlax64/   # or docs/examples — not crates/vaa core modules named hlax64_*
```

Instance packs own: generator TOML, suite manifests, cases (`input.hlx`, tasks, contracts), and thin wrapper scripts. Adding a second generator (e.g. another HLA, a locked template emitter, a research backend) means a new pack + stack-lock entry — **not** a VAA fork.

### 0.3 Coupling red flags (reject in review)

- `use hlax64::…` or subprocess argv built from string literals `"hlax64"` in VAA core;
- schemas that require `.hlx` as the only input kind;
- diagnostics or triage that assume “compiler backend” is the only repair target;
- CI jobs that cannot be parameterized by `generator_id`;
- documentation that says “VAA is the HlaX64 verifier.”

### 0.4 Allowed claim shape

> VAA can accept or reject patches to an **external generator** through locked regeneration, SemASM verification, and suite evidence.  
> The first shipped instance is HlaX64 Win64/SysV backend slices.

Forbidden:

> VAA is an HlaX64 product / only works with HlaX64.

---

## 1. Objective

Create a small integration layer that allows a human developer, coding agent, or other editor to modify an **external generator repository** (first instance: HlaX64) while preserving a strict acceptance boundary:

```text
repository patch (generator under test)
    ↓
identified generator build (binary digest)
    ↓
deterministic candidate regeneration
    ↓
SemASM technical verification
    ↓
multi-case regression suite (instance pack)
    ↓
VAA patch-level acceptance
    ↓
sealed patch evidence
```

The bridge must ensure that:

1. agents may edit generator source, but cannot redefine acceptance criteria;
2. generated assembly is always recreated from locked generator inputs;
3. SemASM remains the technical verification authority;
4. VAA remains the acceptance and evidence authority (generator-agnostic);
5. a patch is accepted only when all required cases pass;
6. every accepted result is bound to repository revision, patch digest, generator binary digest, generation command, candidate artifacts, SemASM reports, and suite outcome;
7. HlaX64-specific paths, file extensions, and commands live in the instance pack, not VAA core types.

---

## 2. Scope

### 2.1 Included (VAA core — generic)

- multi-generator stack lock;
- `ExternalGeneratorSpec` (build + generation + identity + patch policy);
- repository guard (revision, clean worktree, allow/deny paths);
- generator build identity (command, binary digest, toolchain);
- deterministic generation from locked inputs;
- per-case verification via existing SemASM/VAA evidence path;
- regression-suite aggregation;
- repair packet export for coding agents and humans;
- patch-level evidence;
- optional source-to-assembly mapping join;
- stable diagnostic codes;
- CI execution parameterized by `generator_id`;
- bundle and chain verification;
- exact SemASM and VAA revision pinning.

### 2.2 Included (first instance pack — HlaX64)

- HlaX64 repository pin and clean-worktree checks in pack config;
- HlaX64 build/generation command templates;
- HlaX64 case corpus (`input.hlx`, tasks, contracts, vectors);
- thin `build-hlax64` / `verify-*` scripts as conveniences only.

### 2.3 Excluded from the first release

- autonomous Git hosting integration;
- automatic pull-request creation;
- unrestricted arbitrary shell for the agent;
- remote multi-user execution service;
- automatic merge to the default branch;
- formal verification of the entire HlaX64 compiler (or any generator);
- all HlaX64 language features at once;
- all CPU architectures at once;
- automatic weakening of contracts or test vectors;
- automatic changes to VAA or SemASM when a generator fails;
- baking a second generator into VAA core before the generic schema/CLI lands.

---

## 3. Trust model

| Component | Role | Authority |
|---|---|---|
| Coding agent / human | proposes repository changes | no acceptance authority |
| External generator (e.g. HlaX64) | generates candidate assembly | no acceptance authority |
| SemASM | verifies technical properties of generated artifacts | technical evidence authority |
| VAA | locks tasks, evaluates policy, aggregates suites, seals evidence | final acceptance authority |
| locked contracts and vectors | define expected behavior | immutable authority inputs |
| CI runner | executes reproducible workflow | operational executor, not semantic authority |

Core rule:

```text
The generator proposes.
SemASM evaluates.
VAA accepts or rejects.
```

HlaX64 is one generator. SemASM and VAA do not become “HlaX64 products.”

---

## 4. Target architecture

```text
┌─────────────────────────────────────────────────────┐
│ Coding agent / Human                                │
│ Produces a patch limited to allowed repository paths │
└─────────────────────────┬───────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────┐
│ Repository Guard  (generic)                         │
│ - base revision                                     │
│ - clean-worktree requirement                        │
│ - allowed/forbidden path policy                     │
│ - patch digest                                      │
└─────────────────────────┬───────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────┐
│ Generator Build Identity  (generic)                 │
│ - build command                                     │
│ - generator binary digest                           │
│ - toolchain identity                                │
│ - build log digest                                  │
└─────────────────────────┬───────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────┐
│ Deterministic Generator Bridge  (generic)           │
│ - locked input path(s) from instance pack           │
│ - fixed generation command template                 │
│ - clean output directory                            │
│ - generated assembly digest                         │
│ - optional source map                               │
└─────────────────────────┬───────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────┐
│ SemASM Verification  (unchanged role)               │
│ - object/decode/lower/ABI/CFG                       │
│ - behavioral oracle                                 │
│ - alias/region evidence when required               │
│ - structured diagnostics                            │
└─────────────────────────┬───────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────┐
│ VAA Case Acceptance  (generic)                      │
│ - task identity                                     │
│ - evidence profile                                  │
│ - bundle and chain                                  │
│ - per-case final status                             │
└─────────────────────────┬───────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────┐
│ VAA Suite + Patch Acceptance  (generic)             │
│ - all required cases                                │
│ - regression policy                                 │
│ - patch evidence                                    │
│ - final suite seal                                  │
└─────────────────────────────────────────────────────┘
```

---

## 5. Repository layout

Generic schemas and (optional) empty templates live in VAA. **Instance packs** hold generator-specific material.

```text
integrations/
├── README.md                      # how to add a new generator pack
├── _schemas/                      # OR schemas/ under VAA docs — generator-agnostic
│   ├── vaa-generator.schema.json
│   ├── vaa-suite.schema.json
│   ├── repair-packet.schema.json
│   ├── patch-evidence.schema.json
│   └── stack.lock.schema.json
└── hlax64/                        # FIRST INSTANCE PACK (not VAA core)
    ├── stack.lock.toml            # may live at pack root or repo root; see §6
    ├── generators/
    │   └── hlax64.vaa-generator.toml
    ├── suites/
    │   ├── backend-win64-v1.vaa-suite.toml
    │   └── backend-sysv-v1.vaa-suite.toml
    ├── cases/
    │   ├── return_i64/
    │   │   ├── input.hlx          # HlaX64-specific input kind
    │   │   ├── task.vaa.toml
    │   │   ├── contract.sem.toml
    │   │   └── vectors.json
    │   ├── add_i64/
    │   ├── min_i64/
    │   ├── max_i64/
    │   ├── count_byte/
    │   ├── memcmp/
    │   ├── memset/
    │   └── memcpy/
    └── scripts/                   # thin wrappers; logic stays in `vaa` CLI
        ├── build-hlax64.ps1
        ├── build-hlax64.sh
        ├── verify-case.ps1
        ├── verify-case.sh
        ├── verify-suite.ps1
        └── verify-suite.sh
```

A second generator later:

```text
integrations/<other_generator_id>/
  generators/...
  suites/...
  cases/...
```

Recommended HlaX64-side additions (stay in HlaX64 repo):

```text
hlax64/
├── tools/
│   └── source-map-export/
├── tests/
│   └── verified-backend/
└── docs/
    └── verified-backend-workflow.md
```

---

## 6. Stack lock

Create `stack.lock.toml` (pack-scoped or workspace-scoped) to prevent floating
`main` tips from changing acceptance behavior. Support **one or more** generators.

```toml
schema_version = "0.1"

[vaa]
repository = "https://github.com/megaalive/vaa"
revision = "git:<exact-commit>"
binary_sha256 = "sha256:<digest>"

[semasm]
repository = "https://github.com/megaalive/semasm"
revision = "git:<exact-commit>"
binary_sha256 = "sha256:<digest>"

[toolchain]
nasm = "<version>"
lld = "<version>"
rust = "<version>"

# First instance — additional [[generators]] / [generators.*] entries allowed later.
[generators.hlax64]
repository = "https://github.com/megaalive/hlax64"
revision = "git:<exact-commit>"
# optional: binary_sha256 after build identity is established
```

### Invariants

- exact revisions are mandatory;
- `main`, `latest`, and floating version ranges are rejected;
- binary digest mismatch is a hard failure;
- lock changes must be explicit and reviewed;
- lock digest must be included in patch evidence;
- adding a generator is a new lock section + pack, not a VAA API break.

---

## 7. External generator specification

Add a generator sidecar schema (`ExternalGeneratorSpec`). Naming uses
“generator”, not “HlaX64 compiler”, so other emitters fit the same shape.

### 7.1 Example (HlaX64 instance)

```toml
schema_version = "0.1"
generator_id = "hlax64.x86_64.win64"
# Optional taxonomy for triage/docs only — not a VAA core enum gate:
# kind = "compiler" | "translator" | "template_emitter" | "other"

[repository]
path = "../hlax64"
expected_revision = "git:<commit>"
require_clean_worktree = true
allow_untracked_files = false

[build]
command = ["cargo", "build", "--release"]
working_directory = "../hlax64"
# Field name is generic: path to the tool that emits candidates.
generator_binary = "target/release/hlax64.exe"
timeout_seconds = 600

[generation]
# Placeholders are generic: {generator}, {input}, {target}, {output}
command = [
  "{generator}",
  "compile",
  "{input}",
  "--target",
  "{target}",
  "--output",
  "{output}"
]
working_directory = "../hlax64"
clean_output_directory = true
timeout_seconds = 120

[identity]
require_generator_digest = true
require_build_log_digest = true
require_toolchain_identity = true

[patch_policy]
# Paths are relative to the *generator* repository under test.
allowed_paths = [
  "src/backend/**",
  "src/codegen/**",
  "src/ir/**",
  "tests/backend/**"
]

forbidden_paths = [
  # Authority files in the VAA instance pack (never editable via generator repair):
  "**/integrations/hlax64/cases/**/task.vaa.toml",
  "**/integrations/hlax64/cases/**/contract.sem.toml",
  "**/integrations/hlax64/cases/**/vectors.json",
  "**/integrations/hlax64/stack.lock.toml"
]
```

### 7.2 Validation rules

Reject the run when:

- the repository revision is not the expected base;
- the starting worktree is dirty;
- an agent modifies a forbidden path;
- a generated output is missing;
- multiple ambiguous outputs are produced;
- the generator binary digest cannot be established;
- the generation command is not the locked command;
- output exists before generation and cannot be cleaned;
- generation happens outside the declared working directory.

### 7.3 Input kinds (instance concern)

The case directory may use different primary inputs (`input.hlx`, `input.s`,
`program.ir`, …). VAA core only requires that the generator spec’s `{input}`
placeholder resolve to a path declared by the case pack — it does not require
`.hlx`.

---

## 8. VAA command surface

Commands are **generator-agnostic**. HlaX64 scripts may wrap them; they must
not be the only entry point.

### 8.1 `vaa generator-run`

Purpose: build (if needed), generate, and verify one case without invoking a model.

Alias (optional, docs-only convenience): `vaa compiler-run` → same implementation.

```text
vaa generator-run \
  --task cases/min_i64/task.vaa.toml \
  --contract cases/min_i64/contract.sem.toml \
  --generator generators/hlax64.vaa-generator.toml \
  --input cases/min_i64/input.hlx \
  --output out/min_i64.asm
```

Execution:

1. load stack lock;
2. validate generator repository;
3. build generator (per spec);
4. hash generator binary;
5. clean output directory;
6. run generation command;
7. hash generated candidate;
8. run SemASM (via existing VAA verify/ingest path);
9. create VAA candidate bundle;
10. seal case evidence.

### 8.2 `vaa suite run`

Purpose: accept or reject a generator patch against a complete regression suite.

```text
vaa suite run suites/backend-win64-v1.vaa-suite.toml
```

### 8.3 `vaa repair export`

Purpose: generate a constrained repair packet for a coding agent or human.

```text
vaa repair export <run-dir> --format markdown
vaa repair export <run-dir> --format json
```

Packet text is filled from the generator pack (allowed paths, build/verify
commands). VAA does not embed “Fix HlaX64…” as a hard-coded prompt.

### 8.4 `vaa patch verify`

Purpose: validate an already-created repository patch against a suite.

```text
vaa patch verify \
  --base <commit> \
  --generator generators/hlax64.vaa-generator.toml \
  --suite suites/backend-win64-v1.vaa-suite.toml
```

### 8.5 `vaa patch evidence verify`

Purpose: verify the final patch-level evidence bundle.

```text
vaa patch evidence verify <patch-evidence-dir>
```

---

## 9. Suite manifest

### 9.1 Example

```toml
schema_version = "0.1"
suite_id = "hlax64.backend.win64.v1"
target = "x86_64-pc-windows-msvc"

[generator]
spec = "../generators/hlax64.vaa-generator.toml"

[policy]
require_all_cases = true
allow_verified_under_preconditions = false
allow_incomplete = false
stop_on_first_failure = false
max_parallel_cases = 1

required_cases = [
  "../cases/return_i64",
  "../cases/add_i64",
  "../cases/min_i64",
  "../cases/max_i64",
  "../cases/count_byte",
  "../cases/memcmp",
  "../cases/memset",
  "../cases/memcpy"
]
```

### 9.2 Suite status

```text
accepted
rejected
incomplete
failed
```

Rules:

- any required `violated` case → `rejected`;
- any required missing evidence → `incomplete`;
- toolchain, schema, or identity mismatch → `failed`;
- all required cases pass policy → `accepted`.

### 9.3 Suite evidence

The suite digest must bind:

- suite manifest digest;
- stack lock digest;
- base repository revision;
- patch digest;
- compiler binary digest;
- every case acceptance digest;
- every SemASM report digest;
- final suite status.

---

## 10. Patch evidence

Create a distinct artifact from candidate evidence.

### 10.1 Example

```json
{
  "schema_version": "0.1",
  "base_revision": "git:...",
  "patched_revision": "git:...",
  "patch_digest": "sha256:...",
  "changed_files": [
    "src/backend/x64/emitter.rs",
    "tests/backend/min_i64.rs"
  ],
  "forbidden_paths_changed": [],
  "compiler_binary_digest": "sha256:...",
  "generator_spec_digest": "sha256:...",
  "stack_lock_digest": "sha256:...",
  "suite_id": "hlax64.backend.win64.v1",
  "suite_digest": "sha256:...",
  "status": "accepted"
}
```

Prefer field name `generator_binary_digest` in schemas; keep
`compiler_binary_digest` only as a deprecated alias if needed for early
HlaX64 drafts.

### 10.2 Patch acceptance requirements

A patch is accepted only if:

- the base revision matches the locked revision;
- no forbidden path changed;
- the compiler builds successfully;
- the compiler binary identity is recorded;
- every required case is regenerated from clean outputs;
- every required VAA case policy is satisfied;
- the regression suite passes;
- patch evidence verifies;
- case chains and bundle seals verify.

---

## 11. Repair packet

The packet must help an agent modify **generator** source, not generated assembly.

### 11.1 JSON shape

```json
{
  "schema_version": "0.1",
  "task_id": "hlax64.min_i64.win64",
  "repository": {
    "base_revision": "git:...",
    "allowed_paths": [
      "src/backend/**",
      "src/codegen/**",
      "tests/backend/**"
    ],
    "forbidden_paths": [
      "integrations/hlax64/cases/**"
    ]
  },
  "failure": {
    "classification": "abi_violation",
    "diagnostic_code": "ABI_CALLEE_SAVED_001",
    "message": "RBX modified but not restored",
    "instruction_offset": "0x17"
  },
  "generated_artifact": {
    "path": "candidate.asm",
    "digest": "sha256:..."
  },
  "source_mapping": {
    "hla_source": "cases/min_i64/input.hlx:4",
    "ir_node": "CompareSigned#12",
    "compiler_source": "src/backend/x64/emitter.rs:214"
  },
  "commands": {
    "build": "scripts/build-hlax64.ps1",
    "regenerate": "scripts/verify-case.ps1 min_i64 --generate-only",
    "verify": "scripts/verify-case.ps1 min_i64"
  },
  "constraints": [
    "Do not edit generated assembly manually",
    "Do not edit contracts, vectors, task files, or stack.lock.toml",
    "Regenerate all candidates after compiler changes"
  ]
}
```

### 11.2 Markdown variant

The Markdown export should be directly usable as a task prompt for any coding agent or human reviewer.

---

## 12. Stable diagnostic codes

SemASM and VAA diagnostics used for automation must have stable codes.

Initial set:

```text
GEN_BUILD_FAILED_001
GEN_OUTPUT_MISSING_001
GEN_OUTPUT_AMBIGUOUS_001
GEN_NONDETERMINISTIC_001

ABI_CALLEE_SAVED_001
ABI_STACK_BALANCE_001
ABI_RETURN_REGISTER_001

CFG_INDIRECT_BRANCH_001
CFG_INCOMPLETE_001

DECODE_UNKNOWN_INSN_001
LOWER_UNKNOWN_EFFECT_001

MEM_REGION_ESCAPE_001
MEM_PERMISSION_DENIED_001
MEM_ALIAS_UNRESOLVED_001

BEHAVIOR_VECTOR_MISMATCH_001

POLICY_FORBIDDEN_PATH_CHANGED_001
POLICY_STACK_LOCK_MISMATCH_001
POLICY_COMPILER_DIGEST_MISMATCH_001
```

Messages may evolve. Diagnostic codes must remain stable within a schema major version.

---

## 13. HlaX64 source mapping

Add optional compiler output:

```text
candidate.asm
candidate.map.json
```

### 13.1 Map format

```json
{
  "schema_version": "0.1",
  "compiler_revision": "git:...",
  "entries": [
    {
      "assembly_line": 42,
      "instruction_offset": "0x31",
      "hla_source": "input.hlx:8:5",
      "ir_node": "StoreByte#17",
      "compiler_source": "src/backend/x64/store.rs:84"
    }
  ]
}
```

### 13.2 Use

- SemASM emits instruction offset;
- VAA joins the offset with the map;
- repair packet points to HlaX64 input, IR node, and backend source;
- agent edits compiler source rather than candidate assembly.

### 13.3 Fallback

If no map exists:

- verification still works;
- repair packet includes assembly context only;
- status is not downgraded solely because mapping is absent.

---

## 14. Generator-versus-verifier triage

Not every failure should trigger a patch to the generator under test.

### 14.1 Classification

```text
generator_candidate_violated
generator_behavior_failed
generator_build_failed
generator_failed
verifier_incomplete
verifier_unsupported
toolchain_unavailable
policy_mismatch
```

(Aliases `compiler_*` may appear in HlaX64 pack docs; VAA core uses `generator_*`.)

### 14.2 Routing

| Classification | Default action |
|---|---|
| generator_candidate_violated | repair generator repo |
| generator_behavior_failed | repair generator repo |
| generator_build_failed | repair generator build/source |
| generator_failed | repair generator command or generator repo |
| verifier_incomplete | create SemASM coverage issue |
| verifier_unsupported | create SemASM feature issue |
| toolchain_unavailable | repair environment |
| policy_mismatch | repair task/configuration, not generator |

VAA must not direct an agent to alter the generator when SemASM merely lacks semantic coverage.

---

## 15. Agent workflow (batch / autonomous)

Applies to any coding agent that can edit a worktree and run fixed commands
(examples only: Claude, Codex, Cursor, …). Product choice does not change
acceptance.

### 15.1 Preparation

1. create a clean worktree;
2. pin stack revisions;
3. run baseline suite;
4. export repair packet for one failing case;
5. provide one fixed verification command.

### 15.2 Agent instruction

```text
Fix the generator backend under test (see repair packet repository + allowed_paths).

You may edit only the allowed generator and backend test paths.
Do not edit generated assembly, SemASM contracts, VAA tasks,
authoritative vectors, stack lock, or evidence files.

After each change:
1. rebuild the generator;
2. regenerate candidate assembly from the locked generator input;
3. run the supplied case verification command;
4. run the required regression suite before completion.
```

### 15.3 Acceptance

Agent output is accepted only after `vaa patch verify` succeeds.

---

## 16. Interactive editor workflow

Applies to any IDE or interactive coding surface used by a human or agent
(examples only: Claude Code, Cursor, VS Code + agent, …).

### 16.1 Repository rules

Add a project rule that:

- forbids editing authority files;
- forbids manual candidate assembly fixes;
- requires clean regeneration;
- points to the single verification command;
- requires full suite before completion.

### 16.2 Interactive loop

1. open the generator backend source (HlaX64 for the first instance);
2. attach the Markdown repair packet;
3. inspect source map and generated assembly;
4. apply compiler patch;
5. run one case;
6. run full suite;
7. verify patch evidence.

The editor remains a code-reasoning surface, not an acceptance authority.

---

## 17. Initial backend corpus

Start with one target and one ABI.

Recommended first target:

```text
x86_64 Win64
```

or:

```text
x86_64 SysV
```

Choose the one HlaX64 currently handles most reliably.

### 17.1 Phase A — scalar leaf routines

```text
return_i64
add_i64
sub_i64
min_i64
max_i64
abs_i64
```

Covers:

- parameter registers;
- return register;
- signed comparison;
- conditional branches;
- simple control flow.

### 17.2 Phase B — loops and stack

```text
sum_range
countdown_loop
stack_local_i64
forced_register_spill
```

Covers:

- loop control;
- stack frame;
- spill/reload;
- stack alignment;
- callee-saved registers.

### 17.3 Phase C — memory reads

```text
count_byte
find_first_byte
find_last_byte
memcmp
```

Covers:

- pointer parameters;
- width-specific loads;
- bounds;
- alias assumptions;
- behavioral vectors.

### 17.4 Phase D — memory writes

```text
replace_byte
memset
memcpy
```

Covers:

- store permissions;
- region access;
- write bounds;
- caller preconditions;
- overlap handling.

### 17.5 Phase E — calls and data

```text
internal_function_call
nested_call
global_rodata
multiple_exports
small_struct_return
```

Covers:

- call ABI;
- stack preservation;
- symbols;
- sections;
- aggregate layout.

---

## 18. Milestones

## Milestone 0 — Integration freeze

**Status:** Partial — stack lock + `ExternalGeneratorSpec` schemas land in VAA
core with HlaX64 pack stubs (`integrations/hlax64/`). Baseline suite snapshot,
authority ownership checklist, and binary identity remain open.

**Deliverables**

- exact VAA/SemASM pins + first generator pin (HlaX64);
- baseline suite snapshot for the instance pack;
- authority file ownership;
- initial **generic** schemas (`ExternalGeneratorSpec`, suite, patch, repair, stack.lock).

**Acceptance**

- stack lock verifies with `[generators.<id>]` — **done** (`vaa generator validate-lock`);
- `ExternalGeneratorSpec` validates — **done** (`vaa generator validate-spec`);
- baseline build is reproducible enough to identify generator binary — open;
- no feature expansion during bridge work;
- review checklist includes §0 coupling red flags.

---

## Milestone 1 — External generator runner

**Status:** In progress — guard, identity, and deterministic generate landed;
`generator-run` open.

**Deliverables**

- `ExternalGeneratorSpec` — **done**;
- repository guard — **done** (`vaa generator check-repo`);
- generator build identity — **done** (`vaa generator identity`);
- deterministic candidate generation — **done** (`vaa generator generate`);
- `vaa generator-run` (optional alias `compiler-run`).

**Acceptance**

- one HlaX64 case is generated and verified without manual assembly editing;
- generator digest is present in evidence;
- dirty repository and forbidden path changes are rejected;
- no HlaX64-named modules are required in VAA core to run the path.

---

## Milestone 2 — Suite runner

**Deliverables**

- suite manifest;
- `vaa suite run`;
- suite evidence;
- child case digest aggregation.

**Acceptance**

- a deliberately broken compiler patch fails the suite;
- a valid patch passes all required cases;
- suite result is deterministic for the same locked inputs.

---

## Milestone 3 — Patch evidence

**Deliverables**

- patch digest;
- changed-path policy;
- base and patched revision identity;
- compiler binary digest;
- `vaa patch verify`;
- patch evidence verification.

**Acceptance**

- replacing a patch, compiler binary, task, or case evidence breaks verification;
- forbidden authority-file changes are rejected.

---

## Milestone 4 — Repair packet

**Deliverables**

- JSON and Markdown repair packets;
- stable diagnostic codes;
- compiler-versus-verifier triage;
- command reproduction block.

**Acceptance**

- a coding agent or human can repair one deliberately broken backend case without editing authority files;
- VAA correctly routes verifier-incomplete results away from generator repair.

---

## Milestone 5 — Source mapping

**Deliverables**

- `candidate.map.json`;
- SemASM offset joining;
- compiler source and IR references in repair packets.

**Acceptance**

- one behavioral or ABI failure resolves to the HlaX64 input, IR node, and backend source location.

---

## Milestone 6 — Verified backend slice v1

**Deliverables**

- scalar, loop, read, and write corpus;
- one complete target/ABI suite;
- CI gates;
- sealed patch evidence.

**Acceptance**

- one real HlaX64 backend defect is fixed through a coding agent or human;
- all candidates are regenerated;
- all required cases pass;
- patch evidence verifies from a clean checkout.

---

## 19. CI design

### 19.1 Pull request gates

```text
Gate 1: schema and configuration validation
Gate 2: Generator build (instance pack)
Gate 3: deterministic generation
Gate 4: SemASM per-case verification
Gate 5: VAA case bundle verification
Gate 6: full suite acceptance
Gate 7: patch evidence verification
```

### 19.2 Artifacts

Upload:

```text
compiler binary identity
generated assembly per case
source maps
SemASM reports
VAA evidence bundles
candidate chains
suite evidence
patch evidence
```

### 19.3 Cache policy

Cache may accelerate builds, but acceptance must never rely on unverified cache contents.

Every cached compiler binary and generated candidate must be re-hashed.

---

## 20. Security and isolation boundaries

For local developer use:

- agent edits a separate worktree;
- authority files are read-only where practical;
- generated output uses a clean directory;
- VAA and SemASM execution authority remains outside the agent;
- credentials are removed from subprocess environments;
- network is disabled where not required;
- candidate execution uses the existing explicit execution profile.

If generator repair becomes a shared or public service, operational isolation becomes a prerequisite before accepting untrusted remote patches.

---

## 21. Failure policy

| Failure | Final classification |
|---|---|
| compiler build fails | failed |
| candidate missing | failed |
| candidate non-deterministic | failed or incomplete by policy |
| SemASM violated | rejected |
| behavior mismatch | rejected |
| SemASM incomplete | incomplete |
| required evidence missing | incomplete |
| forbidden path changed | failed |
| suite regression | rejected |
| toolchain unavailable | failed |
| stack lock mismatch | failed |

`Incomplete` must never be silently converted into `Accepted`.

---

## 22. Documentation claims

### Allowed after Milestone 6

> External generator patches (first instance: HlaX64 backend) can be evaluated through deterministic regeneration, SemASM verification, VAA regression-suite acceptance, and sealed patch evidence.

> Humans, coding agents, or other editors may propose generator patches, but they do not determine acceptance.

> Accepted patches are bound to repository revision, generator identity, generated artifacts, verification reports, and suite results.

> VAA remains a universal acceptance controller; generator-specific material lives in instance packs.

### Not allowed

> HlaX64 is formally verified.

> Every HlaX64 program is memory safe.

> VAA proves the compiler is correct.

> VAA is an HlaX64-only product.

> SemASM proves all generated assembly is correct.

> Passing one verified backend suite proves all language features.

---

## 23. Recommended first executable slice

Use one deliberately broken backend case:

```text
min_i64 on one x86-64 ABI
```

### Scenario

1. HlaX64 emits an incorrect signed comparison or return path.
2. SemASM or the behavioral oracle rejects the candidate.
3. VAA exports a repair packet.
4. a coding agent or human changes HlaX64 backend source.
5. HlaX64 is rebuilt.
6. `min_i64` assembly is regenerated.
7. SemASM verifies the new candidate.
8. the full scalar suite runs;
9. VAA accepts and seals the patch evidence.

This slice proves the entire bridge without requiring pointer, alias, or memory-region complexity.

---

## 24. Final definition of done

The connecting layer is ready when all statements below are true:

- An **external generator** is a first-class concept in VAA (HlaX64 is the first pack).
- VAA binds evidence to exact generator repository and binary identity via stack lock + spec.
- Generated assembly cannot be accepted when edited manually outside regeneration.
- Coding agents and humans receive constrained repair packets.
- Authority files are protected by policy.
- One patch can be evaluated against multiple required cases.
- SemASM incomplete results are distinguished from generator defects.
- Suite and patch evidence are independently verifiable.
- CI can reproduce the workflow from a clean checkout for a parameterized `generator_id`.
- Adding a second generator does not require changing VAA core APIs — only a new instance pack + lock entry.
- The accepted claim remains limited to the tested backend slice and evidence profile.

---

## 25. Implementation priority

```text
P0  (generic VAA core + first HlaX64 pack config)
1. stack lock (multi-generator) — **done**
2. ExternalGeneratorSpec — **done**
3. repository guard — **done**
4. generator binary identity — **done**
5. deterministic generation — **done**
6. vaa generator-run

P1
7. suite manifest and runner
8. patch-level evidence
9. allowed/forbidden patch paths
10. generator-versus-verifier triage

P2
11. repair packet export
12. stable diagnostics
13. optional source mapping join
14. agent/editor rules and command templates

P3
15. backend feature corpus expansion (HlaX64 pack)
16. target/ABI parity
17. second generator pack smoke (proves universality)
18. stronger isolation for shared execution
```

The bridge should be built in this order. Do not begin with model integration. Establish deterministic generation and patch acceptance first; coding agents and humans can initially remain external repository editors. Prove universality early with schema/CLI design — optionally a tiny second pack stub in P3, not a second production generator.

---

## 26. Additional design notes (recommended)

1. **Reuse existing `vaa ingest` / `verify`** — `generator-run` should call the same evidence path already used for HlaX64 fixture ingest, not a parallel verify stack.
2. **`generator_id` in seals** — seal attribution already has a generator string; bind it to `ExternalGeneratorSpec.generator_id` + binary digest.
3. **Profile honesty** — suite policy may allow `verified_under_preconditions` only when the task profile says so; default in early HlaX64 scalar suite can stay strict (`allow_verified_under_preconditions = false`) until memory leaves are intentionally included.
4. **Worktree layout** — document sibling checkouts (`vaa/`, `semasm/`, `hlax64/`) vs monorepo; stack lock paths must be relative and portable.
5. **Schema versioning** — bump `schema_version` when renaming `compiler_*` → `generator_*`; support one deprecation window.
6. **Negative suite case** — keep one locked “broken generator” fixture to prove suite rejection without depending on live agent edits.
7. **SemASM pin cadence** — instance packs pin SemASM; VAA CI Gate pins remain the SemASM authority for Gate jobs — do not silently diverge.
8. **Non-compiler generators** — if a future pack is a pure translator (no “build binary”), allow `build.command` to be omitted when `generation.command` is self-contained, with identity still hashing the tool that ran.

---
