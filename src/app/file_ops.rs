use super::AuditionApp;
use crate::audio::io::{load_audio_file, save_wav};
use std::path::PathBuf;

impl AuditionApp {
    pub(super) fn open_file_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("모든 지원 오디오 (*.wav;*.ogg;*.mp3;*.flac;*.m4a)", &["wav", "ogg", "mp3", "flac", "m4a", "aac"])
            .add_filter("WAV 무손실 오디오 (*.wav)", &["wav"])
            .add_filter("OGG Vorbis 오디오 (*.ogg)", &["ogg"])
            .add_filter("MP3 오디오 (*.mp3)", &["mp3"])
            .add_filter("FLAC 무손실 오디오 (*.flac)", &["flac"])
            .pick_file()
        else {
            return;
        };

        self.open_path(path);
    }

    /// Loads an audio file from disk (used by the open dialog, command-line
    /// arguments from Windows "Open with" / file association, and drag & drop).
    pub(super) fn open_path(&mut self, path: PathBuf) {
        match load_audio_file(&path) {
            Ok(buf) => {
                let duration = buf.duration_seconds();
                let num_ch = buf.num_channels();
                let rate = buf.sample_rate;
                self.controller.stop();
                self.controller.replace_buffer(buf);
                self.undo_stack.clear();
                self.redo_stack.clear();
                self.markers.clear();
                self.marker_counter = 1;
                self.waveform_state.reset_view_to_fit(duration);
                self.waveform_state.selection = None;
                self.status_message = format!(
                    "파일 로드 완료: {:?} ({} Hz, {} 채널, {:.2}초)",
                    path.file_name().unwrap_or_default(),
                    rate,
                    num_ch,
                    duration
                );
                self.current_filepath = Some(path);
            }
            Err(e) => {
                self.status_message = format!("파일 열기 실패: {}", e);
            }
        }
    }

    pub(super) fn save_file_dialog(&mut self) {
        let dest = self.current_filepath.clone().or_else(|| {
            rfd::FileDialog::new()
                .add_filter("WAV 오디오 (*.wav)", &["wav"])
                .set_file_name("내보내기.wav")
                .save_file()
        });

        let Some(path) = dest else {
            return;
        };
        if let Ok(guard) = self.controller.buffer.read() {
            match save_wav(&path, &guard) {
                Ok(_) => {
                    self.status_message = format!("WAV 파일 저장 완료: {:?}", path.file_name().unwrap());
                    self.current_filepath = Some(path);
                }
                Err(e) => {
                    self.status_message = format!("저장 실패: {}", e);
                }
            }
        }
    }
}
