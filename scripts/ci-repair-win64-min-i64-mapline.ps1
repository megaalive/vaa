# Win64 live-repair path: map-line join → generator_source.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$fixture = "fixtures/repair/hlax64-min-i64-win64-live"
$outDir = "target/repair-win64-mapline"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$packet = Join-Path $outDir "repair-packet.json"

$asm = Join-Path $fixture "candidate.asm"
$hash = (Get-FileHash $asm -Algorithm SHA256).Hash.ToLower()
$digest = "sha256:$hash"

cargo run -q -- repair export `
  --spec integrations/hlax64/generator.spec.toml `
  --task-id min-i64-win64-live-v1 `
  --status BehaviorFailed `
  --message "SemASM behavior_failed: signed min_i64 returned max (locked wrong GreaterThan branch / jg)" `
  --diagnostic-code BEHAVIOR_VECTOR_MISMATCH_001 `
  --map-line 19 `
  --artifact candidate.asm `
  --artifact-digest $digest `
  --map (Join-Path $fixture "candidate.map.json") `
  --regenerate-command "./scripts/run-hlax64-suite.ps1 -Gate -Suite integrations/hlax64/suites/scalar-i64-win64.vaa-suite.toml" `
  --verify-command "dotnet test tests/HlaX64.Compiler.Tests/HlaX64.Compiler.Tests.csproj -c Release" `
  --output $packet

if ($LASTEXITCODE -ne 0) {
    throw "repair export exited $LASTEXITCODE"
}

$pkt = Get-Content -Raw $packet | ConvertFrom-Json
$gs = [string]$pkt.source_mapping.generator_source
$expected = "src/HlaX64.Compiler/Abi/WindowsMsAbiLowerer.cs:442"
if ($gs -ne $expected) {
    throw "generator_source mismatch: got '$gs', expected '$expected'"
}
$gi = [string]$pkt.source_mapping.generator_input
if ($gi -notmatch "min_i64_wrong") {
    throw "expected generator_input to reference min_i64_wrong, got '$gi'"
}

# Assert against the committed golden packet (do not rewrite fixtures in CI).
$golden = Join-Path $fixture "repair-packet.json"
if (-not (Test-Path $golden)) {
    Copy-Item -Force $packet $golden
}
$gold = Get-Content -Raw $golden | ConvertFrom-Json
if ([string]$gold.source_mapping.generator_source -ne $expected) {
    throw "golden packet generator_source drift"
}
cargo run -q -- repair verify $golden
if ($LASTEXITCODE -ne 0) {
    throw "repair verify failed"
}
Write-Host "OK: Win64 map-line repair join filled generator_source ($gs)"
