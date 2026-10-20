// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use piet::Color as PietColor;
use ratatui_core::style::Color as TuiColor;

/// Blend a Piet RGBA color over an existing terminal color.
///
/// Terminals do not have an alpha channel, so translucent drawing is resolved
/// against the current cell background. Unknown terminal colors are treated as
/// black for blending purposes.
pub(crate) fn blend_over(src: PietColor, dst: TuiColor) -> TuiColor {
    if let Some(index) = src.as_indexed() {
        return TuiColor::Indexed(index);
    }

    let (r, g, b, a) = src.as_rgba8();
    if a == 255 {
        return TuiColor::Rgb(r, g, b);
    }
    if a == 0 {
        return dst;
    }

    let (dr, dg, db) = rgb_components(dst).unwrap_or_default();
    let blend = |s: u8, d: u8| {
        let s = s as u32 * a as u32;
        let d = d as u32 * (255 - a as u32);
        ((s + d + 127) / 255) as u8
    };

    TuiColor::Rgb(blend(r, dr), blend(g, dg), blend(b, db))
}

fn rgb_components(color: TuiColor) -> Option<(u8, u8, u8)> {
    match color {
        TuiColor::Rgb(r, g, b) => Some((r, g, b)),
        TuiColor::Indexed(index) => {
            let (r, g, b, _) = PietColor::indexed(index).as_rgba8();
            Some((r, g, b))
        }
        TuiColor::Black => Some((0, 0, 0)),
        TuiColor::Red => Some((128, 0, 0)),
        TuiColor::Green => Some((0, 128, 0)),
        TuiColor::Yellow => Some((128, 128, 0)),
        TuiColor::Blue => Some((0, 0, 128)),
        TuiColor::Magenta => Some((128, 0, 128)),
        TuiColor::Cyan => Some((0, 128, 128)),
        TuiColor::Gray => Some((192, 192, 192)),
        TuiColor::DarkGray => Some((128, 128, 128)),
        TuiColor::LightRed => Some((255, 0, 0)),
        TuiColor::LightGreen => Some((0, 255, 0)),
        TuiColor::LightYellow => Some((255, 255, 0)),
        TuiColor::LightBlue => Some((0, 0, 255)),
        TuiColor::LightMagenta => Some((255, 0, 255)),
        TuiColor::LightCyan => Some((0, 255, 255)),
        TuiColor::White => Some((255, 255, 255)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use piet::AnsiColor;
    use ratatui_core::style::Color as TuiColor;

    use super::blend_over;
    use piet::Color;

    #[test]
    fn opaque_indexed_colors_are_preserved() {
        assert_eq!(
            blend_over(Color::ansi(AnsiColor::Yellow), TuiColor::Reset),
            TuiColor::Indexed(3)
        );
        assert_eq!(
            blend_over(Color::indexed(196), TuiColor::Rgb(1, 2, 3)),
            TuiColor::Indexed(196)
        );
    }

    #[test]
    fn translucent_colors_blend_over_indexed_backgrounds() {
        assert_eq!(
            blend_over(
                Color::ansi(AnsiColor::Green).with_alpha(0.5),
                TuiColor::Indexed(3),
            ),
            TuiColor::Rgb(64, 128, 0)
        );
        assert_eq!(
            blend_over(Color::TRANSPARENT, TuiColor::Indexed(7)),
            TuiColor::Indexed(7)
        );
    }
}
