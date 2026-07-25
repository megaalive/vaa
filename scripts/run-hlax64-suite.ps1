#Requires -Version 5.1
<#
.SYNOPSIS
  Run an HlaX64 pack suite through `vaa suite run` (live generator path).

.DESCRIPTION
  Milestone 6 live suite wiring.

  Default: generate + identity only (-SkipVerify), suite status Incomplete.

  Gate mode (SemASM verify):
    -Gate
  implies -SkipVerify:$false -AllowExecution and expects suite Accepted with
  Verified cases (practice seal key via ci-gate-sign-setup.ps1 /
  VAA_SEAL_SIGNING_KEY). Incomplete is not Verified; emit is not SemASM
  verified; practice seal is not a trust root.
#>
param(
    [string]$Suite = "integrations/hlax64/suites/scalar-win64.vaa-suite.toml",
    [string]$Repo = "",
    [string]$RunDir = "target/hlax64-suite-runs",
    [string]$Output = "target/hlax64-suite-runs/suite-evidence.json",
    [switch]$SkipVerify = $true,
    [switch]$SkipBuild,
    [switch]$AllowExecution,
    [switch]$CheckDeterministic,
    # Convenience: Gate-2 style (verify + allow-execution).
    [switch]$Gate
)
$ErrorActionPreference = "Stop"

$vaaRoot = Split-Path -Parent $PSScriptRoot
Set-Location $vaaRoot

if ($Gate) {
    $SkipVerify = $false
    $AllowExecution = $true
}

if (-not $Repo) {
    if ($env:HLAX64_ROOT) {
        $Repo = $env:HLAX64_ROOT
    } elseif (Test-Path (Join-Path $vaaRoot "hlax64")) {
        $Repo = Join-Path $vaaRoot "hlax64"
    } else {
        $Repo = Join-Path (Split-Path $vaaRoot -Parent) "hlax64"
    }
}
if (-not (Test-Path $Repo)) {
    Write-Error "HlaX64 repo not found at $Repo (set -Repo or HLAX64_ROOT)"
}

if (-not $SkipVerify) {
    if (-not (Get-Command semasm -ErrorAction SilentlyContinue)) {
        Write-Error "semasm not on PATH (required for -Gate / verify mode)"
    }
    if (-not $env:VAA_SEAL_SIGNING_KEY) {
        Write-Host "VAA_SEAL_SIGNING_KEY unset - running scripts/ci-gate-sign-setup.ps1 (practice key)"
        & "$PSScriptRoot/ci-gate-sign-setup.ps1"
        if ($LASTEXITCODE -ne 0) {
            throw "ci-gate-sign-setup failed"
        }
    }
}

New-Item -ItemType Directory -Force -Path $RunDir | Out-Null
$outDir = Split-Path -Parent $Output
if ($outDir) {
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

$cargoArgs = @(
    "run", "-q", "--", "suite", "run", $Suite,
    "--repo", $Repo,
    "--run-dir", $RunDir,
    "--output", $Output,
    "--skip-repo-guard"
)
if ($SkipBuild) {
    $cargoArgs += "--skip-build"
}
if ($SkipVerify) {
    $cargoArgs += "--skip-verify"
}
if ($AllowExecution) {
    $cargoArgs += "--allow-execution"
}
if ($CheckDeterministic) {
    $cargoArgs += "--check-deterministic"
}

Write-Host ("Running: cargo {0}" -f ($cargoArgs -join ' '))
& cargo @cargoArgs
$code = $LASTEXITCODE
if ($code -ne 0) {
    if ($SkipVerify -and (Test-Path $Output)) {
        $evidence = Get-Content -Raw $Output | ConvertFrom-Json
        $st = [string]$evidence.status
        # Generate-only runs are Incomplete by policy (not Verified).
        if ($st -eq "incomplete" -or $st -eq "Incomplete") {
            Write-Host ("Suite evidence: {0} (status={1}; skip-verify Incomplete is not Verified)" -f $Output, $st)
            Get-Content $Output | Select-Object -First 40
            exit 0
        }
    }
    throw ("vaa suite run failed with exit {0}" -f $code)
}

if (Test-Path $Output) {
    $evidence = Get-Content -Raw $Output | ConvertFrom-Json
    $st = [string]$evidence.status
    Write-Host ("Suite evidence: {0} (status={1})" -f $Output, $st)
    if (-not $SkipVerify) {
        if ($st -ne "accepted" -and $st -ne "Accepted") {
            throw ("Gate suite expected Accepted, got status={0}" -f $st)
        }
        Write-Host "Gate pack suite Accepted (Verified cases; practice seal is not a trust root)"
    }
    Get-Content $Output | Select-Object -First 40
}
