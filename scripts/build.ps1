$ErrorActionPreference = 'Stop'
$cargoDirectory = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path -LiteralPath $cargoDirectory) { $env:PATH = $cargoDirectory + ';' + $env:PATH }
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    & npm.cmd run tauri -- build --no-bundle
    if ($LASTEXITCODE -ne 0) { throw "Desktop build failed (exit code $LASTEXITCODE)." }
    & (Join-Path $PSScriptRoot 'package-release.ps1')
} finally { Pop-Location }
