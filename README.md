# AI Token

AI 구독의 잔여 한도를 표시하는 Windows 우선 트레이 앱. Rust + Tauri 2 + TypeScript로 구성한다.

## 현재 기능

- AI Token 앱 아이콘 하나로 통합한 트레이. 서비스별 공식 아이콘은 상세 창과 미니바에 표시
- 설치된 Codex CLI의 `account/rateLimits/read`로 구독 한도 조회 (AI 생성 요청 없음)
- Claude Code 공식 statusline 입력에서 한도만 저장하는 선택적 PowerShell 브리지
- 여러 단기·주간 한도, 초기화 시간, 마지막 갱신 시간 표시
- 조회 실패, 누락, 초기화 시각 경과, 15분 이상 지난 데이터는 최신 잔여량으로 표시하지 않음
- 5/10/15분 자동 조회, 실패 시 간격 증가, 수동 갱신
- 기본 실행은 트레이와 선택한 미니바 생성. 상세 창은 클릭 시 생성하고 닫으면 제거
- 브라우저 미리보기의 예시 데이터는 명시적으로 켰을 때만 표시
- Antigravity / Grok는 아직 미연동. 잔여 한도는 확인 불가로 표시

서비스 아이콘은 공식 사이트/CDN에서 받은 이미지다. 출처는 `BRAND_ASSETS.md`에 기록했다. AI Token은 각 서비스와 제휴하지 않은 독립 앱이다. 알림, 자동 시작, 전용 절전 이벤트 처리, 설치 패키징과 macOS 실행 검증은 후속 작업이다. 현재 30초 메타데이터 검사로 Claude 전달 파일 변경을 감지한다. CPU·메모리 목표 달성 여부는 실제 측정이 필요하다.

## Windows 미니바

- 기본값은 Codex 한 개 표시. 상세 창 → 연결 설정 → 작업표시줄 미니바에서 표시할 AI 선택.
- 단기·주간 잔여 한도를 두 줄로 표시하며 알 수 없는 값은 `—`로 표시.
- Rust + Win32 GDI로 직접 그려 WebView 프로세스를 만들지 않음.
- 모니터별 화면 배율에서 직접 렌더링. 13px 글꼴, 24px 아이콘, 고해상도 공식 이미지의 부드러운 축소로 작은 표시의 가독성 보완.
- 작업표시줄의 트레이 왼쪽에 별도 작은 창을 겹쳐 표시. 앱 버튼/Alt+Tab 목록에는 표시하지 않음. 작업표시줄 공간을 예약하지 않으므로 아이콘이 많으면 위치 조정 필요.
- 클릭하면 상세 창. 우클릭으로 위치 잠금/기본 위치/숨기기. 위치 잠금을 풀면 왼쪽 손잡이로 이동 가능, 위치 저장.
- 미니바 표시 여부는 사용자 설정으로만 결정. 전체화면 크기 추정에 따른 자동 숨김을 제거하여 켜짐/꺼짐 반복 방지. 2초마다 위치·겹침 순서만 보정하며 이 타이머는 한도를 조회하지 않음. 한 번에 화면을 그려 깜빡임을 줄임.
- 트레이 우클릭 메뉴에서 다시 켤 수 있음. macOS 미니바는 아직 미구현.
- 상시 미니바의 CPU·메모리 사용량 목표는 아직 실제 측정하지 않음.

## 개발 환경

Node.js LTS, Rust stable (MSVC), Microsoft C++ Build Tools의 Desktop development with C++ 구성요소, WebView2가 필요하다.

```powershell
npm install
npm run check:env
npm run probe:codex
npm run tauri -- dev
```

트레이 아이콘을 클릭하면 상세 창이 열린다. 우클릭 메뉴에서 종료할 수 있다. 처음부터 창을 열려면 `npm run tauri -- dev -- --show`를 사용한다.

Rust가 PATH에 등록되지 않은 Windows 환경에서는 `powershell -ExecutionPolicy Bypass -File scripts/dev.ps1`로 개발 실행할 수 있다.

```powershell
npm test
npm run build
cd src-tauri
cargo test
```

브라우저 화면만 확인하려면 `npm run dev` 후 `http://127.0.0.1:1420`에 접속한다. 브라우저 모드에서는 실제 계정 정보를 조회하지 않는다.

## 빌드와 배포

```powershell
npm run build:desktop
```

Windows x64 배포용 EXE를 빌드하고 프로젝트의 `release/` 폴더에 실행 파일과 ZIP을 준비한다. `package.json`의 버전을 파일 이름에 사용한다.

```text
release/
  AI-Token-0.1.0-windows-x64.zip
  AI-Token-0.1.0-windows-x64/
    ai-token.exe
    README.txt
    BRAND_ASSETS.md
    LICENSE
```

GitHub Releases에는 ZIP을 직접 첨부한다. `release/`, `dist/`, `src-tauri/target/`는 Git에서 제외하며 로컬 결과물은 그대로 보관한다. 기존 빌드 파일만 다시 묶으려면 `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package-release.ps1`를 실행한다. ZIP에는 앱 설정이나 로그인 정보를 넣지 않는다.

같은 버전의 배포 폴더에서 AI Token을 실행 중이면 앱을 종료한 뒤 빌드한다. 실행 중인 EXE는 Windows에서 덮어쓸 수 없다.

빌드 캐시와 임시 백업은 `npm run clean`으로 정리한다. `dist/`, `src-tauri/target/`, 자동 생성 스키마와 `.tools/`를 삭제하며, 소스·개발 의존성·`release/`의 배포 파일은 유지한다. 다음 빌드는 캐시를 다시 생성하므로 시간이 더 걸릴 수 있다.

## 갱신 방식

- 실시간 스트리밍이 아니라 주기적으로 읽은 구독 한도 잔여 비율이다. 정확한 잔여 토큰 개수를 뜻하지 않는다.
- Codex: 앱 시작 시, 수동 새로고침 시, 기본 5분 간격으로 조회. 설정에서 5/10/15분 선택. 실패 시 간격을 늘려 최대 1시간.
- Claude: 연결된 Claude Code 상태줄이 데이터를 보낼 때 갱신. 앱은 로컬 전달 파일 변경을 30초마다 검사.
- 미니바 위치 보정(2초)과 상태 표시 갱신(30초)은 원격 한도 조회와 별개다.

## Codex 연결

기존 Codex CLI 로그인 계정을 사용한다. CLI를 PATH 또는 Windows Codex 앱의 로컬 설치 폴더에서 찾는다. 별도 경로는 `AI_TOKEN_CODEX_PATH` 환경 변수로 설정할 수 있다. 인증 파일을 직접 읽거나 복사하지 않는다. 조회용 앱 서버는 요청 후 종료하며 20초 제한을 둔다. 일반 ChatGPT 채팅 한도와는 별개다.

## Claude Code 연결

앱의 연결 설정에서 Claude를 켜고, **이 PC용 명령 복사** 버튼으로 복사한 명령을 Claude Code 사용자 설정의 `statusLine.command`에 연결한다. 명령은 현재 PC의 앱 데이터 경로로 자동 생성한다. 다른 PC에는 그 PC에서 복사한 명령을 사용한다. **설정 JSON 복사**는 경로의 따옴표와 역슬래시를 JSON 형식으로 이스케이프한다. 기존 설정에는 `statusLine` 항목만 병합한다. `type`은 `command`로 설정한다. 기존 상태줄이 있다면 설정을 덮어쓰지 말고 기존 명령과 함께 실행하는 래퍼를 사용한다. 앱은 Claude 설정을 자동 변경하지 않는다.

상태줄 실행 시 `rate_limits.five_hour`와 `rate_limits.seven_day` 및 수집 시각만 전달 파일에 저장한다. 대화·인증 정보·컨텍스트 사용률은 저장하지 않는다. Claude Code가 실행되지 않거나 한도 필드가 제공되지 않으면 현재 한도를 보장할 수 없다.

브리지 스크립트는 EXE에 포함되며 앱 실행 시 해당 PC의 앱 데이터 폴더에 자동으로 준비된다. 개발 프로젝트나 EXE 설치 폴더의 위치에 의존하지 않는다.

Windows 기본 데이터 위치는 `%APPDATA%\com.aitoken.desktop`이다. 설정을 끄면 조회를 중단한다. 브리지는 사용자가 별도로 구성한 Claude 상태줄에서 계속 실행될 수 있으므로 완전히 해제하려면 그 연결도 제거한다. 앱 자체는 자격증명을 새로 저장하지 않는다.

## 구조

```text
src/                    상세 화면과 표시 규칙
src-tauri/src/model.rs  공통 한도 모델
src-tauri/src/providers.rs  Codex·Claude 조회 및 파싱
src-tauri/src/runtime.rs    트레이·설정·갱신 스케줄
scripts/                연결 진단·Claude 브리지·검증
PLAN.md                 기능 계획
```

## 공식 자료

- https://learn.chatgpt.com/docs/app-server
- https://code.claude.com/docs/en/statusline
- https://v2.tauri.app/learn/system-tray/
- https://v2.tauri.app/start/prerequisites/

macOS는 공통 조회 로직을 재사용하되 Mac에서 빌드·검증하고 Claude 전달 스크립트를 추가해야 한다.
