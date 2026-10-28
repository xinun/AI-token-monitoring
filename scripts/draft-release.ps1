[CmdletBinding()]
param([switch]$Preview, [switch]$WithoutPortable)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$repository = 'xinun/AI-token-monitoring'
$version = [string]([IO.File]::ReadAllText((Join-Path $projectRoot 'package.json')) | ConvertFrom-Json).version
if ($version -notmatch '^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$') {
    throw 'Draft publishing requires a stable version such as 0.3.1 in package.json.'
}
$tag = 'v' + $version
$releaseRoot = Join-Path $projectRoot ('release/' + $version)
$stem = 'AI-Token-' + $version + '-windows-x64'
$installerName = $stem + '-setup.exe'
$installer = Join-Path $releaseRoot $installerName
$manifestPath = Join-Path $releaseRoot 'latest.json'
$assets = @($installer, $manifestPath)
if (-not $WithoutPortable) { $assets += Join-Path $releaseRoot ($stem + '.zip') }
foreach ($asset in $assets) {
    if (-not (Test-Path -LiteralPath $asset -PathType Leaf) -or (Get-Item -LiteralPath $asset).Length -eq 0) {
        throw "Missing or empty release asset: $asset. Run npm run build:desktop first."
    }
}
$manifest = [IO.File]::ReadAllText($manifestPath) | ConvertFrom-Json
$platform = $manifest.platforms.'windows-x86_64'
$expectedUrl = 'https://github.com/' + $repository + '/releases/download/' + $tag + '/' + $installerName
if ([string]$manifest.version -cne $version -or [string]$platform.url -cne $expectedUrl -or
    [string]::IsNullOrWhiteSpace([string]$platform.signature)) {
    throw 'latest.json must match the package version, fixed repository installer URL, and inline signature.'
}
$changelog = [IO.File]::ReadAllText((Join-Path $projectRoot 'CHANGELOG.md'))
$sectionPattern = '(?ms)^##[ \t]+\[?v?' + [regex]::Escape($version) + '\]?(?:[ \t]+[^\r\n]*)?\r?\n(?<notes>.*?)(?=^##[ \t]+|\z)'
$section = [regex]::Match($changelog, $sectionPattern)
$notes = $section.Groups['notes'].Value.Trim()
if (-not $section.Success -or [string]::IsNullOrWhiteSpace($notes)) {
    throw "CHANGELOG.md must contain a nonempty ## $version section."
}
Write-Output ('Repository: ' + $repository)
Write-Output ('Draft tag: ' + $tag)
Write-Output 'Assets:'
$assets | ForEach-Object { Write-Output ('  ' + $_) }
if ($Preview) {
    Write-Output 'Preview only: no GitHub requests, temporary files, tags, or draft changes.'
    return
}

foreach ($command in @('git', 'gh')) {
    if (-not (Get-Command $command -CommandType Application -ErrorAction SilentlyContinue)) {
        throw "Required command not found: $command. Install it before creating a draft."
    }
}
function Invoke-CheckedGit([string[]]$Arguments) {
    $result = & git -C $projectRoot @Arguments
    if ($LASTEXITCODE -ne 0) { throw 'Git validation failed; no draft was created.' }
    return ($result -join "`n").Trim()
}
$gitRoot = Invoke-CheckedGit -Arguments @('rev-parse', '--show-toplevel')
if (-not [IO.Path]::GetFullPath($gitRoot).Equals([IO.Path]::GetFullPath($projectRoot), [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Run this script from the AI Token repository, not a nested or unrelated folder.'
}
$status = Invoke-CheckedGit -Arguments @('status', '--porcelain=1', '--untracked-files=normal')
if ($status) { throw 'Commit and push all source changes before creating a release draft.' }
$origin = Invoke-CheckedGit -Arguments @('remote', 'get-url', 'origin')
if ($origin -cnotin @('https://github.com/xinun/AI-token-monitoring.git', 'https://github.com/xinun/AI-token-monitoring',
    'git@github.com:xinun/AI-token-monitoring.git', 'ssh://git@github.com/xinun/AI-token-monitoring.git')) {
    throw 'origin must point to the expected xinun/AI-token-monitoring repository.'
}
$head = Invoke-CheckedGit -Arguments @('rev-parse', 'HEAD')
$remoteRefs = Invoke-CheckedGit -Arguments @('ls-remote', 'origin', 'refs/heads/main', "refs/tags/$tag", "refs/tags/$tag^{}")
$refs = @{}
foreach ($line in ($remoteRefs -split "`n")) {
    if ($line -match '^(?<hash>[0-9a-f]{40,64})\s+(?<ref>.+)$') { $refs[$Matches.ref] = $Matches.hash }
}
if (-not $refs.ContainsKey('refs/heads/main') -or $refs['refs/heads/main'] -cne $head) {
    throw 'Local HEAD must equal the current origin/main commit. Push or synchronize main first.'
}
$tagRef = "refs/tags/$tag"
if ($refs.ContainsKey($tagRef)) {
    $tagCommit = $refs[$tagRef]
    if ($refs.ContainsKey($tagRef + '^{}')) { $tagCommit = $refs[$tagRef + '^{}'] }
    if ($tagCommit -cne $head) { throw 'The existing remote version tag points to another commit. Use a new version.' }
}
$existingTags = @(& gh api ('repos/' + $repository + '/releases') --paginate --jq '.[].tag_name')
if ($LASTEXITCODE -ne 0) { throw 'Could not check GitHub releases. Check gh authentication and network access.' }
if ($existingTags -ccontains $tag) { throw "Release $tag already exists. Existing releases will not be overwritten." }

$notesPath = [IO.Path]::Combine([IO.Path]::GetTempPath(), 'ai-token-release-' + [guid]::NewGuid().ToString('N') + '.md')
try {
    [IO.File]::WriteAllText($notesPath, $notes, (New-Object Text.UTF8Encoding($false)))
    & gh release create $tag @assets --repo $repository --target $head --title "AI Token $version" --notes-file $notesPath --draft
    if ($LASTEXITCODE -ne 0) { throw 'Draft creation failed. Inspect GitHub drafts before retrying; no existing assets are replaced.' }
    Write-Output 'Draft created. Review its assets and notes on GitHub, then publish it as the latest stable release.'
} finally {
    if ([IO.File]::Exists($notesPath)) { [IO.File]::Delete($notesPath) }
}
