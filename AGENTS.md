# Audition-RS: Waveform Audio Editor

Adobe Audition의 핵심 파형 편집기(Waveform Editor)를 순수 Rust 기술 스택으로 독립 구현한 전문 오디오 에디터 프로젝트입니다.

---

## 1. 프로젝트 아키텍처 & 런타임 스택
- **Language & Edition:** Rust (Edition 2021)
- **GUI & Graphics Framework:** `eframe` / `egui 0.29` (wgpu 하드웨어 가속 백엔드)
- **Typography & Localization:** Windows 시스템 맑은 고딕(`malgun.ttf`) 기반의 선명한 한국어 UI
- **Audio Output Engine:** `cpal 0.15` (저지연 크로스 플랫폼 오디오 스트리밍)
- **Audio Decoding / File I/O:** 
  - `hound 3.5` (무손실 WAV 16/24/32-bit Float 고속 인코딩/디코딩)
  - `symphonia 0.5` (OGG Vorbis, MP3, FLAC, AAC 등 범용 오디오 디코더 내장)
- **Native File Dialog:** `rfd 0.15` (WAV, OGG, MP3, FLAC 필터 지원)

---

## 2. 설계 의도 및 Rationale (Design Rationale)
1. **리듬게임 채보/오프셋 툴 연계 BPM 비트 그리드 (BPM & Beat Measure Grid System)**:
   - `H:\source\repos\offset tool`의 비트 계산 엔진 및 시각화 파이프라인을 완벽 내장:
     - 마디 길이: `measureDuration = 240.0 / Bpm` (BMS 틱 공식 기반)
     - 비트 간격: `tickIntervalSeconds = (240.0 / Bpm) / BeatDivision`
     - 마디 선(Measure Line): `#F59E0B` (Amber 주황색 실선) 및 마디 번호 배지 (`#000`, `#001`...)
     - 비트 선(Beat Sub-division): `#38BDF8` (Sky 하늘색 점선, 1/4, 1/8, 1/16, 1/32, 1/64 고배율 분할)
     - **원클릭 오프셋 동기화**: `📍 현재 위치를 0마디로 설정` 클릭 시 현재 재생헤드 위치(ms)에 `#000` 마디선 즉각 생성.
     - **비트 점프 버튼**: `⏮ 이전 비트`, `다음 비트 ⏭` 클릭 시 원하는 비트로 즉각 점프하고 화면 자동 센터링.
     - **🧲 비트 자석 스냅(Snap to Beat)**: 재생헤드 클릭/드래그 시 가장 가까운 BPM 비트에 찰칵 달라붙는 스냅 지원.
     - **고배율 줌 비트 가이드**: 화면에 비트선이 들어오지 않을 정도로 극단적 줌 인 상태에서도 양방향 비트 위치 안내 배지 표시.
2. **소수점 넷째자리 초정밀 타임코드 & 룰러 (0.1ms / 100μs Precision)**:
   - 하단 타임코드(`00:00:04.1214 / 00:03:06.1182`), 선택 구간 길이(`선택 구간: 00:00:00.0096`), 타임라인 룰러(`00:04.1250`) 및 마커 목록의 모든 시간 표시를 **소수점 넷째자리(0.0001초 단위)**까지 완벽 지원.
3. **Audition 실기 동일 오실로스코프 파형 렌더링 (Oscilloscope Waveform)**:
   - 기존의 뭉툭하고 빽빽한 수직 빗살 기둥 채우기를 전면 제거.
   - 실제 Adobe Audition과 동일하게, 오디오 신호의 흐름을 연속적인 **선명한 네온 에메랄드 곡선(Continuous Oscilloscope Waveform Curve)**으로 렌더링하여 고배율 및 중간 배율 줌에서도 소리의 진동 주기, 하모닉스, 트랜지언트가 칼날처럼 날렵하게 낱낱이 드러나도록 구현.
   - 중심선(Zero Crossing)을 은은한 레퍼런스 라인으로 배치하여 신호의 극성 교차점을 직관적으로 식별 가능.
4. **재생헤드 마커 직접 잡기 & 실시간 스크러빙 (Direct Playhead Grab & Scrubbing)**:
   - 타임라인 상단 룰러에 큼직한 빨간색 역삼각형 재생헤드 핀(Pin)을 배치하고, 마우스로 핀 또는 룰러를 잡았을 때 **선택 영역 생성 대신 재생헤드 마커를 우선적으로 쥐고 좌우로 쭈루룩 스크러빙(Scrubbing)**되도록 조작 우선순위(Hit Testing) 개편.
   - 드래그 중 화면 가장자리에 도달하면 화면 전체가 자동으로 함께 밀려나는 오토 스크롤(Auto-scroll) 연동.
5. **상단 뷰포트 마커 조그 휠 스크러빙 & 툴바 롱프레스 연속 이동 버튼**:
   - 툴바에 직관적인 **`◀ 이동`**, **`이동 ▶`** 버튼을 배치하고, **버튼을 꾹 누르고 있으면(Hold) 조그 휠처럼 쭈루룩 부드럽게 연속 스크롤**되는 연속 패닝 지원.
   - 상단 미니맵 오버뷰의 뷰포트 마커를 잡고 좌우로 쓱쓱 밀면 파형이 1:1로 쭈루룩 연속 스크롤.
6. **타임라인 눈금자 텍스트 겹침 방지 (Dynamic Ruler Auto-Spacing)**:
   - 라벨 간 최소 96픽셀의 시각적 여백을 강제 보장하는 동적 스텝 알고리즘(`0.0001s ~ 60s` 정밀 스케일)을 적용하여, 극도로 줌 인하거나 줌 아웃해도 시간 숫자가 절대 서로 겹치지 않고 쾌적하게 렌더링됨.

---

## 3. 핵심 모듈 구조
- `src/audio/buffer.rs`: 채널별 샘플 버퍼, 다단계 피크 밉맵, DSP 편집 연산, 묵음 삽입, 톤 제너레이터
- `src/audio/marker.rs`: 마커(Marker) 데이터 모델 및 색상 팔레트
- `src/audio/playback.rs`: CPAL 저지연 재생 엔진 및 원자적 재생헤드/피크 미터 컨트롤러
- `src/audio/io.rs`: Hound(WAV) 및 Symphonia(OGG, MP3, FLAC 등) 디코딩 및 WAV 내보내기
- `src/ui/theme.rs`: Audition 다크 & 민트 테마 및 맑은 고딕 한글 폰트 적용
- `src/ui/waveform_view/`: 메인 파형 에디터 뷰 (책임별 하위 모듈로 분할)
  - `mod.rs`: `WaveformView` 구조체, 레이아웃 오케스트레이션(`show`), 공용 좌표 헬퍼(`time_to_x`, `pixel_to_sample`, `current_time`)
  - `state.rs`: `WaveformViewState`(뷰포트/줌/팬/선택 상태), `BpmSettings`(BPM 틱·스냅 계산), `OverviewDragMode`
  - `timecode.rs`: 타임코드 포맷 함수 (`format_precise_timestamp` 등, 소수점 4자리)
  - `toolbar.rs`: 줌 툴바, 롱프레스 연속 이동 버튼, BPM 컨트롤 바(오프셋, 스냅, 비트 점프)
  - `overview.rs`: 상단 미니맵 오버뷰 & 뷰포트 조그 휠(팬/리사이즈)
  - `input.rs`: 휠 줌/팬, 손 도구 패닝, 재생헤드 스크러빙(비트 스냅), 선택 영역 드래그, 가장자리 오토 스크롤
  - `ruler.rs`: 동적 룰러(텍스트 겹침 방지) & BPM 마디/비트 그리드
  - `waveform.rs`: 오실로스코프 파형 3단계 렌더링(샘플 곡선/피크 트레이스/엔벨로프) & dB 스케일
  - `overlays.rs`: 선택 영역, 마커, 재생헤드 핀 오버레이
- `src/ui/transport_bar.rs`: 한국어 트랜스포트 바 (소수점 4자리 타임코드 표시, 볼륨 슬라이더, 듀얼 VU 미터)
- `src/app/`: 최상위 앱 (책임별 하위 모듈로 분할)
  - `mod.rs`: `AuditionApp` 구조체, 초기화, `eframe::App::update` 패널 디스패치
  - `history.rs`: Undo/Redo 스냅샷 스택 (최대 30단계)
  - `file_ops.rs`: 오디오 열기 / WAV 저장 대화상자
  - `edit_ops.rs`: 잘라내기/복사/붙여넣기/삭제, 마커 추가, 묵음 삽입, DSP(게인·노멀라이즈·페이드·묵음화)
  - `shortcuts.rs`: 키보드 단축키 처리
  - `menu_bar.rs`: 통합 메뉴바(한국어)
  - `panels.rs`: 트랜스포트/상태 줄, 묵음 삽입 모달, 좌측 마커 사이드바(소수점 4자리), 중앙 파형 영역

---

## 4. 빌드 및 실행 방법
```powershell
# 빌드 및 실행
cargo run

# 릴리즈 최적화 빌드
cargo build --release
```

## 5. 단축키 및 인터랙션 안내
- **`🎵 BPM 그리드` 토글**:
  - 버튼을 켜면 BPM(기본 120), 박자 분할(1/4 ~ 1/64), 오프셋(ms), 비트 스냅, 비트 점프 버튼 활성화
  - **`📍 현재 위치를 0마디로 설정`**: 현재 재생헤드 위치에 `#000` 마디선 즉시 생성
  - **`⏮ 이전 비트` / `다음 비트 ⏭`**: 비트선 위치로 즉시 점프 & 화면 센터링
  - **`🧲 비트 자석 스냅`**: 재생헤드가 비트선에 찰칵 달라붙음
- **빨간색 재생헤드(Playhead) 마커 잡기**:
  - 타임라인 상단 룰러의 빨간 핀을 **마우스로 쥐고 좌우로 쭈루룩 스크러빙**
- **툴바 이동 버튼**:
  - **`◀ 이동`**, **`이동 ▶`** 클릭 시 좌우 이동, **꾹 누르고 있으면 휠처럼 쭈루룩 연속 스크롤**
- **상단 뷰포트 마커 (조그 휠)**:
  - 뷰포트 상자를 마우스로 **잡고(Grab) 좌우로 쓱쓱 밀면 파형이 1:1로 쭈루룩 매끄럽게 연속 스크롤**
- **마우스 휠**: 마우스 커서 위치 중심 **부드러운 줌 인/아웃**
- **`Shift + 마우스 휠`**: 좌우 수평 스크롤
- **`Alt + 마우스 휠`**: 수직 진폭(Amplitude) 확대/축소
- **마우스 휠(가운데 버튼) 또는 우클릭 드래그**: 손 도구(Hand Tool) 방식 화면 패닝
- `Space`: 재생 / 일시정지 (Play / Pause)
- `Home`: 재생헤드 맨 앞으로 이동
- **`M`**: 현재 위치에 **마커 추가 (Add Marker)**
- `Ctrl + A`: 전체 영역 선택
- `Ctrl + Z` / `Ctrl + Y`: 실행 취소 (Undo) / 다시 실행 (Redo)
- `Ctrl + X` / `Ctrl + C` / `Ctrl + V`: 잘라내기 / 복사 / 붙여넣기
- `Delete` / `Backspace`: 선택 영역 삭제
- `Ctrl + O`: 오디오 파일 열기 (WAV, OGG, MP3, FLAC 등)
- `Ctrl + S`: WAV 파일 저장

## 6. 탐색기 "연결 프로그램" / 드래그 & 드롭
- 실행 인자 첫 번째(`argv[1]`)가 파일이면 시작 시 자동 로드 (`AuditionApp::new` → `open_path`).
- 창 위로 오디오 파일을 드래그 & 드롭하면 즉시 로드 (`update`에서 `dropped_files` 처리).
- `register_open_with.ps1`: `HKCU\Software\Classes`에 `Applications\audition_rs.exe` + `AuditionRS.Audio` ProgID를 등록해 wav/ogg/mp3/flac/m4a/aac 우클릭 "연결 프로그램" 목록에 표시 (관리자 권한 불필요, `-Unregister`로 해제).
