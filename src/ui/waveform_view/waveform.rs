use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Stroke};

use super::WaveformView;
use crate::audio::buffer::PeakSample;
use crate::ui::theme::AuditionTheme;

/// Audition crisp neon emerald
const WAVE_LINE_COLOR: Color32 = Color32::from_rgb(60, 240, 150);

impl<'a> WaveformView<'a> {
    pub(super) fn draw_waveforms(&self, painter: &Painter, rect: Rect, total_samples: usize) {
        painter.rect_filled(rect, 0.0, AuditionTheme::BG_WAVEFORM);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, AuditionTheme::GRID_LINE));

        let num_ch = self.buffer.num_channels().max(1);
        let ch_height = rect.height() / num_ch as f32;
        let view_start_sample = self.buffer.time_to_sample(self.state.view_start_sec);
        let view_end_sample = self.buffer.time_to_sample(self.state.view_end_sec);
        let visible_samples = view_end_sample.saturating_sub(view_start_sample);

        let width_px = rect.width().max(1.0) as usize;
        let v_zoom = self.state.vertical_zoom;

        for ch in 0..num_ch {
            let ch_rect = Rect::from_min_size(
                pos2(rect.min.x, rect.min.y + ch as f32 * ch_height),
                vec2(rect.width(), ch_height),
            );

            if ch > 0 {
                painter.line_segment(
                    [ch_rect.left_top(), ch_rect.right_top()],
                    Stroke::new(1.0_f32, AuditionTheme::GRID_LINE),
                );
            }

            let center_y = ch_rect.center().y;
            let half_h = (ch_rect.height() - 4.0) * 0.5 * v_zoom;

            // Zero crossing center line
            painter.line_segment(
                [pos2(ch_rect.min.x, center_y), pos2(ch_rect.max.x, center_y)],
                Stroke::new(1.0_f32, Color32::from_rgb(120, 50, 50)),
            );

            let ch_label = if ch == 0 { "L (좌)" } else { "R (우)" };
            painter.text(
                pos2(ch_rect.min.x + 8.0, ch_rect.min.y + 6.0),
                Align2::LEFT_TOP,
                ch_label,
                FontId::proportional(12.0),
                AuditionTheme::WAVEFORM_RMS,
            );

            if total_samples == 0 || visible_samples == 0 {
                continue;
            }

            let ch_data = match self.buffer.channels.get(ch) {
                Some(c) => c,
                None => continue,
            };

            let geo = ChannelGeometry {
                rect: ch_rect,
                center_y,
                half_h,
            };

            if visible_samples <= width_px * 16 {
                // Mode 1: Pure Oscilloscope Connected Waveform
                let samples_per_pixel = visible_samples as f32 / rect.width();
                draw_oscilloscope(painter, &geo, ch_data, view_start_sample, view_end_sample, samples_per_pixel);
            } else {
                let peaks = self.buffer.get_peaks(ch, view_start_sample, view_end_sample, width_px);
                if visible_samples <= width_px * 80 {
                    // Mode 2: Medium Zoom - Fast Oscilloscope Min-Max Peak Trace
                    draw_peak_trace(painter, &geo, rect.min.x, &peaks);
                } else {
                    // Mode 3: Overview Zoom - Envelope Outline + Soft Translucent Fill
                    draw_envelope(painter, &geo, rect.min.x, &peaks);
                }
            }
        }
    }

    pub(super) fn draw_db_scale(&self, painter: &Painter, rect: Rect) {
        painter.rect_filled(rect, 0.0, AuditionTheme::BG_PANEL);
        painter.line_segment([rect.left_top(), rect.left_bottom()], Stroke::new(1.0_f32, AuditionTheme::GRID_LINE));

        let num_ch = self.buffer.num_channels().max(1);
        let ch_h = rect.height() / num_ch as f32;
        let v_zoom = self.state.vertical_zoom;

        let db_levels: &[(f32, &str)] = &[
            (0.0, " 0"),
            (-3.0, "-3"),
            (-6.0, "-6"),
            (-12.0, "-12"),
            (-24.0, "-24"),
        ];

        let draw_label = |y: f32, text: &str| {
            painter.text(
                pos2(rect.min.x + 4.0, y - 5.0),
                Align2::LEFT_TOP,
                text,
                FontId::proportional(9.0),
                AuditionTheme::TEXT_MUTED,
            );
            painter.line_segment([pos2(rect.min.x, y), pos2(rect.min.x + 3.0, y)], Stroke::new(1.0_f32, AuditionTheme::GRID_LINE));
        };

        for ch in 0..num_ch {
            let ch_center_y = rect.min.y + (ch as f32 + 0.5) * ch_h;
            let half_h = (ch_h - 4.0) * 0.5 * v_zoom;

            for &(db, text) in db_levels {
                let linear = 10.0f32.powf(db / 20.0);
                let offset_y = linear * half_h;

                if offset_y < ch_h * 0.48 {
                    draw_label(ch_center_y - offset_y, text);
                    if db < 0.0 {
                        draw_label(ch_center_y + offset_y, text);
                    }
                }
            }
        }
    }
}

/// Per-channel drawing geometry.
struct ChannelGeometry {
    rect: Rect,
    center_y: f32,
    half_h: f32,
}

impl ChannelGeometry {
    /// Maps a sample amplitude to a y coordinate, clamped to the channel lane.
    fn amp_to_y(&self, amp: f32) -> f32 {
        let lim = self.rect.height() * 0.5;
        self.center_y - (amp * self.half_h).clamp(-lim, lim)
    }
}

/// Mode 1: connected sample-by-sample curve (with sample nodes at extreme zoom).
fn draw_oscilloscope(
    painter: &Painter,
    geo: &ChannelGeometry,
    ch_data: &[f32],
    view_start_sample: usize,
    view_end_sample: usize,
    samples_per_pixel: f32,
) {
    let wave_line_stroke = Stroke::new(1.2_f32, WAVE_LINE_COLOR);
    let step = if samples_per_pixel < 1.0 { 1 } else { (samples_per_pixel * 0.5).max(1.0) as usize };

    let mut prev_pt: Option<Pos2> = None;
    let end_idx = view_end_sample.min(ch_data.len().saturating_sub(1));

    let mut s_idx = view_start_sample;
    while s_idx <= end_idx {
        let x = geo.rect.min.x + ((s_idx - view_start_sample) as f32 / samples_per_pixel);
        let cur_pt = pos2(x, geo.amp_to_y(ch_data[s_idx]));

        if let Some(prev) = prev_pt {
            painter.line_segment([prev, cur_pt], wave_line_stroke);
        }

        if samples_per_pixel < 0.25 {
            painter.circle_filled(cur_pt, 2.0, Color32::from_rgb(200, 255, 230));
        }

        prev_pt = Some(cur_pt);
        s_idx += step;
    }
}

/// Mode 2: min-max peak columns joined into a continuous trace.
fn draw_peak_trace(painter: &Painter, geo: &ChannelGeometry, x0: f32, peaks: &[PeakSample]) {
    let stroke = Stroke::new(1.0_f32, WAVE_LINE_COLOR);
    let mut prev_pt: Option<Pos2> = None;

    for (i, p) in peaks.iter().enumerate() {
        let x = x0 + i as f32;
        let top_pt = pos2(x, geo.amp_to_y(p.max));
        let bot_pt = pos2(x, geo.amp_to_y(p.min));

        if let Some(prev) = prev_pt {
            painter.line_segment([prev, top_pt], stroke);
        }
        painter.line_segment([top_pt, bot_pt], stroke);
        prev_pt = Some(bot_pt);
    }
}

/// Mode 3: translucent filled envelope with bright top/bottom outlines.
fn draw_envelope(painter: &Painter, geo: &ChannelGeometry, x0: f32, peaks: &[PeakSample]) {
    let fill_stroke = Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(35, 175, 110, 110));
    let edge_stroke = Stroke::new(1.0_f32, Color32::from_rgb(70, 255, 170));

    let mut prev_top: Option<Pos2> = None;
    let mut prev_bot: Option<Pos2> = None;

    for (i, p) in peaks.iter().enumerate() {
        let x = x0 + i as f32;
        let top_pt = pos2(x, geo.amp_to_y(p.max));
        let bot_pt = pos2(x, geo.amp_to_y(p.min));

        painter.line_segment([top_pt, bot_pt], fill_stroke);

        if let Some(pt) = prev_top {
            painter.line_segment([pt, top_pt], edge_stroke);
        }
        if let Some(pb) = prev_bot {
            painter.line_segment([pb, bot_pt], edge_stroke);
        }

        prev_top = Some(top_pt);
        prev_bot = Some(bot_pt);
    }
}
