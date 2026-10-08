$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$binary = Join-Path $projectRoot 'src-tauri/target/release/ai-token.exe'
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
    throw 'Release executable missing. Run npm run build:desktop first.'
}

$packageVersion = (Get-Content -LiteralPath (Join-Path $projectRoot 'package.json') -Raw | ConvertFrom-Json).version
$tauriVersion = (Get-Content -LiteralPath (Join-Path $projectRoot 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json).version
if ($packageVersion -ne $tauriVersion) {
    throw 'package.json and tauri.conf.json must use the same version.'
}
$archiveStem = 'AI-Token-' + $packageVersion + '-windows-x64'
$releaseRoot = Join-Path $projectRoot 'release'
$packageRoot = Join-Path $releaseRoot $archiveStem
New-Item -ItemType Directory -Path $packageRoot -Force | Out-Null

$outputBinary = Join-Path $packageRoot 'ai-token.exe'
$readmePath = Join-Path $packageRoot 'README.txt'
$brandsPath = Join-Path $packageRoot 'BRAND_ASSETS.md'
$licensePath = Join-Path $packageRoot 'LICENSE'
try {
    Copy-Item -LiteralPath $binary -Destination $outputBinary -Force
} catch {
    throw "Cannot replace $outputBinary. Close AI Token running from the release folder, then run scripts/package-release.ps1 again. $($_.Exception.Message)"
}
Copy-Item -LiteralPath (Join-Path $projectRoot 'BRAND_ASSETS.md') -Destination $brandsPath -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'LICENSE') -Destination $licensePath -Force

$readme = @"
AI Token $packageVersion - Windows x64

1. ZIP을 원하는 폴더에 압축 해제합니다.
2. ai-token.exe를 실행합니다.
3. 작업표시줄 오른쪽 AI Token 아이콘을 클릭하면 상세 창이 열립니다.
4. 설정에서 미니바에 표시할 AI를 선택합니다.
5. 트레이 아이콘 우클릭 메뉴에서 종료합니다.

환경: Windows 10/11 x64, Microsoft Edge WebView2 Runtime.
WebView2: https://developer.microsoft.com/microsoft-edge/webview2/

Codex: 이 PC에 설치되고 로그인된 Codex CLI가 필요합니다.
Claude Code: 앱 설정의 '이 PC용 명령'으로 별도 연결합니다.
Antigravity/Grok: 아직 한도 조회를 지원하지 않습니다.

잔여값은 구독 한도의 비율이며, 정확한 잔여 토큰 수가 아닙니다.
Codex는 기본 5분마다 조회합니다. Claude는 연결된 상태줄의 입력에 따라 갱신합니다.
설정은 해당 PC의 앱 데이터 폴더에 저장됩니다.
ZIP에는 개인 설정이나 로그인 인증 정보를 포함하지 않습니다.

AI Token은 각 AI 서비스와 제휴하지 않은 독립 앱입니다.
서비스 아이콘의 출처는 BRAND_ASSETS.md, 앱 라이선스는 LICENSE를 확인하세요.
"@
$utf8 = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($readmePath, $readme, $utf8)

$archivePath = Join-Path $releaseRoot ($archiveStem + '.zip')
$archiveFiles = @($outputBinary, $readmePath, $brandsPath, $licensePath)
Compress-Archive -LiteralPath $archiveFiles -DestinationPath $archivePath -Force
Get-Item -LiteralPath $outputBinary, $archivePath | Select-Object FullName, Length
