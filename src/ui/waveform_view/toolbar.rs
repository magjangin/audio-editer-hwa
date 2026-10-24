use egui::{Color32, Ui};

use super::WaveformView;
use crate::ui::theme::AuditionTheme;

impl<'a> WaveformView<'a> {
    pub(super) fn draw_zoom_toolbar(&mut self, ui: &mut Ui, total_duration: f32) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            if ui.button("🔍 전체 보기").on_hover_text("곡 전체를 한눈에 볼 수 있도록 확대/축소를 리셋합니다").clicked() {
                self.state.reset_view_to_fit(total_duration);
            }

            if ui.button("🔍+ 확대").on_hover_text("현재 화면 중앙을 기준으로 줌 인합니다").clicked() {
                let center = self.state.view_center();
                self.state.zoom_at(center, 0.7, total_duration);
            }

            if ui.button("🔍- 축소").on_hover_text("현재 화면 중앙을 기준으로 줌 아웃합니다").clicked() {
                let center = self.state.view_center();
                self.state.zoom_at(center, 1.4, total_duration);
            }

            if let Some(sel) = &self.state.selection {
                if sel.end > sel.start {
                    if ui.button("🔍 선택 영역 확대").on_hover_text("선택한 구간에 맞추어 확대합니다").clicked() {
                        let s_t = self.buffer.sample_to_time(sel.start);
                        let e_t = self.buffer.sample_to_time(sel.end);
                        self.state.view_start_sec = s_t.max(0.0);
                        self.state.view_end_sec = e_t.min(total_duration);
                    }
                }
            }

            ui.separator();

            // Pan/Move Buttons with Hold-to-Scroll support (꾹 누르면 휠처럼 연속 이동)
            self.draw_pan_buttons(ui, total_duration);

            ui.separator();

            // BPM Grid Toggle Button
            let bpm_btn_text = if self.state.bpm_settings.enabled { "🎵 BPM 그리드: 켜짐" } else { "🎵 BPM 그리드: 꺼짐" };
            if ui.selectable_label(self.state.bpm_settings.enabled, bpm_btn_text).on_hover_text("BPM 마디/박자 선을 파형 위에 표시합니다").clicked() {
                self.state.bpm_settings.enabled = !self.state.bpm_settings.enabled;
            }

            ui.separator();

            ui.label("수직 진폭:");
            ui.add(
                egui::Slider::new(&mut self.state.vertical_zoom, 0.5..=5.0)
                    .show_value(true)
                    .suffix("x")
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let span = self.state.view_span();
                let span_str = if span < 1.0 {
                    format!("{:.1} ms 표시 중", span * 1000.0)
                } else {
                    format!("{:.2} 초 표시 중", span)
                };
                ui.label(egui::RichText::new(span_str).size(11.0).color(AuditionTheme::TEXT_MUTED));
            });
        });
    }

    /// `◀ 이동` / `이동 ▶` buttons: click = step pan, hold = continuous jog-wheel scroll.
    fn draw_pan_buttons(&mut self, ui: &mut Ui, total_duration: f32) {
        let view_span = self.state.view_span();
        let btn_left = ui.button("◀ 이동").on_hover_text("화면을 왼쪽으로 이동합니다 (꾹 누르고 있으면 휠처럼 연속 이동)");
        let btn_right = ui.button("이동 ▶").on_hover_text("화면을 오른쪽으로 이동합니다 (꾹 누르고 있으면 휠처럼 연속 이동)");

        let dt = ui.input(|i| i.stable_dt).clamp(0.001, 0.05);
        let scroll_speed = (view_span * 2.2).max(0.1);
        let continuous_delta = scroll_speed * dt;
        let click_step = (view_span * 0.3).max(0.05);

        for (btn, dir) in [(btn_left, -1.0f32), (btn_right, 1.0f32)] {
            if btn.clicked() {
                self.state.pan(dir * click_step, total_duration);
            } else if btn.is_pointer_button_down_on() {
                self.state.pan(dir * continuous_delta, total_duration);
                ui.ctx().request_repaint();
            }
        }
    }

    pub(super) fn draw_bpm_controls(&mut self, ui: &mut Ui, total_duration: f32) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            ui.label(egui::RichText::new("BPM:").color(Color32::from_rgb(245, 158, 11)).strong());
            ui.add(egui::DragValue::new(&mut self.state.bpm_settings.bpm).speed(0.1).range(10.0..=999.0));

            ui.separator();

            ui.label("박자 분할:");
            egui::ComboBox::from_id_salt("beat_div_combo")
                .selected_text(format!("1/{} 박", self.state.bpm_settings.beat_division))
                .show_ui(ui, |ui| {
                    const DIVISIONS: &[(usize, &str)] = &[
                        (1, "1/1 (마디선만)"),
                        (2, "1/2 박"),
                        (4, "1/4 박 (기본)"),
                        (8, "1/8 박"),
                        (16, "1/16 박"),
                        (32, "1/32 박"),
                        (64, "1/64 박"),
                    ];
                    for &(div, label) in DIVISIONS {
                        ui.selectable_value(&mut self.state.bpm_settings.beat_division, div, label);
                    }
                });

            ui.separator();

            ui.label("오프셋(ms):");
            ui.add(egui::DragValue::new(&mut self.state.bpm_settings.offset_ms).speed(1.0).suffix(" ms"));

            if ui.button("📍 현재 위치를 0마디로 설정").on_hover_text("현재 재생헤드 위치를 BPM 그리드의 시작 오프셋으로 설정합니다").clicked() {
                let cur_time = self.current_time();
                self.state.bpm_settings.offset_ms = (cur_time * 1000.0) as f64;
            }

            ui.separator();

            // Snap Toggle
            ui.checkbox(&mut self.state.bpm_settings.snap_to_beat, "🧲 비트 자석 스냅");

            // Jump to Prev / Next Beat
            let cur_time = self.current_time() as f64;

            if ui.button("⏮ 이전 비트").on_hover_text("이전 BPM 비트 위치로 이동합니다").clicked() {
                let prev_t = self.state.bpm_settings.prev_tick_time(cur_time);
                self.jump_to_beat(prev_t, total_duration);
            }

            if ui.button("다음 비트 ⏭").on_hover_text("다음 BPM 비트 위치로 이동합니다").clicked() {
                let next_t = self.state.bpm_settings.next_tick_time(cur_time);
                self.jump_to_beat(next_t, total_duration);
            }
        });
    }

    /// Seeks the playhead to `beat_time` and centers the viewport around it.
    fn jump_to_beat(&mut self, beat_time: f64, total_duration: f32) {
        let sample = self.buffer.time_to_sample(beat_time as f32);
        self.playback.seek(sample);

        // Center view around target beat
        let span = self.state.view_span();
        let new_start = (beat_time as f32 - span * 0.5).clamp(0.0, total_duration - span);
        self.state.view_start_sec = new_start;
        self.state.view_end_sec = new_start + span;
    }
}
