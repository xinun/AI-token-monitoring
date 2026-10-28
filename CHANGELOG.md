# 변경 사항 / Changelog

## 0.3.4

### 한국어

- 미니바 표시 방식을 항상 표시·AI 앱 실행 시 자동 표시·숨김 중 하나로 선택하도록 정리.
- Windows Codex 데스크톱의 패키지 식별 정보로 실행 감지 보완. 일반 ChatGPT 앱과 구분.
- 설정에 감지 확인 버튼과 실행 상태 안내 추가. 저장 전에도 현재 앱 실행 여부 검사 가능.
- 자동 표시 선택 시 미니바 활성화와 실행 중 표시를 함께 저장. 기존 설정 형식·창 크기·10초 감지 주기 유지.
- 앱 실행 여부를 기준으로 작은 미니바만 표시하며 토큰 사용 순간을 추정하지 않음.

### English

- Simplified mini bar display settings into three modes: always show, show automatically when an AI app is running, or hide.
- Improved detection of the Windows Codex desktop app using its package identity, distinguishing it from the regular ChatGPT app.
- Added a detection check button and running-status information in Settings. Running apps can be checked before saving.
- Selecting automatic display now saves both mini bar activation and app-based visibility together. The existing settings format, window sizes, and 10-second detection interval are preserved.
- Automatic display shows only the small mini bar based on whether an app is running; it does not detect the exact moment tokens are consumed.

## 0.3.3

### 한국어

- 새 버전이 있을 때만 미니바 우측에 작은 종 아이콘과 빨간 알림 점 표시.
- 알림을 클릭하면 앱의 업데이트 창으로 이동. 창을 처음 생성할 때도 요청 유지.
- 알림이 없으면 아이콘과 전용 빈칸을 표시하지 않으며 기존 미니바 크기 유지.
- 설치 실패 후 재시도 알림 유지. 설치는 기존처럼 사용자의 명시적 승인 후 진행.
- 설정 편집 중에는 입력을 유지하고 설정 창을 닫은 뒤 요청한 업데이트 창 표시.

### English

- Added a small bell icon with a red notification dot on the right side of the mini bar only when a new version is available.
- Clicking the notification opens the app's update view, including when the app window needs to be opened first.
- When no update is available, the icon and its reserved space are hidden, preserving the existing mini bar size.
- Kept the update notification available after an installation failure so users can retry. Installation still requires explicit user approval.
- Preserved Settings input while editing and deferred the requested update view until Settings is closed.

## 0.3.2

### 한국어

- 패널·카드·작업표시줄 미니바 크기를 유지하며 잔여량 숫자와 설명의 대비 개선.
- 20% 이하 주의·10% 이하 부족 색상 구분, 오래된 값은 확인 불가 표시 유지.
- 실제 잔여량이 바뀔 때만 350ms 게이지 전환. 첫 표시·같은 값·만료·숨김 상태에는 효과 생략.
- 한도 갱신 시 기존 카드 요소를 유지하고 값만 갱신하여 반복 생성과 불필요한 효과 재생 감소.
- 창 숨김 및 Windows 동작 줄이기 설정에서 애니메이션 중지. 추가 라이브러리·조회 타이머 없음.

### English

- Improved the contrast of remaining-quota values and descriptions while preserving panel, card, and taskbar mini bar dimensions.
- Added distinct warning colors at 20% or less and critical colors at 10% or less. Stale values remain marked as unavailable.
- Added a 350 ms gauge transition only when the remaining quota actually changes. No transition plays on initial display, unchanged values, expired values, or while hidden.
- Updated quota values within existing cards instead of recreating them, reducing unnecessary rendering and repeated animations.
- Disabled animations while the window is hidden or Windows reduced-motion settings are enabled. No additional libraries or polling timers were added.

## 0.3.1

### 한국어

- AI Token 전용 토큰 코인·잔여량 링 아이콘 적용.
- 배포 결과를 버전별 폴더로 구분하고 압축 전 실행 파일 중복 저장 제거.
- 업데이트 서명을 latest.json에 포함하여 별도 .sig 업로드 생략.
- 한 명령으로 필요한 파일을 첨부하는 GitHub 릴리즈 초안 생성 도구 추가.
- 숨겨진 미니바의 위치 확인 타이머 중지, 디버그 캐시만 정리하는 명령 추가.

### English

- Added a custom AI Token icon combining a token coin with a remaining-quota ring.
- Organized release files into version-specific folders and removed duplicate unpacked executables.
- Embedded the update signature in `latest.json`, eliminating the need to upload a separate `.sig` file.
- Added a command to create a GitHub release draft with the required files attached.
- Stopped the mini bar position-check timer while hidden and added a command to clear only the debug build cache.

## 0.3.0

### 한국어

- 설정에 Windows 로그인 시 AI Token 자동 실행 옵션 추가. 기본값은 꺼짐이며 트레이로 시작.
- 선택한 AI 중 실행 중인 항목만 미니바에 표시하는 옵션 추가. 기본값은 꺼짐.
- Windows용 Codex·Claude 실행 파일을 10초 간격으로 확인. 감지가 꺼져 있으면 프로세스 확인 생략.
- AI Token 자체의 한도 조회 프로세스는 실행 감지에서 제외. 한도 조회 주기는 기존 설정 유지.

### English

- Added an option to start AI Token automatically when signing in to Windows. Disabled by default; starts in the system tray.
- Added an option to show only selected AI tools that are currently running in the mini bar. Disabled by default.
- Checks Windows Codex and Claude processes every 10 seconds. Process checks are skipped when detection is disabled.
- Excludes AI Token's own quota-checking processes from running-app detection. Quota refresh intervals remain unchanged.

## 0.2.0

### 한국어

- 앱 내 종 아이콘과 새 버전 알림.
- 앱 시작 시 및 6시간 간격으로 정식 업데이트 확인, 수동 확인과 자동 확인 설정.
- 릴리즈 페이지에서 직접 다운로드하거나 앱에서 승인 후 설치하는 두 가지 업데이트 방법.
- 사용자가 승인한 버전에 한해 다운로드·서명 검증·설치·재시작을 진행하는 Windows 설치형 업데이트.
- 빌드 시 설치 파일, 서명, latest.json, 포터블 ZIP 생성.

### English

- Added an in-app bell icon and new-version notifications.
- Checks for stable updates at startup and every six hours, with manual checks and an automatic-check setting.
- Choose between downloading from the release page and installing from the app after approval.
- Windows installer updates download, verify the signature, install, and restart the app only for the version the user approved.
- Builds now generate an installer, signature, `latest.json`, and a portable ZIP.

## 0.1.0

### 한국어

AI Token의 첫 Windows 테스트 버전입니다.

- Codex 구독 잔여 한도 표시
- 작업표시줄 미니바와 트레이
- Claude Code 상태줄 연결 지원

사용 방법: ZIP을 압축 해제하고 ai-token.exe를 실행하세요.
지원 환경: Windows 10/11 64비트, WebView2 Runtime

Antigravity·Grok 한도 조회와 자동 업데이트는 아직 지원하지 않습니다.

### English

AI Token's first Windows test release.

- Displays remaining Codex subscription quota
- Taskbar mini bar and system tray icon
- Claude Code statusline integration

Getting started: Extract the ZIP and run `ai-token.exe`.
Requirements: Windows 10/11 (64-bit) and WebView2 Runtime.

Antigravity and Grok quota checks and automatic updates are not yet supported.

## 배포 안내 / Publishing notes

### 한국어

0.1.0 사용자는 0.2.0 설치 파일을 처음 한 번 직접 실행해야 합니다.

GitHub `v0.2.0` 릴리즈의 Assets에는 설치 파일과 `latest.json`을 첨부합니다. 서명은 JSON에 포함되므로 별도 `.sig` 업로드는 필요 없습니다.

- AI-Token-0.2.0-windows-x64-setup.exe
- latest.json

포터블 ZIP은 추가로 제공할 수 있습니다. 자동 업데이트 채널에 포함하려면 Release label은 None으로 선택하고 최신 릴리즈로 게시합니다.

### English

Users of 0.1.0 must manually run the 0.2.0 installer once.

Attach the installer and `latest.json` to the Assets of the GitHub `v0.2.0` release. The signature is included in the JSON, so a separate `.sig` upload is not required.

- AI-Token-0.2.0-windows-x64-setup.exe
- latest.json

A portable ZIP may be provided as an additional download. To include a release in the automatic update channel, select None for Release label and publish it as the latest release.
