use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Rect, Stroke};

use super::{format_ms_timestamp, format_timestamp, WaveformView};
use crate::ui::theme::AuditionTheme;

/// Candidate major tick steps (seconds) for the dynamic no-overlap ruler (0.1ms ~ 1h).
const CANDIDATE_STEPS: &[f32] = &[
    0.0001, 0.0002, 0.0005,
    0.001, 0.002, 0.005,
    0.01, 0.02, 0.05,
    0.1, 0.2, 0.5,
    1.0, 2.0, 5.0, 10.0, 15.0, 30.0,
    60.0, 120.0, 300.0, 600.0, 1800.0, 3600.0,
];

/// Minimum horizontal spacing between ruler labels (pixels).
const MIN_LABEL_PX: f32 = 96.0;

const MEASURE_COLOR: Color32 = Color32::from_rgb(245, 158, 11); // #F59E0B Amber Gold

impl<'a> WaveformView<'a> {
    pub(super) fn draw_ruler(&self, painter: &Painter, rect: Rect, _sample_rate: u32) {
        painter.rect_filled(rect, 0.0, AuditionTheme::BG_PANEL);
        painter.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0_f32, AuditionTheme::GRID_LINE));

        let view_span = self.state.view_span();
        if view_span <= 0.0 || rect.width() <= 10.0 {
            return;
        }

        let px_per_sec = rect.width() / view_span;
        let min_step_sec = MIN_LABEL_PX / px_per_sec;

        let major_step = CANDIDATE_STEPS
            .iter()
            .copied()
            .find(|&c| c >= min_step_sec)
            .unwrap_or(60.0);

        let sub_divisions = if major_step >= 60.0 {
            4
        } else if (major_step * 10.0).fract() == 0.0 && major_step >= 0.1 {
            5
        } else {
            2
        };

        let sub_step = major_step / sub_divisions as f32;

        let start_major = (self.state.view_start_sec / major_step).floor() as i64;
        let end_major = (self.state.view_end_sec / major_step).ceil() as i64;

        let start_sub = (self.state.view_start_sec / sub_step).floor() as i64;
        let end_sub = (self.state.view_end_sec / sub_step).ceil() as i64;

        for sub_idx in start_sub..=end_sub {
            let t = sub_idx as f32 * sub_step;
            if t < self.state.view_start_sec || t > self.state.view_end_sec {
                continue;
            }
            let x = self.time_to_x(t, rect);
            painter.line_segment(
                [pos2(x, rect.max.y - 3.5), pos2(x, rect.max.y)],
                Stroke::new(1.0_f32, Color32::from_rgb(50, 60, 55)),
            );
        }

        for tick in start_major..=end_major {
            let t = tick as f32 * major_step;
            if t < self.state.view_start_sec || t > self.state.view_end_sec {
                continue;
            }

            let x = self.time_to_x(t, rect);

            painter.line_segment(
                [pos2(x, rect.max.y - 7.0), pos2(x, rect.max.y)],
                Stroke::new(1.0_f32, AuditionTheme::TEXT_MUTED),
            );

            let label = if view_span < 2.5 {
                format_ms_timestamp(t)
            } else {
                format_timestamp(t)
            };

            painter.text(
                pos2(x + 3.0, rect.min.y + 4.0),
                Align2::LEFT_TOP,
                label,
                FontId::proportional(11.0),
                AuditionTheme::TEXT_BRIGHT,
            );
        }
    }

    /// Draw BPM Measure and Beat Grid (From offset tool: measure #F59E0B, beat #38BDF8)
    pub(super) fn draw_bpm_grid(&self, painter: &Painter, ruler_rect: Rect, wave_rect: Rect, total_duration: f32) {
        let bpm = self.state.bpm_settings.bpm;
        if bpm <= 0.0 {
            return;
        }

        let division = self.state.bpm_settings.beat_division.max(1);
        let tick_interval = self.state.bpm_settings.tick_interval_sec();
        let start_offset_sec = self.state.bpm_settings.offset_ms / 1000.0;

        let view_span = self.state.view_span() as f64;
        if view_span <= 0.0 {
            return;
        }

        let view_start = self.state.view_start_sec as f64;
        let view_end = self.state.view_end_sec as f64;

        // Determine range of visible ticks
        let first_tick_idx = ((view_start - start_offset_sec) / tick_interval).floor() as i64 - 1;
        let last_tick_idx = ((view_end - start_offset_sec) / tick_interval).ceil() as i64 + 1;

        let measure_stroke = Stroke::new(1.8_f32, MEASURE_COLOR);
        let beat_stroke = Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(56, 189, 248, 160)); // #38BDF8 Sky

        let mut drawn_any = false;

        for tick_idx in first_tick_idx..=last_tick_idx {
            let t = start_offset_sec + tick_idx as f64 * tick_interval;
            if t < 0.0 || t > (total_duration as f64 + 2.0) {
                continue;
            }
            if t < view_start || t > view_end {
                continue;
            }

            drawn_any = true;
            let x = wave_rect.min.x + (((t - view_start) / view_span) as f32) * wave_rect.width();
            let is_measure = (tick_idx % division as i64) == 0;

            if is_measure {
                let measure_num = tick_idx / division as i64;
                // Full vertical measure line across ruler and waveforms
                painter.line_segment([pos2(x, ruler_rect.min.y), pos2(x, wave_rect.max.y)], measure_stroke);

                // Measure badge label in ruler
                let badge_text = format!("#{:03}", measure_num);
                let badge_bg = Rect::from_min_size(pos2(x + 2.0, ruler_rect.min.y + 1.0), vec2(38.0, 13.0));
                painter.rect_filled(badge_bg, 2.0, Color32::from_rgb(40, 25, 0));
                painter.rect_stroke(badge_bg, 2.0, Stroke::new(1.0_f32, MEASURE_COLOR));
                painter.text(
                    pos2(x + 4.0, ruler_rect.min.y + 2.0),
                    Align2::LEFT_TOP,
                    badge_text,
                    FontId::monospace(10.0),
                    Color32::from_rgb(251, 191, 36),
                );
            } else {
                // Beat sub-division line across waveforms
                painter.line_segment([pos2(x, wave_rect.min.y), pos2(x, wave_rect.max.y)], beat_stroke);
            }
        }

        // If zoomed in so close that no tick is in the 30ms window, show directional clue badges
        if !drawn_any {
            let prev_t = self.state.bpm_settings.prev_tick_time(view_start);
            let next_t = self.state.bpm_settings.next_tick_time(view_end);

            let msg = format!(
                "BPM 비트 안내: 이전 비트 {:.4}s ← [현재 화면] → 다음 비트 {:.4}s",
                prev_t, next_t
            );
            painter.text(
                pos2(wave_rect.center().x, wave_rect.min.y + 15.0),
                Align2::CENTER_CENTER,
                msg,
                FontId::proportional(11.5),
                MEASURE_COLOR,
            );
        }
    }
}
