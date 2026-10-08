$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$keyPath = Join-Path $env:LOCALAPPDATA 'AI Token Build/updater.key'
$publicKeyPath = $keyPath + '.pub'
if (-not (Test-Path -LiteralPath $keyPath -PathType Leaf) -or -not (Test-Path -LiteralPath $publicKeyPath -PathType Leaf)) {
    throw 'Updater signing key missing. Run npm run updater:key once on this build PC; keep that key for all future releases.'
}

$configuration = Get-Content -LiteralPath (Join-Path $projectRoot 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
$publicKey = (Get-Content -LiteralPath $publicKeyPath -Raw).Trim()
if ($configuration.plugins.updater.pubkey -ne $publicKey) {
    throw 'The local updater public key does not match src-tauri/tauri.conf.json. Configure the matching public key before building.'
}
if ($configuration.bundle.createUpdaterArtifacts -ne $true -or $configuration.bundle.windows.nsis.installMode -ne 'currentUser' -or $configuration.plugins.updater.requireSignedVersion -ne $true) {
    throw 'Release builds require signed updater artifacts and a currentUser NSIS installer.'
}

$previousPath = $env:PATH
$previousSigningKey = [Environment]::GetEnvironmentVariable('TAURI_SIGNING_PRIVATE_KEY', 'Process')
$previousSigningPassword = [Environment]::GetEnvironmentVariable('TAURI_SIGNING_PRIVATE_KEY_PASSWORD', 'Process')
$cargoDirectory = Join-Path $env:USERPROFILE '.cargo/bin'
Push-Location $projectRoot
try {
    if (Test-Path -LiteralPath $cargoDirectory) { $env:PATH = $cargoDirectory + ';' + $env:PATH }
    # Pass the path, never the key contents, to the build process.
    $env:TAURI_SIGNING_PRIVATE_KEY = $keyPath
    & npm.cmd run tauri -- build --bundles nsis --ci
    if ($LASTEXITCODE -ne 0) { throw "Desktop installer build failed (exit code $LASTEXITCODE)." }
    & (Join-Path $PSScriptRoot 'package-release.ps1')
} finally {
    [Environment]::SetEnvironmentVariable('TAURI_SIGNING_PRIVATE_KEY', $previousSigningKey, 'Process')
    [Environment]::SetEnvironmentVariable('TAURI_SIGNING_PRIVATE_KEY_PASSWORD', $previousSigningPassword, 'Process')
    $env:PATH = $previousPath
    Pop-Location
}
