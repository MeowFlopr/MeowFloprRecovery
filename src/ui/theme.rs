use tiny_skia::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeId {
    ExpressiveDark,
    AmoledPure,
    EmeraldGreen,
    SunsetAmber,
    CyberpunkNeon,
}

impl ThemeId {
    pub const ALL: [ThemeId; 5] = [
        ThemeId::ExpressiveDark,
        ThemeId::AmoledPure,
        ThemeId::EmeraldGreen,
        ThemeId::SunsetAmber,
        ThemeId::CyberpunkNeon,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            ThemeId::ExpressiveDark => "M3 Lavender",
            ThemeId::AmoledPure => "AMOLED Black",
            ThemeId::EmeraldGreen => "Emerald Green",
            ThemeId::SunsetAmber => "Sunset Amber",
            ThemeId::CyberpunkNeon => "Cyberpunk Neon",
        }
    }

    pub fn to_theme(&self) -> M3Theme {
        match self {
            ThemeId::ExpressiveDark => M3Theme::expressive_dark(),
            ThemeId::AmoledPure => M3Theme::amoled_pure(),
            ThemeId::EmeraldGreen => M3Theme::emerald_green(),
            ThemeId::SunsetAmber => M3Theme::sunset_amber(),
            ThemeId::CyberpunkNeon => M3Theme::cyberpunk_neon(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct M3Theme {
    pub background: Color,
    pub surface: Color,
    pub surface_container_low: Color,
    pub surface_container: Color,
    pub surface_container_high: Color,
    pub surface_container_highest: Color,
    pub primary: Color,
    pub primary_container: Color,
    pub on_primary: Color,
    pub on_primary_container: Color,
    pub secondary: Color,
    pub secondary_container: Color,
    pub tertiary: Color,
    pub tertiary_container: Color,
    pub error: Color,
    pub error_container: Color,
    pub outline: Color,
    pub outline_variant: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
}

fn hex(rgba: u32) -> Color {
    let r = ((rgba >> 24) & 0xff) as u8;
    let g = ((rgba >> 16) & 0xff) as u8;
    let b = ((rgba >> 8) & 0xff) as u8;
    let a = (rgba & 0xff) as u8;
    Color::from_rgba8(r, g, b, a)
}

impl M3Theme {
    /// Authentic Material 3 Expressive (Lavender / Indigo signature tone)
    pub fn expressive_dark() -> Self {
        Self {
            background: hex(0x0a0b10ff),               // Pure deep AMOLED background
            surface: hex(0x101217ff),                  // Base surface
            surface_container_low: hex(0x14161eff),    // Elevation 1
            surface_container: hex(0x191c26ff),        // Distinct elevated card background
            surface_container_high: hex(0x222633ff),   // Highlighted / Active
            surface_container_highest: hex(0x2c3242ff),
            primary: hex(0xd0bcffff),                  // M3 Lavender Accent
            primary_container: hex(0x3d2c6cff),        // Soft container tone
            on_primary: hex(0x21005dff),
            on_primary_container: hex(0xeaddffff),
            secondary: hex(0xccc2dcff),
            secondary_container: hex(0x352f44ff),
            tertiary: hex(0x7bd0ffff),                 // Soft Cyan
            tertiary_container: hex(0x003d58ff),
            error: hex(0xffb4abff),
            error_container: hex(0x561515ff),
            outline: hex(0x79747eff),
            outline_variant: Color::from_rgba8(255, 255, 255, 16),
            on_surface: hex(0xf4eff4ff),
            on_surface_variant: hex(0x9d99a8ff),
        }
    }

    pub fn amoled_pure() -> Self {
        let mut t = Self::expressive_dark();
        t.background = hex(0x000000ff);
        t.surface = hex(0x050507ff);
        t.surface_container = hex(0x111218ff);
        t.surface_container_high = hex(0x1a1c24ff);
        t.outline_variant = Color::from_rgba8(255, 255, 255, 22);
        t
    }

    pub fn emerald_green() -> Self {
        Self {
            background: hex(0x070c08ff),
            surface: hex(0x0c140eff),
            surface_container_low: hex(0x101a13ff),
            surface_container: hex(0x142219ff),
            surface_container_high: hex(0x1c2e22ff),
            surface_container_highest: hex(0x263c2eff),
            primary: hex(0x6ddba2ff),                  // Radiant Mint / Emerald
            primary_container: hex(0x005234ff),
            on_primary: hex(0x003822ff),
            on_primary_container: hex(0x89f8beff),
            secondary: hex(0xb3ccbfff),
            secondary_container: hex(0x1e3a2cff),
            tertiary: hex(0x7dd2e4ff),                 // Aquatic Cyan
            tertiary_container: hex(0x004d5aff),
            error: hex(0xffb4abff),
            error_container: hex(0x561515ff),
            outline: hex(0x89938cff),
            outline_variant: Color::from_rgba8(255, 255, 255, 16),
            on_surface: hex(0xe8f2eaff),
            on_surface_variant: hex(0x9cb0a2ff),
        }
    }

    pub fn sunset_amber() -> Self {
        Self {
            background: hex(0x0d0906ff),
            surface: hex(0x160f0aff),
            surface_container_low: hex(0x1e150eff),
            surface_container: hex(0x271c13ff),
            surface_container_high: hex(0x35271aff),
            surface_container_highest: hex(0x443223ff),
            primary: hex(0xffb77bff),                  // Sunset Amber
            primary_container: hex(0x623300ff),
            on_primary: hex(0x452200ff),
            on_primary_container: hex(0xffdcc1ff),
            secondary: hex(0xe0c0a9ff),
            secondary_container: hex(0x463527ff),
            tertiary: hex(0xffb4a9ff),                 // Coral
            tertiary_container: hex(0x680009ff),
            error: hex(0xffb4abff),
            error_container: hex(0x561515ff),
            outline: hex(0x9a8d84ff),
            outline_variant: Color::from_rgba8(255, 255, 255, 16),
            on_surface: hex(0xf8ede6ff),
            on_surface_variant: hex(0xb2a39aff),
        }
    }

    pub fn cyberpunk_neon() -> Self {
        Self {
            background: hex(0x05050aff),
            surface: hex(0x0a0a14ff),
            surface_container_low: hex(0x0f0f1eff),
            surface_container: hex(0x16162aff),
            surface_container_high: hex(0x20203dff),
            surface_container_highest: hex(0x2d2d54ff),
            primary: hex(0x00f0ffff),                  // Electric Cyan
            primary_container: hex(0x00474dff),
            on_primary: hex(0x001f22ff),
            on_primary_container: hex(0x7df7ffff),
            secondary: hex(0xff2a85ff),                // Neon Pink
            secondary_container: hex(0x520025ff),
            tertiary: hex(0xffe600ff),                 // Neon Volt Yellow
            tertiary_container: hex(0x474000ff),
            error: hex(0xff4455ff),
            error_container: hex(0x660011ff),
            outline: hex(0x7a859eff),
            outline_variant: Color::from_rgba8(0, 240, 255, 30),
            on_surface: hex(0xf0f5ffff),
            on_surface_variant: hex(0xa0b0d0ff),
        }
    }
}

pub struct M3Shapes {
    pub corner_small: f32,
    pub corner_medium: f32,
    pub corner_large: f32,
    pub corner_card: f32,      // Refined 22.0dp as requested (between 20..24dp)
    pub corner_full: f32,
}

impl Default for M3Shapes {
    fn default() -> Self {
        Self {
            corner_small: 8.0,
            corner_medium: 14.0,
            corner_large: 18.0,
            corner_card: 22.0,        // 22dp: Tight, clean, modern
            corner_full: 9999.0,      // Full pill
        }
    }
}
