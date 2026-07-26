---
name: vaa-harness
description: Repair and verify a VAA/SemASM assembly corpus leaf (count_byte, find_first_byte, find_last_byte, memcmp, memcpy, memset, replace_byte, sum_i64, max_i64) through `vaa harness` via scripts/agent_harness_adapter.py. Use ONLY for these allowlisted assembly leaves in this repo; decline anything else. Not for general coding.
disable-model-invocation: false
---

# VAA harness skill (leaf repair only)

You drive the VAA agent harness to repair/verify **allowlisted assembly leaves**.
You are a **proposer**; SemASM is the verifier. The canonical claim boundary is
[`docs/HONESTY.md`](../../../docs/HONESTY.md) and it overrides anything here.

## Hard rules

1. **Allowlist only (until admission fully replaces it).** Operate only on leaves
   in
   [`schemas/agent-leaf-allowlist.json`](../../../schemas/agent-leaf-allowlist.json)
   (`count_byte`, `count_byte_linux`, `find_first_byte`, `find_last_byte`,
   `memcmp`, `memcpy`, `memset`, `replace_byte`, `sum_i64`, `max_i64`). The
   SemASM admission snapshot
   ([`fixtures/semasm/capabilities-snapshot.json`](../../../fixtures/semasm/capabilities-snapshot.json))
   must list the same leaf names — decline anything missing from either gate.
   If the user asks for any other shape, free-form `.S`, RISC-V verify, "fix
   my assembly", or generator-repair — **stop and decline** (see the playbook
   decline path). Do not improvise.
2. **Use the adapter, not shell soup.** Prefer
   `python scripts/agent_harness_adapter.py loop-direct …`. Do not hand-roll
   `vaa harness` pipelines.
3. **Parse stdout JSON only.** stderr is noise. Never infer a status, class, or
   seal from stderr, logs, or exit-code guessing. Also read workspace
   `work-packet.json` (else `agent-envelope.json`) and `feedback.json` when
   present. Use `target-profile.json` for ABI — **do not hardcode** registers.
4. **Map `class` → action, nothing else:**
   - `accepted` → done. Report the JSON `class`, `evidence_status`, `seal_digest`.
   - `violated_repairable` → edit the candidate assembly only; resubmit.
   - `policy_blocked` → stop and report (path policy refused it).
   - `toolchain_retryable` → stop and report (missing/broken toolchain); do not retry silently.
   - anything else / `failed` / `incomplete` → stop and report. Never promote to success.
5. **VUP ≠ verified.** Only leaves with `strict_verified_ok: true` (currently
   only `max_i64`) may be called plainly "verified". Buffer/loop leaves reach
   `verified_under_preconditions`; report them as VUP and pass
   `--allow-under-preconditions`.
6. **No claim without evidence.** Never say "sealed"/"verified" without a real
   `seal_digest` in the JSON (or `vaa evidence verify-chain` exit 0). A stub /
   `VAA_BIN` dry-run is **not** evidence.
7. **v1 excludes:** generator-repair, HlaX64/EchoAsm pack work, live model,
   RISC-V agent-verify. Point to the docs and decline.
8. **Never lock.** `vaa author lock` is **human CLI only**. If asked to lock a
   case, grant seal authority, or flip `AUTHOR_STATE` / write `LOCKED` —
   **decline** and point humans to [`docs/author.md`](../../../docs/author.md)
   (`vaa author init|review|lock`). You may help draft task/contract text for
   `vaa author init` / review, but you do not hold acceptance authority.

## Forbidden phrases

Never write or imply: "proven safe", "formally verified memory", "verifies any
assembly", "handles arbitrary asm", "AI-generated and verified secure", or
calling a VUP result plainly "verified".

## Happy path (Win64 `max_i64`, strict verified)

```bash
python scripts/agent_harness_adapter.py loop-direct \
  --task fixtures/run/max_i64/max_i64.vaa.toml \
  --contract fixtures/run/max_i64/max_i64.sem.toml \
  --workspace .vaa/harness/max_i64 \
  --run-base .vaa/runs/max_i64 \
  --candidate fixtures/run/max_i64/01_wrong.asm \
  --candidate fixtures/run/max_i64/02_repaired.asm \
  --allow-execution
```

For a VUP leaf (e.g. `count_byte`) add `--allow-under-preconditions` and report
the result as `verified_under_preconditions`, not plain verified.

See [`docs/agent-playbook.md`](../../../docs/agent-playbook.md) for the full
happy path and the decline path.
