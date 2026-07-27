# Real-tools exercise roadmap

Living plan to harden VAA + SemASM with **everyday utilities**, not hello-world
toys. Update **Status** / **Progress notes** as work lands. Friction logs stay
under `docs/exercises/`; scratch under `.vaa-exercises/` (gitignored).

**Related:** [SUMMARY.md](SUMMARY.md) (E01–E05), [FOLLOW-UPS.md](FOLLOW-UPS.md),
[verifiable-vs-admitted.md](verifiable-vs-admitted.md),
[leaf-vs-hosted.md](../leaf-vs-hosted.md), [HONESTY.md](../HONESTY.md).

## Status legend

| Status | Meaning |
|---|---|
| `planned` | Specced here; not started |
| `in_progress` | Scratch + friction run underway |
| `friction_logged` | Tracked summary committed; follow-ups open |
| `bounded_fix` | At least one tooling/docs fix from friction landed |
| `done_enough` | Tool runs + honest CLAIM; further work optional |
| `wont` | Explicitly deferred / non-goal |

## Principles

1. **One tool = one daily job** (not a product REPL).
2. **Split:** pure leaf(s) that *might* seal + hosted shell (argv, files, stdout).
3. **Deliverables per exercise:**
   - tool runs (hosted)
   - 1–N isolated leaves (admit / verify / feedback)
   - short `CLAIM.md` (or friction Outcomes): sealed / VUP / not claimed
4. **Do not** seal the whole binary because a leaf verified.
5. **Do not** admit shapes into the skill snapshot only because the tool is useful.

Method (unchanged):

```text
Exercise → Friction log → Bounded fix → Honest claim (or decline)
```

## Recommended sequence (≈6 weeks)

| Order | ID | Why first |
|---|---|---|
| 1 | T01 | Reuses buffer leaves; pushes file + argv hosted profile |
| 2 | T04 | Encoding leaf + formatting hosted |
| 3 | T08 | Env import + `replace_byte` composition |
| 4 | T07 | Parse boundary — strong maturity signal |
| 5 | T02 | Streaming / partial-read budgets |
| 6 | Tier C pick | One real workflow tool (owner chooses) |

---

## Tier A — thin CLI (1 file in/out)

| ID | Tool | Leaf focus | Hosted focus | Status | Progress notes |
|---|---|---|---|---|---|
| T01 | `wc`-lite (bytes/lines/LF) | `count_byte` / line-count buffer | argv path, `CreateFile`/`ReadFile`, print counts | `done_enough` | 2026-07-27: argv+`sample.txt`+stdin; leaf VUP; AV from stack misalign fixed in scratch. [t01-wc-lite.md](t01-wc-lite.md) |
| T02 | `head` / `tail` | find-nth-LF / slice-by-offset | file I/O + byte budget | `friction_logged` | 2026-07-27: D4 `CHUNK=64`+carry stream; cicil-2 argv N + `tail.exe`; `find_nth_lf` UNSUPPORTED_SHAPE. [t02-head.md](t02-head.md) |
| T03 | `uniq` consecutive | memcmp window / equal-run | streaming buffer policy | `friction_logged` | 2026-07-27: D4 multi-chunk carry+prev spill; cicil-2 CR/LF; `equal_run` UNSUPPORTED_SHAPE. [t03-uniq.md](t03-uniq.md) |
| T04 | hexdump | nibble encode leaf | format loop, width flags | `friction_logged` | 2026-07-27: cicil-2 argv width 1..32; `nibble_to_hex` UNSUPPORTED_SHAPE+admit decline. [t04-hexdump.md](t04-hexdump.md) |
| T05 | `xor` / `crc32` filter | pure-int or buffer transform | stdin→stdout pipeline | `friction_logged` | 2026-07-27: D5 `crc32` admit decline+UNSUPPORTED_SHAPE; cicil-2 argv key; `xor_*` declined. [t05-xor.md](t05-xor.md) |

## Tier B — tools people use more often

| ID | Tool | Why it matures VAA/SemASM | Status | Progress notes |
|---|---|---|---|---|
| T06 | path join / basename (Win, length-bounded) | string/buffer contracts, VUP honesty | `friction_logged` | 2026-07-27: T06b join+basename; `find_last_byte` VUP; `basename` UNSUPPORTED_SHAPE. [t06-path.md](t06-path.md) |
| T07 | INI/TOML key lookup (read-only, fixed schema) | parse vs leaf seal; templates / `UNSUPPORTED_SHAPE` | `friction_logged` | 2026-07-27: T07b `[section]`+key; D2 quoted values; `memcmp`/`find_first_byte` VUP; `ini_lookup` UNSUPPORTED_SHAPE. [t07-ini-lookup.md](t07-ini-lookup.md) |
| T08 | env-subst (`${FOO}` in template) | `GetEnvironmentVariable` + find/replace leaf | `friction_logged` | 2026-07-27: T08b multi `${NAME}`; T08c/D1 `replace_byte` CR→LF; both leaves VUP. [t08-env-subst.md](t08-env-subst.md) |
| T09 | JSON minify / key extract (strict subset) | structured buffer oracles **or** honest decline | `friction_logged` | 2026-07-27: T09c bare values + T09b/D3 minify argv; `json_get` UNSUPPORTED_SHAPE. [t09-json-get.md](t09-json-get.md) |
| T10 | diff hunk stats (line-oriented; no full Myers) | E04 line-loop → useful metric tool | `friction_logged` | 2026-07-27: hosted unified-diff parser; counts `hunks/added/removed`. [t10-diff-hunk-stats.md](t10-diff-hunk-stats.md) |

## Tier C — pick 2–3 around real workflow (optional)

| ID | Tool idea | Notes | Status | Progress notes |
|---|---|---|---|---|
| T11 | `git status` porcelain summary (count M/A/D) | hosted parse + leaf counters | `friction_logged` | 2026-07-27: hosted parser for porcelain `XY`; counts M/A/D. [t11-git-status-porcelain.md](t11-git-status-porcelain.md) |
| T12 | log grepper (chunked large buffers) | find/count + mmap/chunk friction | `planned` | |
| T13 | checksum tree | dir walk hosted + hash leaf (admit only after oracle) | `planned` | |
| T14 | clipboard / path helper (Win) | hosted-only; build/import UX — **not** skill seal | `planned` | |
| T15 | HTTP HEAD timer | network capability fail-closed stress | `planned` | Expect decline today |
| T16 | CSV column cut | delimiter scan leaf + argv hosted | `friction_logged` | 2026-07-27: col 0–99 + light quotes (T16c/T16b); `find_first_byte` VUP; `csv_cut` UNSUPPORTED_SHAPE. [t16-csv-cut.md](t16-csv-cut.md) |

---

## Per-exercise checklist (copy into friction log)

When starting `Txx`:

- [ ] Scratch: `.vaa-exercises/tXX-<slug>/`
- [ ] Tracked friction: `docs/exercises/tXX-<slug>.md` (from [FRICTION.template.md](FRICTION.template.md))
- [ ] Hosted task validates; admit/verify claims stay honest
- [ ] Leaf path: `vaa admit` before any skill/harness seal language
- [ ] `CLAIM` section: sealed / VUP / runtime-only / declined
- [ ] Update **Status** + **Progress notes** in this file
- [ ] Bounded fix (optional) + link commit/PR in Progress notes

## Explicit non-goals (for this roadmap)

- Full editor, full product REPL, full JSON RFC, TLS-as-leaf-seal
- Auto-admit every SemASM-verifiable name because a tool needs it
- Weakening `UNSUPPORTED_SHAPE` so parsers “pass”
- Claiming “tool verified” from leaf verify + exit code

## Open deepen backlog (tracked debt — do not drop)

Originally left after cicil-2 so debt would not vanish into per-file checkboxes.
**D1–D5 all paid 2026-07-27.** Keep the table as a ledger (struck rows = done).

| ID | Item | Status | Home |
|---|---|---|---|
| ~~D1~~ | ~~`replace_byte` compose (CR→LF before subst)~~ | **Done** — admitted VUP + multi-subst | [t08-env-subst.md](t08-env-subst.md) |
| ~~D2~~ | ~~Quoted INI values (`key="a=b"`)~~ | **Done** — `demo title` → `hello=world` | [t07-ini-lookup.md](t07-ini-lookup.md) |
| ~~D3~~ | ~~JSON minify (whitespace outside strings)~~ | **Done** — `minify` argv mode | [t09-json-get.md](t09-json-get.md) |
| ~~D4~~ | ~~Streaming multi-chunk (partial line / prev spill)~~ | **Done** — `CHUNK=64`+carry (T02/T03) | [t02-head.md](t02-head.md), [t03-uniq.md](t03-uniq.md) |
| ~~D5~~ | ~~`crc32` / rolling checksum leaf pressure~~ | **Done** — admit decline + `UNSUPPORTED_SHAPE` | [t05-xor.md](t05-xor.md) |

No open deepen debt from this list.

## Changelog (roadmap doc)

| Date | Note |
|---|---|
| 2026-07-27 | Initial plan captured from post-E05 discussion (Tier A–C + 6-week sequence) |
| 2026-07-27 | T01 cicil-1: `sample.txt`/stdin wc-lite + `count_byte` VUP; friction [t01-wc-lite.md](t01-wc-lite.md); argv deferred T01b |
| 2026-07-27 | T01 cicil-2: `GetCommandLineA` argv path; Win64 stack-align AV noted; status `done_enough` |
| 2026-07-27 | T04 cicil-1: hexdump + `nibble_to_hex` UNSUPPORTED_SHAPE; [t04-hexdump.md](t04-hexdump.md) |
| 2026-07-27 | T08 cicil-1: env-subst one `${NAME}` + `find_first_byte` VUP; [t08-env-subst.md](t08-env-subst.md) |
| 2026-07-27 | T07 cicil-1: flat INI lookup + dual-buffer `ini_lookup` UNSUPPORTED_SHAPE; [t07-ini-lookup.md](t07-ini-lookup.md) |
| 2026-07-27 | T02 cicil-1: `head` 10 lines + `find_nth_lf` UNSUPPORTED_SHAPE; [t02-head.md](t02-head.md) |
| 2026-07-27 | T03 cicil-1: consecutive `uniq` + `equal_run` UNSUPPORTED_SHAPE; [t03-uniq.md](t03-uniq.md) |
| 2026-07-27 | T05 cicil-1: xor filter + `xor_u8`/`xor_bytes` UNSUPPORTED_SHAPE; [t05-xor.md](t05-xor.md) |
| 2026-07-27 | T06 cicil-1: basename + `find_last_byte` VUP; [t06-path.md](t06-path.md) |
| 2026-07-27 | T09 cicil-1: JSON string-key extract; `json_get` UNSUPPORTED_SHAPE; [t09-json-get.md](t09-json-get.md) |
| 2026-07-27 | T16 cicil-1: CSV column cut; `csv_cut` UNSUPPORTED_SHAPE; [t16-csv-cut.md](t16-csv-cut.md) |
| 2026-07-27 | T09c: bare numeric/bool values; T09b minify skipped; T16c multi-digit col + T16b light quotes |
| 2026-07-27 | T02/T03/T04/T05 cicil-2: argv N+tail, CR/LF uniq, argv width, argv xor key |
| 2026-07-27 | Open deepen backlog D1–D5 captured (replace_byte, quoted INI, JSON minify, multi-chunk, crc32) |
| 2026-07-27 | D4 paid: T02/T03 streaming `CHUNK=64` + carry/prev spill; D5 paid: `crc32` admit decline + `UNSUPPORTED_SHAPE` |
| 2026-07-27 | D1–D3 paid: `replace_byte` compose, quoted INI, JSON minify — backlog D1–D5 clear |
