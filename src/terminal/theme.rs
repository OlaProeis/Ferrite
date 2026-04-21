use eframe::egui::Color32;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalTheme {
    pub name: String,
    pub background: Color32,
    pub foreground: Color32,
    pub selection_bg: Color32,
    pub cursor: Color32,
    pub ansi_colors: [Color32; 16],
}

impl Default for TerminalTheme {
    fn default() -> Self {
        Self::ferrite_dark()
    }
}

impl TerminalTheme {
    pub fn ferrite_dark() -> Self {
        Self {
            name: "Ferrite Dark".to_string(),
            background: Color32::from_rgb(30, 30, 30),
            foreground: Color32::from_rgb(220, 220, 220),
            selection_bg: Color32::from_rgb(60, 80, 110),
            cursor: Color32::from_rgba_unmultiplied(200, 200, 200, 180),
            ansi_colors: [
                Color32::from_rgb(0, 0, 0),       // Black
                Color32::from_rgb(205, 49, 49),   // Red
                Color32::from_rgb(13, 188, 121),  // Green
                Color32::from_rgb(229, 229, 16),  // Yellow
                Color32::from_rgb(36, 114, 200),  // Blue
                Color32::from_rgb(188, 63, 188),  // Magenta
                Color32::from_rgb(17, 168, 205),  // Cyan
                Color32::from_rgb(229, 229, 229), // White
                Color32::from_rgb(102, 102, 102), // Bright Black
                Color32::from_rgb(241, 76, 76),   // Bright Red
                Color32::from_rgb(35, 209, 139),  // Bright Green
                Color32::from_rgb(245, 245, 67),  // Bright Yellow
                Color32::from_rgb(59, 142, 234),  // Bright Blue
                Color32::from_rgb(214, 112, 214), // Bright Magenta
                Color32::from_rgb(41, 184, 219),  // Bright Cyan
                Color32::from_rgb(255, 255, 255), // Bright White
            ],
        }
    }

    pub fn ferrite_light() -> Self {
        Self {
            name: "Ferrite Light".to_string(),
            background: Color32::from_rgb(250, 250, 250),
            foreground: Color32::from_rgb(30, 30, 30),
            selection_bg: Color32::from_rgb(173, 214, 255),
            cursor: Color32::from_rgba_unmultiplied(50, 50, 50, 180),
            ansi_colors: [
                Color32::from_rgb(0, 0, 0),
                Color32::from_rgb(205, 49, 49),
                Color32::from_rgb(13, 188, 121),
                Color32::from_rgb(229, 229, 16),
                Color32::from_rgb(36, 114, 200),
                Color32::from_rgb(188, 63, 188),
                Color32::from_rgb(17, 168, 205),
                Color32::from_rgb(229, 229, 229),
                Color32::from_rgb(102, 102, 102),
                Color32::from_rgb(241, 76, 76),
                Color32::from_rgb(35, 209, 139),
                Color32::from_rgb(245, 245, 67),
                Color32::from_rgb(59, 142, 234),
                Color32::from_rgb(214, 112, 214),
                Color32::from_rgb(41, 184, 219),
                Color32::from_rgb(255, 255, 255),
            ],
        }
    }

    pub fn dracula() -> Self {
        Self {
            name: "Dracula".to_string(),
            background: Color32::from_rgb(40, 42, 54),
            foreground: Color32::from_rgb(248, 248, 242),
            selection_bg: Color32::from_rgb(68, 71, 90),
            cursor: Color32::from_rgb(248, 248, 242),
            ansi_colors: [
                Color32::from_rgb(33, 34, 44),    // Black
                Color32::from_rgb(255, 85, 85),   // Red
                Color32::from_rgb(80, 250, 123),  // Green
                Color32::from_rgb(241, 250, 140), // Yellow
                Color32::from_rgb(189, 147, 249), // Blue (Dracula uses purple for blue)
                Color32::from_rgb(255, 121, 198), // Magenta
                Color32::from_rgb(139, 233, 253), // Cyan
                Color32::from_rgb(248, 248, 242), // White
                Color32::from_rgb(98, 114, 164),  // Bright Black
                Color32::from_rgb(255, 110, 110), // Bright Red
                Color32::from_rgb(105, 255, 148), // Bright Green
                Color32::from_rgb(255, 255, 165), // Bright Yellow
                Color32::from_rgb(214, 172, 255), // Bright Blue
                Color32::from_rgb(255, 146, 223), // Bright Magenta
                Color32::from_rgb(164, 255, 255), // Bright Cyan
                Color32::from_rgb(255, 255, 255), // Bright White
            ],
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::ferrite_dark(), Self::ferrite_light(), Self::dracula()]
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::all().into_iter().find(|t| t.name == name)
    }
}
