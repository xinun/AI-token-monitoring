# AI Token
## how to install 
-> release: https://github.com/xinun/AI-token-monitoring/releases/latest

<img width="529" height="685" alt="image" src="https://github.com/user-attachments/assets/932c913d-a5c3-44c3-8f23-98965e8dc0f6" />

---
# Preview

<img width="257" height="64" alt="image" src="https://github.com/user-attachments/assets/9ae74a0a-e549-4c79-915c-230072acf08f" />
<img width="765" height="760" alt="image" src="https://github.com/user-attachments/assets/1cbeabf5-367f-4c0d-a706-0db66853fb04" />

# Beta 0.3.4
## 현재 기능 구성 Claude,OpenAI
### 1. 메뉴 축소화 가능(윈도우 기준 우측하단 시계,날씨 정보 왼쪽에 볼 수 있도록 미니바 기능)
### 2. 설정에서 조회 간격 조절 가능
### 3. 종 아이콘에서 새 버전 확인, 릴리즈 페이지 열기 및 승인 후 업데이트 설치
### 4. AI 실행 중에만 미니바 표시 및 Windows 로그인 시 자동 실행 선택

## Token 조회 조건
로그인된 Codex CLI가 필요하며, Codex 상세 창이 열려 있지 않아도 한도를 조회할 수 있다. Claude는 Claude Code 상태줄에서 전달받은 최신 한도를 표시한다. 브라우저의 일반 채팅 한도와는 별개다.

AI 구독의 잔여 한도를 표시하는 Windows 우선 트레이 앱. Rust + Tauri 2 + TypeScript로 구성한다.

## 현재 기능

- AI Token 앱 아이콘 하나로 통합한 트레이. 서비스별 공식 아이콘은 상세 창과 미니바에 표시
- 설치된 Codex CLI의 `account/rateLimits/read`로 구독 한도 조회 (AI 생성 요청 없음)
- Claude Code 공식 statusline 입력에서 한도만 저장하는 선택적 PowerShell 브리지
- 여러 단기·주간 한도, 초기화 시간, 마지막 갱신 시간 표시
- 기존 패널·미니바 크기 유지, 20% 이하 주의·10% 이하 부족 구분. 상세 창의 실제 잔여량 변경에만 짧은 게이지 전환 (숨김·동작 줄이기 설정에서는 생략)
- 조회 실패, 누락, 초기화 시각 경과, 15분 이상 지난 데이터는 최신 잔여량으로 표시하지 않음
- 5/10/15분 자동 조회, 실패 시 간격 증가, 수동 갱신
- 기본 실행은 트레이와 선택한 미니바 생성. 상세 창은 클릭 시 생성하고 닫으면 제거
- 브라우저 미리보기의 예시 데이터는 명시적으로 켰을 때만 표시
- 선택한 AI 도구가 실행 중일 때만 해당 미니바 항목 표시, Windows 로그인 시 트레이 자동 실행을 각각 선택 (둘 다 기본 꺼짐)
- 앱 시작 시와 6시간 간격으로 새 버전 확인, 종 아이콘 알림, 사용자가 선택할 때 서명 검증 후 설치·재시작
- Antigravity / Grok는 아직 미연동. 잔여 한도는 확인 불가로 표시

서비스 아이콘은 공식 사이트/CDN에서 받은 이미지다. 출처는 `BRAND_ASSETS.md`에 기록했다. AI Token은 각 서비스와 제휴하지 않은 독립 앱이다. 전용 절전 이벤트 처리와 macOS 실행 검증은 후속 작업이다. 현재 30초 메타데이터 검사로 Claude 전달 파일 변경을 감지한다. CPU·메모리 목표 달성 여부는 실제 측정이 필요하다.

## Windows 미니바

- 기본값은 Codex 한 개 표시. 상세 창 → 연결 설정 → 작업표시줄 미니바에서 표시할 AI 선택.
- 단기·주간 잔여 한도를 두 줄로 표시하며 알 수 없는 값은 `—`로 표시.
- Rust + Win32 GDI로 직접 그려 WebView 프로세스를 만들지 않음.
- 모니터별 화면 배율에서 직접 렌더링. 13px 글꼴, 24px 아이콘, 고해상도 공식 이미지의 부드러운 축소로 작은 표시의 가독성 보완.
- 작업표시줄의 트레이 왼쪽에 별도 작은 창을 겹쳐 표시. 앱 버튼/Alt+Tab 목록에는 표시하지 않음. 작업표시줄 공간을 예약하지 않으므로 아이콘이 많으면 위치 조정 필요.
- 클릭하면 상세 창. 우클릭으로 위치 잠금/기본 위치/숨기기. 위치 잠금을 풀면 왼쪽 손잡이로 이동 가능, 위치 저장.
- 연결 설정의 **미니바 표시 방식**에서 항상 표시·AI 앱 실행 시 자동 표시·숨김 선택. 자동 표시를 저장하면 선택한 도구 중 실행 중인 항목만 미니바에 표시하고 모두 종료되면 숨김. 큰 대시보드를 자동으로 열지 않음. 숨김은 실행 감지보다 우선함.
- 실행 감지는 같은 Windows 세션의 `codex.exe`·`claude.exe`와 Windows Codex 데스크톱을 약 10초 간격으로 확인. `ChatGPT.exe`는 Windows 패키지 식별자가 Codex와 일치할 때만 Codex로 분류. 실제 AI 요청 중인지를 의미하지 않으며 Claude 데스크톱과 Claude Code 실행 파일을 구별하지 않음. 브라우저·WSL·일부 Node.js 실행 및 Antigravity·Grok 자동 감지는 지원하지 않음.
- 실행 파일 이름·부모 PID와 필요한 후보의 Windows 패키지 식별 정보만 확인하며 명령줄·인증 정보·CPU·메모리를 읽지 않음. AI Token 자체의 Codex 한도 조회 프로세스는 제외. 감지 옵션이나 미니바가 꺼져 있으면 프로세스 확인을 생략하고, 감지 실패 시 마지막 상태 유지.
- 전체화면 크기 추정에 따른 자동 숨김은 사용하지 않음. 미니바가 보일 때 2초마다 위치·겹침 순서를 보정하며 숨기면 타이머를 중지함. 이 타이머는 한도를 조회하지 않음. 한 번에 화면을 그려 깜빡임을 줄임.
- 새 버전이 확인되면 미니바 전체의 우측에 종 아이콘과 빨간 점 하나를 표시. 알림이 없으면 숨기고 기존 텍스트 영역 사용. 미니바 크기는 유지하며, 알림 클릭은 앱 업데이트 창·나머지 영역 클릭은 사용량 창을 엶. 설정 편집 중에는 입력을 유지하고 설정 창을 닫은 뒤 업데이트 창을 엶. 알림 클릭만으로 다운로드나 설치를 시작하지 않음.
- **감지 확인** 버튼으로 저장 전에도 현재 실행 중인 지원 앱을 검사 가능. 실행 여부만 읽고 설정·자동 실행 등록·계정 한도를 변경하지 않음. 감지 결과에 나온 AI를 아래 표시 항목에서도 선택해야 함.
- 트레이 우클릭 메뉴에서 다시 켤 수 있음. macOS 미니바는 아직 미구현.
- 상시 미니바의 CPU·메모리 사용량 목표는 아직 실제 측정하지 않음.

## Windows 자동 실행

연결 설정의 **Windows 로그인 시 자동 실행**을 켜고 저장하면 현재 사용자의 로그인 시작 항목에 등록한다. 기본값은 꺼짐이며 대시보드 없이 트레이로 시작한다. 끄고 저장하면 등록을 해제한다. 앱을 시작하거나 기존 설정을 읽는 것만으로 자동 실행을 켜지 않는다.

AI 도구 실행 감지에는 AI Token이 백그라운드에서 실행 중이어야 한다. AI Token을 완전히 종료한 상태에서는 다른 도구가 켜지는 것을 감지할 수 없다. Windows 자동 실행과 실행 중 미니바 표시를 함께 켜면 로그인 후 대기하다가 AI 도구를 열 때 미니바를 표시한다. 포터블 실행 파일을 옮기면 자동 실행 경로를 다시 등록해야 하므로 설치형 사용을 권장한다. Windows 시작 앱 설정에서 차단한 경우에는 해당 OS 설정도 확인한다.

## 개발 환경

Node.js LTS, Rust 1.90 이상 stable (MSVC), Microsoft C++ Build Tools의 Desktop development with C++ 구성요소, WebView2가 필요하다.

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
npm run updater:key
npm run build:desktop
```

`updater:key`는 빌드 PC에서 최초 한 번 실행한다. 개인키는 `%LOCALAPPDATA%\AI Token Build\updater.key`에 저장하며, 공개키 파일의 내용만 `src-tauri/tauri.conf.json`의 `plugins.updater.pubkey`에 넣는다. 이 PC는 공개키 설정까지 완료했다. 다른 PC에서 빌드하려면 기존 개인키를 개인 백업에서 복원한다. 개인키를 GitHub나 릴리즈에 올리지 않으며, 이후 배포에서도 같은 키를 사용한다.

Windows x64 설치 파일과 포터블 ZIP을 `release/<버전>/`에 준비한다. `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`의 버전을 함께 변경한다. 자동 업데이트 서명은 앱 버전에도 연결된다. 빌드 과정에서 앱 아이콘도 원본 SVG로부터 생성한다.

```text
release/
  0.3.0/
  0.3.4/
    AI-Token-0.3.4-windows-x64.zip
    AI-Token-0.3.4-windows-x64-setup.exe
    latest.json
```

GitHub Releases에 자동 업데이트용 **설치 파일과 latest.json 두 파일**을 첨부한다. 서명은 JSON에 포함하므로 별도 `.sig` 파일 업로드는 필요 없다. 포터블 ZIP은 직접 압축 해제하여 쓰는 사용자를 위한 선택 파일이다. 압축 전 실행 파일 폴더는 중복 저장하지 않으며 ZIP에는 실행 파일·안내·라이선스만 포함한다.

`release/`, `dist/`, `src-tauri/target/`는 Git에서 제외한다. 빌드 결과만 다시 묶으려면 `npm run package:desktop`을 실행한다. 배포 파일에는 개인 설정이나 로그인 정보를 넣지 않는다. 설치형 업데이트는 기존 사용자의 호환성을 위해 유지한다.

파일을 일일이 첨부하는 대신 GitHub CLI에 로그인한 후 아래 명령으로 릴리즈 **초안**을 만들 수 있다. 소스를 먼저 커밋·푸시해야 하며 로컬 HEAD가 원격 main과 일치하는지 확인한다. 이미 존재하는 릴리즈는 덮어쓰지 않는다.

```powershell
npm run release:draft -- -Preview
npm run release:draft
```

첫 명령은 업로드할 파일 목록만 보여 주며 네트워크 요청을 하지 않는다. 두 번째 명령은 설치 파일·JSON·ZIP을 첨부한 초안을 만들고, GitHub에서 검토 후 게시한다. ZIP을 제외하려면 `-WithoutPortable`을 추가한다.

빌드 캐시와 임시 백업은 `npm run clean`으로 정리한다. `dist/`, `src-tauri/target/`, 자동 생성 스키마와 `.tools/`를 삭제하며, 소스·개발 의존성·`release/`의 배포 파일은 유지한다. 다음 빌드는 캐시를 다시 생성하므로 시간이 더 걸릴 수 있다.

`npm run clean:debug`는 디버그 캐시만 지우고 릴리즈 빌드 캐시는 남겨 재배포 빌드에 재사용한다. 개발 캐시와 node_modules는 사용자에게 배포되지 않는다. 앱은 사용하지 않는 미니바의 2초 위치 타이머를 중지한다.

앱 전용 아이콘 원본은 `public/app-icon.svg`이며 `npm run icons`로 데스크톱 아이콘만 생성할 수 있다. OS·설치 파일·트레이·대시보드에 같은 토큰 코인·잔여량 링 모양을 사용하며 서비스별 공식 아이콘은 그대로 유지한다.

## 앱 업데이트

자동 업데이트를 쓰려면 `AI-Token-0.3.4-windows-x64-setup.exe`로 설치한다. 0.1.0은 업데이트 기능이 없으므로 처음 한 번은 직접 설치한다. 같은 앱 식별자와 앱 데이터 경로를 유지하므로 기존 연결 설정과 미니바 위치를 사용한다.

앱 시작 시와 6시간 간격으로 작은 버전 안내 파일을 읽는다. 종 아이콘에서 수동 확인할 수 있고, 수동 확인은 1분 간격이다. 새 버전이 있으면 배지가 나타난다. 알림 창의 **릴리즈 페이지 보기**로 브라우저에서 변경 사항을 확인하고 직접 다운로드할 수 있다. 확인된 새 버전이 없을 때는 최신 릴리즈 페이지를 연다. 페이지를 여는 동작은 앱 내 설치를 시작하지 않는다.

앱에서 설치하려면 **업데이트 설치 → 승인하고 설치**를 누른다. 승인하기 전에는 설치 파일을 다운로드하지 않는다. 승인한 버전과 파일의 서명을 검증한 후 설치 프로그램을 실행하고, Windows 설치 프로그램이 앱을 다시 실행한다. 확인 도중 버전 정보가 바뀌면 다시 승인을 받는다. 실패 시 알림에서 재시도할 수 있다. 자동 확인은 설정에서 끌 수 있다.

GitHub 자동 업데이트 채널은 **정식 릴리즈**를 확인한다. `v0.3.4`처럼 버전 태그를 지정하고 `Release label: None`, 최신 릴리즈로 게시한다. `Pre-release`나 초안은 이 채널에서 제외된다. 태그, `latest.json`의 버전과 다운로드 파일 이름을 빌드 버전과 맞춘다. 이후 버전도 설치 파일과 JSON을 함께 올린다. 업데이트 파일을 준비하기 전에는 최신 버전이라고 표시하지 않는다.

빌드 검증은 `npm test`, `cargo test --manifest-path src-tauri/Cargo.toml`로 수행한다. 설치 파일을 만든 뒤 `cargo test --release --manifest-path src-tauri/Cargo.toml --test updater_artifact -- --ignored --test-threads=1`로 정상 서명, 변조 파일, 버전 바꿔치기 거부를 검증한다. 이 검증은 설치 프로그램을 실행하지 않는다.

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
