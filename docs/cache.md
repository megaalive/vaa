# Local content-addressed cache (PR-020)

Layout (default root `./.vaa/cache`, override with `VAA_CACHE_DIR`):

```text
.vaa/cache/
├── blobs/sha256/<hex>
├── verification/<keyhex>.json
├── builds/<keyhex>.json
└── index/
```

## Keys

- **Verification:** source + contract + task digests, target, SemASM version,
  `allow_execution`, capability source pin — never prompt-only.
- **Build:** source digest, tool digests, target, args fingerprint, optional
  container image digest.

## CLI

```bash
vaa cache status
vaa verify … --cache   # opt-in; Gate CI leaves this off
vaa build … --cache
```

## Honesty

- Local store only — not a remote immutable log / Rekor.
- Incomplete/Failed cache hits are never promoted to Verified.
- Path components with `..` or separators are rejected.

See also: reproducibility (PR-021) in `vaa build --check-reproducible` —
same-host twin compare, not cross-host bit-identical.
