# vaa-canonical-json-v1

Deterministic JSON encoding used for VAA content digests (locked task digest and
evidence seal digests). External verifiers (Rust, C#, Go, Zig, …) must implement
these rules rather than relying on one serializer’s defaults.

## Identifiers

| Field | Value |
|---|---|
| `canonicalization` | `vaa-canonical-json-v1` |
| `digest_algorithm` | `sha256` |

Digest form: `sha256:` + lowercase hex of SHA-256 over the canonical UTF-8 bytes.

## Rules

1. **Encoding:** UTF-8.
2. **Objects:** keys sorted in lexicographic (byte) order at every nesting level.
3. **Whitespace:** no insignificant whitespace; compact separators (no spaces after `:` / `,`).
4. **Arrays:** element order is preserved; only nested objects inside elements are re-sorted.
5. **Values:** JSON `null`, `true`, `false`, numbers, and strings as produced by a
   standard JSON encoder after the structure has been normalized.
6. **Numbers:** serialize without unnecessary trailing zeros or exponent forms that
   change the numeric token; VAA currently emits integers and simple floats via
   `serde_json` after key sorting. Do not reorder or rewrite number tokens beyond
   that encoder’s normal form.
7. **Unicode in strings:** emit UTF-8 code points directly inside JSON strings
   (do not require `\uXXXX` escaping for non-ASCII). Escapes for control characters
   (`\n`, `\t`, `\"`, `\\`, …) follow a conforming JSON encoder. Do not NFC/NFD-normalize
   string contents for digests.

## Non-goals

This is **not** RFC 8785 (JCS) verbatim, nor CBOR deterministic encoding. It is a
small named profile sufficient for cross-language verification of VAA digests.
