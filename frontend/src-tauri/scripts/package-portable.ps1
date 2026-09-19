[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ExecutablePath
)

$ErrorActionPreference = 'Stop'

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$configPath = Join-Path $repositoryRoot 'frontend\src-tauri\tauri.conf.json'
$licensePath = Join-Path $repositoryRoot 'LICENSE'
$readmePath = Join-Path $repositoryRoot 'frontend\src-tauri\portable\README.txt'
$resolvedExecutablePath = (Resolve-Path -LiteralPath $ExecutablePath).Path

foreach ($requiredPath in @($configPath, $licensePath, $readmePath)) {
    if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
        throw "Required portable package file was not found: $requiredPath"
    }
}

$tauriConfig = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
$version = [string]$tauriConfig.version

if ([string]::IsNullOrWhiteSpace($version)) {
    throw 'The application version is missing from tauri.conf.json.'
}

$releaseDirectory = Join-Path $repositoryRoot 'release'
$archiveName = "Codex-Session-Observatory_${version}_windows-x64-portable.zip"
$archivePath = Join-Path $releaseDirectory $archiveName

New-Item -ItemType Directory -Path $releaseDirectory -Force | Out-Null

Compress-Archive `
    -LiteralPath @($resolvedExecutablePath, $licensePath, $readmePath) `
    -DestinationPath $archivePath `
    -CompressionLevel Optimal `
    -Force

Write-Host "Portable archive: $archivePath"
