//! Main waveform editor view.
//!
//! Split into sub-modules by responsibility:
//! - `state`     : viewport / BPM settings state (`WaveformViewState`, `BpmSettings`)
//! - `timecode`  : timestamp formatting helpers (0.1ms precision)
//! - `toolbar`   : zoom toolbar & BPM controls bar
//! - `overview`  : minimap overview scrubber (jog wheel)
//! - `input`     : mouse wheel zoom/pan, hand tool, playhead scrubbing, range selection
//! - `ruler`     : dynamic no-overlap timeline ruler & BPM measure/beat grid
//! - `waveform`  : oscilloscope waveform rendering & dB scale
//! - `overlays`  : selection, markers and playhead overlays

mod input;
mod overlays;
mod overview;
mod ruler;
mod state;
mod timecode;
mod toolbar;
mod waveform;

use std::sync::atomic::Ordering;

use egui::{pos2, vec2, Rect, Sense, Ui};

use crate::audio::buffer::AudioBuffer;
use crate::audio::marker::Marker;
use crate::audio::playback::PlaybackController;

pub use state::{OverviewDragMode, WaveformViewState};
pub use timecode::{format_ms_timestamp, format_precise_timestamp, format_timestamp};

pub struct WaveformView<'a> {
    buffer: &'a AudioBuffer,
    playback: &'a PlaybackController,
    state: &'a mut WaveformViewState,
    markers: &'a [Marker],
}

impl<'a> WaveformView<'a> {
    pub fn new(
        buffer: &'a AudioBuffer,
        playback: &'a PlaybackController,
        state: &'a mut WaveformViewState,
        markers: &'a [Marker],
    ) -> Self {
        Self {
            buffer,
            playback,
            state,
            markers,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let total_duration = self.buffer.duration_seconds().max(0.1);
        let total_samples = self.buffer.num_samples();
        let sample_rate = self.buffer.sample_rate;

        if self.state.view_end_sec <= self.state.view_start_sec || self.state.view_end_sec > total_duration * 1.5 {
            self.state.view_end_sec = total_duration;
        }

        let available_size = ui.available_size();
        let toolbar_height = 28.0;
        let bpm_bar_height = if self.state.bpm_settings.enabled { 32.0 } else { 0.0 };
        let overview_height = 46.0;
        let ruler_height = 26.0;
        let waveform_height = (available_size.y - toolbar_height - bpm_bar_height - overview_height - ruler_height - 10.0).max(120.0);

        // 1. Zoom Toolbar
        self.draw_zoom_toolbar(ui, total_duration);

        // 2. BPM Controls Bar (When BPM Grid is Enabled)
        if self.state.bpm_settings.enabled {
            self.draw_bpm_controls(ui, total_duration);
        }

        // 3. Interactive Overview Scrubber (Minimap & Continuous Jog Wheel)
        self.draw_interactive_overview(ui, overview_height, total_duration, total_samples);

        ui.add_space(2.0);

        // 4. Main Waveform + Ruler Frame
        let (rect, response) = ui.allocate_exact_size(
            vec2(available_size.x, ruler_height + waveform_height),
            Sense::click_and_drag(),
        );

        let db_scale_width = 38.0;
        let ruler_rect = Rect::from_min_size(
            rect.min,
            vec2(rect.width() - db_scale_width, ruler_height),
        );
        let wave_rect = Rect::from_min_size(
            pos2(rect.min.x, rect.min.y + ruler_height),
            vec2(rect.width() - db_scale_width, waveform_height),
        );
        let db_rect = Rect::from_min_size(
            pos2(rect.max.x - db_scale_width, rect.min.y + ruler_height),
            vec2(db_scale_width, waveform_height),
        );

        let painter = ui.painter_at(rect);

        // Handle Interactive Input (Zoom, Pan, Playhead Drag, Range Select)
        self.handle_input(ui, &response, ruler_rect, wave_rect, total_duration);

        // Draw Ruler (Dynamic No-Overlap Step Algorithm)
        self.draw_ruler(&painter, ruler_rect, sample_rate);

        // Draw Waveforms (Split Stereo Channels + Oscilloscope Curves)
        self.draw_waveforms(&painter, wave_rect, total_samples);

        // Draw BPM Beat & Measure Grid (offset tool style)
        if self.state.bpm_settings.enabled {
            self.draw_bpm_grid(&painter, ruler_rect, wave_rect, total_duration);
        }

        // Draw dB Scale on the right margin
        self.draw_db_scale(&painter, db_rect);

        // Draw Selection Overlay
        self.draw_selection(&painter, wave_rect, total_samples);

        // Draw Markers (Cue Points & Ranges)
        self.draw_markers(&painter, ruler_rect, wave_rect);

        // Draw Playhead Marker (Grabbable & Scrubbable Red Pin)
        self.draw_playhead(&painter, ruler_rect, wave_rect);
    }

    // ---- Shared coordinate helpers (used by sub-modules) ----

    /// Current playhead time in seconds.
    fn current_time(&self) -> f32 {
        let cur_sample = self.playback.current_sample.load(Ordering::Relaxed);
        self.buffer.sample_to_time(cur_sample)
    }

    /// Converts a time (seconds) to an x pixel coordinate inside `rect` for the current viewport.
    fn time_to_x(&self, time_sec: f32, rect: Rect) -> f32 {
        let view_span = self.state.view_span();
        rect.min.x + ((time_sec - self.state.view_start_sec) / view_span) * rect.width()
    }

    fn pixel_to_sample(&self, pixel_x: f32, rect: Rect) -> usize {
        let ratio = ((pixel_x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
        let view_span = self.state.view_span();
        let time = self.state.view_start_sec + ratio * view_span;
        self.buffer.time_to_sample(time)
    }
}
