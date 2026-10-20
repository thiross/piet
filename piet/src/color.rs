// Copyright 2019 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A simple representation of color

use std::fmt::{Debug, Formatter};

/// One of the 16 standard ANSI terminal palette slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AnsiColor {
    /// ANSI palette slot 0.
    Black = 0,
    /// ANSI palette slot 1.
    Red = 1,
    /// ANSI palette slot 2.
    Green = 2,
    /// ANSI palette slot 3.
    Yellow = 3,
    /// ANSI palette slot 4.
    Blue = 4,
    /// ANSI palette slot 5.
    Magenta = 5,
    /// ANSI palette slot 6.
    Cyan = 6,
    /// ANSI palette slot 7.
    White = 7,
    /// ANSI palette slot 8.
    BrightBlack = 8,
    /// ANSI palette slot 9.
    BrightRed = 9,
    /// ANSI palette slot 10.
    BrightGreen = 10,
    /// ANSI palette slot 11.
    BrightYellow = 11,
    /// ANSI palette slot 12.
    BrightBlue = 12,
    /// ANSI palette slot 13.
    BrightMagenta = 13,
    /// ANSI palette slot 14.
    BrightCyan = 14,
    /// ANSI palette slot 15.
    BrightWhite = 15,
}

impl AnsiColor {
    /// Return the ANSI palette index for this color.
    pub const fn index(self) -> u8 {
        self as u8
    }

    /// Return the ANSI color for `index` if it is in the range 0 through 15.
    pub const fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::Black),
            1 => Some(Self::Red),
            2 => Some(Self::Green),
            3 => Some(Self::Yellow),
            4 => Some(Self::Blue),
            5 => Some(Self::Magenta),
            6 => Some(Self::Cyan),
            7 => Some(Self::White),
            8 => Some(Self::BrightBlack),
            9 => Some(Self::BrightRed),
            10 => Some(Self::BrightGreen),
            11 => Some(Self::BrightYellow),
            12 => Some(Self::BrightBlue),
            13 => Some(Self::BrightMagenta),
            14 => Some(Self::BrightCyan),
            15 => Some(Self::BrightWhite),
            _ => None,
        }
    }
}

/// A datatype representing color.
///
/// Colors are either 32-bit RGBA values or terminal palette indices. Indexed
/// colors retain their palette identity in backends that support it; other
/// backends use a deterministic xterm RGB fallback.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Color {
    #[doc(hidden)]
    Rgba32(u32),

    /// A terminal palette index.
    ///
    /// Indices 0 through 15 normally come from the terminal theme. Indices 16
    /// through 255 use the conventional xterm palette.
    Indexed(u8),
}

/// Errors that can occur when parsing a hex color.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorParseError {
    /// The input string has an incorrect length
    WrongSize(usize),
    /// A byte in the input string is not in one of the ranges `0..=9`,
    /// `a..=f`, or `A..=F`.
    #[allow(missing_docs)]
    NotHex { idx: usize, byte: u8 },
}

impl Color {
    /// Create a color from 8 bit per sample RGB values.
    pub const fn rgb8(r: u8, g: u8, b: u8) -> Color {
        Color::from_rgba32_u32(((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | 0xff)
    }

    /// Create a color from 8 bit per sample RGBA values.
    pub const fn rgba8(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color::from_rgba32_u32(
            ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32),
        )
    }

    /// Create a color from a 32-bit rgba value (alpha as least significant byte).
    pub const fn from_rgba32_u32(rgba: u32) -> Color {
        Color::Rgba32(rgba)
    }

    /// Create a color from one of the 16 standard ANSI terminal palette slots.
    pub const fn ansi(color: AnsiColor) -> Color {
        Color::Indexed(color.index())
    }

    /// Create a color from a raw terminal palette index.
    pub const fn indexed(index: u8) -> Color {
        Color::Indexed(index)
    }

    /// Attempt to create a color from a CSS-style hex string.
    ///
    /// This will accept strings in the following formats, *with or without*
    /// the leading `#`:
    ///
    /// - `rrggbb`
    /// - `rrggbbaa`
    /// - `rbg`
    /// - `rbga`
    ///
    /// This method returns a [`ColorParseError`] if the color cannot be parsed.
    pub const fn from_hex_str(hex: &str) -> Result<Color, ColorParseError> {
        // can't use `map()` in a const function
        match get_4bit_hex_channels(hex) {
            Ok(channels) => Ok(color_from_4bit_hex(channels)),
            Err(e) => Err(e),
        }
    }

    /// Create a color from a grey value.
    ///
    /// ```
    /// use piet::Color;
    ///
    /// let grey_val = 0x55;
    ///
    /// let one = Color::grey8(grey_val);
    /// // is shorthand for
    /// let two = Color::rgb8(grey_val, grey_val, grey_val);
    ///
    /// assert_eq!(one.as_rgba_u32(), two.as_rgba_u32());
    /// ```
    pub const fn grey8(grey: u8) -> Color {
        Color::rgb8(grey, grey, grey)
    }

    /// Create a color with a grey value in the range 0.0..=1.0.
    pub fn grey(grey: f64) -> Color {
        Color::rgb(grey, grey, grey)
    }

    /// Create a color from four floating point values, each in the range 0.0 to 1.0.
    ///
    /// The interpretation is the same as rgba32, and no greater precision is
    /// (currently) assumed.
    pub fn rgba(r: f64, g: f64, b: f64, a: f64) -> Color {
        let r = (r.clamp(0.0, 1.0) * 255.0).round() as u32;
        let g = (g.clamp(0.0, 1.0) * 255.0).round() as u32;
        let b = (b.clamp(0.0, 1.0) * 255.0).round() as u32;
        let a = (a.clamp(0.0, 1.0) * 255.0).round() as u32;
        Color::from_rgba32_u32((r << 24) | (g << 16) | (b << 8) | a)
    }

    /// Create a color from three floating point values, each in the range 0.0 to 1.0.
    ///
    /// The interpretation is the same as rgb8, and no greater precision is
    /// (currently) assumed.
    pub fn rgb(r: f64, g: f64, b: f64) -> Color {
        let r = (r.clamp(0.0, 1.0) * 255.0).round() as u32;
        let g = (g.clamp(0.0, 1.0) * 255.0).round() as u32;
        let b = (b.clamp(0.0, 1.0) * 255.0).round() as u32;
        Color::from_rgba32_u32((r << 24) | (g << 16) | (b << 8) | 0xff)
    }

    /// Create a color from a CIEL\*a\*b\* polar (also known as CIE HCL)
    /// specification.
    ///
    /// The `h` parameter is an angle in degrees, with 0 roughly magenta, 90
    /// roughly yellow, 180 roughly cyan, and 270 roughly blue. The `l`
    /// parameter is perceptual luminance, with 0 black and 100 white.
    /// The `c` parameter is a chrominance concentration, with 0 grayscale
    /// and a nominal maximum of 127 (in the future, higher values might
    /// be useful, for high gamut contexts).
    ///
    /// Currently this is just converted into sRGB, but in the future as we
    /// support high-gamut colorspaces, it can be used to specify more colors
    /// or existing colors with a higher accuracy.
    ///
    /// Currently out-of-gamut values are clipped to the nearest sRGB color,
    /// which is perhaps not ideal (the clipping might change the hue). See
    /// <https://github.com/d3/d3-color/issues/33> for discussion.
    #[allow(non_snake_case)]
    #[allow(clippy::many_single_char_names)]
    #[allow(clippy::unreadable_literal)]
    pub fn hlc(h: f64, L: f64, c: f64) -> Color {
        // The reverse transformation from Lab to XYZ, see
        // https://en.wikipedia.org/wiki/CIELAB_color_space
        fn f_inv(t: f64) -> f64 {
            let d = 6. / 29.;
            if t > d {
                t.powi(3)
            } else {
                3. * d * d * (t - 4. / 29.)
            }
        }
        let th = h * (std::f64::consts::PI / 180.);
        let a = c * th.cos();
        let b = c * th.sin();
        let ll = (L + 16.) * (1. / 116.);
        // Produce raw XYZ values
        let X = f_inv(ll + a * (1. / 500.));
        let Y = f_inv(ll);
        let Z = f_inv(ll - b * (1. / 200.));
        // This matrix is the concatenation of three sources.
        // First, the white point is taken to be ICC standard D50, so
        // the diagonal matrix of [0.9642, 1, 0.8249]. Note that there
        // is some controversy around this value. However, it matches
        // the other matrices, thus minimizing chroma error.
        //
        // Second, an adaption matrix from D50 to D65. This is the
        // inverse of the recommended D50 to D65 adaptation matrix
        // from the W3C sRGB spec:
        // https://www.w3.org/Graphics/Color/srgb
        //
        // Finally, the conversion from XYZ to linear sRGB values,
        // also taken from the W3C sRGB spec.
        let r_lin = 3.02172918 * X - 1.61692294 * Y - 0.40480625 * Z;
        let g_lin = -0.94339358 * X + 1.91584267 * Y + 0.02755094 * Z;
        let b_lin = 0.06945666 * X - 0.22903204 * Y + 1.15957526 * Z;
        fn gamma(u: f64) -> f64 {
            if u <= 0.0031308 {
                12.92 * u
            } else {
                1.055 * u.powf(1. / 2.4) - 0.055
            }
        }
        Color::rgb(gamma(r_lin), gamma(g_lin), gamma(b_lin))
    }

    /// Create a color from a CIEL\*a\*b\* polar specification and alpha.
    ///
    /// The `a` value represents alpha in the range 0.0 to 1.0.
    pub fn hlca(h: f64, l: f64, c: f64, a: f64) -> Color {
        Color::hlc(h, c, l).with_alpha(a)
    }

    /// Change just the alpha value of a color.
    ///
    /// The `a` value represents alpha in the range 0.0 to 1.0.
    pub fn with_alpha(self, a: f64) -> Color {
        let a = (a.clamp(0.0, 1.0) * 255.0).round() as u32;
        match self {
            Color::Indexed(index) if a == 0xff => Color::Indexed(index),
            _ => Color::from_rgba32_u32((self.as_rgba_u32() & !0xff) | a),
        }
    }

    /// Change just the red value of a color.
    ///
    /// The `r` value represents red as a `u8` from 0 to 255.
    pub const fn with_r8(self, r: u8) -> Color {
        Color::from_rgba32_u32((self.as_rgba_u32() & !0xff000000) | (r as u32) << 24)
    }

    /// Change just the green value of a color.
    ///
    /// The `g` value represents green as a `u8` from 0 to 255.
    pub const fn with_g8(self, g: u8) -> Color {
        Color::from_rgba32_u32((self.as_rgba_u32() & !0xff0000) | (g as u32) << 16)
    }

    /// Change just the blue value of a color.
    ///
    /// The `b` value represents blue as a `u8` from 0 to 255.
    pub const fn with_b8(self, b: u8) -> Color {
        Color::from_rgba32_u32((self.as_rgba_u32() & !0xff00) | (b as u32) << 8)
    }

    /// Change just the alpha value of a color.
    ///
    /// The `a` value represents alpha as a `u8` from 0 to 255.
    pub const fn with_a8(self, a: u8) -> Color {
        match self {
            Color::Indexed(index) if a == 0xff => Color::Indexed(index),
            _ => Color::from_rgba32_u32((self.as_rgba_u32() & !0xff) | a as u32),
        }
    }

    /// Convert a color value to a 32-bit rgba value.
    ///
    /// Indexed colors use the conventional xterm RGB value as a fallback.
    pub const fn as_rgba_u32(self) -> u32 {
        match self {
            Color::Rgba32(rgba) => rgba,
            Color::Indexed(index) => indexed_rgba_u32(index),
        }
    }

    /// Return the terminal palette index, if this is an indexed color.
    pub const fn as_indexed(self) -> Option<u8> {
        match self {
            Color::Indexed(index) => Some(index),
            Color::Rgba32(_) => None,
        }
    }

    /// Return the ANSI palette slot for indices 0 through 15.
    pub const fn as_ansi(self) -> Option<AnsiColor> {
        match self {
            Color::Indexed(index) => AnsiColor::from_index(index),
            Color::Rgba32(_) => None,
        }
    }

    /// Convert a color value to four 8-bit rgba values.
    pub fn as_rgba8(self) -> (u8, u8, u8, u8) {
        let rgba = self.as_rgba_u32();
        (
            (rgba >> 24 & 255) as u8,
            ((rgba >> 16) & 255) as u8,
            ((rgba >> 8) & 255) as u8,
            (rgba & 255) as u8,
        )
    }

    /// Convert a color value to four f64 values, each in the range 0.0 to 1.0.
    pub fn as_rgba(self) -> (f64, f64, f64, f64) {
        let rgba = self.as_rgba_u32();
        (
            (rgba >> 24) as f64 / 255.0,
            ((rgba >> 16) & 255) as f64 / 255.0,
            ((rgba >> 8) & 255) as f64 / 255.0,
            (rgba & 255) as f64 / 255.0,
        )
    }

    // basic css3 colors (not including shades for now)

    /// Opaque aqua (or cyan).
    pub const AQUA: Color = Color::rgb8(0, 255, 255);

    /// Opaque black.
    pub const BLACK: Color = Color::rgb8(0, 0, 0);

    /// Opaque blue.
    pub const BLUE: Color = Color::rgb8(0, 0, 255);

    /// Opaque fuchsia (or magenta).
    pub const FUCHSIA: Color = Color::rgb8(255, 0, 255);

    /// Opaque gray.
    pub const GRAY: Color = Color::grey8(128);

    /// Opaque green.
    pub const GREEN: Color = Color::rgb8(0, 128, 0);

    /// Opaque lime.
    pub const LIME: Color = Color::rgb8(0, 255, 0);

    /// Opaque maroon.
    pub const MAROON: Color = Color::rgb8(128, 0, 0);

    /// Opaque navy.
    pub const NAVY: Color = Color::rgb8(0, 0, 128);

    /// Opaque olive.
    pub const OLIVE: Color = Color::rgb8(128, 128, 0);

    /// Opaque purple.
    pub const PURPLE: Color = Color::rgb8(128, 0, 128);

    /// Opaque red.
    pub const RED: Color = Color::rgb8(255, 0, 0);

    /// Opaque silver.
    pub const SILVER: Color = Color::grey8(192);

    /// Opaque teal.
    pub const TEAL: Color = Color::rgb8(0, 128, 128);

    /// Fully transparent
    pub const TRANSPARENT: Color = Color::rgba8(0, 0, 0, 0);

    /// Opaque white.
    pub const WHITE: Color = Color::grey8(255);

    /// Opaque yellow.
    pub const YELLOW: Color = Color::rgb8(255, 255, 0);
}

const ANSI_RGB: [u32; 16] = [
    0x000000, 0x800000, 0x008000, 0x808000, 0x000080, 0x800080, 0x008080, 0xc0c0c0, 0x808080,
    0xff0000, 0x00ff00, 0xffff00, 0x0000ff, 0xff00ff, 0x00ffff, 0xffffff,
];

const XTERM_LEVELS: [u32; 6] = [0, 95, 135, 175, 215, 255];

const fn indexed_rgba_u32(index: u8) -> u32 {
    let (r, g, b) = if index < 16 {
        let rgb = ANSI_RGB[index as usize];
        ((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
    } else if index < 232 {
        let cube = (index - 16) as usize;
        (
            XTERM_LEVELS[cube / 36] as u8,
            XTERM_LEVELS[(cube % 36) / 6] as u8,
            XTERM_LEVELS[cube % 6] as u8,
        )
    } else {
        let grey = 8 + 10 * (index - 232);
        (grey, grey, grey)
    };

    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | 0xff
}

const fn get_4bit_hex_channels(hex_str: &str) -> Result<[u8; 8], ColorParseError> {
    let mut four_bit_channels = match hex_str.as_bytes() {
        &[b'#', r, g, b] | &[r, g, b] => [r, r, g, g, b, b, b'f', b'f'],
        &[b'#', r, g, b, a] | &[r, g, b, a] => [r, r, g, g, b, b, a, a],
        &[b'#', r0, r1, g0, g1, b0, b1] | &[r0, r1, g0, g1, b0, b1] => {
            [r0, r1, g0, g1, b0, b1, b'f', b'f']
        }
        &[b'#', r0, r1, g0, g1, b0, b1, a0, a1] | &[r0, r1, g0, g1, b0, b1, a0, a1] => {
            [r0, r1, g0, g1, b0, b1, a0, a1]
        }
        other => return Err(ColorParseError::WrongSize(other.len())),
    };

    // convert to hex in-place
    // this is written without a for loop to satisfy `const`
    let mut i = 0;
    while i < four_bit_channels.len() {
        let ascii = four_bit_channels[i];
        let as_hex = match hex_from_ascii_byte(ascii) {
            Ok(hex) => hex,
            Err(byte) => return Err(ColorParseError::NotHex { idx: i, byte }),
        };
        four_bit_channels[i] = as_hex;
        i += 1;
    }
    Ok(four_bit_channels)
}

const fn color_from_4bit_hex(components: [u8; 8]) -> Color {
    let [r0, r1, g0, g1, b0, b1, a0, a1] = components;
    Color::rgba8(r0 << 4 | r1, g0 << 4 | g1, b0 << 4 | b1, a0 << 4 | a1)
}

const fn hex_from_ascii_byte(b: u8) -> Result<u8, u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        _ => Err(b),
    }
}

impl Debug for Color {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Color::Rgba32(_) => write!(f, "#{:08x}", self.as_rgba_u32()),
            Color::Indexed(index) => write!(f, "Indexed({index})"),
        }
    }
}

impl std::fmt::Display for ColorParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ColorParseError::WrongSize(n) => write!(f, "Input string has invalid length {n}"),
            ColorParseError::NotHex { idx, byte } => {
                write!(f, "byte {byte:X} at index {idx} is not valid hex digit")
            }
        }
    }
}

impl std::error::Error for ColorParseError {}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn color_from_hex() {
        assert_eq!(Color::from_hex_str("#BAD"), Color::from_hex_str("BBAADD"));
        assert_eq!(
            Color::from_hex_str("#BAD"),
            Ok(Color::from_rgba32_u32(0xBBAADDFF))
        );
        assert_eq!(Color::from_hex_str("BAD"), Color::from_hex_str("BBAADD"));
        assert_eq!(Color::from_hex_str("#BADF"), Color::from_hex_str("BAD"));
        assert_eq!(Color::from_hex_str("#BBAADDFF"), Color::from_hex_str("BAD"));
        assert_eq!(Color::from_hex_str("BBAADDFF"), Color::from_hex_str("BAD"));
        assert_eq!(Color::from_hex_str("bBAadDfF"), Color::from_hex_str("BAD"));
        assert_eq!(Color::from_hex_str("#0f6"), Ok(Color::rgb8(0, 0xff, 0x66)));
        assert_eq!(
            Color::from_hex_str("#0f6a"),
            Ok(Color::rgba8(0, 0xff, 0x66, 0xaa))
        );
        assert!(Color::from_hex_str("#0f6aa").is_err());
        assert!(Color::from_hex_str("#0f").is_err());
        assert!(Color::from_hex_str("x0f").is_err());
        assert!(Color::from_hex_str("#0afa1").is_err());
    }

    #[test]
    fn change_subcolor_values() {
        let color = Color::from_rgba32_u32(0x11aa22bb);

        assert_eq!(color.with_r8(0xff), Color::from_rgba32_u32(0xffaa22bb));
        assert_eq!(color.with_g8(0xff), Color::from_rgba32_u32(0x11ff22bb));
        assert_eq!(color.with_b8(0xff), Color::from_rgba32_u32(0x11aaffbb));
        assert_eq!(color.with_a8(0xff), Color::from_rgba32_u32(0x11aa22ff));
    }

    #[test]
    fn ansi_colors_preserve_palette_indices() {
        assert_eq!(Color::ansi(AnsiColor::Yellow).as_indexed(), Some(3));
        assert_eq!(
            Color::ansi(AnsiColor::Yellow).as_ansi(),
            Some(AnsiColor::Yellow)
        );
        assert_eq!(Color::indexed(196).as_indexed(), Some(196));
        assert_eq!(Color::indexed(196).as_ansi(), None);
        assert_eq!(Color::YELLOW.as_indexed(), None);
        assert_eq!(AnsiColor::from_index(16), None);
    }

    #[test]
    fn indexed_colors_use_xterm_rgb_fallbacks() {
        assert_eq!(
            Color::ansi(AnsiColor::Yellow).as_rgba8(),
            (0x80, 0x80, 0x00, 0xff)
        );
        assert_eq!(Color::indexed(196).as_rgba8(), (0xff, 0x00, 0x00, 0xff));
        assert_eq!(Color::indexed(232).as_rgba8(), (8, 8, 8, 0xff));
        assert_eq!(Color::indexed(255).as_rgba8(), (238, 238, 238, 0xff));
    }

    #[test]
    fn opaque_alpha_operations_preserve_indexed_colors() {
        let green = Color::ansi(AnsiColor::Green);
        assert_eq!(green.with_alpha(1.0), green);
        assert_eq!(green.with_a8(0xff), green);

        let translucent = green.with_alpha(0.5);
        assert_eq!(translucent.as_indexed(), None);
        assert_eq!(translucent.as_rgba8(), (0, 0x80, 0, 0x80));
    }
}
