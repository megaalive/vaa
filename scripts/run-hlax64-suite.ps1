#Requires -Version 5.1
<#
.SYNOPSIS
  Run an HlaX64 pack suite through `vaa suite run` (live generator path).

.DESCRIPTION
  Milestone 6 live suite wiring. Default: scalar-win64 with --skip-verify
  (generation + identity only). Pass -AllowExecution / omit -SkipVerify for
  SemASM verify when `semasm` is on PATH.

  Honesty: --skip-verify ⇒ Incomplete ≠ Verified. Emit ≠ SemASM verified.
#>
param(
    [string]$Suite = "integrations/hlax64/suites/scalar-win64.vaa-suite.toml",
    [string]$Repo = "",
    [string]$RunDir = "target/hlax64-suite-runs",
    [string]$Output = "target/hlax64-suite-runs/suite-evidence.json",
    [switch]$SkipVerify = $true,
    [switch]$SkipBuild,
    [switch]$AllowExecution,
    [switch]$CheckDeterministic
)
$ErrorActionPreference = "Stop"

$vaaRoot = Split-Path -Parent $PSScriptRoot
Set-Location $vaaRoot

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

New-Item -ItemType Directory -Force -Path $RunDir | Out-Null
$outDir = Split-Path -Parent $Output
if ($outDir) { New-Item -ItemType Directory -Force -Path $outDir | Out-Null }

$args = @(
    "run", "-q", "--", "suite", "run", $Suite,
    "--repo", $Repo,
    "--run-dir", $RunDir,
    "--output", $Output,
    "--skip-repo-guard"
)
if ($SkipBuild) { $args += "--skip-build" }
if ($SkipVerify) { $args += "--skip-verify" }
if ($AllowExecution) { $args += "--allow-execution" }
if ($CheckDeterministic) { $args += "--check-deterministic" }

Write-Host "Running: cargo $($args -join ' ')"
& cargo @args
$code = $LASTEXITCODE
if ($code -ne 0) {
    if ($SkipVerify -and (Test-Path $Output)) {
        $evidence = Get-Content -Raw $Output | ConvertFrom-Json
        $st = [string]$evidence.status
        # Generate-only runs are Incomplete by policy (not Verified).
        if ($st -eq "incomplete" -or $st -eq "Incomplete") {
            Write-Host "Suite evidence: $Output (status=$st; skip-verify Incomplete != Verified)"
            Get-Content $Output | Select-Object -First 40
            exit 0
        }
    }
    throw "vaa suite run failed with exit $code"
}
Write-Host "Suite evidence: $Output"
Get-Content $Output | Select-Object -First 40
