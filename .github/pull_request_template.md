## Invariant

<!-- What trust or evidence invariant does this PR protect? -->

## Summary

<!-- What changed? -->

## Tests

- [ ] Unit / adapter tests added or updated
- [ ] Tests fail before the change where practical
- [ ] Acceptance commands recorded below

## Acceptance commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Capability / evidence impact

<!-- Update docs/implementation-baseline.md or capability notes if SemASM assumptions changed. -->

## Residual limitations

<!-- What is still incomplete, unsupported, or intentionally out of scope? -->

## Checklist

- [ ] No incomplete analysis is reported as verified
- [ ] No model or tool output weakens the locked task/policy
- [ ] Generated artifacts and run logs are not committed unless intentional fixtures
- [ ] README claims remain no stronger than executable evidence
