use egui::{CursorIcon, PointerButton, Rect, Response, Ui};

use super::WaveformView;

impl<'a> WaveformView<'a> {
    pub(super) fn handle_input(
        &mut self,
        ui: &mut Ui,
        response: &Response,
        ruler_rect: Rect,
        wave_rect: Rect,
        total_duration: f32,
    ) {
        let view_span = self.state.view_span();
        let playhead_x = self.time_to_x(self.current_time(), wave_rect);

        // Cursor hint when hovering the playhead pin
        if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
            let near_playhead_x = (pos.x - playhead_x).abs() <= 10.0;
            let in_ruler_or_pin = pos.y >= ruler_rect.min.y && pos.y <= ruler_rect.max.y + 12.0;

            if near_playhead_x && in_ruler_or_pin {
                ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
            }
        }

        // 1. Mouse Wheel Zoom & Pan
        self.handle_wheel(ui, response, wave_rect, total_duration);

        // 2. Middle-click or Right-click Drag for Panning (Hand Tool style)
        self.handle_hand_tool(ui, response, wave_rect, total_duration);

        // 3. Playhead Marker Dragging VS Range Selection
        if response.drag_started_by(PointerButton::Primary) && !self.state.is_panning {
            if let Some(pos) = response.interact_pointer_pos() {
                let near_playhead_x = (pos.x - playhead_x).abs() <= 12.0;
                let in_ruler_or_pin = pos.y >= ruler_rect.min.y && pos.y <= ruler_rect.max.y + 10.0;

                if near_playhead_x || in_ruler_or_pin {
                    let sample = self.snapped_sample_at(pos.x, wave_rect);
                    self.state.is_dragging_playhead = true;
                    self.playback.seek(sample);
                } else {
                    let sample = self.pixel_to_sample(pos.x, wave_rect);
                    self.state.is_dragging_selection = true;
                    self.state.drag_start_sample = sample;
                    self.state.selection = Some(sample..sample);
                    self.playback.seek(sample);
                }
            }
        }

        // While dragging Playhead Marker
        if self.state.is_dragging_playhead {
            ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let sample = self.snapped_sample_at(pos.x, wave_rect);
                self.playback.seek(sample);
                self.edge_auto_scroll(pos.x, wave_rect, 15.0, view_span, total_duration);
            }
        }

        // While dragging Range Selection
        if self.state.is_dragging_selection && response.dragged_by(PointerButton::Primary) {
            if let Some(pos) = response.interact_pointer_pos() {
                let cur_sample = self.pixel_to_sample(pos.x, wave_rect);
                let start = self.state.drag_start_sample.min(cur_sample);
                let end = self.state.drag_start_sample.max(cur_sample);
                self.state.selection = Some(start..end);
                self.playback.set_loop_range(Some(start..end));
                self.edge_auto_scroll(pos.x, wave_rect, 10.0, view_span, total_duration);
            }
        }

        if response.drag_stopped() {
            self.state.is_dragging_playhead = false;
            self.state.is_dragging_selection = false;
            if let Some(sel) = &self.state.selection {
                if sel.start == sel.end {
                    self.state.selection = None;
                    self.playback.set_loop_range(None);
                }
            }
        }
    }

    /// Wheel = zoom at cursor, Shift+wheel / horizontal wheel = pan, Alt+wheel = vertical zoom.
    fn handle_wheel(&mut self, ui: &mut Ui, response: &Response, wave_rect: Rect, total_duration: f32) {
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
        if !(response.hovered() && (scroll_delta.y.abs() > 0.1 || scroll_delta.x.abs() > 0.1)) {
            return;
        }

        let modifiers = ui.input(|i| i.modifiers);
        let view_span = self.state.view_span();
        let mouse_x = ui.input(|i| i.pointer.hover_pos()).map(|p| p.x).unwrap_or(wave_rect.center().x);
        let mouse_ratio = ((mouse_x - wave_rect.min.x) / wave_rect.width()).clamp(0.0, 1.0);
        let mouse_sec = self.state.view_start_sec + mouse_ratio * view_span;

        if modifiers.alt {
            let factor = if scroll_delta.y > 0.0 { 1.15 } else { 0.87 };
            self.state.vertical_zoom = (self.state.vertical_zoom * factor).clamp(0.3, 10.0);
        } else if modifiers.shift || scroll_delta.x.abs() > 0.1 {
            let delta = if scroll_delta.x.abs() > 0.1 { -scroll_delta.x } else { -scroll_delta.y };
            let pan_sec = delta * (view_span / 600.0);
            self.state.pan(pan_sec, total_duration);
        } else {
            let zoom_factor = if scroll_delta.y > 0.0 { 0.82 } else { 1.22 };
            self.state.zoom_at(mouse_sec, zoom_factor, total_duration);
        }
    }

    /// Middle / right button drag pans the view like a hand tool.
    fn handle_hand_tool(&mut self, ui: &mut Ui, response: &Response, wave_rect: Rect, total_duration: f32) {
        if response.drag_started_by(PointerButton::Middle) || response.drag_started_by(PointerButton::Secondary) {
            if let Some(pos) = response.interact_pointer_pos() {
                self.state.is_panning = true;
                self.state.pan_start_x = pos.x;
                self.state.pan_start_view_start = self.state.view_start_sec;
            }
        }

        if self.state.is_panning {
            let view_span = self.state.view_span();
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let dx = pos.x - self.state.pan_start_x;
                let dt = -dx / wave_rect.width() * view_span;
                let target_start = (self.state.pan_start_view_start + dt).clamp(0.0, total_duration - view_span);
                self.state.view_start_sec = target_start;
                self.state.view_end_sec = target_start + view_span;
            }
            if !ui.input(|i| i.pointer.button_down(PointerButton::Middle) || i.pointer.button_down(PointerButton::Secondary)) {
                self.state.is_panning = false;
            }
        }
    }

    /// Sample under `pixel_x`, snapped to the nearest BPM beat when beat snapping is enabled.
    fn snapped_sample_at(&self, pixel_x: f32, wave_rect: Rect) -> usize {
        let sample = self.pixel_to_sample(pixel_x, wave_rect);
        let bpm = &self.state.bpm_settings;
        if bpm.enabled && bpm.snap_to_beat {
            let t = self.buffer.sample_to_time(sample) as f64;
            let snapped_t = bpm.snap_time(t);
            self.buffer.time_to_sample(snapped_t as f32)
        } else {
            sample
        }
    }

    /// Auto-scrolls the view when dragging within `margin` px of the waveform edges.
    fn edge_auto_scroll(&mut self, pointer_x: f32, wave_rect: Rect, margin: f32, view_span: f32, total_duration: f32) {
        if pointer_x < wave_rect.min.x + margin {
            self.state.pan(-view_span * 0.05, total_duration);
        } else if pointer_x > wave_rect.max.x - margin {
            self.state.pan(view_span * 0.05, total_duration);
        }
    }
}
