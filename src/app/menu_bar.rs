use super::AuditionApp;
use crate::audio::buffer::AudioBuffer;
use crate::ui::theme::AuditionTheme;

impl AuditionApp {
    pub(super) fn show_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("파일 (F)", |ui| self.file_menu(ui));
                ui.menu_button("편집 (E)", |ui| self.edit_menu(ui));
                ui.menu_button("효과 / DSP (C)", |ui| self.effects_menu(ui));
                ui.menu_button("보기 (V)", |ui| self.view_menu(ui));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("Audition-RS (Rust 오디오 에디터)")
                            .italics()
                            .color(AuditionTheme::TEXT_MUTED),
                    );
                });
            });
        });
    }

    fn file_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("📂 오디오 파일 열기... (Ctrl+O)").clicked() {
            self.open_file_dialog();
            ui.close_menu();
        }
        if ui.button("💾 WAV 파일로 저장 (Ctrl+S)").clicked() {
            self.save_file_dialog();
            ui.close_menu();
        }
        ui.separator();
        if ui.button("🎵 신스 데모 트랙 생성").clicked() {
            self.push_undo();
            let demo = AudioBuffer::generate_demo_track(44100, 20.0);
            let duration = demo.duration_seconds();
            self.controller.replace_buffer(demo);
            self.markers.clear();
            self.waveform_state.reset_view_to_fit(duration);
            self.status_message = "새로운 신스 데모 트랙이 생성되었습니다.".to_string();
            ui.close_menu();
        }
        ui.separator();
        if ui.button("❌ 종료 (Exit)").clicked() {
            std::process::exit(0);
        }
    }

    fn edit_menu(&mut self, ui: &mut egui::Ui) {
        if ui.add_enabled(!self.undo_stack.is_empty(), egui::Button::new("실행 취소 (Ctrl+Z)")).clicked() {
            self.undo();
            ui.close_menu();
        }
        if ui.add_enabled(!self.redo_stack.is_empty(), egui::Button::new("다시 실행 (Ctrl+Y)")).clicked() {
            self.redo();
            ui.close_menu();
        }
        ui.separator();
        if ui.button("잘라내기 (Ctrl+X)").clicked() {
            self.apply_cut();
            ui.close_menu();
        }
        if ui.button("복사 (Ctrl+C)").clicked() {
            self.apply_copy();
            ui.close_menu();
        }
        if ui.button("붙여넣기 (Ctrl+V)").clicked() {
            self.apply_paste();
            ui.close_menu();
        }
        if ui.button("삭제 (Delete)").clicked() {
            self.apply_delete();
            ui.close_menu();
        }
        if ui.button("전체 선택 (Ctrl+A)").clicked() {
            // NOTE: unlike the Ctrl+A shortcut, the menu item does not set the loop range (original behaviour).
            if let Ok(guard) = self.controller.buffer.read() {
                let total = guard.num_samples();
                self.waveform_state.selection = Some(0..total);
            }
            ui.close_menu();
        }
        ui.separator();
        if ui.button("➕ 마커 추가 (M)").clicked() {
            self.add_marker();
            ui.close_menu();
        }
        if ui.button("🔇 묵음 삽입... (Insert Silence)").clicked() {
            self.show_silence_dialog = true;
            ui.close_menu();
        }
    }

    fn effects_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("정규화 노멀라이즈 (-0.1 dB)").clicked() {
            self.apply_normalize();
            ui.close_menu();
        }
        if ui.button("선택 구간 묵음화 (Silence Selection)").clicked() {
            self.apply_silence_selection();
            ui.close_menu();
        }
        if ui.button("볼륨 증폭 (+3 dB)").clicked() {
            self.apply_gain(1.4125);
            ui.close_menu();
        }
        if ui.button("볼륨 감쇄 (-3 dB)").clicked() {
            self.apply_gain(0.7079);
            ui.close_menu();
        }
        ui.separator();
        if ui.button("페이드 인 (Fade In)").clicked() {
            self.apply_fade(true);
            ui.close_menu();
        }
        if ui.button("페이드 아웃 (Fade Out)").clicked() {
            self.apply_fade(false);
            ui.close_menu();
        }
    }

    fn view_menu(&mut self, ui: &mut egui::Ui) {
        let total_dur = self.total_duration_or(1.0);
        if ui.button("🔍 전체 화면 맞추기 (Zoom All)").clicked() {
            self.waveform_state.reset_view_to_fit(total_dur);
            ui.close_menu();
        }
        if ui.button("🔍+ 확대 (Zoom In)").clicked() {
            let center = self.waveform_state.view_center();
            self.waveform_state.zoom_at(center, 0.7, total_dur);
            ui.close_menu();
        }
        if ui.button("🔍- 축소 (Zoom Out)").clicked() {
            let center = self.waveform_state.view_center();
            self.waveform_state.zoom_at(center, 1.4, total_dur);
            ui.close_menu();
        }
        if let Some(sel) = self.waveform_state.selection.clone().filter(|s| s.end > s.start) {
            if ui.button("🔍 선택 영역 확대 (Zoom Selection)").clicked() {
                if let Ok(buf) = self.controller.buffer.read() {
                    self.waveform_state.view_start_sec = buf.sample_to_time(sel.start);
                    self.waveform_state.view_end_sec = buf.sample_to_time(sel.end);
                }
                ui.close_menu();
            }
        }
        ui.separator();
        if ui.checkbox(&mut self.show_marker_panel, "마커 목록 패널 (Markers)").clicked() {
            ui.close_menu();
        }
    }
}
