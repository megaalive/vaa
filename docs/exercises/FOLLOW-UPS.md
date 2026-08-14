# Exercise follow-ups (from E01–E05 friction)

Status after the ladder close-out. Prefer bounded fixes; keep non-goals closed.

## Done in-ladder

| Item | From | Fix |
|---|---|---|
| Subsystem PE hint | E02 | `windows_link_hint` |
| `--linker-arg` | E03 | `vaa build` |
| Win SDK env for `lld-link` | E03 | `toolchain_subprocess_allowed_env` |
| `--extra-object` | E04 | `vaa build` |
| Reject callable + imports | E05 | `validate_task` |
| Admit decline hint (verify ≠ admit) | E01 | `vaa admit` JSON/terminal |

## Closed this pass

| Item | From | Action |
|---|---|---|
| Ladder summary | — | [`SUMMARY.md`](SUMMARY.md) |
| Verifiable-but-not-admitted inventory | E01 | [`verifiable-vs-admitted.md`](verifiable-vs-admitted.md) |
| `vaa build --object-only` | E02 | Assemble without PE link |
| Hosted imports → linker lib map | E03 | Docs + `suggested_win64_linker_args` helper/hint |
| Capabilities are declarative in 0.1 | E03 #7 | Documented in SUMMARY / leaf-vs-hosted (not asm-enforced) |
| Multi-source build | E04 | Deferred; `--extra-object` covers the exercised case |

## Remain non-goals / deferred

| Item | Why |
|---|---|
| Admit `min_i64` / `increment_i64` / WriteFile / REPL | Honesty: snapshot is intentional, not “whatever verifies” |
| Admit `nibble_to_hex` / `hexdump` from T04 alone | Need oracle + freeze; tool success ≠ admission (see [t04-hexdump.md](t04-hexdump.md)) |
| Full TTY line editing / ReadConsole | Out of leaf scope; E05 sketch is enough |
| Cross-check `extern` vs `capabilities.imports` | Larger static-analysis feature; keep declarative for now |
| Auto-pass `/DEFAULTLIB` from task TOML into `vaa build` | Keep linker args explicit; suggestions only |

## Next workstream

Everyday utilities (not hello-world): track and update progress in
[`REAL-TOOLS-ROADMAP.md`](REAL-TOOLS-ROADMAP.md).

Compiler dogfood (HlaX64 pack Gate, not tools):
[`g01-hlax64-compiler-demo.md`](g01-hlax64-compiler-demo.md),
[`../compiler-demo.md`](../compiler-demo.md).

**Deepen debt D1–D5** (ledger in
[Open deepen backlog](REAL-TOOLS-ROADMAP.md#open-deepen-backlog-tracked-debt--do-not-drop)):
**all paid 2026-07-27.**

1. ~~`replace_byte` compose (T08)~~ — **done** (D1)
2. ~~Quoted INI values (T07)~~ — **done** (D2)
3. ~~JSON minify (T09)~~ — **done** (D3)
4. ~~Streaming multi-chunk (T02/T03)~~ — **done** (D4)
5. ~~`crc32` leaf pressure (T05)~~ — **done** (D5)
