use super::AuditionApp;
use crate::audio::buffer::AudioBuffer;
use crate::audio::marker::Marker;

/// Maximum number of undo snapshots kept in memory.
const MAX_UNDO: usize = 30;

impl AuditionApp {
    pub(super) fn push_undo(&mut self) {
        if let Ok(guard) = self.controller.buffer.read() {
            self.undo_stack.push((guard.clone(), self.markers.clone()));
            self.redo_stack.clear();
            if self.undo_stack.len() > MAX_UNDO {
                self.undo_stack.remove(0);
            }
        }
    }

    pub(super) fn undo(&mut self) {
        if let Some(snapshot) = self.undo_stack.pop() {
            if let Ok(cur) = self.controller.buffer.read() {
                self.redo_stack.push((cur.clone(), self.markers.clone()));
            }
            self.restore_snapshot(snapshot);
            self.status_message = "실행 취소(Undo) 완료".to_string();
        }
    }

    pub(super) fn redo(&mut self) {
        if let Some(snapshot) = self.redo_stack.pop() {
            if let Ok(cur) = self.controller.buffer.read() {
                self.undo_stack.push((cur.clone(), self.markers.clone()));
            }
            self.restore_snapshot(snapshot);
            self.status_message = "다시 실행(Redo) 완료".to_string();
        }
    }

    fn restore_snapshot(&mut self, (buf, markers): (AudioBuffer, Vec<Marker>)) {
        let duration = buf.duration_seconds();
        self.controller.replace_buffer(buf);
        self.markers = markers;
        self.waveform_state.reset_view_to_fit(duration);
    }
}
