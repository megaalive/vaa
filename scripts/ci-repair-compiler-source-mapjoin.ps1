# Repair E2E: assert repair export joins compiler_source → generator_source.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$outDir = "target/repair-mapjoin"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$packet = Join-Path $outDir "repair-packet.json"

cargo run -q -- repair export `
  --spec integrations/echoasm/generator.spec.toml `
  --task-id compiler-source-mapjoin-v1 `
  --status Violated `
  --message "map-join e2e: join compiler_source into repair packet" `
  --instruction-offset 0x10 `
  --artifact candidate.asm `
  --artifact-digest "sha256:0000000000000000000000000000000000000000000000000000000000000000" `
  --map fixtures/repair/compiler-source-mapjoin/candidate.map.json `
  --regenerate-command "echo regenerate" `
  --verify-command "echo verify" `
  --output $packet

if ($LASTEXITCODE -ne 0) {
    throw "repair export exited $LASTEXITCODE"
}

$pkt = Get-Content -Raw $packet | ConvertFrom-Json
$gs = [string]$pkt.source_mapping.generator_source
$expected = "src/HlaX64.Compiler/Abi/Win64AbiLowerer.cs:214"
if (-not $gs) {
    throw "expected source_mapping.generator_source from map join; got none"
}
if ($gs -ne $expected) {
    throw "generator_source mismatch: got '$gs', expected '$expected'"
}
Write-Host "OK: repair export map-join filled generator_source ($gs)"
