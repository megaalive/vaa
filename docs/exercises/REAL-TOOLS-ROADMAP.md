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
| T01 | `wc`-lite (bytes/lines/LF) | `count_byte` / line-count buffer | argv path, `CreateFile`/`ReadFile`, print counts | `planned` | |
| T02 | `head` / `tail` | find-nth-LF / slice-by-offset | file I/O + byte budget | `planned` | After T01 |
| T03 | `uniq` consecutive | memcmp window / equal-run | streaming buffer policy | `planned` | |
| T04 | hexdump | nibble encode leaf | format loop, width flags | `planned` | |
| T05 | `xor` / `crc32` filter | pure-int or buffer transform | stdin→stdout pipeline | `planned` | |

## Tier B — tools people use more often

| ID | Tool | Why it matures VAA/SemASM | Status | Progress notes |
|---|---|---|---|---|
| T06 | path join / basename (Win, length-bounded) | string/buffer contracts, VUP honesty | `planned` | |
| T07 | INI/TOML key lookup (read-only, fixed schema) | parse vs leaf seal; templates / `UNSUPPORTED_SHAPE` | `planned` | |
| T08 | env-subst (`${FOO}` in template) | `GetEnvironmentVariable` + find/replace leaf | `planned` | |
| T09 | JSON minify / key extract (strict subset) | structured buffer oracles **or** honest decline | `planned` | |
| T10 | diff hunk stats (line-oriented; no full Myers) | E04 line-loop → useful metric tool | `planned` | |

## Tier C — pick 2–3 around real workflow (optional)

| ID | Tool idea | Notes | Status | Progress notes |
|---|---|---|---|---|
| T11 | `git status` porcelain summary (count M/A/D) | hosted parse + leaf counters | `planned` | Owner pick |
| T12 | log grepper (chunked large buffers) | find/count + mmap/chunk friction | `planned` | |
| T13 | checksum tree | dir walk hosted + hash leaf (admit only after oracle) | `planned` | |
| T14 | clipboard / path helper (Win) | hosted-only; build/import UX — **not** skill seal | `planned` | |
| T15 | HTTP HEAD timer | network capability fail-closed stress | `planned` | Expect decline today |
| T16 | CSV column cut | delimiter scan leaf + argv hosted | `planned` | |

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

## Changelog (roadmap doc)

| Date | Note |
|---|---|
| 2026-07-27 | Initial plan captured from post-E05 discussion (Tier A–C + 6-week sequence) |
