# EchoAsm repair fixture (Milestone 6)

Locked demonstration that patch evidence verifies from a clean checkout.

## Narrative

1. **Base** (`base/echoasm.cmd`): broken generator prepends `; BROKEN`.
2. **Patched** (`patched/echoasm.cmd`): restores the clean copy tool
   (same as `integrations/echoasm/tools/echoasm.cmd`).
3. **Allowed change:** `integrations/echoasm/tools/echoasm.cmd` (matches
   `patch_policy.allowed_paths`).
4. **`patch-evidence.json`:** status `Accepted` (suite accepted + no
   forbidden paths).
5. **`patch-evidence.forbidden-failed.json`:** touches
   `stack.lock.toml` → status `Failed`.

## Verify

```text
vaa patch evidence-verify fixtures/repair/echoasm-passthrough/patch-evidence.json
vaa patch evidence-verify fixtures/repair/echoasm-passthrough/patch-evidence.forbidden-failed.json --format json
```

Rebuild:

```powershell
./scripts/rebuild-echoasm-repair-fixture.ps1
```

## Honesty

- EchoAsm universality repair smoke ≠ live HlaX64 backend defect fix.
- Suite “accepted” here means the generation/identity smoke for the
  echo pack, **not** SemASM Gate Verified.
- Incomplete ≠ Verified; SoftHSM ≠ hardware HSM.
