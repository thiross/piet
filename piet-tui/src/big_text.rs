// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Big-text glyph rasterization built on top of `tui-big-text`.

use piet::Error;
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::Line;
use ratatui_core::widgets::Widget;
use tui_big_text::{BigText, PixelSize};

/// The resolution of a big-text glyph.
///
/// Big text is a display-font mode, not a continuously scalable font. Each
/// glyph comes from the fixed 8x8 `font8x8` bitmap and is represented by one of
/// these terminal-cell footprints.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BigTextSize {
    /// Each glyph occupies 8 columns by 8 rows.
    Full,
    /// Each glyph occupies 8 columns by 4 rows.
    HalfHeight,
    /// Each glyph occupies 4 columns by 8 rows.
    HalfWidth,
    /// Each glyph occupies 4 columns by 4 rows.
    Quadrant,
}

impl BigTextSize {
    pub(crate) const fn glyph_cols(self) -> usize {
        match self {
            Self::Full | Self::HalfHeight => 8,
            Self::HalfWidth | Self::Quadrant => 4,
        }
    }

    pub(crate) const fn glyph_rows(self) -> usize {
        match self {
            Self::Full | Self::HalfWidth => 8,
            Self::HalfHeight | Self::Quadrant => 4,
        }
    }

    const fn pixel_size(self) -> PixelSize {
        match self {
            Self::Full => PixelSize::Full,
            Self::HalfHeight => PixelSize::HalfHeight,
            Self::HalfWidth => PixelSize::HalfWidth,
            Self::Quadrant => PixelSize::Quadrant,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BigTextRaster {
    width_cells: usize,
    height_cells: usize,
    glyph_cols: usize,
    glyph_rows: usize,
    symbols: Vec<char>,
}

impl BigTextRaster {
    pub(crate) const fn glyph_cols(&self) -> usize {
        self.glyph_cols
    }

    pub(crate) const fn glyph_rows(&self) -> usize {
        self.glyph_rows
    }

    pub(crate) fn symbol(&self, x: usize, y: usize) -> char {
        if x >= self.width_cells || y >= self.height_cells {
            return ' ';
        }
        self.symbols[y * self.width_cells + x]
    }

    pub(crate) fn is_blank(&self, x: usize, y: usize, width: usize, height: usize) -> bool {
        (y..y + height).all(|row| (x..x + width).all(|col| self.symbol(col, row) == ' '))
    }
}

pub(crate) fn rasterize(
    lines: &[&str],
    size: BigTextSize,
    width_cells: usize,
) -> Result<BigTextRaster, Error> {
    let height_cells = lines
        .len()
        .checked_mul(size.glyph_rows())
        .ok_or(Error::InvalidInput)?;
    let width = u16::try_from(width_cells).map_err(|_| Error::InvalidInput)?;
    let height = u16::try_from(height_cells).map_err(|_| Error::InvalidInput)?;

    let mut symbols = vec![
        ' ';
        width_cells
            .checked_mul(height_cells)
            .ok_or(Error::InvalidInput)?
    ];
    if width > 0 && height > 0 {
        let area = Rect::new(0, 0, width, height);
        let big_lines: Vec<Line<'_>> = lines.iter().map(|line| Line::from(*line)).collect();
        let widget = BigText::builder()
            .pixel_size(size.pixel_size())
            .lines(big_lines)
            .build();
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer);

        for y in 0..height_cells {
            for x in 0..width_cells {
                symbols[y * width_cells + x] = buffer
                    .cell((x as u16, y as u16))
                    .and_then(|cell| cell.symbol().chars().next())
                    .unwrap_or(' ');
            }
        }
    }

    Ok(BigTextRaster {
        width_cells,
        height_cells,
        glyph_cols: size.glyph_cols(),
        glyph_rows: size.glyph_rows(),
        symbols,
    })
}
