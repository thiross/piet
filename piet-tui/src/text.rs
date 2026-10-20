// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Text layout and rich-text style resolution for the terminal backend.

use std::ops::{Range, RangeBounds};
use std::rc::Rc;
use std::sync::Arc;

use piet::kurbo::{Point, Rect, Size};
use piet::util::{LayoutDefaults, resolve_range};
use piet::{
    Color, Error, FontFamily, FontStyle, FontWeight, HitTestPoint, HitTestPosition, LineMetric,
    TextAlignment, TextAttribute, TextLayout, TextLayoutBuilder, TextStorage,
};
use ratatui_core::buffer::CellWidth;
use unicode_segmentation::UnicodeSegmentation;

#[cfg(feature = "big-text")]
use crate::big_text::{BigTextRaster, BigTextSize, rasterize};
use crate::config::TuiConfig;

#[derive(Clone, Copy, Debug, Default)]
enum LayoutMode {
    #[default]
    Cell,
    #[cfg(feature = "big-text")]
    BigText(BigTextSize),
}

impl LayoutMode {
    const fn is_big_text(self) -> bool {
        !matches!(self, Self::Cell)
    }

    const fn glyph_cols(self) -> usize {
        match self {
            Self::Cell => 1,
            #[cfg(feature = "big-text")]
            Self::BigText(size) => size.glyph_cols(),
        }
    }

    const fn glyph_rows(self) -> usize {
        match self {
            Self::Cell => 1,
            #[cfg(feature = "big-text")]
            Self::BigText(size) => size.glyph_rows(),
        }
    }
}

/// A text factory for the terminal backend.
#[derive(Clone, Debug)]
pub struct TuiText {
    config: Arc<TuiConfig>,
}

impl TuiText {
    pub(crate) fn new(config: Arc<TuiConfig>) -> Self {
        Self { config }
    }

    /// The configuration used by layouts created from this factory.
    pub fn config(&self) -> &TuiConfig {
        &self.config
    }
}

impl piet::Text for TuiText {
    type TextLayoutBuilder = TuiTextLayoutBuilder;
    type TextLayout = TuiTextLayout;

    fn font_family(&mut self, family_name: &str) -> Option<FontFamily> {
        Some(FontFamily::new_unchecked(family_name))
    }

    fn load_font(&mut self, _data: &[u8]) -> Result<FontFamily, Error> {
        Err(Error::NotSupported)
    }

    fn new_text_layout(&mut self, text: impl TextStorage) -> Self::TextLayoutBuilder {
        TuiTextLayoutBuilder {
            text: Rc::new(text),
            config: Arc::clone(&self.config),
            defaults: LayoutDefaults::default(),
            attributes: Vec::new(),
            backgrounds: Vec::new(),
            max_width: f64::INFINITY,
            alignment: TextAlignment::default(),
            mode: LayoutMode::Cell,
        }
    }
}

/// Builder for [`TuiTextLayout`].
pub struct TuiTextLayoutBuilder {
    text: Rc<dyn TextStorage>,
    config: Arc<TuiConfig>,
    defaults: LayoutDefaults,
    attributes: Vec<(Range<usize>, TextAttribute)>,
    backgrounds: Vec<(Range<usize>, Color)>,
    max_width: f64,
    alignment: TextAlignment,
    mode: LayoutMode,
}

impl std::fmt::Debug for TuiTextLayoutBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TuiTextLayoutBuilder")
            .field("text", &self.text.as_str())
            .field("max_width", &self.max_width)
            .field("alignment", &self.alignment)
            .field("attribute_count", &self.attributes.len())
            .field("background_count", &self.backgrounds.len())
            .field("mode", &self.mode)
            .finish()
    }
}

impl TuiTextLayoutBuilder {
    /// Set a background color for a byte range.
    ///
    /// Piet's [`TextAttribute`] does not include background color, so this is an
    /// additive terminal-specific extension. Later ranges override earlier ones.
    pub fn background_color(mut self, range: impl RangeBounds<usize>, color: Color) -> Self {
        let range = resolve_range(range, self.text.as_str().len());
        self.backgrounds.push((range, color));
        self
    }

    /// Render this layout with a fixed-resolution big-text display font.
    ///
    /// This is a terminal-specific layout mode. `FontFamily` and `FontSize`
    /// attributes are ignored because the glyph resolution is selected by
    /// [`BigTextSize`]. Big text cannot be mixed inline with normal cell text;
    /// create a separate layout for titles and body text.
    #[cfg(feature = "big-text")]
    pub fn big_text(mut self, size: BigTextSize) -> Self {
        self.mode = LayoutMode::BigText(size);
        self
    }
}

impl TextLayoutBuilder for TuiTextLayoutBuilder {
    type Out = TuiTextLayout;

    fn max_width(mut self, width: f64) -> Self {
        self.max_width = if width.is_nan() || width < 0.0 {
            f64::INFINITY
        } else {
            width
        };
        self
    }

    fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    fn default_attribute(mut self, attribute: impl Into<TextAttribute>) -> Self {
        self.defaults.set(attribute);
        self
    }

    fn range_attribute(
        mut self,
        range: impl RangeBounds<usize>,
        attribute: impl Into<TextAttribute>,
    ) -> Self {
        let range = resolve_range(range, self.text.as_str().len());
        self.attributes.push((range, attribute.into()));
        self
    }

    fn build(self) -> Result<Self::Out, Error> {
        TuiTextLayout::from_builder(self)
    }
}

/// A text layout on the terminal character grid.
#[derive(Clone)]
pub struct TuiTextLayout {
    data: Rc<LayoutData>,
}

impl TuiTextLayout {
    fn from_builder(builder: TuiTextLayoutBuilder) -> Result<Self, Error> {
        let layout = LayoutBuilder {
            text: builder.text.as_str(),
            config: &builder.config,
            defaults: &builder.defaults,
            attributes: &builder.attributes,
            backgrounds: &builder.backgrounds,
            max_width: builder.max_width,
            alignment: builder.alignment,
            mode: builder.mode,
            lines: Vec::new(),
            graphemes: Vec::new(),
        }
        .build()?;

        Ok(Self {
            data: Rc::new(LayoutData {
                text: builder.text,
                config: builder.config,
                size: layout.size,
                trailing_whitespace_width: layout.trailing_whitespace_width,
                line_height_cells: layout.line_height_cells,
                lines: layout.lines,
                graphemes: layout.graphemes,
                #[cfg(feature = "big-text")]
                big_text: layout.big_text,
            }),
        })
    }

    pub(crate) fn data(&self) -> &LayoutData {
        &self.data
    }
}

/// Convert a requested drawing origin to the nearest terminal cell origin.
///
/// This is the single source of truth shared by [`TextLayout::layout_origin`]
/// and `draw_text`.
pub(crate) fn origin_cell(config: &TuiConfig, origin: Point) -> (isize, isize) {
    (
        (origin.x / config.cell_width()).round() as isize,
        (origin.y / config.cell_height()).round() as isize,
    )
}

impl std::fmt::Debug for TuiTextLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TuiTextLayout")
            .field("text", &self.data.text.as_str())
            .field("size", &self.data.size)
            .field("line_count", &self.data.lines.len())
            .finish()
    }
}

impl TextLayout for TuiTextLayout {
    fn size(&self) -> Size {
        self.data.size
    }

    fn layout_origin(&self, requested: Point) -> Point {
        let (col, row) = origin_cell(&self.data.config, requested);
        Point::new(
            col as f64 * self.data.config.cell_width(),
            row as f64 * self.data.config.cell_height(),
        )
    }

    fn trailing_whitespace_width(&self) -> f64 {
        self.data.trailing_whitespace_width
    }

    fn image_bounds(&self) -> Rect {
        self.data.size.to_rect()
    }

    fn text(&self) -> &str {
        self.data.text.as_str()
    }

    fn line_text(&self, line_number: usize) -> Option<&str> {
        self.data
            .lines
            .get(line_number)
            .map(|line| &self.data.text.as_str()[line.start..line.end])
    }

    fn line_metric(&self, line_number: usize) -> Option<LineMetric> {
        let line = self.data.lines.get(line_number)?;
        let cell_height = self.data.config.cell_height();
        let line_height = self.data.line_height_cells as f64 * cell_height;
        Some(LineMetric {
            start_offset: line.start,
            end_offset: line.end,
            trailing_whitespace: line.trailing_whitespace,
            baseline: line_height * self.data.config.baseline_ratio,
            height: line_height,
            y_offset: line_number as f64 * line_height,
        })
    }

    fn line_count(&self) -> usize {
        self.data.lines.len()
    }

    fn hit_test_point(&self, point: Point) -> HitTestPoint {
        let data = &self.data;
        let cell_width = data.config.cell_width();
        let cell_height = data.config.cell_height();
        let line_height = data.line_height_cells as f64 * cell_height;
        let is_inside = point.x >= 0.0
            && point.y >= 0.0
            && point.x <= data.size.width
            && point.y <= data.size.height;

        let line_index = ((point.y / line_height).floor() as isize)
            .clamp(0, data.lines.len().saturating_sub(1) as isize)
            as usize;
        let line = &data.lines[line_index];
        let relative_x = point.x - line.x_offset;

        let mut best_idx = line.start;
        let mut best_distance = relative_x.abs();
        let mut x = 0.0;

        for grapheme in &data.graphemes[line.grapheme_start..line.grapheme_end] {
            let distance = (relative_x - x).abs();
            if distance < best_distance {
                best_distance = distance;
                best_idx = grapheme.start;
            }

            x += grapheme.cols as f64 * cell_width;

            let distance = (relative_x - x).abs();
            if distance < best_distance {
                best_distance = distance;
                best_idx = grapheme.end;
            }
        }

        HitTestPoint::new(best_idx, is_inside)
    }

    fn hit_test_text_position(&self, idx: usize) -> HitTestPosition {
        let data = &self.data;
        let text = data.text.as_str();
        let idx = idx.min(text.len());
        assert!(text.is_char_boundary(idx));

        let line_index = data
            .lines
            .iter()
            .position(|line| idx >= line.start && idx <= line.end)
            .or_else(|| data.lines.iter().rposition(|line| line.start <= idx))
            .unwrap_or(0);
        let line = &data.lines[line_index];
        let cell_width = data.config.cell_width();
        let cell_height = data.config.cell_height();
        let line_height = data.line_height_cells as f64 * cell_height;

        let mut x = line.x_offset;
        for grapheme in &data.graphemes[line.grapheme_start..line.grapheme_end] {
            if idx <= grapheme.start {
                break;
            }
            if idx >= grapheme.end {
                x += grapheme.cols as f64 * cell_width;
            } else {
                break;
            }
        }

        let y = line_index as f64 * line_height + line_height * data.config.baseline_ratio;
        HitTestPosition::new(Point::new(x, y), line_index)
    }
}

pub(crate) struct LayoutData {
    pub(crate) text: Rc<dyn TextStorage>,
    pub(crate) config: Arc<TuiConfig>,
    pub(crate) size: Size,
    pub(crate) trailing_whitespace_width: f64,
    pub(crate) line_height_cells: usize,
    pub(crate) lines: Vec<LineData>,
    pub(crate) graphemes: Vec<GraphemeData>,
    #[cfg(feature = "big-text")]
    pub(crate) big_text: Option<BigTextRaster>,
}

pub(crate) struct LineData {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) trailing_whitespace: usize,
    pub(crate) full_cols: usize,
    pub(crate) trimmed_cols: usize,
    pub(crate) x_offset: f64,
    pub(crate) grapheme_start: usize,
    pub(crate) grapheme_end: usize,
}

pub(crate) struct GraphemeData {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) cols: usize,
    #[cfg_attr(not(feature = "big-text"), allow(dead_code))]
    pub(crate) col: usize,
    pub(crate) x: f64,
    pub(crate) style: TextStyle,
}

pub(crate) struct TextStyle {
    pub(crate) fg: Color,
    pub(crate) bg: Option<Color>,
    pub(crate) weight: FontWeight,
    pub(crate) italic: bool,
    pub(crate) underline: bool,
    pub(crate) strikethrough: bool,
}

struct BuildResult {
    size: Size,
    trailing_whitespace_width: f64,
    line_height_cells: usize,
    lines: Vec<LineData>,
    graphemes: Vec<GraphemeData>,
    #[cfg(feature = "big-text")]
    big_text: Option<BigTextRaster>,
}

struct ParaGrapheme {
    start: usize,
    end: usize,
    base_cols: usize,
    is_tab: bool,
    is_whitespace: bool,
}

impl ParaGrapheme {
    fn cols_at(&self, col: usize, tab_width: usize) -> usize {
        if self.is_tab {
            let tab_width = tab_width.max(1);
            tab_width - (col % tab_width)
        } else {
            self.base_cols
        }
    }
}

struct LayoutBuilder<'a> {
    text: &'a str,
    config: &'a TuiConfig,
    defaults: &'a LayoutDefaults,
    attributes: &'a [(Range<usize>, TextAttribute)],
    backgrounds: &'a [(Range<usize>, Color)],
    max_width: f64,
    alignment: TextAlignment,
    mode: LayoutMode,
    lines: Vec<LineData>,
    graphemes: Vec<GraphemeData>,
}

impl LayoutBuilder<'_> {
    fn build(mut self) -> Result<BuildResult, Error> {
        let mut paragraph_start = 0;
        loop {
            let paragraph_end = self.text[paragraph_start..]
                .find('\n')
                .map(|offset| paragraph_start + offset)
                .unwrap_or(self.text.len());
            let content_end = if paragraph_end > paragraph_start
                && self.text.as_bytes()[paragraph_end - 1] == b'\r'
            {
                paragraph_end - 1
            } else {
                paragraph_end
            };

            self.push_paragraph(paragraph_start, content_end);

            if paragraph_end == self.text.len() {
                break;
            }
            paragraph_start = paragraph_end + 1;
        }

        if self.lines.is_empty() {
            self.lines.push(LineData {
                start: 0,
                end: 0,
                trailing_whitespace: 0,
                full_cols: 0,
                trimmed_cols: 0,
                x_offset: 0.0,
                grapheme_start: 0,
                grapheme_end: 0,
            });
        }

        let cell_width = self.config.cell_width();
        let cell_height = self.config.cell_height();
        let actual_width = self
            .lines
            .iter()
            .map(|line| line.trimmed_cols)
            .max()
            .unwrap_or(0) as f64
            * cell_width;
        let available_cols = if self.max_width.is_finite() {
            (self.max_width / cell_width).floor().max(0.0)
        } else {
            f64::INFINITY
        };

        for line in &mut self.lines {
            let extra = (available_cols - line.trimmed_cols as f64).max(0.0);
            line.x_offset = match self.alignment {
                TextAlignment::End => extra,
                TextAlignment::Center => extra * 0.5,
                TextAlignment::Start | TextAlignment::Justified => 0.0,
            } * cell_width;
        }

        let width = if self.max_width.is_finite() {
            self.max_width.max(actual_width)
        } else {
            actual_width
        };
        let trailing_whitespace_width = self
            .lines
            .iter()
            .map(|line| line.full_cols)
            .max()
            .unwrap_or(0) as f64
            * cell_width;

        let line_height_cells = self.mode.glyph_rows();
        #[cfg(feature = "big-text")]
        let big_text = self.build_big_text()?;

        Ok(BuildResult {
            size: Size::new(
                width,
                self.lines.len().max(1) as f64 * line_height_cells as f64 * cell_height,
            ),
            trailing_whitespace_width,
            line_height_cells,
            lines: self.lines,
            graphemes: self.graphemes,
            #[cfg(feature = "big-text")]
            big_text,
        })
    }

    #[cfg(feature = "big-text")]
    fn build_big_text(&self) -> Result<Option<BigTextRaster>, Error> {
        let LayoutMode::BigText(size) = self.mode else {
            return Ok(None);
        };

        let width_cells = self
            .lines
            .iter()
            .map(|line| line.full_cols)
            .max()
            .unwrap_or(0);
        let line_texts = self
            .lines
            .iter()
            .map(|line| &self.text[line.start..line.end])
            .collect::<Vec<_>>();
        let raster = rasterize(&line_texts, size, width_cells)?;

        for (line_index, line) in self.lines.iter().enumerate() {
            let y = line_index * size.glyph_rows();
            for grapheme in &self.graphemes[line.grapheme_start..line.grapheme_end] {
                let text = &self.text[grapheme.start..grapheme.end];
                if grapheme.cols == 0 || text.chars().all(char::is_whitespace) {
                    continue;
                }
                if raster.is_blank(grapheme.col, y, grapheme.cols, size.glyph_rows()) {
                    return Err(Error::NotSupported);
                }
            }
        }

        Ok(Some(raster))
    }

    fn push_paragraph(&mut self, start: usize, end: usize) {
        let paragraph = &self.text[start..end];
        let mut infos = Vec::new();
        let is_big_text = self.mode.is_big_text();

        for (offset, grapheme) in paragraph.grapheme_indices(true) {
            let is_tab = !is_big_text && grapheme == "\t";
            let is_whitespace = grapheme.chars().all(char::is_whitespace);
            let base_cols = if is_big_text {
                self.mode.glyph_cols()
            } else if is_tab || grapheme.chars().all(char::is_control) {
                0
            } else {
                grapheme.cell_width() as usize
            };
            infos.push(ParaGrapheme {
                start: start + offset,
                end: start + offset + grapheme.len(),
                base_cols,
                is_tab,
                is_whitespace,
            });
        }

        if infos.is_empty() {
            self.push_line(&infos, 0, 0, start);
            return;
        }

        let max_cols = if self.max_width.is_finite() {
            ((self.max_width / self.config.cell_width()).floor() as usize).max(1)
        } else {
            usize::MAX
        };

        let mut line_start = 0;
        let mut col = 0;
        let mut last_break = None;
        let mut idx = 0;

        while idx < infos.len() {
            let width = infos[idx].cols_at(col, self.config.tab_width);

            if width > 0 && col > 0 && col.saturating_add(width) > max_cols {
                if infos[idx].is_whitespace {
                    self.push_line(&infos, line_start, idx + 1, start);
                    line_start = idx + 1;
                    col = 0;
                    last_break = None;
                    idx += 1;
                    continue;
                }

                let break_at = last_break
                    .filter(|&break_at| break_at > line_start)
                    .unwrap_or(idx);
                if break_at == line_start {
                    self.push_line(&infos, line_start, idx + 1, start);
                    line_start = idx + 1;
                    col = 0;
                    last_break = None;
                    idx += 1;
                    continue;
                }

                self.push_line(&infos, line_start, break_at, start);
                line_start = break_at;
                col = recompute_cols(&infos[line_start..idx], self.config.tab_width);
                last_break = None;
                continue;
            }

            col += width;
            if infos[idx].is_whitespace {
                last_break = Some(idx + 1);
            }
            idx += 1;
        }

        if line_start < infos.len() {
            self.push_line(&infos, line_start, infos.len(), start);
        }
    }

    fn push_line(&mut self, infos: &[ParaGrapheme], from: usize, to: usize, empty_offset: usize) {
        let line_infos = &infos[from..to];
        let start = line_infos
            .first()
            .map(|info| info.start)
            .unwrap_or(empty_offset);
        let end = line_infos
            .last()
            .map(|info| info.end)
            .unwrap_or(empty_offset);
        let grapheme_start = self.graphemes.len();
        let cell_width = self.config.cell_width();

        let mut col = 0;
        let mut x = 0.0;
        for info in line_infos {
            let cols = info.cols_at(col, self.config.tab_width);
            self.graphemes.push(GraphemeData {
                start: info.start,
                end: info.end,
                cols,
                col,
                x,
                style: self.resolve_style(info.start),
            });
            col += cols;
            x += cols as f64 * cell_width;
        }
        let grapheme_end = self.graphemes.len();

        let mut trailing_whitespace = 0;
        let mut trailing_cols = 0;
        for (grapheme, info) in self.graphemes[grapheme_start..grapheme_end]
            .iter()
            .rev()
            .zip(line_infos.iter().rev())
        {
            if !info.is_whitespace {
                break;
            }
            trailing_whitespace += grapheme.end - grapheme.start;
            trailing_cols += grapheme.cols;
        }

        self.lines.push(LineData {
            start,
            end,
            trailing_whitespace,
            full_cols: col,
            trimmed_cols: col.saturating_sub(trailing_cols),
            x_offset: 0.0,
            grapheme_start,
            grapheme_end,
        });
    }

    fn resolve_style(&self, position: usize) -> TextStyle {
        let fg = resolve_attribute(self.attributes, position, |attribute| match attribute {
            TextAttribute::TextColor(color) => Some(*color),
            _ => None,
        })
        .unwrap_or(self.defaults.fg_color);
        let weight = resolve_attribute(self.attributes, position, |attribute| match attribute {
            TextAttribute::Weight(weight) => Some(*weight),
            _ => None,
        })
        .unwrap_or(self.defaults.weight);
        let italic = resolve_attribute(self.attributes, position, |attribute| match attribute {
            TextAttribute::Style(style) => Some(*style == FontStyle::Italic),
            _ => None,
        })
        .unwrap_or(self.defaults.style == FontStyle::Italic);
        let underline = resolve_attribute(self.attributes, position, |attribute| match attribute {
            TextAttribute::Underline(underline) => Some(*underline),
            _ => None,
        })
        .unwrap_or(self.defaults.underline);
        let strikethrough =
            resolve_attribute(self.attributes, position, |attribute| match attribute {
                TextAttribute::Strikethrough(strikethrough) => Some(*strikethrough),
                _ => None,
            })
            .unwrap_or(self.defaults.strikethrough);

        let mut bg = None;
        for (range, color) in self.backgrounds {
            if range.contains(&position) {
                bg = Some(*color);
            }
        }

        TextStyle {
            fg,
            bg,
            weight,
            italic,
            underline,
            strikethrough,
        }
    }
}

fn resolve_attribute<T: Copy>(
    attributes: &[(Range<usize>, TextAttribute)],
    position: usize,
    project: impl Fn(&TextAttribute) -> Option<T>,
) -> Option<T> {
    let mut best = None;
    for (range, attribute) in attributes {
        if range.contains(&position)
            && let Some(value) = project(attribute)
        {
            best = Some(value);
        }
    }
    best
}

fn recompute_cols(infos: &[ParaGrapheme], tab_width: usize) -> usize {
    let mut col = 0;
    for info in infos {
        col += info.cols_at(col, tab_width);
    }
    col
}
