param(
    [string]$Mode = "serve",
    [string]$SecondArg,
    [string]$ThirdArg
)

$ErrorActionPreference = "Stop"

switch ($Mode) {
    "serve" { }
    "init" { }
    "migrate" { }
    "doctor" { }
    "bootstrap-local" { }
    default {
        [Console]::Error.WriteLine("unsupported mode: $Mode")
        [Console]::Error.WriteLine("usage: pwsh -File .\scripts\agent-llm-mm.ps1 [serve|init|migrate|doctor|bootstrap-local] [config_path]")
        exit 2
    }
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = (Resolve-Path (Join-Path $scriptDir "..")).Path
$doctorMode = $null
$configPath = $null

if ($Mode -eq "doctor") {
    if ($SecondArg -and $SecondArg.StartsWith("--")) {
        $doctorMode = $SecondArg
        $configPath = $ThirdArg
    }
    else {
        $doctorMode = "--read-only"
        $configPath = $SecondArg
        if ($ThirdArg) {
            [Console]::Error.WriteLine("too many arguments for mode: doctor")
            exit 2
        }
    }
    if ($doctorMode -notin @("--read-only", "--allow-bootstrap")) {
        [Console]::Error.WriteLine("unsupported doctor mode: $doctorMode")
        [Console]::Error.WriteLine("usage: pwsh -File .\scripts\agent-llm-mm.ps1 doctor [--read-only|--allow-bootstrap] [config_path]")
        exit 2
    }
}
else {
    $configPath = $SecondArg
    if ($ThirdArg) {
        [Console]::Error.WriteLine("too many arguments for mode: $Mode")
        exit 2
    }
}

Push-Location $projectRoot
try {
    if ($Mode -eq "bootstrap-local") {
        $targetPath = if ($configPath) { $configPath } else { "agent-llm-mm.local.toml" }
        $sourcePath = Join-Path $projectRoot "examples/agent-llm-mm.dev.example.toml"
        $targetParent = Split-Path -Parent $targetPath
        if (-not $targetParent) {
            $targetParent = "."
        }

        if (Test-Path -LiteralPath $targetPath) {
            [Console]::Error.WriteLine("target already exists; refusing to overwrite: $targetPath")
            exit 1
        }
        if (-not (Test-Path -LiteralPath $targetParent -PathType Container)) {
            [Console]::Error.WriteLine("parent directory does not exist: $targetParent")
            exit 1
        }

        try {
            [System.IO.File]::Copy($sourcePath, $targetPath, $false)
        }
        catch [System.IO.IOException] {
            [Console]::Error.WriteLine("target already exists; refusing to overwrite: $targetPath")
            exit 1
        }
        $quotedTargetPath = "'" + ($targetPath -replace "'", "''") + "'"
        Write-Output "created local config: $targetPath"
        Write-Output "Next commands:"
        Write-Output "  pwsh -File .\scripts\agent-llm-mm.ps1 init $quotedTargetPath"
        Write-Output "  pwsh -File .\scripts\agent-llm-mm.ps1 doctor --read-only $quotedTargetPath"
        Write-Output "  pwsh -File .\scripts\agent-llm-mm.ps1 serve $quotedTargetPath"
        exit 0
    }

    if ($configPath) {
        $resolvedConfigPath = (Resolve-Path $configPath).Path
        $env:AGENT_LLM_MM_CONFIG = $resolvedConfigPath
    }

    if ($Mode -eq "doctor") {
        & cargo run --quiet --bin agent_llm_mm -- $Mode $doctorMode
    }
    else {
        & cargo run --quiet --bin agent_llm_mm -- $Mode
    }
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
