use egui::{pos2, vec2, Color32, CursorIcon, Rect, Sense, Stroke, Ui};

use super::{OverviewDragMode, WaveformView};
use crate::ui::theme::AuditionTheme;

impl<'a> WaveformView<'a> {
    /// Minimap overview with a grabbable viewport box (pan) and resizable edges (zoom).
    pub(super) fn draw_interactive_overview(
        &mut self,
        ui: &mut Ui,
        height: f32,
        total_duration: f32,
        total_samples: usize,
    ) {
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        painter.rect_filled(rect, 2.0, AuditionTheme::OVERVIEW_BG);
        painter.rect_stroke(rect, 2.0, Stroke::new(1.0_f32, AuditionTheme::GRID_LINE));

        if total_samples == 0 || total_duration <= 0.0 {
            return;
        }

        let width_px = rect.width() as usize;
        let ch0_peaks = self.buffer.get_peaks(0, 0, total_samples, width_px);
        let center_y = rect.center().y;
        let half_h = (rect.height() - 6.0) * 0.5;

        for (i, p) in ch0_peaks.iter().enumerate() {
            let x = rect.min.x + i as f32;
            let y_top = center_y - p.max * half_h;
            let y_bot = center_y - p.min * half_h;
            painter.line_segment(
                [pos2(x, y_top), pos2(x, y_bot)],
                Stroke::new(1.0_f32, AuditionTheme::OVERVIEW_WAVE),
            );
        }

        for m in self.markers {
            let m_t = self.buffer.sample_to_time(m.start_sample);
            let x = rect.min.x + (m_t / total_duration).clamp(0.0, 1.0) * rect.width();
            painter.line_segment(
                [pos2(x, rect.min.y), pos2(x, rect.max.y)],
                Stroke::new(1.0_f32, m.color),
            );
        }

        let view_span = self.state.view_span();
        let start_ratio = (self.state.view_start_sec / total_duration).clamp(0.0, 1.0);
        let end_ratio = (self.state.view_end_sec / total_duration).clamp(0.0, 1.0);

        let vp_left = rect.min.x + start_ratio * rect.width();
        let vp_right = rect.min.x + end_ratio * rect.width();
        let min_handle_w = 16.0;
        let vp_box_right = vp_right.max(vp_left + min_handle_w);
        let vp_rect = Rect::from_min_max(pos2(vp_left, rect.min.y), pos2(vp_box_right, rect.max.y));

        let edge_hit_w = 8.0;
        let hover_pos = ui.input(|i| i.pointer.hover_pos());

        if response.hovered() {
            if let Some(m_pos) = hover_pos {
                if (m_pos.x - vp_left).abs() <= edge_hit_w || (m_pos.x - vp_box_right).abs() <= edge_hit_w {
                    ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
                } else if vp_rect.contains(m_pos) {
                    ui.output_mut(|o| o.cursor_icon = CursorIcon::Grab);
                }
            }
        }

        let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
        let scroll_x = ui.input(|i| i.smooth_scroll_delta.x);
        if response.hovered() && (scroll_y.abs() > 0.1 || scroll_x.abs() > 0.1) {
            let eff_scroll = if scroll_x.abs() > 0.1 { -scroll_x } else { -scroll_y };
            let pan_sec = eff_scroll * (view_span / 400.0);
            self.state.pan(pan_sec, total_duration);
        }

        if response.drag_started() {
            if let Some(pos) = response.interact_pointer_pos() {
                if (pos.x - vp_left).abs() <= edge_hit_w {
                    self.state.overview_drag_mode = OverviewDragMode::ResizeLeft;
                } else if (pos.x - vp_box_right).abs() <= edge_hit_w {
                    self.state.overview_drag_mode = OverviewDragMode::ResizeRight;
                } else if vp_rect.contains(pos) {
                    self.state.overview_drag_mode = OverviewDragMode::Pan;
                } else {
                    // Clicked outside the viewport box: jump-center the viewport there, then pan
                    let ratio = ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
                    let target_center = ratio * total_duration;
                    let mut new_start = target_center - view_span * 0.5;
                    if new_start < 0.0 {
                        new_start = 0.0;
                    } else if new_start + view_span > total_duration {
                        new_start = (total_duration - view_span).max(0.0);
                    }
                    self.state.view_start_sec = new_start;
                    self.state.view_end_sec = (new_start + view_span).min(total_duration);
                    self.state.overview_drag_mode = OverviewDragMode::Pan;
                }
            }
        }

        if response.dragged() {
            let delta_x = ui.input(|i| i.pointer.delta().x);
            if delta_x != 0.0 {
                let delta_sec = (delta_x / rect.width()) * total_duration;

                match self.state.overview_drag_mode {
                    OverviewDragMode::Pan => {
                        self.state.pan(delta_sec, total_duration);
                    }
                    OverviewDragMode::ResizeLeft => {
                        let new_start = (self.state.view_start_sec + delta_sec).clamp(0.0, self.state.view_end_sec - 0.001);
                        self.state.view_start_sec = new_start;
                    }
                    OverviewDragMode::ResizeRight => {
                        let new_end = (self.state.view_end_sec + delta_sec).clamp(self.state.view_start_sec + 0.001, total_duration);
                        self.state.view_end_sec = new_end;
                    }
                    OverviewDragMode::None => {}
                }
            }
        }

        if response.drag_stopped() {
            self.state.overview_drag_mode = OverviewDragMode::None;
        }

        let is_active = self.state.overview_drag_mode != OverviewDragMode::None;
        let fill_color = if is_active {
            Color32::from_rgba_premultiplied(90, 190, 255, 95)
        } else {
            AuditionTheme::OVERVIEW_VIEWPORT
        };
        let border_color = if is_active {
            Color32::from_rgb(140, 220, 255)
        } else {
            AuditionTheme::SELECTION_BORDER
        };

        painter.rect_filled(vp_rect, 2.0, fill_color);
        painter.rect_stroke(vp_rect, 2.0, Stroke::new(1.8_f32, border_color));

        let grip_w = 4.0;
        let left_grip = Rect::from_min_max(pos2(vp_rect.min.x, vp_rect.min.y + 4.0), pos2(vp_rect.min.x + grip_w, vp_rect.max.y - 4.0));
        let right_grip = Rect::from_min_max(pos2(vp_rect.max.x - grip_w, vp_rect.min.y + 4.0), pos2(vp_rect.max.x, vp_rect.max.y - 4.0));
        painter.rect_filled(left_grip, 1.0, Color32::from_rgb(200, 240, 255));
        painter.rect_filled(right_grip, 1.0, Color32::from_rgb(200, 240, 255));
    }
}
