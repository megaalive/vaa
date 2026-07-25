# Repair packet: `min-usize-sysv-v1` (generator `hlax64`)

Fix the **generator under test**, not the generated assembly.

## Failure

- classification: `generator_candidate_violated`
- diagnostic_code: `BEHAVIOR_VECTOR_MISMATCH_001`
- message: SemASM behavior_failed: unsigned min returned max for 4/6 vectors (a smaller observed 2, b smaller observed 5, zero and large observed 1000000, wide spread observed 100)

## Generated artifact

- path: `target/live-repair/broken-sysv2/suite-out/min_usize_sysv/candidate.asm`
- digest: `sha256:d420b3e72ce66cedc570b2886aed82e5557c0638855f260f5a93d89f0496762b`

## Source mapping

- generator_input: `integrations/hlax64/cases/min_usize_sysv/input.hla64:18`
- ir_node: `CompareKind.LessThanUnsigned`
- generator_source: `src/HlaX64.Compiler/Abi/SysVAbiLowerer.cs:476`

## Repository

- base_revision: `git:4546be1da3bac70e68e9027e9b323903e1800e71`

Allowed paths:

- `src/HlaX64.Compiler/Abi/**`
- `src/HlaX64.Compiler/Ir/**`
- `src/HlaX64.Backend.Nasm/**`
- `tests/HlaX64.Compiler.Tests/**`

Forbidden paths (never edit):

- `**/integrations/hlax64/cases/**/task.vaa.toml`
- `**/integrations/hlax64/cases/**/contract.sem.toml`
- `**/integrations/hlax64/cases/**/vectors.json`
- `**/integrations/hlax64/stack.lock.toml`

## Commands

1. build: `dotnet build src/HlaX64.Cli/HlaX64.Cli.csproj -c Release --nologo`
2. regenerate: `./scripts/run-hlax64-suite.sh --gate --suite integrations/hlax64/suites/scalar-sysv.vaa-suite.toml --repo ../hlax64-live-repair`
3. verify: `cargo test --project tests/HlaX64.Compiler.Tests && ./scripts/run-hlax64-suite.sh --gate --suite integrations/hlax64/suites/scalar-sysv.vaa-suite.toml`

## Constraints

- Do not edit generated assembly manually
- Do not edit contracts, vectors, task files, or stack.lock.toml
- Regenerate all candidates after generator changes
- Run the required regression suite before completion
