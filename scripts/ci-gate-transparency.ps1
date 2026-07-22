# T1: export + verify-transparency for a Gate run directory (fail-closed).
param(
    [Parameter(Mandatory = $true)]
    [string]$RunBase,

    [Parameter(Mandatory = $true)]
    [string]$OutDir,

    [Parameter(Mandatory = $false)]
    [string]$PublicKeySrc = ""
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $RunBase)) {
    throw "run base missing: $RunBase"
}

$run = Get-ChildItem $RunBase -Directory -ErrorAction Stop |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
if (-not $run) {
    throw "no run under $RunBase"
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

cargo build -q
$vaa = Join-Path (Get-Location) "target\debug\vaa.exe"
if (-not (Test-Path $vaa)) {
    $vaa = Join-Path (Get-Location) "target\debug\vaa"
}
if (-not (Test-Path $vaa)) {
    throw "vaa binary missing after cargo build"
}

$transparency = Join-Path $OutDir "transparency.json"
& $vaa evidence export-transparency $run.FullName -o $transparency
if ($LASTEXITCODE -ne 0) {
    throw "export-transparency failed for $($run.FullName)"
}

$sealLog = Join-Path $run.FullName "evidence\seal-log.jsonl"
if (-not (Test-Path $sealLog)) {
    throw "seal-log.jsonl missing under $($run.FullName)"
}
Copy-Item $sealLog (Join-Path $OutDir "seal-log.jsonl")

if ($PublicKeySrc -and (Test-Path $PublicKeySrc)) {
    $destPk = Join-Path $OutDir "public_key.txt"
    $srcFull = (Resolve-Path $PublicKeySrc).Path
    $destParent = Split-Path -Parent $destPk
    if (-not (Test-Path $destParent)) {
        New-Item -ItemType Directory -Force -Path $destParent | Out-Null
    }
    # Skip when already written into OutDir by sign-setup.
    if ($srcFull -ne (Join-Path (Resolve-Path $OutDir).Path "public_key.txt")) {
        Copy-Item $PublicKeySrc $destPk -Force
    }
}

& $vaa evidence verify-transparency $transparency --against $run.FullName
if ($LASTEXITCODE -ne 0) {
    throw "verify-transparency failed against $($run.FullName)"
}

Write-Host "T1 transparency OK from $($run.FullName) → $OutDir"
Write-Host "  (CI artifact ≠ remote immutable log)"
