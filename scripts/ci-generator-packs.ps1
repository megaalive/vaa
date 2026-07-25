#Requires -Version 5.1
<#
.SYNOPSIS
  CI Gate: generator pack matrix (schema, parity, echoasm generate, patch evidence).

.DESCRIPTION
  Milestone 6 pack CI. Does not claim HlaX64 Gate Verified or live agent repair.
#>
$ErrorActionPreference = "Stop"
$vaaRoot = Split-Path -Parent $PSScriptRoot
Set-Location $vaaRoot

function Invoke-Vaa {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$CmdArgs)
    & cargo run -q -- @CmdArgs
    if ($LASTEXITCODE -ne 0) {
        throw "vaa $($CmdArgs -join ' ') failed with exit $LASTEXITCODE"
    }
}

Write-Host "== Gate 1: validate pack locks + specs =="
Invoke-Vaa generator validate-lock integrations/hlax64/stack.lock.toml
Invoke-Vaa generator validate-spec integrations/hlax64/generator.spec.toml
Invoke-Vaa generator validate-spec integrations/hlax64/generator.sysv.spec.toml
Invoke-Vaa generator validate-lock integrations/echoasm/stack.lock.toml
Invoke-Vaa generator validate-spec integrations/echoasm/generator.spec.toml

Write-Host "== Gate 1b: validate suites =="
$suites = @(
    "integrations/hlax64/suites/smoke.vaa-suite.toml",
    "integrations/hlax64/suites/scalar-win64.vaa-suite.toml",
    "integrations/hlax64/suites/scalar-i64-win64.vaa-suite.toml",
    "integrations/hlax64/suites/scalar-sysv.vaa-suite.toml",
    "integrations/hlax64/suites/loop-win64.vaa-suite.toml",
    "integrations/hlax64/suites/loop-stack-win64.vaa-suite.toml",
    "integrations/hlax64/suites/scalar-i64-sysv.vaa-suite.toml",
    "integrations/hlax64/suites/loop-stack-sysv.vaa-suite.toml",
    "integrations/hlax64/suites/memory-read-win64.vaa-suite.toml",
    "integrations/hlax64/suites/memory-write-win64.vaa-suite.toml",
    "integrations/hlax64/suites/memory-concrete-win64.vaa-suite.toml",
    "integrations/hlax64/suites/calls-data-win64.vaa-suite.toml",
    "integrations/hlax64/suites/negative-reject-win64.vaa-suite.toml",
    "integrations/hlax64/suites/backend-win64.vaa-suite.toml",
    "integrations/echoasm/suites/smoke.vaa-suite.toml",
    "integrations/echoasm/suites/gate-load-byte0-win64.vaa-suite.toml",
    "integrations/echoasm/suites/gate-concrete-win64.vaa-suite.toml",
    "integrations/echoasm/suites/gate-scalar-i64-win64.vaa-suite.toml",
    "integrations/echoasm/suites/gate-phase-b-loops-win64.vaa-suite.toml",
    "integrations/echoasm/suites/gate-phase-b-stack-win64.vaa-suite.toml",
    "integrations/echoasm/suites/gate-phase-e-calls-win64.vaa-suite.toml",
    "integrations/echoasm/suites/gate-memory-read-win64.vaa-suite.toml",
    "integrations/echoasm/suites/gate-memory-write-win64.vaa-suite.toml"
)
foreach ($s in $suites) {
    Invoke-Vaa suite validate $s
}

Write-Host "== Gate 1c: target/ABI parity =="
Invoke-Vaa suite check-parity integrations/hlax64/suites/scalar-win64.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/hlax64/suites/scalar-i64-win64.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/hlax64/suites/scalar-sysv.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/hlax64/suites/loop-stack-win64.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/hlax64/suites/scalar-i64-sysv.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/hlax64/suites/loop-stack-sysv.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/hlax64/suites/memory-concrete-win64.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/hlax64/suites/calls-data-win64.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/hlax64/suites/negative-reject-win64.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/hlax64/suites/backend-win64.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/echoasm/suites/smoke.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/echoasm/suites/gate-load-byte0-win64.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/echoasm/suites/gate-concrete-win64.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/echoasm/suites/gate-scalar-i64-win64.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/echoasm/suites/gate-phase-b-loops-win64.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/echoasm/suites/gate-phase-b-stack-win64.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/echoasm/suites/gate-phase-e-calls-win64.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/echoasm/suites/gate-memory-read-win64.vaa-suite.toml
Invoke-Vaa suite check-parity integrations/echoasm/suites/gate-memory-write-win64.vaa-suite.toml

Write-Host "== Gate 3: EchoAsm deterministic generation =="
$gen = (Resolve-Path "integrations/echoasm/tools/echoasm.cmd").Path
$in = (Resolve-Path "integrations/echoasm/cases/passthrough/input.asm").Path
$outDir = Join-Path $vaaRoot "target/ci-echoasm"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$out = Join-Path $outDir "candidate.asm"
Invoke-Vaa generator generate `
    integrations/echoasm/generator.spec.toml `
    --generator $gen `
    --input $in `
    --output $out `
    --target x86_64-pc-windows-msvc `
    --check-deterministic

$h1 = (Get-FileHash $out -Algorithm SHA256).Hash
$h2 = (Get-FileHash $in -Algorithm SHA256).Hash
if ($h1 -ne $h2) {
    throw "EchoAsm output digest mismatch (universality smoke broken)"
}
Write-Host "EchoAsm digest match OK"

Write-Host "== Gate 7: patch evidence fixtures =="
$accepted = "fixtures/repair/echoasm-passthrough/patch-evidence.json"
$forbidden = "fixtures/repair/echoasm-passthrough/patch-evidence.forbidden-failed.json"
if (-not (Test-Path $accepted)) {
    Write-Host "Repair fixtures missing - rebuilding"
    & "$PSScriptRoot/rebuild-echoasm-repair-fixture.ps1"
    if ($LASTEXITCODE -ne 0) { throw "rebuild repair fixture failed" }
}
Invoke-Vaa patch evidence-verify $accepted
Invoke-Vaa repair verify fixtures/repair/hlax64-min-usize-sysv-live/repair-packet.json
Invoke-Vaa patch evidence-verify fixtures/repair/hlax64-min-usize-sysv-live/patch-evidence.json
Write-Host "== Gate 7b: repair export compiler_source map-join =="
& "$PSScriptRoot/ci-repair-compiler-source-mapjoin.ps1"
if ($LASTEXITCODE -ne 0) { throw "compiler_source map-join e2e failed" }
Write-Host "== Gate 7c: Win64 min_i64 map-line repair join =="
& "$PSScriptRoot/ci-repair-win64-min-i64-mapline.ps1"
if ($LASTEXITCODE -ne 0) { throw "Win64 map-line repair e2e failed" }
Write-Host "== Gate 7d: Win64 signed worktree live repair fixtures =="
Invoke-Vaa repair verify fixtures/repair/hlax64-min-i64-win64-live-worktree/repair-packet.json
Invoke-Vaa patch evidence-verify fixtures/repair/hlax64-min-i64-win64-live-worktree/patch-evidence.json
Write-Host "== Gate 7e: Win64 unsigned worktree live repair fixtures =="
Invoke-Vaa repair verify fixtures/repair/hlax64-min-usize-win64-live-worktree/repair-packet.json
Invoke-Vaa patch evidence-verify fixtures/repair/hlax64-min-usize-win64-live-worktree/patch-evidence.json
Write-Host "== Gate 7f: SysV signed worktree live repair fixtures =="
Invoke-Vaa repair verify fixtures/repair/hlax64-min-i64-sysv-live-worktree/repair-packet.json
Invoke-Vaa patch evidence-verify fixtures/repair/hlax64-min-i64-sysv-live-worktree/patch-evidence.json
Write-Host "== Gate 7g: Win64 max unsigned worktree live repair fixtures =="
Invoke-Vaa repair verify fixtures/repair/hlax64-max-usize-win64-live-worktree/repair-packet.json
Invoke-Vaa patch evidence-verify fixtures/repair/hlax64-max-usize-win64-live-worktree/patch-evidence.json
Write-Host "== Gate 7h: Win64 max signed worktree live repair fixtures =="
Invoke-Vaa repair verify fixtures/repair/hlax64-max-i64-win64-live-worktree/repair-packet.json
Invoke-Vaa patch evidence-verify fixtures/repair/hlax64-max-i64-win64-live-worktree/patch-evidence.json
Write-Host "== Gate 7i: SysV max signed worktree live repair fixtures =="
Invoke-Vaa repair verify fixtures/repair/hlax64-max-i64-sysv-live-worktree/repair-packet.json
Invoke-Vaa patch evidence-verify fixtures/repair/hlax64-max-i64-sysv-live-worktree/patch-evidence.json
Write-Host "== Gate 7j: SysV max unsigned worktree live repair fixtures =="
Invoke-Vaa repair verify fixtures/repair/hlax64-max-usize-sysv-live-worktree/repair-packet.json
Invoke-Vaa patch evidence-verify fixtures/repair/hlax64-max-usize-sysv-live-worktree/patch-evidence.json
Write-Host "== Gate 7k: Win64 stack-balance worktree live repair fixtures =="
Invoke-Vaa repair verify fixtures/repair/hlax64-stack-balance-win64-live-worktree/repair-packet.json
Invoke-Vaa patch evidence-verify fixtures/repair/hlax64-stack-balance-win64-live-worktree/patch-evidence.json
Write-Host "== Gate 7l: Win64 callee-saved worktree live repair fixtures =="
Invoke-Vaa repair verify fixtures/repair/hlax64-callee-saved-win64-live-worktree/repair-packet.json
Invoke-Vaa patch evidence-verify fixtures/repair/hlax64-callee-saved-win64-live-worktree/patch-evidence.json
Write-Host "== Gate 7m: SysV stack-balance worktree live repair fixtures =="
Invoke-Vaa repair verify fixtures/repair/hlax64-stack-balance-sysv-live-worktree/repair-packet.json
Invoke-Vaa patch evidence-verify fixtures/repair/hlax64-stack-balance-sysv-live-worktree/patch-evidence.json
$jsonText = & cargo run -q -- patch evidence-verify $forbidden --format json
if ($LASTEXITCODE -ne 0) { throw "forbidden fixture failed structural verify" }
$parsed = $jsonText | ConvertFrom-Json
$status = [string]$parsed.evidence.status
if ($status -ne "Failed" -and $status -ne "failed") {
    throw "forbidden-path fixture status must be Failed, got: $status"
}
Write-Host "forbidden fixture correctly Failed"

Write-Host "== isolation audit =="
Invoke-Vaa generator isolation-check

Write-Host "OK: generator pack matrix passed"
