use std::ops::Range;

#[derive(Clone, Debug)]
pub struct BpmSettings {
    pub enabled: bool,
    pub bpm: f64,
    pub beat_division: usize,
    pub offset_ms: f64,
    pub snap_to_beat: bool,
}

impl Default for BpmSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            bpm: 120.0,
            beat_division: 4,
            offset_ms: 0.0,
            snap_to_beat: false,
        }
    }
}

impl BpmSettings {
    pub fn tick_interval_sec(&self) -> f64 {
        if self.bpm <= 0.0 {
            return 0.5;
        }
        // Measure = 240 / BPM (from offset tool / BMS formula)
        let measure_duration = 240.0 / self.bpm;
        measure_duration / self.beat_division.max(1) as f64
    }

    pub fn snap_time(&self, time_sec: f64) -> f64 {
        let interval = self.tick_interval_sec();
        let offset = self.offset_ms / 1000.0;
        let relative = time_sec - offset;
        let tick = (relative / interval).round();
        (offset + tick * interval).max(0.0)
    }

    pub fn prev_tick_time(&self, time_sec: f64) -> f64 {
        let interval = self.tick_interval_sec();
        let offset = self.offset_ms / 1000.0;
        let relative = time_sec - offset - 0.00005;
        let tick = (relative / interval).floor();
        (offset + tick * interval).max(0.0)
    }

    pub fn next_tick_time(&self, time_sec: f64) -> f64 {
        let interval = self.tick_interval_sec();
        let offset = self.offset_ms / 1000.0;
        let relative = time_sec - offset + 0.00005;
        let tick = (relative / interval).ceil();
        (offset + tick * interval).max(0.0)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum OverviewDragMode {
    None,
    Pan,
    ResizeLeft,
    ResizeRight,
}

pub struct WaveformViewState {
    pub view_start_sec: f32,
    pub view_end_sec: f32,
    pub vertical_zoom: f32,
    pub selection: Option<Range<usize>>,
    pub is_dragging_selection: bool,
    pub is_dragging_playhead: bool,
    pub is_panning: bool,
    pub overview_drag_mode: OverviewDragMode,
    pub drag_start_sample: usize,
    pub pan_start_x: f32,
    pub pan_start_view_start: f32,
    pub bpm_settings: BpmSettings,
}

impl WaveformViewState {
    pub fn new() -> Self {
        Self {
            view_start_sec: 0.0,
            view_end_sec: 10.0,
            vertical_zoom: 1.0,
            selection: None,
            is_dragging_selection: false,
            is_dragging_playhead: false,
            is_panning: false,
            overview_drag_mode: OverviewDragMode::None,
            drag_start_sample: 0,
            pan_start_x: 0.0,
            pan_start_view_start: 0.0,
            bpm_settings: BpmSettings::default(),
        }
    }

    /// Visible time span (seconds) of the current viewport.
    pub fn view_span(&self) -> f32 {
        self.view_end_sec - self.view_start_sec
    }

    /// Center time (seconds) of the current viewport.
    pub fn view_center(&self) -> f32 {
        (self.view_start_sec + self.view_end_sec) * 0.5
    }

    pub fn reset_view_to_fit(&mut self, duration: f32) {
        self.view_start_sec = 0.0;
        self.view_end_sec = duration.max(0.1);
        self.vertical_zoom = 1.0;
    }

    pub fn zoom_at(&mut self, anchor_sec: f32, zoom_factor: f32, total_duration: f32) {
        let max_dur = total_duration.max(0.1);
        let cur_span = self.view_end_sec - self.view_start_sec;
        if cur_span <= 0.0 {
            return;
        }

        let min_span = 0.0005f32;
        let new_span = (cur_span * zoom_factor).clamp(min_span, max_dur);

        let anchor_ratio = ((anchor_sec - self.view_start_sec) / cur_span).clamp(0.0, 1.0);
        let mut new_start = anchor_sec - anchor_ratio * new_span;
        let mut new_end = new_start + new_span;

        if new_start < 0.0 {
            new_start = 0.0;
            new_end = (new_start + new_span).min(max_dur);
        } else if new_end > max_dur {
            new_end = max_dur;
            new_start = (new_end - new_span).max(0.0);
        }

        self.view_start_sec = new_start;
        self.view_end_sec = new_end;
    }

    pub fn pan(&mut self, delta_sec: f32, total_duration: f32) {
        let max_dur = total_duration.max(0.1);
        let cur_span = self.view_end_sec - self.view_start_sec;
        let mut new_start = self.view_start_sec + delta_sec;
        if new_start < 0.0 {
            new_start = 0.0;
        } else if new_start + cur_span > max_dur {
            new_start = (max_dur - cur_span).max(0.0);
        }
        self.view_start_sec = new_start;
        self.view_end_sec = (new_start + cur_span).min(max_dur);
    }
}
