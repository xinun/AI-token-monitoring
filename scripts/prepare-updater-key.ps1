$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) { throw 'LOCALAPPDATA is required on the Windows build PC.' }
$keyDirectory = Join-Path $env:LOCALAPPDATA 'AI Token Build'
$keyPath = Join-Path $keyDirectory 'updater.key'
$publicKeyPath = $keyPath + '.pub'
$hasPrivateKey = Test-Path -LiteralPath $keyPath -PathType Leaf
$hasPublicKey = Test-Path -LiteralPath $publicKeyPath -PathType Leaf
if ($hasPrivateKey -or $hasPublicKey) {
    if (-not ($hasPrivateKey -and $hasPublicKey)) {
        throw 'Only part of the updater key pair exists. Restore the matching key pair from your private backup; this script will not overwrite or replace it.'
    }
    Write-Output "Existing updater key preserved. Public key: $publicKeyPath"
    exit 0
}

New-Item -ItemType Directory -Path $keyDirectory -Force | Out-Null
if ((Get-Item -LiteralPath $keyDirectory).Attributes -band [IO.FileAttributes]::ReparsePoint) {
    throw 'The updater key directory must be a local directory, not a link.'
}
# Restrict this build-only directory before writing the unencrypted signing key.
$identity = [Security.Principal.WindowsIdentity]::GetCurrent().User
$acl = New-Object Security.AccessControl.DirectorySecurity
$acl.SetOwner($identity)
$acl.SetAccessRuleProtection($true, $false)
$rule = New-Object Security.AccessControl.FileSystemAccessRule($identity, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
$acl.AddAccessRule($rule)
# Use the .NET Framework API directly so an inherited PowerShell 7 module path
# cannot break the Windows PowerShell 5.1 Security module import.
[IO.Directory]::SetAccessControl($keyDirectory, $acl)

Push-Location $projectRoot
try {
    # -w is mandatory: without it the CLI prints the private key. Keep all CLI output captured as an additional precaution.
    $ErrorActionPreference = 'Continue'
    $generationOutput = & npm.cmd run tauri -- signer generate --ci --write-keys $keyPath 2>&1
    $generationExitCode = $LASTEXITCODE
    $generationOutput = $null
    $ErrorActionPreference = 'Stop'
    if ($generationExitCode -ne 0) { throw "Updater key generation failed (exit code $generationExitCode); CLI output was withheld to protect private material." }
} finally {
    $ErrorActionPreference = 'Stop'
    Pop-Location
}
if (-not (Test-Path -LiteralPath $keyPath -PathType Leaf) -or -not (Test-Path -LiteralPath $publicKeyPath -PathType Leaf)) {
    throw 'The CLI did not produce both updater key files.'
}
Write-Output "Updater key created in this PC's private build directory. Public key: $publicKeyPath"
Write-Output 'Keep a private backup of updater.key; never upload it to GitHub or put it in release assets.'
