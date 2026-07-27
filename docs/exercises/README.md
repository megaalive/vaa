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
**Deepen backlog D1–D5 is paid** (ledger kept struck in the roadmap) — not lost debt.

| Logged | Summary |
|---|---|
| [T01](t01-wc-lite.md) | wc-lite via `count_byte` VUP; argv+file+stdin; tool not sealed |
| [T02](t02-head.md) | D4 stream `CHUNK=64`+carry; cicil-2 argv N/`tail`; `find_nth_lf` declined |
| [T03](t03-uniq.md) | D4 multi-chunk carry+prev; cicil-2 CR/LF; `equal_run` UNSUPPORTED_SHAPE |
| [T04](t04-hexdump.md) | cicil-2: argv width; `nibble_to_hex` UNSUPPORTED_SHAPE |
| [T05](t05-xor.md) | D5 `crc32` admit decline+UNSUPPORTED_SHAPE; cicil-2 argv xor key |
| [T06](t06-path.md) | cicil-2: basename + path_join; `basename` UNSUPPORTED_SHAPE |
| [T07](t07-ini-lookup.md) | D2 quoted values + `[section]`; `ini_lookup` UNSUPPORTED_SHAPE |
| [T08](t08-env-subst.md) | D1 `replace_byte` CR→LF + multi `${}`; leaves VUP |
| [T09](t09-json-get.md) | D3 minify + bare values; `json_get` decline |
| [T10](t10-diff-hunk-stats.md) | unified diff hunk stats; counts `hunks/added/removed`; tool not sealed |
| [T11](t11-git-status-porcelain.md) | `git status --porcelain` parse; counts M/A/D; tool not sealed |
| [T12](t12-log-grepper.md) | chunked log grepper; substring counts with carry overlap |
| [T13](t13-checksum-tree.md) | bounded checksum tree (top-level files); sum-bytes in 16-hex |
| [T16](t16-csv-cut.md) | cicil-2: col 0–99 + light quotes; `csv_cut` UNSUPPORTED_SHAPE |

See also [`leaf-vs-hosted.md`](../leaf-vs-hosted.md).
