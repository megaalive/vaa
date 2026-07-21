# VAA — Reviewed and Hardened Architecture Plan

> **Working name:** Verifiable Assembly Agent (VAA)  
> **Document type:** Critical design review, revised architecture, and agent-executable implementation plan  
> **Document status:** Engineering baseline — intentionally not labeled production-ready  
> **Prepared for:** `megaalive/semasm` ecosystem  
> **Intended audience:** repository-capable coding agents and human maintainers  
> **Review date:** 2026-07-18  
> **Repository language:** English for source code, comments, diagnostics, documentation, issues, and pull requests  
> **Initial implementation language:** Rust  
> **Initial deployment form:** Local CLI, single-user, verify-only by default  

---

## 0. Executive Decision

The original VAA idea is valuable, but the version 1.0 blueprint is too optimistic and too infrastructure-heavy for the maturity of SemASM and for the first useful vertical slice.

The revised decision is:

> Build VAA as a small, separate, fail-closed orchestration tool that converts a constrained task specification into one or more assembly candidates, asks SemASM and the native toolchain for evidence, and returns an evidence bundle. VAA must never present incomplete analysis as verified code.

VAA is **not** initially:

- a production API service;
- a Python framework around SemASM;
- a distributed worker platform;
- a general autonomous software factory;
- a formal proof system;
- a guarantee of memory safety;
- a guarantee of optimal performance;
- a system that executes generated binaries directly on the host;
- a system that allows an LLM to weaken the requested contract;
- a multi-architecture product on day one.

The first version should prove one narrow statement:

> For one declared target and one artifact class, VAA can generate an assembly candidate, preserve an immutable contract, obtain honest verification coverage, build it in an isolated environment, optionally run trusted tests in a sandbox, and produce reproducible evidence.

---

## 1. Directive for implementing agents

The implementing agent must follow these rules throughout the work:

1. Inspect the current SemASM repository and its machine-readable behavior before writing VAA integration code.
2. Do not assume a SemASM capability exists because a crate, module, target name, or roadmap item exists.
3. Treat `verified`, `violated`, `incomplete`, and `failed` as distinct terminal states.
4. Never convert `unsupported`, `unknown`, tool failure, timeout, missing evidence, or parse failure into success.
5. Do not let the model edit:
   - the approved task specification;
   - security policy;
   - target identity;
   - authoritative tests;
   - resource budgets;
   - verification thresholds.
6. Keep the first implementation in one Rust binary crate with internal modules. Add crates only when a demonstrated boundary requires independent versioning or reuse.
7. Avoid Python, FastAPI, Redis, LiteLLM, Instructor, Docker SDKs, databases, async runtimes, plugin systems, and telemetry stacks in the first vertical slice.
8. Use external tools through narrow, typed process adapters.
9. Default to no network access during assembly, link, inspection, and execution stages.
10. Default to verify-only. Dynamic execution requires an explicit policy and an available sandbox backend.
11. Every pull request must include:
    - the invariant being protected;
    - tests that fail before the change where practical;
    - acceptance commands;
    - updated capability evidence;
    - residual limitations.
12. Keep generated source, logs, and artifacts out of Git unless they are intentional fixtures.
13. Use content-addressed run directories and never overwrite evidence from a previous run.
14. Do not make marketing claims in README that are stronger than current executable evidence.
15. Do not begin target expansion until the first target passes the full release gate.

---

## 2. Review of the Original Blueprint

### 2.1 What the original idea gets right

The original design correctly recognizes several important needs:

- the LLM must not be trusted as the verifier;
- generation needs a bounded correction loop;
- assembly and linking require isolation;
- semantic metadata is more valuable than syntax success alone;
- target selection must affect toolchain behavior;
- diagnostics should be fed back to the model;
- auditability and reproducibility matter;
- SemASM should remain the semantic authority rather than being duplicated in VAA.

These ideas should be preserved.

### 2.2 Main verdict

The original blueprint is a useful product sketch, but it is not yet a production-ready technical design.

Its main weaknesses are:

1. claims exceed the current evidence;
2. hosted executables and bare-metal artifacts are mixed together;
3. static verification, dynamic testing, and formal proof are conflated;
4. the proposed stack adds substantial bloat before the first vertical slice;
5. the trust boundary between user contract, model output, SemASM, and runtime evidence is not explicit;
6. the cache design is unsafe and not reproducible;
7. the repair loop can drift from the original intent;
8. Docker is described as absolute isolation;
9. the integration strategy couples VAA to unstable SemASM internals too early;
10. success is represented as a boolean rather than an evidence-based status.

---

## 3. Critical Corrections

### C-001 — Replace “production-ready” with an evidence maturity label

**Problem**

The original file labels itself a production-ready blueprint while SemASM is still in early development and its first hosted vertical slice is not yet a stable production backend.

**Correction**

Use these maturity levels:

| Level | Meaning |
|---|---|
| `concept` | Architecture only; no executable evidence |
| `experimental` | One local vertical slice works under controlled conditions |
| `alpha` | Repeatable CI evidence on one supported target |
| `beta` | Stable schemas, negative corpus, hardened sandbox, compatibility policy |
| `production-candidate` | Release gates pass and threat model is independently reviewed |

VAA begins as `experimental`.

---

### C-002 — Replace “zero runtime overhead”

**Problem**

Producing assembly without a high-level runtime does not mean zero overhead. The program may still contain:

- startup code;
- operating-system interfaces;
- runtime fragments;
- stack setup;
- error handling;
- allocator or I/O code;
- alignment and ABI costs.

**Correct value proposition**

> No mandatory high-level language runtime. Delivered artifacts contain only selected instructions, data, platform interfaces, and explicitly chosen runtime fragments.

Performance and size must be measured, never assumed.

---

### C-003 — Do not claim deterministic safety

**Problem**

Static metadata and partial instruction semantics cannot prove arbitrary assembly safe. Unsupported instructions, unknown memory aliases, indirect control flow, concurrency, self-modification, system calls, and external interfaces can make analysis incomplete.

**Correct value proposition**

> Deterministic evidence production with explicit coverage and uncertainty.

The deterministic part is the report and pipeline behavior, not universal program safety.

---

### C-004 — Do not call the initial system formal verification

**Problem**

SemASM semantic analysis is valuable, but it is not automatically a theorem prover or proof-carrying code system.

**Correction**

Use:

- semantic verification;
- contract checking;
- static analysis;
- artifact inspection;
- behavioral conformance testing.

Use “formal proof” only for a specific property backed by a named formal method and a verifiable proof artifact.

---

### C-005 — Separate hosted, callable, and bare-metal artifacts

“Assembly” is not one deployment model.

VAA must distinguish:

#### `callable-function`

A function conforming to a declared ABI.

- no process entry point;
- normally no system calls;
- invoked through a trusted test harness;
- best first artifact class.

#### `hosted-program`

An executable running under an operating system.

- explicit entry point;
- explicit syscall or imported API capability set;
- sandbox execution may validate exit code and output.

#### `freestanding-image`

Firmware or bare-metal image.

- machine model;
- memory map;
- reset vector;
- interrupt model;
- device assumptions;
- emulator or board profile.

This class is deferred until hosted verification is stable.

---

### C-006 — Do not let the model invent authoritative contract assertions

**Problem**

The original output asks the model to return assertions such as `no_heap` or `execution_time < 1000_cycles`. A model can claim anything.

**Correction**

Authoritative constraints come from:

1. a user-approved task specification;
2. repository policy;
3. a target kit;
4. trusted test vectors;
5. administrator policy.

The model may return:

- implementation assumptions;
- a short design summary;
- suggested additional tests;
- known uncertainty.

These fields are untrusted hints, not proof.

---

### C-007 — Remove requested chain-of-thought

Do not request or store a `thinking` field.

Use:

```json
{
  "design_summary": "Uses a counted loop and preserves callee-saved registers.",
  "assumptions": [
    "The input pointer is valid for length * 8 bytes."
  ]
}
```

VAA needs concise, auditable decisions—not hidden reasoning traces.

---

### C-008 — Do not parse supported ISA from source files

**Problem**

Generating `supported_isa.txt` by scraping a SemASM crate is brittle and may not describe:

- lowering coverage;
- analysis coverage;
- operand forms;
- ABI support;
- object-format support;
- execution support;
- maturity.

**Correction**

SemASM should expose a versioned capability document:

```text
semasm capabilities --target <target-id> --format json
```

VAA must use that document and record its digest in every run.

---

### C-009 — Use a stable process protocol before PyO3

**Problem**

PyO3 binds VAA directly to SemASM internals and Rust/Python packaging before the SemASM result schema is stable.

**Correction**

Initial boundary:

```text
VAA process
    -> semasm CLI with JSON input/output
    -> versioned report schema
```

Advantages:

- process isolation;
- language independence;
- easy command transcript reproduction;
- no Python ABI packaging;
- VAA can pin a SemASM binary version;
- failures are explicit process outcomes.

A direct Rust library adapter can be considered after the CLI schema has a compatibility policy.

---

### C-010 — Do not fork SemASM for VAA

VAA should be a separate repository.

Recommended relationship:

```text
megaalive/semasm   = semantic and verification engine
megaalive/vaa      = model orchestration and evidence workflow
```

VAA may pin:

- a SemASM release;
- a commit during early development;
- an expected schema range.

Forking should be reserved for emergency patches, not normal integration.

---

### C-011 — Replace prompt-only caching

The original cache key is only a hash of the user prompt. This is incorrect.

The same prompt can produce different valid artifacts depending on:

- target identity;
- artifact kind;
- contract;
- tests;
- policy;
- model;
- system prompt;
- temperature;
- SemASM version;
- toolchain versions;
- sandbox image;
- runtime fragments;
- source candidate.

Use separate caches.

#### Generation request key

```text
hash(
  normalized_task_spec
  + target_capability_digest
  + model_id
  + model_parameters
  + prompt_template_version
)
```

#### Verification key

```text
hash(
  source_bytes
  + contract_digest
  + target_kit_digest
  + semasm_version
  + verifier_options
)
```

#### Build key

```text
hash(
  source_bytes
  + build_manifest
  + assembler_version
  + linker_version
  + sandbox_image_digest
)
```

A cached artifact must never bypass policy or compatibility checks.

---

### C-012 — Docker is not “absolute isolation”

Containers share a kernel and can contain configuration errors or vulnerable runtimes.

The security wording must be:

> A rootless, network-disabled container is the initial isolation backend on supported hosts. It reduces risk but is not an absolute security boundary.

Higher-risk deployments may use:

- a disposable VM;
- Hyper-V isolated containers;
- Firecracker;
- gVisor;
- a dedicated remote build worker;
- QEMU system emulation.

The backend is policy-selected, not hardcoded as “Docker”.

---

### C-013 — Separate build sandbox and execution sandbox

Assembly and link tools process attacker-controlled input and therefore need isolation.

The generated binary also needs a separate execution policy.

```text
build sandbox:
  source -> object -> artifact inspection -> binary

execution sandbox:
  binary + trusted test harness -> runtime evidence
```

A build sandbox must not automatically grant permission to execute the result.

---

### C-014 — “No direct execution” and `require_exit_zero` conflict

Exit code is dynamic evidence. It cannot be required without running the program somewhere.

Revised modes:

| Mode | Behavior |
|---|---|
| `verify-only` | No candidate execution; static and artifact evidence only |
| `sandbox-test` | Candidate runs only in an approved isolated backend |
| `deliver` | Returns accepted artifact and evidence; does not launch it on host |

`verify-only` is the default.

---

### C-015 — Replace blanket syscall prohibition with capabilities

A hosted Linux program often requires at least an exit mechanism. Windows programs may import APIs.

Instead of:

```toml
disallow_syscalls = true
```

use:

```toml
[capabilities]
mode = "allowlist"
linux_syscalls = ["exit", "exit_group"]
network = false
filesystem = "none"
environment = "none"
clock = false
random = false
```

For a callable function, the allowlist can remain empty.

---

### C-016 — A boolean verification result is insufficient

Replace:

```python
is_ok: bool
```

with:

```rust
enum EvidenceStatus {
    Verified,
    Violated,
    Incomplete,
    Failed,
}
```

Meanings:

- `verified`: all required checks completed and passed;
- `violated`: evidence demonstrates a contract or policy violation;
- `incomplete`: one or more required facts could not be established;
- `failed`: the pipeline could not reliably produce a conclusion.

VAA may deliver only statuses allowed by the active policy.

---

### C-017 — Retry count alone is not a repair strategy

Use bounded budgets:

```toml
[budgets]
max_candidates = 4
max_repairs_per_candidate = 2
max_model_tokens = 24000
max_model_cost_usd = 2.00
max_wall_time_seconds = 300
max_no_progress_iterations = 1
```

Stop when:

- a source hash repeats;
- the same diagnostic set repeats;
- no stage progress occurs;
- the cost budget is exhausted;
- a hard policy violation is detected;
- required SemASM coverage is unavailable.

---

### C-018 — The model must not repair the contract

The contract is immutable after approval.

The repair loop may change:

- assembly source;
- labels;
- instruction selection;
- register allocation;
- stack layout;
- model-proposed non-authoritative tests.

The repair loop may not change:

- expected behavior;
- input/output definitions;
- target;
- ABI;
- allowed capabilities;
- authoritative tests;
- memory limits;
- delivery policy.

Every iteration stores the same `contract_digest`.

---

### C-019 — Model-generated tests are supplemental

The model may suggest useful tests, but those tests cannot be the only proof that its own implementation is correct.

Test authority order:

1. user or repository fixtures;
2. VAA trusted harness templates;
3. target conformance suite;
4. independently generated property cases;
5. model-suggested tests.

---

### C-020 — Audit logging must respect privacy

Logging every source forever is not automatically good security.

Required policy:

- local mode stores artifacts in the run directory;
- service mode has a retention policy;
- secrets are redacted from environment and prompts;
- content hashes may be stored without source where policy requires;
- sensitive source may be encrypted at rest;
- logs have size limits;
- telemetry never includes source by default.

---

## 4. Revised Product Definition

### 4.1 Product statement

VAA is a model-assisted assembly production controller.

It accepts a versioned, constrained task specification and coordinates:

1. target capability resolution;
2. candidate generation;
3. syntax and structure checks;
4. SemASM contract and semantic checks;
5. assembly and linking;
6. object inspection;
7. optional sandboxed behavioral tests;
8. reproducibility checks;
9. evidence packaging.

VAA does not decide that code is safe merely because it assembled.

### 4.2 Primary user value

- direct assembly implementation without a mandatory high-level runtime;
- explicit contract preservation;
- visible unsupported semantics;
- reproducible build and verification evidence;
- bounded automated repair;
- target-specific artifact production;
- small delivered artifact surface.

### 4.3 Non-goals for the first releases

- arbitrary application generation;
- GUI application generation;
- kernel modules;
- drivers;
- network services;
- self-modifying code;
- JIT code;
- multithreaded assembly;
- privileged instructions;
- UEFI;
- firmware flashing;
- production remote execution;
- model marketplace;
- automatic performance superiority claims.

---

## 5. First Supported Vertical Slice

### 5.1 Recommended first target

```text
x86_64-unknown-linux-gnu
```

Exact target identity must include:

- ISA and extensions;
- little-endian;
- 64-bit word size;
- System V AMD64 ABI;
- ELF object format;
- NASM syntax;
- assembler version;
- linker;
- execution profile;
- sandbox backend.

### 5.2 Recommended first artifact kind

```text
callable-function
```

This is safer and more testable than a standalone arbitrary program.

Example task:

> Implement `sum_i64(const int64_t* values, size_t length) -> int64_t`, preserve all required callee-saved registers, use no syscalls, perform no allocation, and read only the declared input range.

### 5.3 Why not quicksort first

Quicksort adds:

- mutation policy;
- bounds reasoning;
- recursion or explicit stack;
- worst-case stack growth;
- pivot behavior;
- duplicate handling;
- larger test space.

Start with:

1. `add_i64`;
2. `sum_i64`;
3. `strlen_bounded`;
4. `memcpy_no_overlap`;
5. `max_i64`;
6. only then a sorting routine.

### 5.4 Trusted test harness

VAA should generate the harness from repository-owned templates, not from the LLM.

The candidate object is linked with a trusted harness that:

- supplies test inputs;
- protects guard pages where practical;
- checks outputs;
- checks preserved registers;
- records process outcome;
- limits runtime and output.

---

## 6. System Boundaries

```text
+---------------------------+
| User / Repository Policy  |
| task spec + trusted tests |
+-------------+-------------+
              |
              v
+---------------------------+
| VAA Controller            |
| state machine + budgets   |
| immutable contract        |
+--+-----------+----------+-+
   |           |          |
   v           v          v
Model       SemASM      Sandbox backend
adapter     adapter     build / optional run
   |           |          |
   +-----------+----------+
               |
               v
+---------------------------+
| Evidence Store            |
| sources, reports, hashes, |
| artifacts, provenance     |
+---------------------------+
```

Trust rules:

| Component | Trust level |
|---|---|
| User-approved task spec | Authoritative |
| Repository policy | Authoritative |
| Trusted test fixtures | Authoritative |
| Target kit | Authoritative within declared version |
| LLM output | Untrusted |
| Assembly source | Untrusted |
| Assembler/linker output | Evidence, not automatically trusted |
| SemASM result | Evidence with explicit coverage and version |
| Sandbox result | Empirical evidence, not proof of all behavior |
| VAA controller | Trusted computing base |
| Container runtime / VM | Security boundary with documented assumptions |

---

## 7. Revised Architecture

### 7.1 Initial architecture: one binary, internal modules

Do not start with many crates.

```text
vaa/
├── Cargo.toml
├── rust-toolchain.toml
├── README.md
├── SECURITY.md
├── LICENSE-APACHE
├── LICENSE-MIT
├── docs/
│   ├── architecture.md
│   ├── threat-model.md
│   ├── evidence-model.md
│   └── compatibility.md
├── schemas/
│   ├── task-spec-v0.1.schema.json
│   ├── candidate-v0.1.schema.json
│   └── run-report-v0.1.schema.json
├── fixtures/
│   ├── tasks/
│   ├── model-responses/
│   ├── diagnostics/
│   └── expected-reports/
├── templates/
│   └── harness/
└── src/
    ├── main.rs
    ├── cli.rs
    ├── config.rs
    ├── task.rs
    ├── policy.rs
    ├── state.rs
    ├── orchestration.rs
    ├── candidate.rs
    ├── evidence.rs
    ├── cache.rs
    ├── process.rs
    ├── adapters/
    │   ├── mod.rs
    │   ├── model.rs
    │   ├── semasm.rs
    │   └── sandbox.rs
    └── commands/
        ├── mod.rs
        ├── doctor.rs
        ├── plan.rs
        ├── generate.rs
        ├── verify.rs
        ├── build.rs
        └── inspect.rs
```

### 7.2 When to split crates

Only split a crate when at least one condition is true:

- an API must be versioned independently;
- another program already needs the code;
- a security boundary needs a separate process;
- compile features require genuine isolation;
- the module has stable behavior and tests;
- the split reduces, rather than increases, dependency coupling.

### 7.3 Logical components

#### Task compiler

- parses the task specification;
- validates schema;
- resolves defaults;
- locks the contract;
- computes digests;
- identifies ambiguity before model use.

#### Orchestrator

- owns the state machine;
- applies budgets;
- records events;
- prevents policy mutation;
- chooses the next allowed action.

#### Model adapter

- sends only the minimum context;
- requests structured candidate submission;
- normalizes provider errors;
- cannot execute shell commands;
- has no direct access to the artifact store.

#### SemASM adapter

- invokes a pinned SemASM binary;
- checks process outcome;
- validates report schema and version;
- rejects missing coverage;
- records raw and normalized reports.

#### Sandbox adapter

- prepares isolated build and optional run environments;
- uses explicit mounts and environment;
- enforces resource limits;
- captures bounded output;
- kills complete process trees;
- records backend identity and image digest.

#### Evidence store

- creates immutable run directories;
- content-addresses large blobs;
- stores provenance;
- never silently replaces an artifact.

---

## 8. SemASM Integration Contract

### 8.1 Prerequisites in SemASM

VAA integration should depend on these SemASM capabilities:

1. machine-readable capability manifest;
2. versioned JSON verification report;
3. explicit `verified`, `violated`, `incomplete`, `failed` result;
4. instruction coverage counts;
5. unsupported instruction details;
6. target identity in every report;
7. reliable non-zero process outcomes;
8. bounded process execution;
9. deterministic evidence mode;
10. stable schema compatibility rules.

If any required item is unavailable, VAA must report `dependency_incomplete`.

### 8.2 Expected commands

Illustrative interface:

```bash
semasm --version --format json

semasm capabilities \
  --target x86_64-unknown-linux-gnu \
  --format json

semasm contract check \
  task.sem.toml \
  --format json

semasm verify \
  --target x86_64-unknown-linux-gnu \
  --contract task.sem.toml \
  --source candidate.asm \
  --format json \
  --evidence-dir evidence/semasm
```

VAA must not scrape human terminal output.

### 8.3 Schema negotiation

At startup:

1. read SemASM version;
2. read report schema version;
3. compare with the accepted range;
4. reject an incompatible major version;
5. warn or reject unknown minor fields according to policy;
6. record the exact version and binary digest.

### 8.4 Capability gate

A target can be used only when all required stages declare sufficient maturity:

```json
{
  "target_id": "x86_64-unknown-linux-gnu",
  "capabilities": {
    "decode": "supported",
    "lower": "partial",
    "abi_check": "supported",
    "object_inspect": "supported",
    "link": "supported",
    "sandbox_run": "supported"
  }
}
```

A policy may require:

```toml
[verification.requirements]
decode = "supported"
lower = "supported"
abi_check = "supported"
object_inspect = "supported"
```

`partial` must not be silently promoted.

---

## 9. Task Specification

### 9.1 Natural language is not the final contract

The user may begin with natural language, but VAA should compile it into a structured draft.

Commands:

```bash
vaa plan "Create a bounded strlen routine for x86-64 Linux"
vaa plan --from request.txt --out task.vaa.toml
vaa build task.vaa.toml
```

The `plan` stage does not generate a binary. It creates a reviewable task file.

### 9.2 Example `task.vaa.toml`

```toml
schema_version = "0.1"
task_id = "sum-i64-v1"
artifact_kind = "callable-function"
target = "x86_64-unknown-linux-gnu"

[entry]
symbol = "sum_i64"
abi = "sysv64"

[inputs.values]
kind = "pointer"
element = "i64"
access = "read"
length_from = "length"
nullable = false

[inputs.length]
kind = "usize"

[output]
kind = "i64"

[behavior]
summary = "Return the wrapping sum of all input elements."
integer_overflow = "wrap"
empty_input_result = 0

[capabilities]
syscalls = []
imports = []
heap = false
filesystem = false
network = false
environment = false
clock = false
random = false

[memory]
max_stack_bytes = 128
allow_global_writable = false
allow_self_modifying_code = false

[instructions]
required_features = ["x86-64-baseline"]
forbidden_mnemonics = []
allow_unknown_semantics = false

[verification]
require_complete_lowering = true
require_abi_check = true
require_object_inspection = true
require_behavioral_tests = true
require_reproducible_build = true

[budgets]
max_candidates = 4
max_repairs_per_candidate = 2
max_wall_time_seconds = 300
max_model_tokens = 24000
max_no_progress_iterations = 1

[delivery]
include_source = true
include_object = true
include_binary = false
include_evidence = true
```

### 9.3 Authoritative tests

```toml
[[tests]]
name = "empty"
input.values = []
input.length = 0
expected = 0

[[tests]]
name = "positive"
input.values = [1, 2, 3, 4]
input.length = 4
expected = 10

[[tests]]
name = "mixed"
input.values = [-5, 2, 10]
input.length = 3
expected = 7
```

Tests must be part of the locked task digest.

---

## 10. Candidate Submission Protocol

### 10.1 Model tool

Prefer a structured model tool call named `submit_candidate`.

```json
{
  "schema_version": "0.1",
  "candidate_id": "candidate-0001",
  "target_id": "x86_64-unknown-linux-gnu",
  "dialect": "nasm",
  "source": "bits 64\nsection .text\n...",
  "entry_symbols": ["sum_i64"],
  "design_summary": "Uses RAX as accumulator and RCX as index.",
  "assumptions": [
    "The pointer and length satisfy the locked preconditions."
  ],
  "suggested_tests": [
    "Test a length of one."
  ]
}
```

### 10.2 Validation

Before writing the source:

- validate schema;
- reject unknown required-enum values;
- enforce source-size limits;
- reject NUL bytes;
- normalize line endings;
- compute source hash;
- verify target and dialect match the locked task;
- reject repeated candidate hash;
- store raw model response separately from normalized candidate.

### 10.3 Model context

The model receives:

- locked task specification;
- relevant target capability subset;
- allowed assembly dialect summary;
- exact ABI obligations;
- selected trusted examples;
- diagnostics from the immediately previous candidate;
- remaining budgets.

The model does not receive:

- unrelated host environment;
- API keys;
- arbitrary repository files;
- unrestricted shell;
- mutable policy files;
- previous hidden reasoning;
- secrets from logs.

---

## 11. State Machine

### 11.1 States

```text
RECEIVED
  -> TASK_VALIDATED
  -> TARGET_RESOLVED
  -> CONTRACT_LOCKED
  -> CANDIDATE_REQUESTED
  -> CANDIDATE_RECEIVED
  -> SOURCE_VALIDATED
  -> STATIC_VERIFYING
  -> STATIC_VERIFIED | STATIC_VIOLATED | STATIC_INCOMPLETE | STATIC_FAILED
  -> BUILDING
  -> BUILD_SUCCEEDED | BUILD_FAILED
  -> ARTIFACT_INSPECTING
  -> ARTIFACT_ACCEPTED | ARTIFACT_REJECTED | ARTIFACT_INCOMPLETE
  -> SANDBOX_TESTING (optional)
  -> TESTS_PASSED | TESTS_FAILED | TESTS_INCOMPLETE
  -> REPRODUCIBILITY_CHECKING (policy dependent)
  -> ACCEPTED | REJECTED | INCOMPLETE | FAILED | BUDGET_EXHAUSTED
```

### 11.2 No success shortcut

The accepted path requires every policy-required gate.

Example:

```text
source assembled successfully
```

does not imply:

```text
contract verified
```

and:

```text
tests passed
```

does not imply:

```text
all memory behavior is safe
```

### 11.3 Repair transitions

Repair may occur after:

- source parse failure;
- unsupported instruction;
- ABI violation;
- forbidden capability;
- assembler diagnostic;
- linker diagnostic;
- object inspection mismatch;
- trusted test failure.

No repair occurs after:

- incompatible SemASM schema;
- unavailable required sandbox;
- evidence store corruption;
- controller invariant violation;
- budget exhaustion;
- repeated identical candidate;
- administrator hard block.

### 11.4 Progress metric

A repair iteration is progress only if one or more are true:

- the pipeline reaches a later stage;
- diagnostic severity decreases;
- the number of unique violations decreases;
- semantic coverage increases;
- unsupported instruction count decreases;
- authoritative test pass count increases;
- source hash changes and diagnostics materially change.

Merely generating different text is not progress.

---

## 12. Verification Policy

### 12.1 Evidence layers

#### Layer A — Task validity

- schema valid;
- all referenced inputs exist;
- target recognized;
- budgets within administrator limits;
- contract digest locked.

#### Layer B — Source validity

- structured candidate valid;
- dialect matches;
- size within limit;
- no forbidden directives;
- no unexpected extra files.

#### Layer C — Semantic verification

- decoding coverage;
- lowering coverage;
- ABI conformance;
- register preservation;
- stack behavior;
- control-flow properties;
- memory effects where supported;
- capability use.

#### Layer D — Build evidence

- assembler success;
- linker success;
- expected format;
- expected architecture;
- symbol and section policy;
- relocation policy;
- import/syscall policy;
- executable-stack policy.

#### Layer E — Behavioral evidence

- trusted tests;
- timeout;
- output limits;
- crash and signal status;
- guard checks;
- deterministic output where required.

#### Layer F — Reproducibility evidence

- same inputs produce same canonical artifact;
- volatile metadata is separated;
- tool versions and image digests recorded.

### 12.2 Final decision

```rust
struct FinalDecision {
    status: EvidenceStatus,
    reason_codes: Vec<ReasonCode>,
    required_checks: Vec<CheckOutcome>,
    optional_checks: Vec<CheckOutcome>,
    coverage: CoverageSummary,
    artifact_digest: Option<Digest>,
    evidence_digest: Digest,
}
```

### 12.3 Forbidden success language

Do not output:

- “safe”;
- “fully secure”;
- “formally verified”;
- “zero overhead”;
- “guaranteed optimal”;
- “clean” when coverage is incomplete.

Prefer:

> Accepted under policy `local-function-v0.1`; all required checks completed. Instruction semantic coverage: 100% for this artifact. Behavioral tests: 12/12 passed. This does not prove safety outside the declared contract and environment.

---

## 13. Repair Loop Design

### 13.1 Diagnostic packet

The repair model receives structured diagnostics:

```json
{
  "candidate_id": "candidate-0002",
  "stage": "semantic_verify",
  "status": "violated",
  "diagnostics": [
    {
      "code": "ABI_CALLEE_SAVED_CLOBBER",
      "severity": "error",
      "location": {
        "line": 14,
        "column": 5
      },
      "message": "RBX is modified but not restored.",
      "rule": "sysv64.callee_saved"
    }
  ],
  "coverage": {
    "decoded": 11,
    "lowered": 11,
    "unsupported": 0
  },
  "remaining_budget": {
    "candidates": 2,
    "model_tokens": 12000
  }
}
```

### 13.2 Minimal feedback

Do not dump unlimited tool output into the model context.

Include:

- normalized diagnostic;
- relevant source excerpt;
- target rule;
- previous attempted fix summary;
- remaining budget.

Store full raw logs in evidence, not necessarily in the model prompt.

### 13.3 Security violation handling

Classify violations.

#### Repairable policy mismatch

Example:

- accidental `syscall` in a callable function;
- writable executable section;
- missing register restore.

Action:

- discard the current artifact;
- request a new candidate or repair;
- do not weaken policy.

#### Hard security block

Example:

- model attempts to write outside workspace;
- model output includes tool-command injection;
- sandbox escape indicator;
- generated source intentionally invokes forbidden privileged behavior after explicit correction.

Action:

- terminate run;
- mark `rejected`;
- preserve evidence according to retention policy;
- do not retry automatically.

---

## 14. Build Pipeline

### 14.1 Build manifest

Every build uses an explicit manifest.

```json
{
  "schema_version": "0.1",
  "target_id": "x86_64-unknown-linux-gnu",
  "source_digest": "sha256:...",
  "assembler": {
    "name": "nasm",
    "version": "2.x",
    "arguments": ["-f", "elf64"]
  },
  "linker": {
    "name": "ld.lld",
    "version": "...",
    "arguments": []
  },
  "environment": {
    "allowlist": ["PATH", "LANG", "LC_ALL"],
    "values": {
      "LANG": "C",
      "LC_ALL": "C"
    }
  },
  "sandbox": {
    "backend": "rootless-container",
    "image_digest": "sha256:...",
    "network": false
  }
}
```

### 14.2 Process execution requirements

- argv array, never shell string concatenation;
- stdin closed unless explicit bytes are provided;
- environment cleared, then allowlisted;
- fixed working directory under the run;
- bounded stdout and stderr;
- wall-clock timeout;
- CPU and memory limits;
- file-size and process-count limits;
- complete process-tree termination;
- no inherited handles;
- explicit success exit codes;
- raw command outcome recorded.

### 14.3 Filesystem policy

Build sandbox:

- read-only toolchain image;
- read-only input mount;
- writable ephemeral output directory;
- no host home directory;
- no Docker socket;
- no SSH agent;
- no cloud credentials;
- no package manager network;
- no arbitrary device access.

### 14.4 Artifact inspection before execution

Require checks for:

- architecture;
- object format;
- sections;
- entry symbols;
- imports;
- dynamic dependencies;
- executable stack;
- writable+executable segments;
- relocations;
- unexpected symbols;
- debug sections according to delivery policy;
- artifact size limits.

---

## 15. Execution Sandbox

### 15.1 Default

Dynamic execution is disabled unless:

- the task requires behavioral evidence;
- a sandbox backend passes `vaa doctor`;
- policy permits the artifact class;
- the target execution profile is supported.

### 15.2 Minimum Linux container policy

Illustrative requirements:

- rootless runtime;
- user namespace;
- non-root user;
- no network namespace connectivity;
- read-only root filesystem;
- no new privileges;
- capability set dropped;
- seccomp policy;
- PID limit;
- memory limit;
- CPU quota;
- file-size limit;
- wall-clock timeout;
- output limit;
- temporary writable directory;
- process-tree cleanup.

### 15.3 Higher assurance mode

A disposable VM is recommended when:

- testing hosted programs with broader syscalls;
- testing malformed binaries;
- running fuzzed artifacts;
- processing untrusted external submissions;
- exposing VAA as a multi-user service.

### 15.4 Cross-architecture execution

QEMU user-mode is acceptable for some hosted tests but must be reported as an emulator profile, not native execution.

Evidence must state:

```json
{
  "execution_kind": "emulated",
  "emulator": "qemu-aarch64",
  "emulator_version": "...",
  "machine_model": null
}
```

Bare-metal uses system emulation and an explicit machine model.

---

## 16. Evidence and Provenance

### 16.1 Run directory

```text
.vaa/runs/<timestamp>-<run-id>/
├── task/
│   ├── original-request.txt
│   ├── task.vaa.toml
│   └── task.digest
├── target/
│   ├── capabilities.json
│   └── capabilities.digest
├── candidates/
│   ├── 0001/
│   │   ├── raw-model-response.json
│   │   ├── candidate.json
│   │   ├── candidate.asm
│   │   ├── semasm-report.json
│   │   ├── build-result.json
│   │   └── diagnostics.json
│   └── 0002/
├── accepted/
│   ├── source.asm
│   ├── object.o
│   ├── artifact
│   └── artifact.sha256
├── evidence/
│   ├── run-report.json
│   ├── provenance.json
│   ├── checks.json
│   └── evidence.sha256
└── events.jsonl
```

### 16.2 Provenance fields

Record:

- VAA version and binary digest;
- SemASM version and binary digest;
- model provider and model ID;
- model parameters that affect output;
- prompt template version;
- task digest;
- policy digest;
- capability digest;
- candidate source digest;
- toolchain versions;
- sandbox backend and image digest;
- host OS class without unnecessary personal data;
- timestamps as non-canonical metadata;
- artifact digest;
- evidence digest.

### 16.3 Canonical versus volatile evidence

Canonical evidence excludes:

- wall-clock timestamp;
- local absolute path;
- process ID;
- random run ID;
- host username.

This allows reproducibility comparison.

Volatile evidence remains available for audit but does not affect canonical digests.

---

## 17. Cache Design

### 17.1 Initial implementation

Use a local filesystem content-addressed store.

Do not add Redis.

```text
.vaa/cache/
├── blobs/sha256/<digest>
├── verification/<key>.json
├── builds/<key>.json
└── index/
```

SQLite may be added later only for metadata queries, not as a requirement for the core pipeline.

### 17.2 Cache acceptance checks

Before reuse:

- schema compatible;
- all referenced blobs exist;
- digests match;
- policy still permits reuse;
- toolchain identity matches the key;
- no previous terminal status was `failed`;
- incomplete evidence is not reused as verified;
- artifact provenance is intact.

### 17.3 No automatic prompt-to-binary response

A previous prompt match may suggest a candidate, but VAA must still bind it to the current locked task, target, policy, and verification requirements.

---

## 18. Provider Strategy

### 18.1 Initial adapter

Implement one provider family first:

```text
OpenAI-compatible Responses/Chat endpoint
```

The adapter should be isolated behind a trait.

```rust
trait ModelProvider {
    fn submit_candidate(
        &self,
        request: CandidateRequest,
    ) -> Result<CandidateEnvelope, ModelError>;
}
```

The exact HTTP client should be chosen after measuring dependency cost.

### 18.2 Provider-neutral core

Core code must not contain:

- OpenAI-specific status enums;
- Anthropic-specific content blocks;
- provider SDK types;
- LiteLLM response objects;
- provider names in domain models.

### 18.3 Model profile

A model profile defines:

```toml
[model]
provider = "openai-compatible"
model = "configured-by-user"
temperature = 0.1
max_output_tokens = 8000
structured_output = true
```

API keys come only from explicit secret configuration and are never written to run evidence.

### 18.4 Fallback model policy

Do not automatically switch models unless configured.

A different model changes provenance and possibly behavior. Record a new generation key and event.

---

## 19. CLI Design

### 19.1 Core commands

```bash
vaa doctor
vaa capabilities --target <target>
vaa plan <request>
vaa validate <task.vaa.toml>
vaa generate <task.vaa.toml>
vaa verify <task.vaa.toml> --source candidate.asm
vaa build <task.vaa.toml>
vaa inspect <run-id>
vaa report <run-id> --format json
vaa clean --runs --older-than <policy>
```

### 19.2 `vaa doctor`

Checks:

- VAA version;
- configuration validity;
- model endpoint connectivity, only when requested;
- SemASM binary and schema;
- assembler;
- linker;
- object inspector;
- sandbox backend;
- QEMU profile where configured;
- writable evidence directory;
- security-sensitive configuration;
- target capability compatibility.

Doctor output must distinguish:

- `available`;
- `unavailable`;
- `incompatible`;
- `degraded`;
- `not configured`.

### 19.3 Exit codes

Suggested:

| Code | Meaning |
|---:|---|
| 0 | Accepted / command succeeded |
| 2 | Invalid user input or task schema |
| 3 | Contract or policy violated |
| 4 | Evidence incomplete |
| 5 | Tool or pipeline failure |
| 6 | Dependency incompatible |
| 7 | Budget exhausted |
| 8 | Security block |
| 9 | Internal invariant failure |

---

## 20. Optional Service Mode — Deferred

FastAPI should not be the starting point.

When a service is justified, define it as a separate adapter or process.

Required before service mode:

- stable local CLI;
- stable run report schema;
- authentication;
- authorization;
- tenant isolation;
- cost quotas;
- request-size limits;
- rate limits;
- encrypted secrets;
- remote worker trust model;
- artifact retention policy;
- sandbox capacity controls;
- abuse prevention;
- audit log access policy.

A public API that generates machine code is a security product, not merely a web wrapper.

---

## 21. Error Taxonomy

### 21.1 Categories

```text
TASK_*
TARGET_*
MODEL_*
CANDIDATE_*
SEMASM_*
ANALYSIS_*
POLICY_*
BUILD_*
ARTIFACT_*
SANDBOX_*
TEST_*
REPRO_*
CACHE_*
EVIDENCE_*
BUDGET_*
SECURITY_*
INTERNAL_*
```

### 21.2 Examples

| Code | Terminal? | Repairable? | Meaning |
|---|---|---|---|
| `TASK_SCHEMA_INVALID` | yes | no | Task file is invalid |
| `TARGET_CAPABILITY_INSUFFICIENT` | yes | no | Required target evidence unavailable |
| `MODEL_OUTPUT_INVALID` | no | yes | Structured candidate invalid |
| `CANDIDATE_REPEATED` | maybe | no | Candidate hash already attempted |
| `SEMASM_SCHEMA_INCOMPATIBLE` | yes | no | Report protocol cannot be trusted |
| `ANALYSIS_UNSUPPORTED_INSTRUCTION` | maybe | yes | Candidate uses unsupported semantics |
| `POLICY_FORBIDDEN_SYSCALL` | maybe | yes | Candidate violates capability policy |
| `BUILD_ASSEMBLER_FAILED` | no | yes | Assembly failed |
| `ARTIFACT_EXECUTABLE_STACK` | no | yes | Artifact violates section policy |
| `SANDBOX_UNAVAILABLE` | yes when required | no | Required dynamic evidence unavailable |
| `TEST_BEHAVIOR_MISMATCH` | no | yes | Trusted test failed |
| `REPRO_ARTIFACT_MISMATCH` | yes | maybe | Rebuild differs |
| `BUDGET_NO_PROGRESS` | yes | no | Repair loop stopped |
| `SECURITY_WORKSPACE_ESCAPE` | yes | no | Hard security block |
| `INTERNAL_CONTRACT_MUTATED` | yes | no | Controller invariant broken |

---

## 22. Threat Model

### 22.1 Assets

- host system;
- API credentials;
- source repositories;
- user task data;
- generated artifacts;
- evidence integrity;
- build worker;
- model cost budget;
- SemASM and toolchain binaries.

### 22.2 Adversaries and faults

- malicious user prompt;
- prompt injection embedded in source or diagnostics;
- malicious model output;
- accidental model hallucination;
- malicious assembly directives;
- assembler or linker vulnerability;
- sandbox escape;
- artifact intended to exfiltrate data;
- dependency compromise;
- cache poisoning;
- evidence tampering;
- path traversal;
- decompression or output bomb;
- denial of service;
- model cost exhaustion;
- unsupported instruction hidden by analysis;
- target mismatch;
- stale capability data.

### 22.3 Key mitigations

- immutable task digest;
- no arbitrary model tools;
- process argv, not shell;
- environment clearing;
- no network in build/run;
- rootless sandbox;
- bounded output and files;
- process-tree termination;
- content-addressed evidence;
- schema validation;
- version pinning;
- target identity checks;
- complete/incomplete distinction;
- negative corpus;
- supply-chain checks;
- trusted harnesses;
- explicit budgets.

### 22.4 Residual risk

Even after all controls:

- a container or VM runtime may have vulnerabilities;
- static analysis may be incomplete;
- dynamic tests cannot cover every input;
- a valid artifact can still contain logic errors outside the contract;
- target hardware may differ from the modeled environment;
- microarchitectural behavior and side channels may remain unverified.

Reports must state these limitations.

---

## 23. Testing Strategy

### 23.1 Unit tests

- task schema parsing;
- contract immutability;
- digest calculation;
- status aggregation;
- error normalization;
- budget accounting;
- candidate deduplication;
- cache key stability;
- path safety;
- report canonicalization.

### 23.2 Adapter contract tests

- model structured response;
- invalid JSON;
- timeout;
- rate limit;
- SemASM non-zero exit;
- malformed SemASM JSON;
- incompatible schema;
- partial coverage;
- assembler failure;
- linker failure;
- sandbox timeout;
- output truncation.

### 23.3 Golden fixtures

Keep deterministic fixtures for:

- valid function;
- ABI clobber;
- forbidden syscall;
- unknown instruction;
- executable stack;
- wrong target object;
- malformed ELF;
- infinite loop;
- crash;
- output bomb;
- repeated candidate;
- task mutation attempt.

### 23.4 Negative corpus

The negative corpus is a first-class asset.

Categories:

- parser abuse;
- macro expansion abuse;
- include path escape;
- section flag abuse;
- relocation abuse;
- symbol collision;
- stack imbalance;
- callee-saved corruption;
- indirect branch uncertainty;
- unsupported instruction;
- privileged instruction;
- forbidden syscall;
- hidden import;
- W+X memory;
- fork/process bomb;
- stdout/stderr bomb;
- timeout;
- malformed object;
- corrupted cache record.

### 23.5 Property and fuzz tests

Candidates:

- schema parsers never panic;
- canonical report serialization is stable;
- path normalization never escapes run root;
- arbitrary tool output cannot create `verified`;
- any missing required check prevents acceptance;
- task digest remains unchanged across repair iterations;
- candidate source normalization is idempotent.

### 23.6 End-to-end tests

Required first:

1. no-model fixture candidate accepted;
2. deterministic mocked model candidate accepted;
3. one repair after assembler error;
4. one repair after ABI violation;
5. incomplete SemASM coverage rejected;
6. sandbox test timeout handled;
7. reproducibility mismatch rejected;
8. no-progress loop stops.

Live model tests are optional and non-blocking because provider output is nondeterministic and costs money.

---

## 24. CI Design

### 24.1 Required fast jobs

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
cargo deny check
```

### 24.2 Integration job

Pinned environment containing:

- compatible SemASM;
- NASM;
- linker;
- object tools;
- rootless sandbox backend or safe CI substitute;
- fixture-only model adapter.

This job runs all deterministic end-to-end fixtures.

### 24.3 Platform jobs

Initial required:

- Linux x86-64 controller;
- Windows controller compile and unit tests.

The supported generated target remains x86-64 Linux until the full target gate passes.

### 24.4 Scheduled jobs

- fuzz smoke;
- dependency audit;
- reproducibility rebuild;
- negative corpus;
- optional live-provider smoke using protected secrets;
- sandbox hardening checks.

---

## 25. Dependency Policy

### 25.1 Initial dependency principles

- standard library first;
- one CLI parser;
- one serialization stack;
- one error library if justified;
- one hashing library;
- one HTTP client only when the model adapter becomes live;
- no async runtime until measured concurrency requires it;
- no embedded database initially;
- no general plugin framework;
- no provider SDK;
- no container SDK.

### 25.2 External command interfaces

Use the installed CLI for:

- SemASM;
- NASM;
- linker;
- object tools;
- sandbox runtime.

This keeps the dependency graph smaller and command transcripts reproducible.

### 25.3 Optional features later

```toml
[features]
default = ["local-cli"]
live-model = ["dep:http-client"]
sandbox-container = []
sandbox-vm = []
service = []
sqlite-index = []
```

Default features must remain useful and minimal.

---

## 26. Implementation Phases

Phases are evidence gates, not calendar promises.

### Phase 0 — SemASM readiness gate

Deliverables:

- compatibility checklist;
- required SemASM commands documented;
- fixture reports captured;
- status model verified;
- missing SemASM prerequisites listed.

Exit criteria:

- VAA can reliably distinguish SemASM success, violation, incomplete analysis, and failure.

### Phase 1 — Offline controller skeleton

No model API.

Deliverables:

- CLI;
- task schema;
- policy;
- immutable digests;
- run directory;
- event log;
- fixture candidate input;
- report aggregation.

Exit criteria:

- `vaa verify task.vaa.toml --source fixture.asm` produces a complete evidence report.

### Phase 2 — Isolated build vertical slice

Deliverables:

- process runner;
- sandbox adapter;
- assembler;
- linker;
- artifact inspection;
- bounded logs;
- build manifest.

Exit criteria:

- trusted fixture builds reproducibly without host environment leakage.

### Phase 3 — Trusted behavioral harness

Deliverables:

- callable-function harness;
- authoritative test fixtures;
- sandbox run;
- timeout and process cleanup;
- result normalization.

Exit criteria:

- valid routine passes and intentionally broken routines fail for the correct reason.

### Phase 4 — Deterministic model adapter

Use a fixture or scripted model.

Deliverables:

- `submit_candidate` protocol;
- candidate validation;
- one repair loop;
- no-progress detection;
- budget accounting.

Exit criteria:

- deterministic end-to-end repair test passes in CI.

### Phase 5 — Live model adapter

Deliverables:

- one OpenAI-compatible adapter;
- secret handling;
- structured output;
- provider error taxonomy;
- opt-in live smoke command.

Exit criteria:

- local user can generate a routine, while CI remains deterministic without external API dependency.

### Phase 6 — Evidence hardening

Deliverables:

- canonical reports;
- provenance;
- content-addressed cache;
- reproducibility check;
- tamper detection.

Exit criteria:

- a run can be independently replayed from recorded inputs and pinned tools.

### Phase 7 — Alpha release

Deliverables:

- truthful README;
- security policy;
- threat model;
- compatibility policy;
- release checklist;
- signed checksums where available.

Exit criteria:

- one target and one artifact class satisfy all alpha gates.

---

## 27. Ordered Pull Request Plan

### PR-001 — Repository bootstrap

- initialize one Rust binary crate;
- licenses;
- code style;
- CI;
- dependency policy;
- no functional claims.

### PR-002 — Task schema v0.1

- typed task model;
- strict parsing;
- JSON Schema export or checked-in schema;
- fixtures;
- validation diagnostics.

### PR-003 — Policy and immutable task digest

- canonical serialization;
- digest;
- mutation guard;
- tests proving repair cannot alter policy.

### PR-004 — Run directory and event log

- safe paths;
- atomic writes;
- immutable candidate folders;
- bounded event records.

### PR-005 — SemASM doctor and version negotiation

- binary discovery;
- version read;
- schema compatibility;
- explicit degraded states.

### PR-006 — SemASM capabilities adapter

- target query;
- capability digest;
- policy requirement matching;
- incomplete target rejection.

### PR-007 — SemASM verification adapter

- process outcome;
- JSON parsing;
- status mapping;
- raw and normalized reports;
- malformed report fixtures.

### PR-008 — Final evidence status aggregator

- `verified`, `violated`, `incomplete`, `failed`;
- required versus optional checks;
- CLI exit codes.

### PR-009 — Hardened process runner

- null stdin;
- environment allowlist;
- output bounds;
- timeout;
- process-tree kill;
- explicit working directory.

### PR-010 — Build sandbox backend

- backend trait;
- rootless container implementation or controlled local fixture backend;
- network disabled;
- resource limits;
- image digest.

### PR-011 — NASM and linker pipeline

- explicit argv;
- build manifest;
- no shell;
- object and binary outputs;
- complete command evidence.

### PR-012 — Artifact inspection gate

- architecture and format;
- symbols and sections;
- imports;
- executable stack;
- W+X;
- size policy.

### PR-013 — Trusted callable-function harness

- repository-owned templates;
- test-vector conversion;
- register preservation checks where practical;
- deterministic result protocol.

### PR-014 — Execution sandbox

- explicit opt-in;
- runtime limits;
- bounded output;
- signal/exit normalization;
- full cleanup.

### PR-015 — Candidate protocol

- structured `submit_candidate`;
- source normalization;
- target echo;
- size limits;
- repeated-hash rejection.

### PR-016 — Fixture model adapter

- deterministic scripted responses;
- assembler-error repair fixture;
- ABI-error repair fixture;
- no-progress fixture.

### PR-017 — Orchestrator state machine

- legal transitions;
- event persistence;
- restart behavior;
- terminal states;
- invariant tests.

### PR-018 — Budget and progress controller

- token/cost fields;
- candidate count;
- wall time;
- repeated diagnostics;
- no-progress stop.

### PR-019 — Live model adapter

- one provider protocol;
- structured tool submission;
- secret handling;
- opt-in command;
- no provider types in core.

### PR-020 — Content-addressed cache

- verification key;
- build key;
- integrity checks;
- no prompt-only binary reuse.

### PR-021 — Reproducibility report

- canonical versus volatile metadata;
- second build comparison;
- mismatch diagnostics.

### PR-022 — Negative corpus and fuzz entry points

- path abuse;
- tool output abuse;
- malformed reports;
- output bombs;
- schema fuzzing.

### PR-023 — Documentation and alpha release gate

- README;
- architecture;
- security;
- threat model;
- compatibility;
- known limitations;
- release checklist.

No PR should mix multiple major boundaries merely to reduce the number of pull requests.

---

## 28. Acceptance Commands

Exact commands may evolve, but the repository must provide a script or documented equivalent.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
cargo deny check
```

CLI smoke:

```bash
cargo run -- doctor
cargo run -- validate fixtures/tasks/sum-i64.vaa.toml
cargo run -- capabilities --target x86_64-unknown-linux-gnu
cargo run -- verify fixtures/tasks/sum-i64.vaa.toml \
  --source fixtures/candidates/sum-i64-valid.asm
```

Deterministic end-to-end:

```bash
cargo test --test e2e_offline
cargo test --test e2e_repair
cargo test --test e2e_incomplete_analysis
cargo test --test e2e_sandbox_limits
cargo test --test e2e_reproducibility
```

Repository hygiene:

```bash
git diff --check
git status --short
```

The agent must record actual command outcomes in each pull request.

---

## 29. Definition of Done

### 29.1 Experimental vertical slice

- one target;
- one artifact kind;
- no live model required;
- immutable task;
- SemASM report consumed;
- honest incomplete status;
- isolated build;
- trusted tests;
- evidence bundle;
- deterministic CI.

### 29.2 Alpha

- live model optional;
- bounded repair;
- hardened process runner;
- negative corpus;
- capability and schema negotiation;
- canonical provenance;
- content-addressed cache;
- reproducible build check;
- documented threat model;
- no known fail-open path.

### 29.3 Beta

- stable public schemas;
- compatibility matrix;
- at least two independent sandbox profiles or a stronger VM profile;
- target conformance suite;
- fuzzing history;
- supply-chain controls;
- external security review of the execution boundary;
- clear retention and privacy policy for service mode.

### 29.4 Production-candidate

- operational deployment model documented;
- authentication and tenant isolation if networked;
- incident response;
- backup and evidence retention;
- signed releases;
- rollback;
- resource and cost controls;
- measured service reliability;
- independent review of security claims;
- no marketing wording beyond demonstrated evidence.

---

## 30. Work Explicitly Deferred

Do not implement during the first target:

- AArch64;
- RISC-V;
- bare-metal;
- Redis;
- FastAPI;
- web dashboard;
- Prometheus;
- OpenTelemetry;
- distributed queue;
- Kubernetes;
- plugin marketplace;
- automatic model fallback;
- multiple provider SDKs;
- dynamic Rust plugin ABI;
- arbitrary user-supplied build scripts;
- arbitrary shell tools for the model;
- network-enabled generated programs;
- performance autotuning;
- superoptimization;
- theorem proving;
- automatic firmware flashing.

Each deferred item requires a new threat and dependency review.

---

## 31. Revised README Positioning

Suggested concise product description:

> VAA is an experimental controller for generating and validating small assembly artifacts with an LLM, SemASM, native toolchains, and isolated tests. It preserves an immutable task contract, treats model output as untrusted, and produces explicit evidence for what was verified, violated, incomplete, or failed.

Suggested honesty note:

> VAA does not guarantee memory safety, optimal performance, or correctness beyond the declared contract and completed evidence. Unsupported semantics and unavailable checks are reported as incomplete, never as success.

---

## 32. Example End-to-End User Flow

```bash
# 1. Check dependencies and security profile
vaa doctor

# 2. Convert a request into a reviewable task
vaa plan \
  "Implement a System V x86-64 function that sums signed 64-bit values" \
  --out sum-i64.vaa.toml

# 3. User reviews and locks the task
vaa validate sum-i64.vaa.toml

# 4. Generate, verify, build, and test under policy
vaa build sum-i64.vaa.toml

# 5. Inspect evidence
vaa inspect <run-id>

# 6. Export a canonical report
vaa report <run-id> --format json > run-report.json
```

Example terminal result:

```text
status: accepted
policy: local-callable-function-v0.1
target: x86_64-unknown-linux-gnu
candidate attempts: 2
semantic coverage: complete for required checks
ABI checks: passed
artifact checks: passed
trusted tests: 8/8 passed
reproducible build: passed
artifact sha256: ...
evidence sha256: ...

Limitation: acceptance applies only to the locked task, target profile,
toolchain, and tests recorded in the evidence bundle.
```

---

## 33. Final Recommendation

The VAA concept should continue, but not in the original “Python + LiteLLM + Instructor + PyO3 + Redis + Docker + FastAPI + Prometheus” form.

That stack can produce a demo quickly, but it creates too many moving parts around an immature verification core. It would make debugging trust failures harder and would conflict with the goal of a compact, understandable system.

The recommended path is:

1. stabilize SemASM’s evidence semantics;
2. create VAA as a separate Rust CLI;
3. begin with a structured task, not unrestricted natural language;
4. begin with one callable-function target;
5. use SemASM through a versioned JSON process boundary;
6. isolate build and execution separately;
7. treat the contract as immutable;
8. use a four-state evidence result;
9. add a deterministic fixture model before a live model;
10. delay service infrastructure until the local evidence pipeline is trustworthy.

The important product is not an agent that “always fixes assembly.”

The important product is an agent controller that can say, with reproducible evidence:

> This exact artifact satisfies these exact declared requirements under this exact target and verification profile—or the system clearly explains why that conclusion cannot be established.
