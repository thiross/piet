// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use piet::kurbo::Size;

/// Configuration for the terminal renderer.
///
/// The default is **cell mode**, where one logical Piet unit is one terminal
/// cell. This makes `max_width(80.0)` mean "80 columns" and
/// `Rect::new(0.0, 0.0, 80.0, 24.0)` cover an 80x24 terminal area.
///
/// For porting existing GUI code, use [`TuiConfig::with_cell_size`] with a
/// nominal cell size such as `Size::new(8.0, 16.0)`. Geometry is then
/// interpreted in those logical units, while text still occupies one cell per
/// grapheme cluster.
#[derive(Clone, Debug, PartialEq)]
pub struct TuiConfig {
    /// Size of one terminal cell in logical units.
    pub cell_size: Size,
    /// Baseline position as a fraction of the cell height.
    pub baseline_ratio: f64,
    /// Number of columns between tab stops.
    pub tab_width: usize,
}

impl TuiConfig {
    /// Create a cell-mode configuration where one logical unit is one cell.
    pub fn cells() -> Self {
        Self::default()
    }

    /// Create a configuration with an explicit logical cell size.
    pub fn with_cell_size(cell_size: Size) -> Self {
        Self {
            cell_size,
            ..Self::default()
        }
    }

    /// Width of one terminal cell in logical units.
    pub fn cell_width(&self) -> f64 {
        self.cell_size.width.max(f64::EPSILON)
    }

    /// Height of one terminal cell in logical units.
    pub fn cell_height(&self) -> f64 {
        self.cell_size.height.max(f64::EPSILON)
    }

    /// Width of one terminal cell in logical units.
    pub fn logical_cell_width(&self) -> f64 {
        self.cell_width()
    }

    /// Height of one terminal cell in logical units.
    pub fn logical_cell_height(&self) -> f64 {
        self.cell_height()
    }
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            cell_size: Size::new(1.0, 1.0),
            baseline_ratio: 0.8,
            tab_width: 4,
        }
    }
}
