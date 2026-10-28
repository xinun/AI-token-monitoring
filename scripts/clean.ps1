[CmdletBinding(SupportsShouldProcess = $true)]
param([switch]$DebugOnly)

$ErrorActionPreference = 'Stop'
$projectRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$rootItem = Get-Item -LiteralPath $projectRoot -Force
if (($rootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw 'Refusing to clean a linked project directory.'
}
$rootPrefix = $projectRoot.TrimEnd('\') + '\'
$cacheRelativePath = if ($DebugOnly) { 'src-tauri/target/debug' } else { 'src-tauri/target' }
$targetPrefix = [System.IO.Path]::GetFullPath((Join-Path $projectRoot $cacheRelativePath)).TrimEnd('\') + '\'
$runningFromTarget = @(Get-Process -Name 'ai-token' -ErrorAction SilentlyContinue | Where-Object {
    $_.Path -and $_.Path.StartsWith($targetPrefix, [System.StringComparison]::OrdinalIgnoreCase)
})
if ($runningFromTarget.Count -gt 0) {
    throw 'Close AI Token running from src-tauri/target before cleaning. The release folder is preserved.'
}

$targets = @()
$relativePaths = if ($DebugOnly) { @('src-tauri/target/debug') } else { @('.tools', 'dist', 'src-tauri/target', 'src-tauri/gen/schemas') }
foreach ($relativePath in $relativePaths) {
    $targetPath = [System.IO.Path]::GetFullPath((Join-Path $projectRoot $relativePath))
    if (-not $targetPath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw ('Cleanup path outside project: ' + $targetPath)
    }
    if (-not (Test-Path -LiteralPath $targetPath)) { continue }
    $resolvedPath = (Resolve-Path -LiteralPath $targetPath).ProviderPath
    if (-not $resolvedPath.Equals($targetPath, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw ('Unexpected resolved path: ' + $resolvedPath)
    }
    $targetItem = Get-Item -LiteralPath $targetPath -Force
    $linkedChildren = @(Get-ChildItem -LiteralPath $targetPath -Recurse -Force | Where-Object {
        ($_.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0
    })
    if (($targetItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0 -or $linkedChildren.Count -gt 0) {
        throw ('Refusing recursive cleanup of linked paths: ' + $targetPath)
    }
    $targets += $targetPath
}
foreach ($targetPath in $targets) {
    if ($PSCmdlet.ShouldProcess($targetPath, 'Remove generated files and caches')) {
        Remove-Item -LiteralPath $targetPath -Recurse -Force
        Write-Output ('Removed: ' + $targetPath)
    }
}
