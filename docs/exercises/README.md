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
See [`SUMMARY.md`](SUMMARY.md), [`FOLLOW-UPS.md`](FOLLOW-UPS.md),
[`verifiable-vs-admitted.md`](verifiable-vs-admitted.md), and the next ladder
[`REAL-TOOLS-ROADMAP.md`](REAL-TOOLS-ROADMAP.md) (everyday utilities T01+).

| Logged | Summary |
|---|---|
| [E01](e01-leaf-clinic.md) | Intent→name→behavior→repair; SemASM verified ≠ admit |
| [E02](e02-hello-leaf-win64.md) | Leaf verify vs admit decline; thin Win64 main exit 42 |
| [E03](e03-writefile-win64.md) | Hosted WriteFile/`kernel32`; admit+verify decline; `--linker-arg` |
| [E04](e04-line-loop-win64.md) | Admitted `max_i64` + hosted line loop; seal leaf-only; `--extra-object` |
| [E05](e05-repl-sketch-win64.md) | REPL sketch runs; no REPL seal; reject callable+imports |

## Next ladder — real tools

Track status and progress on [`REAL-TOOLS-ROADMAP.md`](REAL-TOOLS-ROADMAP.md)
(T01 `wc`-lite → … → Tier C picks). Same method; higher everyday utility.
Remaining cicil deepenings are listed there as **Open deepen backlog (D1–D5)**
so they stay visible debt, not lost checkboxes.

| Logged | Summary |
|---|---|
| [T01](t01-wc-lite.md) | wc-lite via `count_byte` VUP; argv+file+stdin; tool not sealed |
| [T02](t02-head.md) | cicil-2: `head`/`tail` + argv N; `find_nth_lf` UNSUPPORTED_SHAPE |
| [T03](t03-uniq.md) | cicil-2: ignore CR before LF; `equal_run` UNSUPPORTED_SHAPE |
| [T04](t04-hexdump.md) | cicil-2: argv width; `nibble_to_hex` UNSUPPORTED_SHAPE |
| [T05](t05-xor.md) | cicil-2: argv hex key; `xor_u8`/`xor_bytes` UNSUPPORTED_SHAPE |
| [T06](t06-path.md) | cicil-2: basename + path_join; `basename` UNSUPPORTED_SHAPE |
| [T07](t07-ini-lookup.md) | cicil-2: `[section]`+key; `ini_lookup` UNSUPPORTED_SHAPE |
| [T08](t08-env-subst.md) | cicil-2: multi `${NAME}` loop; `find_first_byte` VUP |
| [T09](t09-json-get.md) | cicil-2: string+bare numeric/bool; minify skipped; `json_get` decline |
| [T16](t16-csv-cut.md) | cicil-2: col 0–99 + light quotes; `csv_cut` UNSUPPORTED_SHAPE |

See also [`leaf-vs-hosted.md`](../leaf-vs-hosted.md).
