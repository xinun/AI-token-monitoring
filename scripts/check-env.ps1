$ErrorActionPreference = 'Continue'
foreach ($tool in @('node', 'npm.cmd', 'cargo', 'rustc', 'codex')) {
    $command = Get-Command $tool -ErrorAction SilentlyContinue
    if ($command) { Write-Output ('OK: ' + $tool + ' - ' + $command.Source) }
    else { Write-Output ('MISSING: ' + $tool) }
}
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (Test-Path -LiteralPath $vswhere) { & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath }
else { Write-Output 'MISSING: Visual Studio C++ Build Tools' }
