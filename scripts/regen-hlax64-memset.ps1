#Requires -Version 5.1
<#
.SYNOPSIS
  Regenerate fixtures/ingest/hlax64_memset/candidate.asm from HlaX64.

.DESCRIPTION
  Requires HLAX64_ROOT (default: sibling ../hlax64) and either `hla64` on PATH
  or the HlaX64.Cli project under that root. Fail-closed on emit errors.
#>

$ErrorActionPreference = "Stop"

$vaaRoot = Split-Path -Parent $PSScriptRoot
$hlaxRoot = if ($env:HLAX64_ROOT) { $env:HLAX64_ROOT } else { Join-Path (Split-Path $vaaRoot -Parent) "hlax64" }
$source = Join-Path $hlaxRoot "examples/interop/semasm-vaa/memset/memset.hla64"
$outAsm = Join-Path $vaaRoot "fixtures/ingest/hlax64_memset/candidate.asm"

if (-not (Test-Path $source)) {
    Write-Error "HlaX64 source not found: $source (set HLAX64_ROOT)"
}

$hla64 = Get-Command hla64 -ErrorAction SilentlyContinue
$tmp = [System.IO.Path]::GetTempFileName() + ".asm"

try {
    if ($hla64) {
        & $hla64.Source emit-nasm $source --target windows-x64-msabi --output-kind shared-library -o $tmp
        if ($LASTEXITCODE -ne 0) { throw "hla64 emit-nasm failed with exit $LASTEXITCODE" }
    } else {
        $cli = Join-Path $hlaxRoot "src/HlaX64.Cli/HlaX64.Cli.csproj"
        if (-not (Test-Path $cli)) {
            Write-Error "hla64 not on PATH and CLI project missing: $cli"
        }
        dotnet run --project $cli --no-launch-profile -- emit-nasm $source `
            --target windows-x64-msabi --output-kind shared-library -o $tmp
        if ($LASTEXITCODE -ne 0) { throw "dotnet emit-nasm failed with exit $LASTEXITCODE" }
    }

    $text = Get-Content -Raw $tmp
    if ($text -notmatch "global memset") {
        throw "emit output missing global memset"
    }
    if ($text -match "global _start" -or $text -match "ExitProcess") {
        throw "emit output still contains program entry; use --output-kind shared-library"
    }
    if ($text -match "mov rax, -1" -or $text -match "mov rax,-1") {
        throw "emit contains model-hostile mov rax,-1; use sub after zero"
    }

    $outDir = Split-Path $outAsm -Parent
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
    Copy-Item -Force $tmp $outAsm
    Write-Host "Wrote $outAsm"
}
finally {
    Remove-Item -Force $tmp -ErrorAction SilentlyContinue
}
