# Codex JSONL Observatory

[![최신 릴리스](https://img.shields.io/github/v/release/revertable/codex-jsonl-observatory?display_name=tag&label=release)](https://github.com/revertable/codex-jsonl-observatory/releases/latest)

**언어:** [English](README.md) | 한국어

Codex JSONL Observatory는 Codex 세션 JSONL 파일을 읽고 버전 관리가 가능한 작업 로그 번들을 내보내는 로컬 데스크톱 도구입니다. Codex CLI와 Codex Windows 앱에서 생성된 세션을 지원하며, Rust, Svelte, Tauri로 구축되어 모든 세션 데이터를 로컬에서 처리합니다.

대화 기록을 읽거나, 필터링된 대화 내용을 텍스트로 캡처하거나, 전체 세션을 프로젝트 작업 공간·문서·내부 아카이브·저장소에 함께 보관할 수 있는 구조화된 파일로 변환할 때 사용할 수 있습니다.

## 계보 및 릴리스 상태

Codex JSONL Observatory는 이 제품 계보의 이전 도구인 [Codex Chat Viewer](https://github.com/revertable/codex-chat-viewer)를 잇는 2세대 앱입니다. Codex 세션 JSONL 파일을 읽는다는 동일한 목적을 유지하면서, 전체 작업 흐름을 Rust/Svelte/Tauri 기반의 로컬 데스크톱 앱으로 다시 구축했습니다.

현재 공개된 Windows 포터블 릴리스는 `v1.1.2`입니다. 핵심 대화 중심의 기본 필터, 대화 상단으로 빠르게 이동하는 기능, 불필요한 Terminal 여백 제거로 transcript 탐색을 다듬었습니다.

`main` 브랜치는 개발 버전 `1.1.3`으로 준비 중입니다. 따라서 최신 공개 릴리스가 `v1.1.2`인 동안에도 `main`에서 직접 빌드한 결과물에는 다음 개발 버전이 사용됩니다.

Windows 포터블 ZIP을 내려받아 압축을 풀고 앱을 실행하면 됩니다. 별도의 서버 설정, 클라우드 계정, 개발 환경은 필요하지 않습니다.

## 주요 기능

- **Select JSONL** 파일 선택기로 Codex CLI나 Codex Windows 앱의 세션 JSONL 파일을 엽니다.
- 파싱된 대화 블록을 **Terminal Style**, **Markdown Style**, **DM Style**, **DM Style (Dark)** 중 원하는 테마로 읽습니다.
- You, Codex, 도구 호출, 도구 결과, 메타데이터 필터로 원하는 대화 내용에 집중합니다.
- 사람이 작성한 요청과 확인된 ChatGPT→Work handoff 데이터 및 자동 주입된 ambient UI context를 구분합니다.
- cached preview를 대화에 반복하지 않고 참조한 ChatGPT conversation의 title과 conversation ID를 provenance로 표시합니다.
- Guardian review session을 내부 specialized session으로 식별하고, 검증된 thread ID로 로컬 parent session을 엽니다.
- **Capture Transcript**로 현재 필터와 테마가 적용된 대화 내용을 클립보드 텍스트로 캡처합니다.
- 상단 컨트롤이나 대화 내용 아래의 작업 버튼에서 선택한 세션을 새로고침합니다.
- **Copy Resume Command**로 감지된 `codex resume <session-id>` 명령을 복사합니다.
- **Visit Cosmic Horizon**으로 관련 [Cosmic Horizon Archive](https://riu-salze-studio.gitbook.io/cosmic-horizon)를 엽니다.
- 더 새로운 공개 버전이 있으면 초기 화면에 GitHub Releases 링크와 함께 업데이트 안내를 표시합니다.
- **Export Worklog**로 전체 세션을 버전 관리 가능한 작업 로그 번들로 내보냅니다.

## 세션 읽기

**Select JSONL**을 사용해 Codex CLI 또는 Codex Windows 앱의 세션 JSONL 파일을 선택합니다. 선택한 경로는 읽기 전용으로 표시되며, **Refresh**는 현재 선택한 파일을 디스크에서 다시 읽습니다.

기본 대화 영역은 파싱된 블록을 선택한 읽기 테마로 표시합니다. 역할 필터는 원본 세션을 변경하지 않고 이 영역에 표시되는 내용만 바꿉니다. **Capture Transcript**는 필터링된 블록을 포함해 현재 대화 영역에 표시된 텍스트를 복사합니다. 대화 영역 아래의 두 번째 **Refresh** 버튼을 사용하면 화면 상단으로 다시 스크롤하지 않아도 선택한 세션을 다시 불러올 수 있습니다. **loaded** 상태를 클릭하면 선택한 세션을 비우고 앱을 초기 idle 상태로 되돌립니다.

세션 ID를 확인할 수 있으면 **Copy Resume Command**가 해당 Codex CLI 재개 명령을 클립보드에 복사합니다.

Observatory는 시작할 때 공개 GitHub Releases API에 인증 없는 요청을 한 번 보내 최신 공개 릴리스와 실행 중인 앱 버전을 비교합니다. 세션 내용, 선택한 경로, 세션 식별자는 전송하지 않습니다. 확인할 수 없거나 버전 형식을 인식할 수 없으면 업데이트 안내 없이 기존 작업을 계속합니다.

### 사용자 요청과 주입된 context

JSONL에서 `role=user`인 payload가 항상 사람이 직접 입력한 문장을 뜻하지는 않습니다. ChatGPT→Work session에서는 참조 conversation metadata, 자동 생성된 delegated task, 브라우저나 runtime이 공급한 context가 같은 role에 함께 들어갈 수 있습니다.

Observatory는 실제로 확인된 transport envelope만 보수적으로 식별합니다. `Continuing from ...` delegated task는 title과 `chatgpt-conversation://` ID가 파싱된 reference metadata와 정확히 일치할 때만 숨깁니다. 확인된 `ambient-ui-state` envelope에서는 `## My request:` 뒤의 텍스트만 `[YOU]`로 표시합니다. 알 수 없거나 불완전한 구조는 삭제하지 않고 원문 표시로 fallback합니다.

참조한 ChatGPT conversation은 title과 conversation ID를 가진 session provenance로 한 번만 표시합니다. bounded `priorConversation` cache는 기본 대화 화면에 다시 노출하지 않습니다. 원본 JSONL 파일은 절대 수정하지 않습니다.

### Specialized session

Guardian review rollout은 일반 `[YOU]`/`[CODEX]` 대화가 아니라 내부 specialized session으로 표시합니다. Guardian child session에는 Resume과 Worklog 내보내기를 제공하지 않습니다. 검증된 parent thread ID를 로컬 Codex session 저장소에서 찾으면 **Open Parent Session**으로 기존 session workflow를 통해 해당 rollout을 열고, 찾지 못하면 parent가 로컬에 없다는 상태를 명확히 표시합니다.

## 작업 로그 내보내기

**Export Worklog**는 작업의 출발점이 된 요청별로 세션을 구성한 폴더 번들을 만듭니다. 내보낼 상위 디렉터리를 선택하면 앱이 다음 구조로 번들을 생성합니다.

```text
<selected-parent>/
└─ codex-worklog/
   └─ YYYY-MM-DD/
      └─ HHMMSS_<source-id>/
         ├─ 000_index.md
         ├─ 001_HHMMSS.md
         ├─ 002_HHMMSS.md
         ├─ ...
         └─ manifest.json
```

사람이 작성한 각 `[YOU]` 블록은 하나의 작업 단위를 시작합니다. 이후의 Codex/assistant 응답, 도구 호출, 도구 결과, 보고 메시지는 다음 `[YOU]` 블록이 나타날 때까지 같은 작업 단위에 포함됩니다. 확인된 delegated handoff task와 ambient UI context는 작업 단위 요청이 되지 않습니다. 참조 conversation의 title, conversation ID, preview availability 상태는 `000_index.md`에 기록하지만 cached preview message는 내보내지 않습니다. `manifest.json`은 생성된 번들을 기록해 안전하고 호환되는 갱신을 지원합니다.

내보내기는 현재 필터링된 대화 화면이 아니라 항상 전체 원본 세션을 사용합니다. 같은 세션에서 만든 호환 가능한 번들을 다시 내보내면 `manifest.json`을 기준으로 생성 파일을 갱신합니다. 내보내기에 성공하면 운영 체제의 파일 탐색기에서 생성된 번들 폴더를 엽니다.

디렉터리와 파일 이름에는 사용자에게 표시되는 로컬 시간이 사용됩니다. 원본 타임스탬프가 있으면 생성된 콘텐츠와 메타데이터에 그대로 유지됩니다.

> 내보낸 작업 로그에는 프롬프트, 로컬 경로, 명령 출력, 코드 조각, 프로젝트별 정보가 포함될 수 있습니다. 공유하기 전에 번들 내용을 검토하세요.

## Windows에서 빌드 및 실행

Windows에서 소스 코드를 직접 빌드하려면 npm이 포함된 Node.js와 Rust stable toolchain을 설치합니다. 새로 받은 저장소에서는 먼저 `frontend/`에서 잠긴 프런트엔드 의존성을 설치합니다.

```text
npm ci
```

그런 다음 저장소 루트에서 `build-and-run.bat`을 더블클릭하거나 다음 명령을 실행합니다.

```text
build-and-run.bat
```

이 스크립트는 설치 프로그램을 만들지 않고 release 애플리케이션을 빌드하고 다음 포터블 압축 파일을 생성한 뒤 앱을 시작합니다.

```text
release\Codex-JSONL-Observatory_1.1.3_windows-x64-portable.zip
```

압축 파일에는 `codex-jsonl-observatory.exe`, `LICENSE`, 영문·한국어가 함께 수록된 `README.txt`가 포함됩니다. 빌드된 앱은 다음 경로에서 시작됩니다.

```text
frontend\src-tauri\target\release\codex-jsonl-observatory.exe
```

## 실행 환경 및 개발

이 애플리케이션은 Tauri 데스크톱 셸 안에서 Svelte 프런트엔드를 사용합니다. Tauri는 로컬 JSONL 처리를 위해 Rust 파서와 내보내기 계층을 직접 호출합니다.

대화형 데스크톱 개발 실행은 `frontend/`에서 다음 명령을 사용합니다.

```text
npm run tauri:dev
```

자동 테스트, 정적 검사, 프로덕션 빌드, Rust 패키지 테스트, 데스크톱 빌드 검증과 변경 범위별 요구 사항은 [검증 매트릭스](docs/test-matrix.md)를 따릅니다.
