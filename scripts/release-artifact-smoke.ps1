# Smoke a staged VAA release binary against a real SemASM binary on Win64.
#
# This script parses stdout JSON only. Stderr is retained solely for failure
# diagnostics and never influences acceptance.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$VaaBin,

    [Parameter(Mandatory = $true)]
    [string]$SemasmBin,

    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = "Stop"

$vaa = (Resolve-Path -LiteralPath $VaaBin).Path
$semasm = (Resolve-Path -LiteralPath $SemasmBin).Path
$root = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$smokeRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("vaa-release-smoke-" + [guid]::NewGuid().ToString("N"))
$stdoutPath = Join-Path $smokeRoot "stdout.log"
$stderrPath = Join-Path $smokeRoot "stderr.log"
New-Item -ItemType Directory -Force -Path $smokeRoot | Out-Null

function Invoke-VaaJson {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    Remove-Item -Force -LiteralPath $stdoutPath, $stderrPath -ErrorAction SilentlyContinue
    $process = Start-Process `
        -FilePath $vaa `
        -ArgumentList $Arguments `
        -RedirectStandardOutput $stdoutPath `
        -RedirectStandardError $stderrPath `
        -WindowStyle Hidden `
        -Wait `
        -PassThru
    $stdout = (Get-Content -Raw -LiteralPath $stdoutPath).Trim()
    if ($process.ExitCode -ne 0) {
        $stderr = if (Test-Path -LiteralPath $stderrPath) {
            Get-Content -Raw -LiteralPath $stderrPath
        } else {
            ""
        }
        throw "vaa $($Arguments -join ' ') exited $($process.ExitCode); stderr=$stderr"
    }
    if ([string]::IsNullOrWhiteSpace($stdout)) {
        throw "vaa $($Arguments -join ' ') emitted empty stdout"
    }
    try {
        return $stdout | ConvertFrom-Json
    } catch {
        throw "vaa $($Arguments -join ' ') emitted invalid stdout JSON: $stdout"
    }
}

$oldSemasm = $env:SEMASM_BIN
$oldTemp = $env:TEMP
$oldTmp = $env:TMP
try {
    # A dedicated writable temp root makes the release smoke cover the exact
    # subprocess environment variables whose omission broke VAA v0.1.1.
    $env:TEMP = $smokeRoot
    $env:TMP = $smokeRoot

    & $vaa version | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "vaa version failed" }
    & $vaa status | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "vaa status failed" }

    $env:SEMASM_BIN = $semasm
    $doctorFile = Invoke-VaaJson -Arguments @("doctor", "--format", "json")
    if ($doctorFile.status -notin @("Available", "Degraded")) {
        throw "doctor did not accept SEMASM_BIN file: $($doctorFile.status)"
    }

    $env:SEMASM_BIN = Split-Path -Parent $semasm
    $doctorDir = Invoke-VaaJson -Arguments @("doctor", "--format", "json")
    if ($doctorDir.status -notin @("Available", "Degraded")) {
        throw "doctor did not accept SEMASM_BIN directory: $($doctorDir.status)"
    }

    $admission = Invoke-VaaJson -Arguments @("admit", "--list", "--format", "json")
    if (-not $admission.admitted -or $admission.leaves.Count -eq 0) {
        throw "vaa admit --list returned no admitted leaves"
    }

    $env:SEMASM_BIN = $semasm
    $fixture = Join-Path $root "fixtures\semasm\sum_i64"
    $report = Invoke-VaaJson -Arguments @(
        "verify",
        (Join-Path $fixture "sum_i64.vaa.toml"),
        "--source", (Join-Path $fixture "sum_i64_win64.asm"),
        "--contract", (Join-Path $fixture "sum_i64.sem.toml"),
        "--allow-execution",
        "--format", "json"
    )
    if ($report.final_status -ne "VerifiedUnderPreconditions") {
        throw "release smoke expected VerifiedUnderPreconditions, got $($report.final_status)"
    }
    if ($report.verify_report.raw_status -ne "verified_under_preconditions") {
        throw "release smoke did not preserve the SemASM VUP status"
    }

    $sumRange = Join-Path $root "fixtures\semasm\sum_range"
    $bound = Invoke-VaaJson -Arguments @(
        "verify",
        (Join-Path $root "fixtures\tasks\sum_range_win64_0_2.vaa.toml"),
        "--source", (Join-Path $sumRange "sum_range_win64.asm"),
        "--contract", (Join-Path $sumRange "sum_range.sem.toml"),
        "--allow-execution",
        "--format", "json"
    )
    if ($bound.final_status -ne "Verified") {
        throw "schema 0.2 release smoke expected Verified, got $($bound.final_status)"
    }
    $raw = $bound.verify_report.raw_json | ConvertFrom-Json
    if ($raw.schema_version -ne "0.6" -or $raw.vector_set.external_case_count -ne 2) {
        throw "schema 0.2 release smoke did not bind both task vectors"
    }

    Write-Host "VAA staged release smoke passed (VUP preserved; schema 0.2 vectors bound)."
} finally {
    $env:SEMASM_BIN = $oldSemasm
    $env:TEMP = $oldTemp
    $env:TMP = $oldTmp
    Remove-Item -Recurse -Force -LiteralPath $smokeRoot -ErrorAction SilentlyContinue
}
