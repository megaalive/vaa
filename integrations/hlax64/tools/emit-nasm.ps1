#Requires -Version 5.1
<#
.SYNOPSIS
  Emit NASM for one HlaX64 pack case input (locked generator wrapper).

.DESCRIPTION
  Resolves HLAX64_ROOT and runs emit-nasm with Win64 shared-library settings
  used by the SemASM/VAA bridge. Honesty: emit ≠ SemASM verified.
#>
param(
    [Parameter(Mandatory = $true, Position = 0)][string]$InputPath,
    [Parameter(Mandatory = $true, Position = 1)][string]$OutputPath,
    [Parameter(Mandatory = $false)][string]$Target = "windows-x64-msabi"
)
$ErrorActionPreference = "Stop"

$packRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$vaaRoot = (Resolve-Path (Join-Path $packRoot "..\..")).Path

function Resolve-HlaX64Root {
    if ($env:HLAX64_ROOT -and (Test-Path (Join-Path $env:HLAX64_ROOT "src\HlaX64.Cli\HlaX64.Cli.csproj"))) {
        return $env:HLAX64_ROOT
    }
    foreach ($candidate in @(
        (Join-Path $vaaRoot "hlax64"),
        (Join-Path (Split-Path $vaaRoot -Parent) "hlax64")
    )) {
        if (Test-Path (Join-Path $candidate "src\HlaX64.Cli\HlaX64.Cli.csproj")) {
            return $candidate
        }
    }
    return $null
}

$hlaxRoot = Resolve-HlaX64Root
if (-not $hlaxRoot) {
    Write-Error "HlaX64 root not found (set HLAX64_ROOT to a checkout with src/HlaX64.Cli)"
}

$cli = Join-Path $hlaxRoot "src\HlaX64.Cli\HlaX64.Cli.csproj"
if (-not (Test-Path -LiteralPath $InputPath)) {
    Write-Error "input not found: $InputPath"
}

$outDir = Split-Path -Parent $OutputPath
if ($outDir -and -not (Test-Path -LiteralPath $outDir)) {
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

$hla64 = Get-Command hla64 -ErrorAction SilentlyContinue
if ($hla64) {
    & $hla64.Source emit-nasm $InputPath --target $Target --output-kind shared-library -o $OutputPath
    if ($LASTEXITCODE -ne 0) { throw "hla64 emit-nasm failed: $LASTEXITCODE" }
} else {
    dotnet run --project $cli --no-launch-profile -- emit-nasm `
        $InputPath --target $Target --output-kind shared-library -o $OutputPath
    if ($LASTEXITCODE -ne 0) { throw "dotnet emit-nasm failed: $LASTEXITCODE" }
}

if (-not (Test-Path -LiteralPath $OutputPath)) {
    Write-Error "emit-nasm did not produce $OutputPath"
}
Write-Host "hlax64 emit-nasm: wrote $OutputPath"
