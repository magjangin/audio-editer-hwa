//! Top-level application.
//!
//! Split into sub-modules by responsibility:
//! - `history`   : undo / redo snapshot stacks
//! - `file_ops`  : open / save dialogs
//! - `edit_ops`  : cut / copy / paste / delete, markers and DSP edit commands
//! - `shortcuts` : keyboard shortcut handling
//! - `menu_bar`  : Korean top menu bar
//! - `panels`    : transport bar, status line, silence dialog, marker sidebar, central waveform

mod edit_ops;
mod file_ops;
mod history;
mod menu_bar;
mod panels;
mod shortcuts;

use std::path::PathBuf;
use std::sync::atomic::Ordering;

use crate::audio::buffer::AudioBuffer;
use crate::audio::marker::Marker;
use crate::audio::playback::{AudioPlaybackEngine, PlaybackController};
use crate::ui::theme::AuditionTheme;
use crate::ui::waveform_view::WaveformViewState;

pub struct AuditionApp {
    _playback_engine: Option<AudioPlaybackEngine>,
    controller: PlaybackController,
    waveform_state: WaveformViewState,
    markers: Vec<Marker>,
    marker_counter: usize,
    show_marker_panel: bool,
    show_silence_dialog: bool,
    silence_duration_sec: f32,
    undo_stack: Vec<(AudioBuffer, Vec<Marker>)>,
    redo_stack: Vec<(AudioBuffer, Vec<Marker>)>,
    clipboard: Option<AudioBuffer>,
    current_filepath: Option<PathBuf>,
    status_message: String,
}

impl AuditionApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        AuditionTheme::apply(&_cc.egui_ctx);

        // Generate initial rich stereo synth track for immediate playback
        let demo_buffer = AudioBuffer::generate_demo_track(44100, 15.0);
        let duration = demo_buffer.duration_seconds();

        let engine = AudioPlaybackEngine::try_new(demo_buffer.clone()).ok();
        let controller = if let Some(eng) = &engine {
            eng.controller.clone()
        } else {
            PlaybackController::new(demo_buffer)
        };

        let mut waveform_state = WaveformViewState::new();
        waveform_state.reset_view_to_fit(duration);

        // Initial default markers for demonstration
        let markers = vec![
            Marker::new(1, "인트로 (Intro)".to_string(), 0, 0),
            Marker::new(2, "비트 드롭 (Beat Drop)".to_string(), (44100.0 * 3.75) as usize, 0),
            Marker::new(3, "메인 루프 (Main Loop)".to_string(), (44100.0 * 7.5) as usize, (44100.0 * 3.75) as usize),
        ];

        let mut app = Self {
            _playback_engine: engine,
            controller,
            waveform_state,
            markers,
            marker_counter: 4,
            show_marker_panel: true,
            show_silence_dialog: false,
            silence_duration_sec: 1.0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            clipboard: None,
            current_filepath: None,
            status_message: "준비 완료. 전자 신스 데모 트랙이 로드되었습니다.".to_string(),
        };

        // Windows "연결 프로그램" / 파일 연결 / exe 위로 드래그: 첫 번째 인자를 파일 경로로 연다.
        if let Some(arg) = std::env::args_os().nth(1) {
            let path = PathBuf::from(arg);
            if path.is_file() {
                app.open_path(path);
            }
        }

        app
    }

    /// Total duration (seconds) of the current buffer, or `default` if the lock fails.
    fn total_duration_or(&self, default: f32) -> f32 {
        self.controller.buffer.read().map(|b| b.duration_seconds()).unwrap_or(default)
    }
}

impl eframe::App for AuditionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Redraw continuously when playing for smooth 60fps playhead & VU meters
        if self.controller.is_playing.load(Ordering::Relaxed) {
            ctx.request_repaint();
        }

        // 창 위로 드래그 & 드롭한 오디오 파일 열기
        let dropped = ctx.input(|i| i.raw.dropped_files.iter().find_map(|f| f.path.clone()));
        if let Some(path) = dropped {
            self.open_path(path);
        }

        self.handle_shortcuts(ctx);

        // 1. Top Menu Bar (Korean)
        self.show_menu_bar(ctx);

        // 2. Bottom Transport Bar (Korean)
        self.show_transport_panel(ctx);

        // 3. Bottom Status Line
        self.show_status_line(ctx);

        // 4. Silence Insertion Modal Window
        if self.show_silence_dialog {
            self.show_silence_dialog_window(ctx);
        }

        // 5. Left Markers Panel (Audition Style)
        if self.show_marker_panel {
            self.show_marker_sidebar(ctx);
        }

        // 6. Main Waveform View Area
        self.show_waveform_area(ctx);
    }
}
