<div align="center">

# 🎧 Audition-RS

**Adobe Audition 스타일의 파형(Waveform) 오디오 편집기 — 순수 Rust로 구현**

리듬게임 채보 / 오프셋 작업을 위한 **BPM 비트 그리드**와 **0.1ms 초정밀 타임코드** 내장

![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)
![egui](https://img.shields.io/badge/GUI-egui%200.29%20(wgpu)-blue)
![Platform](https://img.shields.io/badge/Platform-Windows-0078D6?logo=windows)

[**⬇ 단독 실행 exe 다운로드 (Releases)**](https://github.com/magjangin/audio-editer-hwa/releases/latest)

</div>

---

## ✨ 주요 기능

### 🌊 파형 편집
- **오실로스코프 파형 렌더링** — Audition처럼 연속 곡선으로 그려 고배율 줌에서도 진동 주기·트랜지언트가 선명
- 다단계 피크 밉맵으로 긴 곡도 부드러운 줌/스크롤
- 잘라내기 · 복사 · 붙여넣기 · 삭제, **Undo/Redo (30단계)**
- 게인, 노멀라이즈, 페이드 인/아웃, 묵음화, **묵음 삽입**
- 마커 추가 / 사이드바 마커 목록

### 🎵 리듬게임 BPM 비트 그리드
- 마디 길이 `240 / BPM` (BMS 틱 공식) 기반 마디선(주황) · 비트선(하늘색, 1/4 ~ 1/64)
- **📍 현재 위치를 0마디로 설정** — 원클릭 오프셋 동기화
- **⏮ 이전 비트 / 다음 비트 ⏭** 점프 & 자동 센터링
- **🧲 비트 자석 스냅** — 재생헤드가 가장 가까운 비트에 달라붙음

### ⏱ 초정밀 타임코드
- 타임코드 · 선택 구간 · 룰러 · 마커 모두 **소수점 넷째 자리(0.0001초)** 표시
- 줌 배율에 따라 라벨이 겹치지 않는 동적 룰러

### 🖱 조작성
- 룰러의 빨간 재생헤드 핀을 잡고 **스크러빙**, 가장자리 오토 스크롤
- 상단 미니맵 뷰포트 **조그 휠** 드래그
- `◀ 이동` / `이동 ▶` 길게 누르면 연속 스크롤

### 📂 파일
- 열기: **WAV, OGG, MP3, FLAC, M4A/AAC** (symphonia / hound)
- 저장: WAV
- **탐색기 "연결 프로그램"**, exe 위로 드래그, **창에 드래그 & 드롭**으로 바로 열기

---

## 🚀 설치 & 실행

### 방법 1 — 단독 exe (추천)
1. [Releases](https://github.com/magjangin/audio-editer-hwa/releases/latest)에서 `audition_rs.exe` 다운로드
2. 원하는 폴더에 두고 실행 — **설치 · 런타임(VC++ 재배포 패키지) 불필요**

> Windows 10/11 x64, 한국어 UI는 시스템 폰트 `맑은 고딕`을 사용합니다.

### 방법 2 — 소스 빌드
```powershell
git clone https://github.com/magjangin/audio-editer-hwa.git
cd audio-editer-hwa
cargo build --release
# → target\release\audition_rs.exe
```

### 우클릭 "연결 프로그램"에 등록 (선택)
exe와 같은 위치 기준으로 실행:
```powershell
powershell -ExecutionPolicy Bypass -File .\register_open_with.ps1 -ExePath .\audition_rs.exe
# 해제
powershell -ExecutionPolicy Bypass -File .\register_open_with.ps1 -Unregister
```
현재 사용자(HKCU)에만 등록되므로 관리자 권한이 필요 없습니다.

---

## ⌨ 단축키

| 키 | 동작 |
|---|---|
| `Space` | 재생 / 일시정지 |
| `Home` | 맨 앞으로 |
| `M` | 현재 위치에 마커 추가 |
| `Ctrl + A` | 전체 선택 |
| `Ctrl + Z` / `Ctrl + Y` | 실행 취소 / 다시 실행 |
| `Ctrl + X` / `C` / `V` | 잘라내기 / 복사 / 붙여넣기 |
| `Delete` / `Backspace` | 선택 영역 삭제 |
| `Ctrl + O` / `Ctrl + S` | 열기 / WAV 저장 |
| 마우스 휠 | 커서 중심 줌 |
| `Shift + 휠` | 수평 스크롤 |
| `Alt + 휠` | 진폭 확대/축소 |
| 가운데 버튼 / 우클릭 드래그 | 손 도구 패닝 |

---

## 🛠 기술 스택

| 영역 | 라이브러리 |
|---|---|
| GUI | `eframe` / `egui 0.29` (wgpu) |
| 오디오 출력 | `cpal 0.15` |
| 디코딩 | `hound 3.5` (WAV), `symphonia 0.5` (OGG/MP3/FLAC/AAC) |
| 파일 대화상자 | `rfd 0.15` |

```
src/
├─ audio/        버퍼·밉맵·DSP, 마커, CPAL 재생 엔진, 파일 I/O
├─ ui/
│  ├─ waveform_view/  툴바·미니맵·입력·룰러·파형·오버레이
│  ├─ transport_bar.rs
│  └─ theme.rs
└─ app/          앱 상태, Undo/Redo, 파일/편집 명령, 단축키, 메뉴, 패널
```

---

<div align="center">

Made by **화영왕 (Hwa-young-wang)** · Muse Dash Archive & Custom Project

</div>
