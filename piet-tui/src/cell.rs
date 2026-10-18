// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Low-level terminal cell drawing extensions.
//!
//! Piet's [`piet::RenderContext`] is designed around portable vector graphics:
//! paths, affine transforms, clips, text layouts, and images. Some terminal UI
//! primitives cannot be represented faithfully by that model. For example, a
//! one-cell border is not a vector outline; it is a set of box-drawing glyphs
//! written directly into terminal cells.
//!
//! [`TuiCellExt`] exposes those terminal-native operations without changing the
//! Piet trait or adding widget semantics to this backend. Higher-level concepts
//! such as blocks, borders, titles, padding, and layout remain the
//! responsibility of the application or UI framework.
//!
//! # Coordinate system
//!
//! Cell coordinates are zero-based and relative to
//! [`TuiRenderContext::area`]. Coordinate `(0, 0)` is the top-left cell of the
//! render context, regardless of where that area is located in the backing
//! ratatui buffer.
//!
//! # Clipping and transforms
//!
//! Cell operations respect the context area and the currently active clip. They
//! intentionally ignore the current affine transform. Terminal cells have no
//! sub-cell transform model, and UI chrome such as borders should normally stay
//! aligned to the terminal grid.
//!
//! # Wide symbols
//!
//! [`TuiCellExt::set_cell`] writes exactly one cell. Callers must use symbols
//! that occupy one terminal cell, or explicitly manage continuation cells when
//! using wide glyphs. Box-drawing characters such as `┌`, `─`, and `│` are safe
//! for this API.

use piet::Color;
use ratatui_core::layout::Rect as TuiRect;

use crate::color::blend_over;
use crate::render::TuiRenderContext;

/// Terminal-native cell drawing operations.
///
/// This trait is an extension implemented by [`TuiRenderContext`]. It is kept
/// separate from [`piet::RenderContext`] so portable drawing code does not
/// depend on terminal-specific cell APIs.
///
/// The intended layering is:
///
/// - Piet renders portable content such as text, fills, strokes, clips, and
///   images.
/// - The application or UI framework owns widget semantics such as a `Block`,
///   border style, title, and padding.
/// - TUI-specific widget code uses this trait to emit terminal cells and glyphs.
///
/// These methods use cell coordinates relative to
/// [`TuiRenderContext::area`], ignore the current transform, and respect the
/// context area and active clip.
///
/// # Drawing a one-cell border
///
/// A framework can implement a simple rectangular border by composing box
/// drawing glyphs with [`set_cell`](Self::set_cell):
///
/// ```rust,no_run
/// use piet::{Color, RenderContext};
/// use piet_tui::{TuiCellExt, TuiConfig, TuiRenderContext};
/// use ratatui_core::buffer::Buffer;
///
/// fn draw_border(buffer: &mut Buffer) {
///     let mut context = TuiRenderContext::new(buffer, TuiConfig::default());
///     let fg = Color::rgb8(180, 190, 210);
///     let bg = Color::BLACK;
///
///     context.set_cell(0, 0, "┌", fg, bg);
///     context.set_cell(1, 0, "─", fg, bg);
///     context.set_cell(2, 0, "┐", fg, bg);
///     context.set_cell(0, 1, "│", fg, bg);
///     context.set_cell(2, 1, "│", fg, bg);
///     context.set_cell(0, 2, "└", fg, bg);
///     context.set_cell(1, 2, "─", fg, bg);
///     context.set_cell(2, 2, "┘", fg, bg);
/// }
/// ```
///
/// More advanced border rules, such as rounded corners, double lines, titles,
/// and junctions, should be implemented by the framework selecting the glyphs
/// and calling [`set_cell`](Self::set_cell). This backend deliberately does not
/// expose a `draw_border` method because that would mix widget policy into the
/// renderer.
pub trait TuiCellExt {
    /// Set one cell's symbol and colors.
    ///
    /// `col` and `row` are zero-based coordinates relative to
    /// [`TuiRenderContext::area`]. The call is a no-op when the cell is outside
    /// the context area or the active clip.
    ///
    /// The operation ignores the current affine transform. It resets the cell to
    /// remove previous symbols, colors, and modifiers before assigning `symbol`,
    /// `fg`, and `bg`.
    ///
    /// Colors are resolved against the existing cell where necessary. The
    /// background is blended first, and the foreground is then blended over the
    /// resulting background. This preserves the same translucent-color behavior
    /// used by the text and image renderers.
    ///
    /// `symbol` should occupy exactly one terminal cell. This method does not
    /// clear or manage continuation cells for wide glyphs; that responsibility
    /// belongs to the caller.
    fn set_cell(&mut self, col: u16, row: u16, symbol: &str, fg: Color, bg: Color);

    /// Fill a local cell rectangle with spaces and explicit foreground and
    /// background colors.
    ///
    /// `area` uses cell coordinates relative to the render context area. Cells
    /// outside the context area or active clip are ignored. The rectangle is
    /// intersected with the context area before iteration.
    ///
    /// Every covered cell is reset and assigned a space symbol together with the
    /// requested `fg` and `bg`. As with [`set_cell`](Self::set_cell), the
    /// operation ignores the current transform.
    fn fill_cells(&mut self, area: TuiRect, fg: Color, bg: Color);
}

impl TuiCellExt for TuiRenderContext<'_> {
    fn set_cell(&mut self, col: u16, row: u16, symbol: &str, fg: Color, bg: Color) {
        // `cell_mut` enforces both the context area and the active clip.
        let Some(cell) = self.cell_mut(col as usize, row as usize) else {
            return;
        };

        // Resolve alpha before resetting the cell so translucent backgrounds can
        // still composite over whatever was previously present.
        let background = blend_over(bg, cell.bg);
        cell.reset();
        cell.bg = background;
        cell.fg = blend_over(fg, cell.bg);
        cell.set_symbol(symbol);
    }

    fn fill_cells(&mut self, area: TuiRect, fg: Color, bg: Color) {
        // Convert the caller's local rectangle into the intersection of the
        // requested area and the render context area. Per-cell clipping is still
        // applied by `set_cell`.
        let available = TuiRect::new(0, 0, self.area().width, self.area().height);
        let area = area.intersection(available);
        for row in area.y..area.bottom() {
            for col in area.x..area.right() {
                self.set_cell(col, row, " ", fg, bg);
            }
        }
    }
}
