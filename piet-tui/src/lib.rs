// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A terminal backend for Piet, built on top of `ratatui-core`.
//!
//! The backend renders onto a [`ratatui_core::buffer::Buffer`]. Terminal I/O is
//! deliberately left to the application (for example through
//! `ratatui-crossterm`), which keeps this crate independent of any operating
//! system graphics stack.
//!
//! Text is laid out on a character-cell grid. Rich text attributes that map to
//! terminal capabilities are supported: foreground color, bold, italic,
//! underline, strikethrough, wrapping, and alignment. Font family and font size
//! are accepted for API compatibility but are not rendered differently.
//!
//! Images support grayscale, RGB, separate-alpha RGBA, and premultiplied RGBA
//! buffers, with nearest-neighbor or bilinear sampling. Because a terminal cell
//! has only foreground and background colors, each cell renders two vertical
//! image samples using an upper-half block (`▀`).
//!
//! With the optional `big-text` feature, text layouts can use fixed-resolution
//! 8x8 bitmap display fonts through `BigTextSize`. Big text is a layout-wide
//! mode: it has discrete cell footprints and cannot be mixed inline with normal
//! text. It is intended for short display titles rather than body text.

mod color;
mod config;
mod render;
mod text;

#[cfg(feature = "big-text")]
mod big_text;

#[cfg(feature = "big-text")]
pub use big_text::BigTextSize;
pub use config::TuiConfig;
pub use render::{TuiBrush, TuiImage, TuiRenderContext, render_with};
pub use text::{TuiText, TuiTextLayout, TuiTextLayoutBuilder};

pub use piet;

#[cfg(test)]
mod test {
    use static_assertions as sa;

    use super::*;

    sa::assert_impl_all!(TuiText: Clone);
    sa::assert_impl_all!(TuiTextLayout: Clone);
    sa::assert_impl_all!(TuiBrush: Clone);
    sa::assert_impl_all!(TuiImage: Clone);
}
