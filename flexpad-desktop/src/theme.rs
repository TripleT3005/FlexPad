use iced::theme::{self, Mode};
use iced::{Color, Theme, color};

// Types

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasColors {
    pub background: Color,
    pub dots: Color,
    pub tile: Color,
    pub tile_border: Color,
    pub selected: Color,
    pub selected_border: Color,
    pub highlighted: Color,
    pub highlighted_border: Color,
    pub tile_text: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub name: &'static str,
    pub background: Color,
    pub text: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub canvas: CanvasColors,
}

// Palettes

const TILES: CanvasColors = CanvasColors {
    background: Color::TRANSPARENT,
    dots: Color::TRANSPARENT,
    tile: color!(0x24523C),
    tile_border: color!(0x4F8A6C),
    selected: color!(0x3A7D5C),
    selected_border: color!(0x86BC9C),
    highlighted: color!(0x357A57),
    highlighted_border: color!(0xD6EDE0),
    tile_text: Color::WHITE,
};

pub const LIGHT: Palette = Palette {
    name: "FlexPad Light",
    background: color!(0xF3F3F3),
    text: color!(0x1C1C1C),
    accent: color!(0x2F6B4E),
    success: color!(0x2E7D4F),
    warning: color!(0xA86A00),
    danger: color!(0xB3261E),
    canvas: CanvasColors {
        background: color!(0xE8E8E8),
        dots: Color::from_rgba(0.0, 0.0, 0.0, 0.22),
        ..TILES
    },
};

pub const DARK: Palette = Palette {
    name: "FlexPad Dark",
    background: color!(0x202020),
    text: color!(0xE8E8E8),
    accent: color!(0x2F6B4E),
    success: color!(0x4CAF7A),
    warning: color!(0xE0B04A),
    danger: color!(0xE5675F),
    canvas: CanvasColors {
        background: color!(0x191919),
        dots: Color::from_rgba(1.0, 1.0, 1.0, 0.15),
        ..TILES
    },
};

// Theme

pub fn palette(mode: Mode) -> &'static Palette {
    match mode {
        Mode::Dark => &DARK,
        Mode::Light | Mode::None => &LIGHT,
    }
}

pub fn app_theme(mode: Mode) -> Theme {
    let palette = palette(mode);
    Theme::custom(
        palette.name,
        theme::Palette {
            background: palette.background,
            text: palette.text,
            primary: palette.accent,
            success: palette.success,
            warning: palette.warning,
            danger: palette.danger,
        },
    )
}

pub fn canvas_colors(theme: &Theme) -> &'static CanvasColors {
    let mode = if theme.extended_palette().is_dark {
        Mode::Dark
    } else {
        Mode::Light
    };
    &palette(mode).canvas
}

pub fn mode_override() -> Option<Mode> {
    match std::env::var("ICED_THEME").ok()?.as_str() {
        "Light" => Some(Mode::Light),
        "Dark" => Some(Mode::Dark),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(color: Color) -> f32 {
        let channel = |value: f32| {
            if value <= 0.039_28 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
    }

    fn contrast(a: Color, b: Color) -> f32 {
        let (light, dark) = {
            let (a, b) = (luminance(a), luminance(b));
            if a > b { (a, b) } else { (b, a) }
        };
        (light + 0.05) / (dark + 0.05)
    }

    // Contrast

    #[test]
    fn text_is_readable_on_the_background() {
        for palette in [&LIGHT, &DARK] {
            assert!(
                contrast(palette.text, palette.background) >= 7.0,
                "{}",
                palette.name
            );
        }
    }

    #[test]
    fn white_text_is_readable_on_buttons_and_tiles() {
        for palette in [&LIGHT, &DARK] {
            let canvas = palette.canvas;
            for fill in [
                palette.accent,
                canvas.tile,
                canvas.selected,
                canvas.highlighted,
            ] {
                assert!(
                    contrast(Color::WHITE, fill) >= 4.5,
                    "{} {fill:?}",
                    palette.name
                );
            }
            assert_eq!(canvas.tile_text, Color::WHITE);
        }
    }

    #[test]
    fn accent_uses_the_green_of_the_logo() {
        assert_eq!(LIGHT.accent, color!(0x2F6B4E));
        assert_eq!(DARK.accent, color!(0x2F6B4E));
    }

    // Selection

    #[test]
    fn mode_selects_the_palette() {
        assert_eq!(palette(Mode::Dark).name, "FlexPad Dark");
        assert_eq!(palette(Mode::Light).name, "FlexPad Light");
        assert_eq!(palette(Mode::None).name, "FlexPad Light");
        assert!(app_theme(Mode::Dark).extended_palette().is_dark);
        assert!(!app_theme(Mode::Light).extended_palette().is_dark);
        assert_eq!(canvas_colors(&app_theme(Mode::Dark)), &DARK.canvas);
    }
}
