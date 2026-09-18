[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$repositoryRoot = $PSScriptRoot
$frontendDirectory = Join-Path $repositoryRoot 'frontend'
$backendManifest = Join-Path $repositoryRoot 'backend\Cargo.toml'
$tauriManifest = Join-Path $frontendDirectory 'src-tauri\Cargo.toml'

function Resolve-ApplicationPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name
    )

    $commandInfo = Get-Command $Name `
        -CommandType Application `
        -ErrorAction Stop |
        Select-Object -First 1

    if ($null -eq $commandInfo) {
        throw "Required application was not found: $Name"
    }

    return [string]$commandInfo.Source
}

function Invoke-VerificationStep {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,

        [Parameter(Mandatory = $true)]
        [string]$Command,

        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,

        [Parameter(Mandatory = $true)]
        [string]$WorkingDirectory
    )

    Write-Host "[VERIFY] $Name" -ForegroundColor Cyan

    Push-Location -LiteralPath $WorkingDirectory
    try {
        & $Command @Arguments
        $exitCode = $LASTEXITCODE
    }
    finally {
        Pop-Location
    }

    if ($exitCode -ne 0) {
        throw "$Name failed with exit code $exitCode."
    }
}

if (-not (Test-Path -LiteralPath (Join-Path $frontendDirectory 'package.json') -PathType Leaf)) {
    throw 'The frontend package.json was not found.'
}

foreach ($manifestPath in @($backendManifest, $tauriManifest)) {
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        throw "Cargo manifest was not found: $manifestPath"
    }
}

$npmCommand = Resolve-ApplicationPath -Name 'npm.cmd'
$cargoCommand = Resolve-ApplicationPath -Name 'cargo.exe'

Write-Host "[TOOL] npm: $npmCommand" -ForegroundColor DarkGray
Write-Host "[TOOL] cargo: $cargoCommand" -ForegroundColor DarkGray

Invoke-VerificationStep `
    -Name 'Frontend tests' `
    -Command $npmCommand `
    -Arguments @('test') `
    -WorkingDirectory $frontendDirectory

Invoke-VerificationStep `
    -Name 'Svelte and TypeScript checks' `
    -Command $npmCommand `
    -Arguments @('run', 'check') `
    -WorkingDirectory $frontendDirectory

Invoke-VerificationStep `
    -Name 'Frontend production build' `
    -Command $npmCommand `
    -Arguments @('run', 'build') `
    -WorkingDirectory $frontendDirectory

Invoke-VerificationStep `
    -Name 'Backend Rust tests' `
    -Command $cargoCommand `
    -Arguments @('test', '--manifest-path', $backendManifest) `
    -WorkingDirectory $repositoryRoot

Invoke-VerificationStep `
    -Name 'Tauri bridge Rust tests' `
    -Command $cargoCommand `
    -Arguments @('test', '--manifest-path', $tauriManifest) `
    -WorkingDirectory $repositoryRoot

Invoke-VerificationStep `
    -Name 'Tauri release build without bundling' `
    -Command $npmCommand `
    -Arguments @('run', 'tauri:build', '--', '--no-bundle') `
    -WorkingDirectory $frontendDirectory

Write-Host '[PASS] Full automated verification completed.' -ForegroundColor Green
