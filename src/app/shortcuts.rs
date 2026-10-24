use egui::{Key, Modifiers};

use super::AuditionApp;

impl AuditionApp {
    pub(super) fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let pressed = |mods: Modifiers, key: Key| ctx.input_mut(|i| i.consume_key(mods, key));

        // Space: Toggle Play / Pause
        if pressed(Modifiers::NONE, Key::Space) {
            self.controller.toggle_play();
        }

        // Home: Rewind to beginning
        if pressed(Modifiers::NONE, Key::Home) {
            self.controller.seek(0);
        }

        // M: Add Marker
        if pressed(Modifiers::NONE, Key::M) {
            self.add_marker();
        }

        // Ctrl+A: Select All
        if pressed(Modifiers::CTRL, Key::A) {
            self.select_all();
        }

        // Ctrl+Z: Undo
        if pressed(Modifiers::CTRL, Key::Z) {
            self.undo();
        }

        // Ctrl+Y: Redo
        if pressed(Modifiers::CTRL, Key::Y) {
            self.redo();
        }

        // Delete / Backspace: Delete Selection
        if pressed(Modifiers::NONE, Key::Delete) || pressed(Modifiers::NONE, Key::Backspace) {
            self.apply_delete();
        }

        // Ctrl+C: Copy
        if pressed(Modifiers::CTRL, Key::C) {
            self.apply_copy();
        }

        // Ctrl+X: Cut
        if pressed(Modifiers::CTRL, Key::X) {
            self.apply_cut();
        }

        // Ctrl+V: Paste
        if pressed(Modifiers::CTRL, Key::V) {
            self.apply_paste();
        }

        // Ctrl+O: Open
        if pressed(Modifiers::CTRL, Key::O) {
            self.open_file_dialog();
        }

        // Ctrl+S: Save
        if pressed(Modifiers::CTRL, Key::S) {
            self.save_file_dialog();
        }

        // Arrow Left / Right: Pan Viewport Left / Right
        let total_dur = self.total_duration_or(1.0);
        let pan_amt = (self.waveform_state.view_span() * 0.25).max(0.01);

        if pressed(Modifiers::NONE, Key::ArrowLeft) {
            self.waveform_state.pan(-pan_amt, total_dur);
        }
        if pressed(Modifiers::NONE, Key::ArrowRight) {
            self.waveform_state.pan(pan_amt, total_dur);
        }
    }
}
