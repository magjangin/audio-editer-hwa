use std::ops::Range;
use std::sync::atomic::Ordering;

use super::AuditionApp;
use crate::audio::buffer::AudioBuffer;
use crate::audio::marker::Marker;

impl AuditionApp {
    // ---- Markers ----

    pub(super) fn add_marker(&mut self) {
        let cur_sample = self.current_sample();
        let sel = self.waveform_state.selection.clone();

        let (start, duration) = match sel {
            Some(range) if range.end > range.start => (range.start, range.end - range.start),
            _ => (cur_sample, 0),
        };

        let id = self.marker_counter;
        self.marker_counter += 1;
        let name = format!("마커 {:02}", id);
        let marker = Marker::new(id, name, start, duration);
        self.markers.push(marker);
        self.markers.sort_by_key(|m| m.start_sample);
        self.status_message = format!("새 마커(ID: {})가 추가되었습니다.", id);
    }


    // ---- Selection helpers ----

    fn current_sample(&self) -> usize {
        self.controller.current_sample.load(Ordering::Relaxed)
    }

    /// Returns the current selection only if it is a non-empty range.
    fn non_empty_selection(&self) -> Option<Range<usize>> {
        self.waveform_state.selection.clone().filter(|sel| sel.end > sel.start)
    }

    /// Ctrl+A behaviour: select the whole buffer and loop it.
    pub(super) fn select_all(&mut self) {
        if let Ok(guard) = self.controller.buffer.read() {
            let total = guard.num_samples();
            if total > 0 {
                self.waveform_state.selection = Some(0..total);
                self.controller.set_loop_range(Some(0..total));
            }
        }
    }

    // ---- Clipboard edits ----

    pub(super) fn apply_delete(&mut self) {
        let Some(sel) = self.non_empty_selection() else {
            return;
        };
        self.push_undo();
        if let Ok(mut guard) = self.controller.buffer.write() {
            let len = sel.end - sel.start;
            guard.delete_range(sel.clone());
            let duration = guard.duration_seconds();
            drop(guard);

            // Adjust markers affected by deletion
            self.markers.retain_mut(|m| {
                if m.start_sample >= sel.end {
                    m.start_sample -= len;
                    true
                } else if m.start_sample >= sel.start && m.start_sample < sel.end {
                    false // Marker fell in deleted zone
                } else {
                    true
                }
            });

            self.waveform_state.selection = None;
            self.waveform_state.reset_view_to_fit(duration);
            self.controller.seek(sel.start);
            self.status_message = "선택 영역이 삭제되었습니다.".to_string();
        }
    }

    pub(super) fn apply_copy(&mut self) {
        let Some(sel) = self.non_empty_selection() else {
            return;
        };
        if let Ok(guard) = self.controller.buffer.read() {
            let clip = guard.copy_range(sel);
            self.clipboard = Some(clip);
            self.status_message = "선택 영역이 클립보드에 복사되었습니다.".to_string();
        }
    }

    pub(super) fn apply_cut(&mut self) {
        let Some(sel) = self.non_empty_selection() else {
            return;
        };
        self.push_undo();
        if let Ok(mut guard) = self.controller.buffer.write() {
            let clip = guard.cut_range(sel.clone());
            self.clipboard = Some(clip);
            let duration = guard.duration_seconds();
            drop(guard);
            self.waveform_state.selection = None;
            self.waveform_state.reset_view_to_fit(duration);
            self.controller.seek(sel.start);
            self.status_message = "선택 영역을 잘라내어 클립보드에 저장했습니다.".to_string();
        }
    }

    pub(super) fn apply_paste(&mut self) {
        let Some(clip) = self.clipboard.clone() else {
            return;
        };
        self.push_undo();
        let pos = self.current_sample();
        let clip_len = clip.num_samples();
        if let Ok(mut guard) = self.controller.buffer.write() {
            guard.paste_at(pos, &clip);
            let duration = guard.duration_seconds();
            drop(guard);

            // Shift markers behind paste position
            shift_markers_after(&mut self.markers, pos, clip_len);

            self.waveform_state.reset_view_to_fit(duration);
            self.controller.seek(pos + clip_len);
            self.status_message = "클립보드 내용을 붙여넣었습니다.".to_string();
        }
    }

    pub(super) fn apply_insert_silence(&mut self, duration_sec: f32) {
        if duration_sec <= 0.0 {
            return;
        }
        self.push_undo();
        let pos = self.current_sample();
        if let Ok(mut guard) = self.controller.buffer.write() {
            let sample_count = (duration_sec * guard.sample_rate as f32).round() as usize;
            guard.insert_silence(pos, sample_count);
            let total_dur = guard.duration_seconds();
            drop(guard);

            // Shift markers behind inserted silence
            shift_markers_after(&mut self.markers, pos, sample_count);

            self.waveform_state.reset_view_to_fit(total_dur);
            self.status_message = format!("{:.2}초 길이의 묵음을 삽입했습니다.", duration_sec);
        }
    }

    // ---- DSP edits (apply to selection, or whole buffer if nothing is selected) ----

    /// Pushes an undo snapshot and runs `op` on the selection (or the whole buffer).
    /// `status` is shown only if the buffer lock succeeded.
    fn apply_dsp(&mut self, status: String, op: impl FnOnce(&mut AudioBuffer, Range<usize>)) {
        let sel = self.waveform_state.selection.clone();
        self.push_undo();
        if let Ok(mut guard) = self.controller.buffer.write() {
            let total = guard.num_samples();
            let range = sel.unwrap_or(0..total);
            op(&mut guard, range);
            self.status_message = status;
        }
    }

    pub(super) fn apply_gain(&mut self, factor: f32) {
        self.apply_dsp(format!("볼륨 게인 적용 완료: {:.2}x", factor), |buf, range| {
            buf.apply_gain_range(range, factor)
        });
    }

    pub(super) fn apply_silence_selection(&mut self) {
        self.apply_dsp("선택 구간을 묵음(음소거) 처리했습니다.".to_string(), |buf, range| {
            buf.silence_range(range)
        });
    }

    pub(super) fn apply_normalize(&mut self) {
        self.apply_dsp("노멀라이즈(-0.1 dB 피크) 적용 완료".to_string(), |buf, range| {
            buf.normalize_range(range, 0.99) // -0.1 dB peak
        });
    }

    pub(super) fn apply_fade(&mut self, fade_in: bool) {
        let status = if fade_in { "페이드 인 적용 완료" } else { "페이드 아웃 적용 완료" };
        self.apply_dsp(status.to_string(), |buf, range| buf.fade(range, fade_in));
    }
}

/// Shifts every marker at or after `pos` forward by `len` samples (after an insertion).
fn shift_markers_after(markers: &mut [Marker], pos: usize, len: usize) {
    for m in markers {
        if m.start_sample >= pos {
            m.start_sample += len;
        }
    }
}