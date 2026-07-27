# Maturity exercises

Hands-on programs used to harden VAA + SemASM. **Work products stay local** under
[`.vaa-exercises/`](../.vaa-exercises/) (gitignored). **Friction summaries** are
committed here so lessons survive.

## Method

```text
Exercise → Friction log → Bounded fix → Honest claim (or decline)
```

Do **not** weaken schemas or promote VUP/incomplete to success to make an
exercise “pass”.

## Ladder

| ID | Exercise | Goal |
|---|---|---|
| E01 | Leaf clinic from intent | Naming/oracle/`vaa admit`/feedback without copying fixtures blindly |
| E02 | Hello leaf + thin main (Win64) | `callable-function` verify + hosted link boundary |
| E03 | I/O stub WriteFile | Imports / `kernel32` / hosted capabilities |
| E04 | Line loop | Orchestration vs leaf seal |
| E05 | REPL sketch | Integration only; no false verify claims |

## Starting an exercise

```bash
# scratch tree (ignored)
mkdir -p .vaa-exercises/e02-hello-leaf-win64

# after the run, write/update the tracked summary under docs/exercises/
```

Template for summaries: [`FRICTION.template.md`](FRICTION.template.md).

**Status:** E01–E05 friction logs are committed (scratch under `.vaa-exercises/`).
See [`SUMMARY.md`](SUMMARY.md), [`FOLLOW-UPS.md`](FOLLOW-UPS.md), and
[`verifiable-vs-admitted.md`](verifiable-vs-admitted.md).

| Logged | Summary |
|---|---|
| [E01](e01-leaf-clinic.md) | Intent→name→behavior→repair; SemASM verified ≠ admit |
| [E02](e02-hello-leaf-win64.md) | Leaf verify vs admit decline; thin Win64 main exit 42 |
| [E03](e03-writefile-win64.md) | Hosted WriteFile/`kernel32`; admit+verify decline; `--linker-arg` |
| [E04](e04-line-loop-win64.md) | Admitted `max_i64` + hosted line loop; seal leaf-only; `--extra-object` |
| [E05](e05-repl-sketch-win64.md) | REPL sketch runs; no REPL seal; reject callable+imports |

See also [`leaf-vs-hosted.md`](../leaf-vs-hosted.md).
