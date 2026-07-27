# Maturity ladder summary (E01–E05)

Hands-on stress tests of VAA + SemASM on Win64. Scratch lives under
`.vaa-exercises/` (gitignored); friction logs are the durable product.

## Verdict in one line

**Leaf seals are narrow and honest; hosted I/O can run and validate as design
docs, but never inherits a SemASM/skill seal.**

## Per-exercise

| ID | Built | Honest outcome | Tooling that landed |
|---|---|---|---|
| [E01](e01-leaf-clinic.md) | `min_i64` from intent (wrong name → wrong body → repair) | SemASM **verified**; `vaa admit` **decline** (vs `max_i64` admitted) | Admit decline `hint` (verify ≠ admit) |
| [E02](e02-hello-leaf-win64.md) | `increment_i64` + thin `ExitProcess` main | Leaf may verify; admit decline; PE exit 42 ≠ seal | Subsystem link hint; later `--object-only` |
| [E03](e03-writefile-win64.md) | `GetStdHandle` + `WriteFile` stub | Hosted validate ok; admit/verify decline; print works | `--linker-arg`; Win SDK env for `kernel32.lib` |
| [E04](e04-line-loop-win64.md) | Admitted `max_i64` + ReadFile line loop | **Leaf** sealed; loop declined; exit 5 | `--extra-object` |
| [E05](e05-repl-sketch-win64.md) | Prompt/echo/quit REPL + leaf metric | REPL runs; no REPL seal | Reject `callable-function` + `imports` |

## Dual gates (never collapse these)

```text
intent / naming  →  SemASM agent-verify (shape + behavior)
                 →  vaa admit (frozen snapshot)  →  skill harness only if admitted
hosted I/O       →  vaa validate (task schema) + build/run
                 →  never claim verified/sealed for the program
```

- **SemASM verified** ≠ **skill admitted** (E01 `min_i64`).
- **Program runs / exit code** ≠ **seal** (E02–E05).
- **VUP** (`verified_under_preconditions`) ≠ plain **verified** ([`HONESTY.md`](../HONESTY.md)).

## Agent failure modes these exercises catch

1. Synonym names (`smaller_i64`) → `UNSUPPORTED_SHAPE` (need `min_*` / `inc_*` …).
2. Stale `semasm.exe` / dry-run treated as evidence.
3. Tagging a REPL as `callable-function` with Win32 imports.
4. Claiming the whole program verified because a called leaf verified.
5. PE link without `/subsystem` / `/entry` / `kernel32` (or without SDK env under VAA).

## Non-goals (leave fail-closed)

- Do not auto-admit every SemASM-verifiable name (`min_i64`, `increment_i64`, …).
- Do not admit WriteFile / line-loop / REPL into the skill snapshot.
- Do not promote VUP / incomplete / runtime exit to success.

## Follow-ups from friction (tracked)

See [`FOLLOW-UPS.md`](FOLLOW-UPS.md).

## Next ladder

Everyday tools (wc-lite, hexdump, env-subst, …):
[`REAL-TOOLS-ROADMAP.md`](REAL-TOOLS-ROADMAP.md).
