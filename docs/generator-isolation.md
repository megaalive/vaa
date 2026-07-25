# Generator subprocess isolation (plan §20)

VAA generator build/generate runs through `ProcessRunner`, which always
`env_clear`s the child environment and re-injects only an allowlist.

## Rules

1. Credentials never inherit from the parent process.
2. Generator profile allowlist is broader than the SemASM default (so
   `cmd` / Rust / .NET toolchains can run) but still closed.
3. Credential-shaped names are denied even if someone proposes adding them
   to the allowlist (`is_credential_env_name`).
4. `extra_env` injections that look like credentials are refused
   (`reject_credential_extra_env`).
5. Candidate execution still requires SemASM's explicit execution profile
   (`--allow-execution`); that authority stays outside the coding agent.

## Audit

```text
vaa generator isolation-check
vaa generator isolation-check --format json
```

Reports: allowlist size, which present vars would pass, which credential
vars are present-but-denied, and which other vars are stripped.

## Honesty

Isolation here is **local developer / CI hardening**, not a claim of a
multi-tenant sandbox or production shared-service isolation. Untrusted
remote patches still require operational isolation before acceptance.
