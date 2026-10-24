use super::AuditionApp;
use crate::ui::theme::AuditionTheme;
use crate::ui::transport_bar::show_transport_bar;
use crate::ui::waveform_view::{format_precise_timestamp, WaveformView};

impl AuditionApp {
    pub(super) fn show_transport_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("transport_panel")
            .min_height(38.0)
            .show(ctx, |ui| {
                let buf_guard = self.controller.buffer.read().ok();
                let total_duration = buf_guard.as_ref().map(|b| b.duration_seconds()).unwrap_or(0.0);
                let sel_duration = self.waveform_state.selection.as_ref().map(|sel| {
                    let start_t = buf_guard.as_ref().map(|b| b.sample_to_time(sel.start)).unwrap_or(0.0);
                    let end_t = buf_guard.as_ref().map(|b| b.sample_to_time(sel.end)).unwrap_or(0.0);
                    (end_t - start_t).max(0.0)
                });

                show_transport_bar(ui, &self.controller, total_duration, sel_duration);
            });
    }

    pub(super) fn show_status_line(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_line").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&self.status_message).size(11.0).color(AuditionTheme::TEXT_MUTED));
            });
        });
    }

    pub(super) fn show_silence_dialog_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("🔇 묵음 삽입 (Insert Silence)")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("현재 재생헤드 위치에 삽입할 묵음 길이를 설정하세요:");
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label("길이 (초):");
                    ui.add(egui::DragValue::new(&mut self.silence_duration_sec).speed(0.1).range(0.05..=60.0).suffix(" 초"));
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("삽입 (확인)").clicked() {
                        let dur = self.silence_duration_sec;
                        self.apply_insert_silence(dur);
                        self.show_silence_dialog = false;
                    }
                    if ui.button("취소").clicked() {
                        self.show_silence_dialog = false;
                    }
                });
            });
    }

    pub(super) fn show_marker_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("markers_sidebar")
            .resizable(true)
            .default_width(220.0)
            .min_width(160.0)
            .max_width(360.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("🔖 마커 목록");
                    if ui.button("➕ 추가").clicked() {
                        self.add_marker();
                    }
                });
                ui.separator();

                let buf_guard = self.controller.buffer.read().ok();
                let mut delete_idx = None;
                let mut jump_sample = None;

                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.markers.is_empty() {
                        ui.label(egui::RichText::new("등록된 마커가 없습니다.\n(단축키 M으로 추가)").weak());
                    }

                    for (i, m) in self.markers.iter_mut().enumerate() {
                        let time_str = if let Some(buf) = &buf_guard {
                            format_precise_timestamp(buf.sample_to_time(m.start_sample))
                        } else {
                            "00:00:00.000".to_string()
                        };

                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                // Color swatch
                                let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 2.0, m.color);

                                ui.text_edit_singleline(&mut m.name);
                                if ui.small_button("🗑").on_hover_text("마커 삭제").clicked() {
                                    delete_idx = Some(i);
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(&time_str).monospace().size(11.0).weak());
                                if ui.small_button("이동").clicked() {
                                    jump_sample = Some(m.start_sample);
                                }
                            });
                        });
                    }
                });

                // Release the buffer read lock before mutating state below.
                drop(buf_guard);

                if let Some(idx) = delete_idx {
                    self.markers.remove(idx);
                    self.status_message = "마커가 삭제되었습니다.".to_string();
                }
                if let Some(sample) = jump_sample {
                    self.controller.seek(sample);
                }
            });
    }

    pub(super) fn show_waveform_area(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Ok(buf) = self.controller.buffer.read() {
                let mut wave_view = WaveformView::new(
                    &buf,
                    &self.controller,
                    &mut self.waveform_state,
                    &self.markers,
                );
                wave_view.show(ui);
            }
        });
    }
}
