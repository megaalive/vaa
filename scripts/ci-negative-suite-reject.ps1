#Requires -Version 5.1
<#
.SYNOPSIS
  Assert the locked negative HlaX64 suite Rejects under SemASM Gate.

.DESCRIPTION
  Runs `min_i64_wrong` (implements max instead of min) with -Gate and
  requires suite status Rejected with a Violated case. Proves fail-closed
  suite aggregation without live agent edits.
#>
param(
    [string]$Repo = "",
    [string]$RunDir = "target/hlax64-negative-reject",
    [string]$Output = "target/hlax64-negative-reject/suite-evidence.json"
)
$ErrorActionPreference = "Stop"
$vaaRoot = Split-Path -Parent $PSScriptRoot
Set-Location $vaaRoot

if (-not $Repo) {
    if ($env:HLAX64_ROOT) { $Repo = $env:HLAX64_ROOT }
    elseif (Test-Path (Join-Path $vaaRoot "hlax64")) { $Repo = Join-Path $vaaRoot "hlax64" }
    else { $Repo = Join-Path (Split-Path $vaaRoot -Parent) "hlax64" }
}
if (-not (Test-Path $Repo)) { Write-Error "HlaX64 repo not found at $Repo" }
if (-not (Get-Command semasm -ErrorAction SilentlyContinue)) {
    Write-Error "semasm not on PATH (required for negative Gate)"
}
if (-not $env:VAA_SEAL_SIGNING_KEY) {
    & "$PSScriptRoot/ci-gate-sign-setup.ps1"
    if ($LASTEXITCODE -ne 0) { throw "ci-gate-sign-setup failed" }
}

New-Item -ItemType Directory -Force -Path $RunDir | Out-Null

# Suite run is expected to fail (Rejected). Capture evidence anyway.
& cargo run -q -- suite run `
    integrations/hlax64/suites/negative-reject-win64.vaa-suite.toml `
    --repo $Repo `
    --run-dir $RunDir `
    --output $Output `
    --skip-repo-guard `
    --allow-execution
# Non-zero exit is OK for Rejected; we judge by evidence JSON.
if (-not (Test-Path $Output)) {
    throw "negative suite evidence missing at $Output"
}

$ev = Get-Content -Raw $Output | ConvertFrom-Json
$st = [string]$ev.status
if ($st -ne "rejected" -and $st -ne "Rejected") {
    throw ("negative suite expected Rejected, got status={0}" -f $st)
}
$violated = @($ev.cases | Where-Object {
    $s = [string]$_.status
    $s -eq "Violated" -or $s -eq "violated"
})
if ($violated.Count -lt 1) {
    throw "negative suite expected at least one Violated case"
}
Write-Host ("OK: negative suite Rejected with {0} Violated case(s)" -f $violated.Count)
Get-Content $Output | Select-Object -First 30
exit 0
