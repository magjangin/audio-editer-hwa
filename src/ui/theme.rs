use egui::{Color32, Stroke, Visuals};

pub struct AuditionTheme;

impl AuditionTheme {
    pub const BG_WINDOW: Color32 = Color32::from_rgb(24, 25, 28);
    pub const BG_PANEL: Color32 = Color32::from_rgb(32, 33, 36);
    pub const BG_WAVEFORM: Color32 = Color32::from_rgb(18, 22, 20);
    
    // Audition signature mint green waveforms
    pub const WAVEFORM_PEAK: Color32 = Color32::from_rgb(46, 218, 140);
    pub const WAVEFORM_RMS: Color32 = Color32::from_rgb(25, 140, 85);
    pub const GRID_LINE: Color32 = Color32::from_rgb(36, 46, 40);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(120, 135, 128);
    pub const TEXT_BRIGHT: Color32 = Color32::from_rgb(220, 225, 220);

    // Selection & Playhead
    pub const SELECTION_FILL: Color32 = Color32::from_rgba_premultiplied(80, 150, 240, 45);
    pub const SELECTION_BORDER: Color32 = Color32::from_rgb(120, 180, 255);
    pub const PLAYHEAD: Color32 = Color32::from_rgb(255, 60, 60);

    // Overview bar
    pub const OVERVIEW_BG: Color32 = Color32::from_rgb(15, 18, 16);
    pub const OVERVIEW_WAVE: Color32 = Color32::from_rgb(40, 150, 100);
    pub const OVERVIEW_VIEWPORT: Color32 = Color32::from_rgba_premultiplied(100, 200, 255, 60);

    // Meters
    pub const METER_BG: Color32 = Color32::from_rgb(20, 20, 20);
    pub const METER_GREEN: Color32 = Color32::from_rgb(40, 210, 80);
    pub const METER_YELLOW: Color32 = Color32::from_rgb(230, 200, 40);
    pub const METER_RED: Color32 = Color32::from_rgb(240, 50, 50);

    pub fn apply(ctx: &egui::Context) {
        // Load Windows Korean font (Malgun Gothic)
        let mut fonts = egui::FontDefinitions::default();
        if let Ok(font_data) = std::fs::read(r"C:\Windows\Fonts\malgun.ttf") {
            fonts.font_data.insert(
                "malgun".to_owned(),
                egui::FontData::from_owned(font_data),
            );
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "malgun".to_owned());
            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .push("malgun".to_owned());
            ctx.set_fonts(fonts);
        }

        let mut visuals = Visuals::dark();
        visuals.override_text_color = Some(Self::TEXT_BRIGHT);
        visuals.panel_fill = Self::BG_WINDOW;
        visuals.window_fill = Self::BG_PANEL;
        visuals.faint_bg_color = Self::BG_PANEL;
        visuals.extreme_bg_color = Self::BG_WAVEFORM;
        visuals.widgets.noninteractive.bg_fill = Self::BG_PANEL;
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(42, 44, 48);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(55, 58, 64);
        visuals.widgets.active.bg_fill = Color32::from_rgb(68, 72, 80);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(50, 52, 58));
        ctx.set_visuals(visuals);
    }
}
