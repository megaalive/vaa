# VAA Dependency Policy

This document is the human-readable companion to `deny.toml`.

## Principles (first vertical slice)

1. Prefer the Rust standard library.
2. Allow one CLI parser (`clap`).
3. Add one serialization stack only when task/evidence schemas land.
4. Add one error library only if typed errors justify it.
5. Add one hashing library only when digests are implemented.
6. Add one HTTP client only for the live model adapter PR — never by default.
7. Do not add an async runtime until measured concurrency requires it.
8. Do not add embedded databases, plugin frameworks, provider SDKs, or container SDKs.
9. Do not add Python, FastAPI, Redis, LiteLLM, Instructor, or similar stacks.
10. Prefer external CLIs (SemASM, NASM, linker, object tools, sandbox runtime) over embedding their libraries.

## Current direct dependencies

| Crate | Why |
|---|---|
| `clap` | Argument parsing for the local CLI |
| `serde` | Typed task model derive |
| `serde_json` | Canonical JSON for digests and `--format json` CLI output |
| `toml` | Parse `task.vaa.toml` |
| `sha2` | SHA-256 task content digests |
| `object` | Read ELF/PE/Mach-O for artifact inspection |
| `thiserror` | Typed `TaskError` diagnostics |
| `ed25519-dalek` | Optional Ed25519 seal authenticity (`acceptance_digest`) |
| `rand` | CSPRNG for `vaa evidence keygen-seal` only |
| `base64` | Encode/decode seal `public_key_b64` / `sig_b64` |
| `win32job` (Windows only) | Job Object ownership so timeout/overflow kills the full process tree |
| `ureq` (**optional**, feature `live-model`) | Sync OpenAI-compatible HTTP for PR-019 — never default |

## Optional features

```toml
[features]
default = ["local-cli"]
local-cli = []
live-model = ["dep:ureq"]
```

Default features remain offline and free of live provider SDKs. Enable live generate with:

```bash
cargo run --features live-model -- generate task.vaa.toml --output out.asm --live
# requires VAA_MODEL_API_KEY; optional VAA_MODEL_BASE_URL / VAA_MODEL_NAME
```

## License allow-list

See `deny.toml` `[licenses].allow`. Dual-license crate choice should prefer MIT OR Apache-2.0 when available.
`CDLA-Permissive-2.0` is allowed for `webpki-roots` (pulled by optional `ureq` / `live-model`).

## Review triggers

Any new dependency requires:

- a one-line justification in this file;
- license compatibility with MIT OR Apache-2.0 distribution;
- no known critical advisory at merge time (`cargo deny check` when available);
- no silent expansion of network, plugin, or sandbox attack surface.

## Optional features (planned, not enabled yet)

```toml
[features]
default = ["local-cli"]
live-model = []   # enabled: see above (ureq)
sandbox-container = []
sandbox-vm = []
service = []
sqlite-index = []
```

Default features must remain useful offline and free of live provider or container SDKs.
