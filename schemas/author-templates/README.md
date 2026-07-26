# Author templates (Release C)

Bounded task + contract stubs for `vaa author init --template <name>`.

These are **drafts until a human locks** them with `vaa author lock`. Agents may
propose edits to draft cases; agents must **not** lock acceptance authority.
Admission is checked before lock. Experimental templates / unadmitted leaf names
require `--experimental` on lock and stay `authoring_only` (never
`sealed_acceptance`).

## Catalog

| Template | Shape sketch | Known-CI shape? |
|---|---|---|
| `pure-int-unary` | single `i64` → `i64` (abs-like) | no (experimental) |
| `pure-int-binary` | two `i64` → `i64` (`max_i64` / `sum_i64`-like) | yes |
| `pure-int-ternary` | three `i64` → `i64` (clamp-like sketch) | no (experimental) |
| `buffer-read` | buffer + length + needle (`count_byte`-like) | yes |
| `buffer-write` | buffer fill (`memset` / `replace_byte`-like) | yes |
| `dual-buffer` | two buffers + length (`memcmp` / `memcpy`-like) | yes |

Placeholders filled by `vaa author init`: `__NAME__`, `__TARGET__`, `__ABI__`,
`__TASK_ID__`.

After fill-in, `vaa validate <case>/task.vaa.toml` should succeed.
