use egui::{
    pos2, vec2, Rect, Stroke, Ui,
};
use std::sync::atomic::Ordering;

use crate::audio::playback::PlaybackController;
use super::theme::AuditionTheme;
use super::waveform_view::format_precise_timestamp;

pub fn show_transport_bar(
    ui: &mut Ui,
    playback: &PlaybackController,
    total_duration: f32,
    selection_duration: Option<f32>,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        let is_playing = playback.is_playing.load(Ordering::Relaxed);
        let is_looping = playback.is_looping.load(Ordering::Relaxed);

        // Transport Buttons
        if is_playing {
            if ui.button("⏸ 일시정지 (Space)").clicked() {
                playback.pause();
            }
        } else {
            if ui.button("▶ 재생 (Space)").clicked() {
                playback.play();
            }
        }

        if ui.button("⏹ 정지").clicked() {
            playback.stop();
        }

        let loop_text = if is_looping { "🔁 루프: 켜짐" } else { "🔁 루프: 꺼짐" };
        if ui.selectable_label(is_looping, loop_text).clicked() {
            playback.set_looping(!is_looping);
        }

        ui.separator();

        // Time Counter
        let cur_sample = playback.current_sample.load(Ordering::Relaxed);
        let cur_time = if let Ok(buf) = playback.buffer.read() {
            buf.sample_to_time(cur_sample)
        } else {
            0.0
        };

        ui.label(
            egui::RichText::new(format!(
                "{} / {}",
                format_precise_timestamp(cur_time),
                format_precise_timestamp(total_duration)
            ))
            .monospace()
            .size(15.0)
            .color(AuditionTheme::WAVEFORM_PEAK),
        );

        if let Some(sel_len) = selection_duration {
            ui.label(
                egui::RichText::new(format!("선택 구간: {}", format_precise_timestamp(sel_len)))
                    .monospace()
                    .size(13.0)
                    .color(AuditionTheme::SELECTION_BORDER),
            );
        }

        ui.separator();

        // Volume Slider
        ui.label("볼륨:");
        let mut vol = playback.get_volume();
        if ui.add(egui::Slider::new(&mut vol, 0.0..=1.5).show_value(false)).changed() {
            playback.set_volume(vol);
        }

        ui.separator();

        // Stereo Level Meters
        let (peak_l, peak_r) = playback.get_peaks();
        let meter_width = 160.0;
        let meter_height = 20.0;
        let (meter_rect, _) = ui.allocate_exact_size(vec2(meter_width, meter_height), egui::Sense::hover());
        let painter = ui.painter_at(meter_rect);

        // Background
        painter.rect_filled(meter_rect, 1.0, AuditionTheme::METER_BG);
        painter.rect_stroke(meter_rect, 1.0, Stroke::new(1.0_f32, AuditionTheme::GRID_LINE));

        let half_h = meter_rect.height() * 0.5;

        // Channel L meter
        draw_meter_bar(&painter, Rect::from_min_size(meter_rect.min, vec2(meter_width, half_h - 1.0)), peak_l);
        // Channel R meter
        draw_meter_bar(
            &painter,
            Rect::from_min_size(pos2(meter_rect.min.x, meter_rect.min.y + half_h), vec2(meter_width, half_h - 1.0)),
            peak_r,
        );
    });
}

fn draw_meter_bar(painter: &egui::Painter, rect: Rect, linear_peak: f32) {
    // Convert to dB: 0.0 -> -inf, 1.0 -> 0dB
    let db = if linear_peak > 0.00001 {
        20.0 * linear_peak.log10()
    } else {
        -60.0
    };

    // Range: -60dB to +3dB
    let ratio = ((db + 60.0) / 63.0).clamp(0.0, 1.0);
    let fill_w = rect.width() * ratio;

    if fill_w > 0.0 {
        let fill_rect = Rect::from_min_size(rect.min, vec2(fill_w, rect.height()));
        let color = if db > -0.1 {
            AuditionTheme::METER_RED // Clipping
        } else if db > -6.0 {
            AuditionTheme::METER_YELLOW
        } else {
            AuditionTheme::METER_GREEN
        };
        painter.rect_filled(fill_rect, 0.0, color);
    }
}
