# Repair rules for generator `hlax64`

These rules bind any coding agent or interactive editor working on this
generator. Acceptance authority is VAA/SemASM evidence, never agent output.

## Scope

- repository: `../hlax64` at `git:4546be1da3bac70e68e9027e9b323903e1800e71`
- fix the **generator source**, never the generated assembly.

## Allowed paths (editable)

- `src/backend/**`
- `src/codegen/**`
- `src/ir/**`
- `tests/backend/**`

## Forbidden paths (never edit)

- `**/integrations/hlax64/cases/**/task.vaa.toml`
- `**/integrations/hlax64/cases/**/contract.sem.toml`
- `**/integrations/hlax64/cases/**/vectors.json`
- `**/integrations/hlax64/stack.lock.toml`

Additionally forbidden everywhere: generated candidate assembly,
SemASM contracts, VAA tasks, authoritative vectors, `stack.lock.toml`,
and evidence files.

## Fixed commands

| Step | Command |
|---|---|
| build | `cargo build --release` |
| regenerate one case | `vaa generator-run --spec integrations/hlax64/generator.spec.toml --lock integrations/hlax64/stack.lock.toml --task <case>/task.vaa.toml --contract <case>/contract.sem.toml --input <case>/input.txt --skip-verify` |
| verify one case | `vaa generator-run --spec integrations/hlax64/generator.spec.toml --lock integrations/hlax64/stack.lock.toml --task <case>/task.vaa.toml --contract <case>/contract.sem.toml --input <case>/input.txt` |
| full regression suite | `vaa suite run integrations/hlax64/suites/smoke.vaa-suite.toml` |

## Loop (after each change)

1. rebuild the generator;
2. regenerate candidate assembly from the locked generator input;
3. run the supplied case verification command;
4. run the required regression suite before completion.

## Acceptance

- output is accepted only after patch evidence verifies
(`vaa patch evidence-verify`); agent self-report is not acceptance;
- `Incomplete` / `verified_under_preconditions` is **not** `Verified` and
is not, by itself, a generator defect (run `vaa generator triage`);
- never weaken contracts, vectors, or the stack lock to make a case pass.
