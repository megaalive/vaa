#Requires -Version 5.1
<#
.SYNOPSIS
  Rebuild locked EchoAsm repair patch-evidence fixture (Milestone 6).

.DESCRIPTION
  Simulates a generator repair on the EchoAsm pack:
  1) broken generator prepends a hostile marker (suite would reject / fail digest)
  2) patched generator restores clean copy (allowed path under tools/**)
  3) build suite evidence (accepted via generation identity smoke) + patch evidence
  4) verify patch evidence from the written JSON

  Honesty: this is an EchoAsm universality repair smoke — not a live HlaX64
  backend defect fix, and not SemASM Gate Verified.
#>
param(
    [string]$OutDir = "fixtures/repair/echoasm-passthrough"
)
$ErrorActionPreference = "Stop"
$vaaRoot = Split-Path -Parent $PSScriptRoot
Set-Location $vaaRoot

$fixture = Join-Path $vaaRoot $OutDir
New-Item -ItemType Directory -Force -Path $fixture | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $fixture "base") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $fixture "patched") | Out-Null

$toolsCmd = Join-Path $vaaRoot "integrations\echoasm\tools\echoasm.cmd"
$good = Get-Content -Raw $toolsCmd

$broken = @"
@echo off
REM BROKEN echoasm — prepends a hostile marker (repair demo base).
if "%~1"=="" exit /b 2
if "%~2"=="" exit /b 2
echo ; BROKEN > "%~2"
type "%~1" >> "%~2"
exit /b 0
"@

Set-Content -Encoding ascii (Join-Path $fixture "base\echoasm.cmd") $broken
Set-Content -Encoding ascii (Join-Path $fixture "patched\echoasm.cmd") $good

# Unified diff for patch_digest
$diffPath = Join-Path $fixture "repair.patch"
$diff = @"
--- a/integrations/echoasm/tools/echoasm.cmd
+++ b/integrations/echoasm/tools/echoasm.cmd
@@
-REM BROKEN echoasm — prepends a hostile marker (repair demo base).
-echo ; BROKEN > "%~2"
-type "%~1" >> "%~2"
+REM EchoAsm generator (cmd twin): copy locked input bytes to candidate output.
+copy /Y "%~1" "%~2" >nul
"@
Set-Content -Encoding ascii $diffPath $diff

# Generate candidate with patched tool to prove clean digest match.
$gen = (Resolve-Path "integrations\echoasm\tools\echoasm.cmd").Path
$in = (Resolve-Path "integrations\echoasm\cases\passthrough\input.asm").Path
$outDir = Join-Path $vaaRoot "target\repair-echoasm"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$cand = Join-Path $outDir "candidate.asm"
& cargo run -q -- generator generate `
    integrations/echoasm/generator.spec.toml `
    --generator $gen `
    --input $in `
    --output $cand `
    --target x86_64-pc-windows-msvc `
    --check-deterministic
if ($LASTEXITCODE -ne 0) { throw "generate failed" }

$binDigest = (Get-FileHash $gen -Algorithm SHA256).Hash.ToLowerInvariant()
$binDigest = "sha256:$binDigest"
$patchBytes = [System.IO.File]::ReadAllBytes($diffPath)
$patchDigest = "sha256:" + ([System.BitConverter]::ToString(
    [System.Security.Cryptography.SHA256]::Create().ComputeHash($patchBytes)
)).Replace("-", "").ToLowerInvariant()

$candDigest = "sha256:" + (Get-FileHash $cand -Algorithm SHA256).Hash.ToLowerInvariant()
$suitePath = Join-Path $fixture "suite-evidence.accepted.json"
$suiteJson = @"
{
  "schema_version": "0.1",
  "suite_id": "echoasm.smoke.v0",
  "suite_digest": "sha256:978ba97b4ef40429580265edd0e36b1ca589cba431d04744f89b15b937dcf44f",
  "status": "accepted",
  "generator_binary_digest": "$binDigest",
  "cases": [
    {
      "case_id": "passthrough",
      "case_dir": "integrations/echoasm/cases/passthrough",
      "status": "accepted",
      "candidate_digest": "$candDigest"
    }
  ]
}
"@
$utf8 = New-Object System.Text.UTF8Encoding $false
[System.IO.File]::WriteAllText($suitePath, $suiteJson, $utf8)

# Build patch evidence via CLI
$patchOut = Join-Path $fixture "patch-evidence.json"
& cargo run -q -- patch evidence-build `
    --suite-evidence $suitePath `
    --base "git:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" `
    --patched "git:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" `
    --generator-binary-digest $binDigest `
    --changed "integrations/echoasm/tools/echoasm.cmd" `
    --spec "integrations/echoasm/generator.spec.toml" `
    --output $patchOut
if ($LASTEXITCODE -ne 0) { throw "patch evidence-build failed" }

& cargo run -q -- patch evidence-verify $patchOut
if ($LASTEXITCODE -ne 0) { throw "patch evidence-verify failed" }

# Forbidden-path negative fixture
$negOut = Join-Path $fixture "patch-evidence.forbidden-failed.json"
& cargo run -q -- patch evidence-build `
    --suite-evidence $suitePath `
    --base "git:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" `
    --patched "git:cccccccccccccccccccccccccccccccccccccccc" `
    --generator-binary-digest $binDigest `
    --changed "integrations/echoasm/stack.lock.toml" `
    --spec "integrations/echoasm/generator.spec.toml" `
    --output $negOut
# Non-zero exit expected when status is Failed.
if (-not (Test-Path $negOut)) { throw "forbidden patch evidence was not written" }

Write-Host "Wrote repair fixtures under $fixture"
Write-Host "  patch digest (file bytes): $patchDigest"
