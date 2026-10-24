use egui::{pos2, Align2, Color32, FontId, Painter, Rect, Stroke};

use super::WaveformView;
use crate::ui::theme::AuditionTheme;

impl<'a> WaveformView<'a> {
    pub(super) fn draw_markers(&self, painter: &Painter, ruler_rect: Rect, wave_rect: Rect) {
        if self.state.view_span() <= 0.0 {
            return;
        }

        for marker in self.markers {
            let start_t = self.buffer.sample_to_time(marker.start_sample);
            let x_start = self.time_to_x(start_t, wave_rect);

            if marker.is_range() {
                let end_t = self.buffer.sample_to_time(marker.end_sample());
                let x_end = self.time_to_x(end_t, wave_rect);

                let x1 = x_start.clamp(wave_rect.min.x, wave_rect.max.x);
                let x2 = x_end.clamp(wave_rect.min.x, wave_rect.max.x);

                if x2 > x1 {
                    let c = marker.color;
                    let fill_color = Color32::from_rgba_premultiplied(c.r(), c.g(), c.b(), 35);
                    let m_rect = Rect::from_min_max(pos2(x1, wave_rect.min.y), pos2(x2, wave_rect.max.y));
                    painter.rect_filled(m_rect, 0.0, fill_color);
                }

                if x_end >= wave_rect.min.x && x_end <= wave_rect.max.x {
                    painter.line_segment(
                        [pos2(x_end, ruler_rect.min.y), pos2(x_end, wave_rect.max.y)],
                        Stroke::new(1.0_f32, marker.color),
                    );
                }
            }

            if x_start >= wave_rect.min.x - 20.0 && x_start <= wave_rect.max.x + 20.0 {
                painter.line_segment(
                    [pos2(x_start, ruler_rect.min.y), pos2(x_start, wave_rect.max.y)],
                    Stroke::new(1.2_f32, marker.color),
                );

                let pin_pts = vec![
                    pos2(x_start, ruler_rect.min.y + 12.0),
                    pos2(x_start + 7.0, ruler_rect.min.y + 4.0),
                    pos2(x_start + 7.0, ruler_rect.min.y),
                    pos2(x_start - 7.0, ruler_rect.min.y),
                    pos2(x_start - 7.0, ruler_rect.min.y + 4.0),
                ];
                painter.add(egui::Shape::convex_polygon(
                    pin_pts,
                    marker.color,
                    Stroke::new(1.0_f32, Color32::BLACK),
                ));

                painter.text(
                    pos2(x_start + 4.0, ruler_rect.min.y + 2.0),
                    Align2::LEFT_TOP,
                    &marker.name,
                    FontId::proportional(10.0),
                    Color32::WHITE,
                );
            }
        }
    }

    pub(super) fn draw_selection(&self, painter: &Painter, rect: Rect, total_samples: usize) {
        let Some(sel) = &self.state.selection else {
            return;
        };
        if sel.end <= sel.start || total_samples == 0 || self.state.view_span() <= 0.0 {
            return;
        }

        let x_left = self.time_to_x(self.buffer.sample_to_time(sel.start), rect);
        let x_right = self.time_to_x(self.buffer.sample_to_time(sel.end), rect);

        let x_left_clamped = x_left.clamp(rect.min.x, rect.max.x);
        let x_right_clamped = x_right.clamp(rect.min.x, rect.max.x);

        if x_right_clamped > x_left_clamped {
            let sel_rect = Rect::from_min_max(
                pos2(x_left_clamped, rect.min.y),
                pos2(x_right_clamped, rect.max.y),
            );
            let border = Stroke::new(1.0_f32, AuditionTheme::SELECTION_BORDER);
            painter.rect_filled(sel_rect, 0.0, AuditionTheme::SELECTION_FILL);
            painter.line_segment([sel_rect.left_top(), sel_rect.left_bottom()], border);
            painter.line_segment([sel_rect.right_top(), sel_rect.right_bottom()], border);
        }
    }

    pub(super) fn draw_playhead(&self, painter: &Painter, ruler_rect: Rect, wave_rect: Rect) {
        if self.state.view_span() <= 0.0 {
            return;
        }

        let x = self.time_to_x(self.current_time(), wave_rect);

        if x >= wave_rect.min.x - 10.0 && x <= wave_rect.max.x + 10.0 {
            let playhead_color = if self.state.is_dragging_playhead {
                Color32::from_rgb(255, 100, 100)
            } else {
                AuditionTheme::PLAYHEAD
            };

            painter.line_segment(
                [pos2(x, ruler_rect.min.y), pos2(x, wave_rect.max.y)],
                Stroke::new(1.8_f32, playhead_color),
            );

            let pin_pts = vec![
                pos2(x - 7.0, ruler_rect.min.y),
                pos2(x + 7.0, ruler_rect.min.y),
                pos2(x + 7.0, ruler_rect.min.y + 7.0),
                pos2(x, ruler_rect.min.y + 14.0),
                pos2(x - 7.0, ruler_rect.min.y + 7.0),
            ];
            painter.add(egui::Shape::convex_polygon(
                pin_pts,
                playhead_color,
                Stroke::new(1.0_f32, Color32::WHITE),
            ));
        }
    }
}
