//! Themes on Rune's kit tokens: the system look, plus Dark, Light and Kanagawa Lotus.

#![forbid(unsafe_code)]

use rune::Color;
use rune_kit::{Kit, PaletteSeed};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    #[default]
    System,
    Dark,
    Light,
    KanagawaLotus,
}

pub const ALL: [Theme; 4] = [Theme::System, Theme::Dark, Theme::Light, Theme::KanagawaLotus];

impl Theme {
    pub fn name(self) -> &'static str {
        match self {
            Theme::System => "System",
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::KanagawaLotus => "Kanagawa Lotus",
        }
    }

    pub fn kit(self, system_dark: bool) -> Kit {
        match self {
            Theme::System => Kit::neutral(system_dark),
            Theme::Dark => Kit::neutral(true),
            Theme::Light => Kit::neutral(false),
            // lotusWhite3, lotusInk1 and lotusBlue4 from the Kanagawa Lotus palette.
            Theme::KanagawaLotus => {
                Kit::from_palette(PaletteSeed { bg: hex(0xf2ecbc), fg: hex(0x545464), accent: hex(0x4d699b) })
            }
        }
    }
}

/// An sRGB hex colour as Rune's linear colour.
fn hex(rgb: u32) -> Color {
    let channel = |shift: u32| rune::theme::srgb_decode(((rgb >> shift) & 0xff) as f32 / 255.0);
    Color { r: channel(16), g: channel(8), b: channel(0), a: 1.0 }
}
