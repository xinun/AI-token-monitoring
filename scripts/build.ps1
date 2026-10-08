$ErrorActionPreference = 'Stop'
$cargoDirectory = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path -LiteralPath $cargoDirectory) { $env:PATH = $cargoDirectory + ';' + $env:PATH }
Push-Location (Split-Path -Parent $PSScriptRoot)
try { & npm.cmd run tauri -- build --no-bundle } finally { Pop-Location }
