# A1: ephemeral Ed25519 seal signing key for Gate CI.
# Not a trust root — practice key only; regenerated each job.
param(
    [Parameter(Mandatory = $false)]
    [string]$PublicKeyOut = ""
)

$ErrorActionPreference = "Stop"

$tempRoot = $env:RUNNER_TEMP
if (-not $tempRoot) {
    $tempRoot = [System.IO.Path]::GetTempPath()
}
$seedPath = Join-Path $tempRoot "vaa-ci-seal.seed"

cargo build -q
$vaa = Join-Path (Get-Location) "target\debug\vaa.exe"
if (-not (Test-Path $vaa)) {
    $vaa = Join-Path (Get-Location) "target\debug\vaa"
}
if (-not (Test-Path $vaa)) {
    throw "vaa binary missing after cargo build"
}

$out = & $vaa evidence keygen-seal --out $seedPath 2>&1 | Out-String
if ($LASTEXITCODE -ne 0) {
    throw "keygen-seal failed: $out"
}

$pkHex = $null
$pkB64 = $null
foreach ($line in ($out -split "`r?`n")) {
    if ($line -match '^\s*public_key_hex:\s*(\S+)\s*$') {
        $pkHex = $Matches[1]
    }
    elseif ($line -match '^\s*public_key_b64:\s*(\S+)\s*$') {
        $pkB64 = $Matches[1]
    }
}
if (-not $pkHex -or -not $pkB64) {
    throw "keygen-seal did not print public_key_hex/b64; output=$out"
}

$env:VAA_SEAL_SIGNING_KEY = $seedPath
$env:VAA_REQUIRE_SEAL_SIGNATURE = "1"

# Persist for subsequent GHA steps when present.
if ($env:GITHUB_ENV) {
    Add-Content -Path $env:GITHUB_ENV -Value "VAA_SEAL_SIGNING_KEY=$seedPath"
    Add-Content -Path $env:GITHUB_ENV -Value "VAA_REQUIRE_SEAL_SIGNATURE=1"
}

if ($PublicKeyOut) {
    $parent = Split-Path -Parent $PublicKeyOut
    if ($parent) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    @"
# Ephemeral CI practice key — not a trust root / not Rekor / not HSM.
public_key_hex=$pkHex
public_key_b64=$pkB64
"@ | Set-Content -Path $PublicKeyOut -Encoding utf8
}

Write-Host "A1 seal sign setup OK (ephemeral CI practice key, not a trust root)"
Write-Host "  VAA_SEAL_SIGNING_KEY=$seedPath"
Write-Host "  VAA_REQUIRE_SEAL_SIGNATURE=1"
Write-Host "  public_key_hex=$pkHex"
