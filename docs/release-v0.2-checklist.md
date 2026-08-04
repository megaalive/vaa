# VAA v0.2.0 release checklist

Tag only the exact clean candidate that passes all required gates:

- Cargo package version and tag both equal `0.2.0` / `v0.2.0`.
- Formatting, clippy, workspace tests, protocol-freeze gates, and docs pass.
- Task schemas 0.1/0.2, JSON Schema, examples, and honesty rules agree.
- VAA schema 0.2 Win64 E2E includes builtin plus task vectors and validates
  SemASM report 0.6 digest/origin/expected bindings.
- SemASM `v0.5.0` is already published and its Windows archive passed its own
  post-package oracle smoke.
- VAA Windows archive is extracted and passes `release-artifact-smoke.ps1`
  against the published SemASM archive; Linux archive version/status probes
  also pass after extraction.
- Combined `SHA256SUMS` verifies before release publication.

Any skipped or failed required gate blocks the tag.
