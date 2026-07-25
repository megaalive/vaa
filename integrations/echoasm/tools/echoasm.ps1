# EchoAsm generator: copy locked input bytes to the candidate output.
# Usage: powershell -File echoasm.ps1 <input> <output>
param(
    [Parameter(Mandatory = $true, Position = 0)][string]$InputPath,
    [Parameter(Mandatory = $true, Position = 1)][string]$OutputPath
)
$ErrorActionPreference = "Stop"
if (-not (Test-Path -LiteralPath $InputPath)) {
    Write-Error "echoasm: input not found: $InputPath"
    exit 1
}
$outDir = Split-Path -Parent $OutputPath
if ($outDir -and -not (Test-Path -LiteralPath $outDir)) {
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}
Copy-Item -LiteralPath $InputPath -Destination $OutputPath -Force
Write-Host "echoasm: wrote $OutputPath"
exit 0
