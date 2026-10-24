use egui::Color32;

#[derive(Clone, Debug, PartialEq)]
pub struct Marker {
    pub id: usize,
    pub name: String,
    pub start_sample: usize,
    pub duration_samples: usize,
    pub color: Color32,
}

impl Marker {
    pub const PALETTE: &'static [Color32] = &[
        Color32::from_rgb(0, 200, 255),   // Cyan / Sky
        Color32::from_rgb(255, 180, 0),   // Amber
        Color32::from_rgb(255, 80, 120),  // Coral / Red
        Color32::from_rgb(140, 230, 80),  // Lime
        Color32::from_rgb(200, 120, 255), // Purple
        Color32::from_rgb(255, 230, 80),  // Yellow
    ];

    pub fn new(id: usize, name: String, start_sample: usize, duration_samples: usize) -> Self {
        let color = Self::PALETTE[id % Self::PALETTE.len()];
        Self {
            id,
            name,
            start_sample,
            duration_samples,
            color,
        }
    }

    pub fn is_range(&self) -> bool {
        self.duration_samples > 0
    }

    pub fn end_sample(&self) -> usize {
        self.start_sample + self.duration_samples
    }
}
