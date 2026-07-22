# R6: after SemASM is on PATH, assert `vaa doctor` is Available and Gate
# golden live_probe compares are aligned (agent gate-usable vs embedded snapshot).
$ErrorActionPreference = "Stop"

cargo build -q
$vaa = Join-Path (Get-Location) "target\debug\vaa.exe"
if (-not (Test-Path $vaa)) {
    $vaa = Join-Path (Get-Location) "target\debug\vaa"
}
if (-not (Test-Path $vaa)) {
    throw "vaa binary missing after cargo build"
}

$raw = & $vaa doctor --format json
if ($LASTEXITCODE -ne 0) {
    throw "vaa doctor exited $LASTEXITCODE; stdout=$raw"
}

$doc = $raw | ConvertFrom-Json
if ($doc.status -ne "Available") {
    throw "expected doctor Available, got $($doc.status); details=$($doc.details | ConvertTo-Json -Compress)"
}
if (-not $doc.live_probe) {
    throw "doctor JSON missing live_probe"
}
if ($doc.live_probe.capability_schema -ne "0.1") {
    throw "expected capability_schema 0.1, got $($doc.live_probe.capability_schema)"
}

foreach ($target in @("x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu")) {
    $cmp = @($doc.live_probe.compares | Where-Object { $_.target_id -eq $target })
    if ($cmp.Count -ne 1) {
        throw "expected one live_probe compare for $target"
    }
    if ($cmp[0].outcome -ne "aligned") {
        throw "$target compare=$($cmp[0].outcome) axes=$($cmp[0].axes | ConvertTo-Json -Compress)"
    }
}

Write-Host "R6 doctor live_probe OK (Available + Gate goldens aligned)"
