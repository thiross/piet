// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The Piet render context implementation backed by a ratatui `Buffer`.

use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

use piet::kurbo::{
    Affine, BezPath, Cap as KurboCap, Join as KurboJoin, PathEl, Point, Rect, Shape, Size, Stroke,
    StrokeOpts, flatten, stroke as expand_stroke,
};
use piet::{
    Color, Error, FixedGradient, GradientStop, Image as PietImage, ImageFormat, InterpolationMode,
    IntoBrush, LineCap, LineJoin, RenderContext, StrokeStyle,
};
use ratatui_core::buffer::{Buffer, Cell};
use ratatui_core::layout::Rect as TuiRect;
use ratatui_core::style::Modifier;

use crate::color::blend_over;
use crate::config::TuiConfig;
#[cfg(feature = "big-text")]
use crate::text::LayoutData;
use crate::text::{TextStyle, TuiText, TuiTextLayout};

/// A brush for the terminal backend.
#[derive(Clone, Debug)]
pub enum TuiBrush {
    /// A single color.
    Solid(Color),
    /// A gradient sampled per terminal cell.
    Gradient(FixedGradient),
}

impl TuiBrush {
    fn color_at(&self, point: Point) -> Color {
        match self {
            Self::Solid(color) => *color,
            Self::Gradient(gradient) => gradient_color(gradient, point),
        }
    }
}

impl<'a> IntoBrush<TuiRenderContext<'a>> for TuiBrush {
    fn make_brush<'b>(
        &'b self,
        _piet: &mut TuiRenderContext<'a>,
        _bbox: impl FnOnce() -> Rect,
    ) -> Cow<'b, TuiBrush> {
        Cow::Borrowed(self)
    }
}

/// An image handle for the terminal backend.
///
/// Pixels are normalized to RGBA and rendered with upper-half block characters,
/// so each terminal cell can represent two vertical image samples.
#[derive(Clone)]
pub struct TuiImage {
    width: usize,
    height: usize,
    pixels: Arc<[[u8; 4]]>,
}

impl PietImage for TuiImage {
    fn size(&self) -> Size {
        Size::new(self.width as f64, self.height as f64)
    }
}

impl fmt::Debug for TuiImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TuiImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl TuiImage {
    /// The width of the image in pixels.
    pub fn width(&self) -> usize {
        self.width
    }

    /// The height of the image in pixels.
    pub fn height(&self) -> usize {
        self.height
    }

    fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
        self.pixels[y * self.width + x]
    }

    fn sample(&self, x: f64, y: f64, interpolation: InterpolationMode) -> [u8; 4] {
        if self.width == 0
            || self.height == 0
            || !x.is_finite()
            || !y.is_finite()
            || x < 0.0
            || y < 0.0
            || x >= self.width as f64
            || y >= self.height as f64
        {
            return [0; 4];
        }

        match interpolation {
            InterpolationMode::NearestNeighbor => self.pixel(x as usize, y as usize),
            InterpolationMode::Bilinear => {
                let x0 = x.floor() as usize;
                let y0 = y.floor() as usize;
                let x1 = (x0 + 1).min(self.width - 1);
                let y1 = (y0 + 1).min(self.height - 1);
                let tx = x - x0 as f64;
                let ty = y - y0 as f64;
                let samples = [
                    (self.pixel(x0, y0), (1.0 - tx) * (1.0 - ty)),
                    (self.pixel(x1, y0), tx * (1.0 - ty)),
                    (self.pixel(x0, y1), (1.0 - tx) * ty),
                    (self.pixel(x1, y1), tx * ty),
                ];
                bilinear_samples(samples)
            }
        }
    }
}

#[derive(Clone)]
struct State {
    transform: Affine,
    clip: Option<Arc<ClipMask>>,
}

impl State {
    fn new() -> Self {
        Self {
            transform: Affine::IDENTITY,
            clip: None,
        }
    }
}

#[derive(Clone)]
struct ClipMask {
    width: usize,
    height: usize,
    mask: Vec<bool>,
}

impl ClipMask {
    fn empty(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            mask: vec![false; width * height],
        }
    }

    fn contains(&self, col: usize, row: usize) -> bool {
        col < self.width
            && row < self.height
            && self
                .mask
                .get(row * self.width + col)
                .copied()
                .unwrap_or(false)
    }

    fn intersect(&mut self, other: &Self) {
        let width = self.width.min(other.width);
        let height = self.height.min(other.height);
        for row in 0..height {
            for col in 0..width {
                let index = row * self.width + col;
                self.mask[index] &= other.mask[row * other.width + col];
            }
        }
        for row in 0..self.height {
            for col in width..self.width {
                self.mask[row * self.width + col] = false;
            }
        }
        for row in height..self.height {
            for col in 0..self.width {
                self.mask[row * self.width + col] = false;
            }
        }
    }
}

/// A Piet render context that paints into a ratatui buffer.
pub struct TuiRenderContext<'a> {
    buffer: &'a mut Buffer,
    area: TuiRect,
    config: Arc<TuiConfig>,
    text: TuiText,
    state: State,
    stack: Vec<State>,
}

impl<'a> TuiRenderContext<'a> {
    /// Create a context covering the entire buffer.
    pub fn new(buffer: &'a mut Buffer, config: TuiConfig) -> Self {
        let area = *buffer.area();
        Self::with_area(buffer, area, config)
    }

    /// Create a context covering a sub-area of the buffer.
    pub fn with_area(buffer: &'a mut Buffer, area: TuiRect, config: TuiConfig) -> Self {
        let area = area.intersection(*buffer.area());
        let config = Arc::new(config);
        let text = TuiText::new(Arc::clone(&config));
        Self {
            buffer,
            area,
            config,
            text,
            state: State::new(),
            stack: Vec::new(),
        }
    }

    /// The configuration in use.
    pub fn config(&self) -> &TuiConfig {
        &self.config
    }

    /// The buffer area covered by this context.
    pub fn area(&self) -> TuiRect {
        self.area
    }

    fn clipped(&self, col: usize, row: usize) -> bool {
        if col >= self.area.width as usize || row >= self.area.height as usize {
            return true;
        }
        self.state
            .clip
            .as_ref()
            .is_some_and(|clip| !clip.contains(col, row))
    }

    fn buffer_cell_mut(&mut self, col: usize, row: usize) -> Option<&mut Cell> {
        if col >= self.area.width as usize || row >= self.area.height as usize {
            return None;
        }
        let x = self.area.x + col as u16;
        let y = self.area.y + row as u16;
        self.buffer.cell_mut((x, y))
    }

    pub(crate) fn cell_mut(&mut self, col: usize, row: usize) -> Option<&mut Cell> {
        if self.clipped(col, row) {
            return None;
        }
        self.buffer_cell_mut(col, row)
    }

    fn cell_center(&self, col: usize, row: usize) -> Point {
        Point::new(
            (col as f64 + 0.5) * self.config.cell_width(),
            (row as f64 + 0.5) * self.config.cell_height(),
        )
    }

    fn cell_range(&self, bbox: Rect) -> (usize, usize, usize, usize) {
        let width = self.area.width as usize;
        let height = self.area.height as usize;
        if !bbox.x0.is_finite()
            || !bbox.y0.is_finite()
            || !bbox.x1.is_finite()
            || !bbox.y1.is_finite()
        {
            return (0, 0, width, height);
        }

        let col0 = (bbox.x0 / self.config.cell_width()).floor().max(0.0) as usize;
        let col1 = (bbox.x1 / self.config.cell_width()).ceil().max(0.0) as usize;
        let row0 = (bbox.y0 / self.config.cell_height()).floor().max(0.0) as usize;
        let row1 = (bbox.y1 / self.config.cell_height()).ceil().max(0.0) as usize;

        (
            col0.min(width),
            row0.min(height),
            col1.min(width),
            row1.min(height),
        )
    }

    fn paint_background(&mut self, col: usize, row: usize, color: Color) {
        if color.as_rgba8().3 == 0 {
            return;
        }
        if let Some(cell) = self.cell_mut(col, row) {
            let bg = blend_over(color, cell.bg);
            cell.reset();
            cell.bg = bg;
        }
    }

    fn fill_impl(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>, even_odd: bool) {
        let bbox = shape.bounding_box();
        let brush = brush.make_brush(self, || bbox);
        let mut path = shape.into_path(0.1);
        path.apply_affine(self.state.transform);
        self.fill_path(&path, &brush, even_odd);
    }

    fn fill_path(&mut self, path: &BezPath, brush: &TuiBrush, even_odd: bool) {
        let flattened = FlattenedPath::new(path);
        let (col0, row0, col1, row1) = self.cell_range(path.bounding_box());

        for row in row0..row1 {
            for col in col0..col1 {
                if self.clipped(col, row) {
                    continue;
                }
                let point = self.cell_center(col, row);
                let inside = if even_odd {
                    flattened.even_odd_contains(point)
                } else {
                    flattened.winding(point) != 0
                };
                if inside {
                    let color = brush.color_at(point);
                    self.paint_background(col, row, color);
                }
            }
        }
    }

    fn stroke_impl(
        &mut self,
        shape: impl Shape,
        brush: &impl IntoBrush<Self>,
        width: f64,
        style: &StrokeStyle,
    ) {
        if !width.is_finite() || width <= 0.0 {
            return;
        }

        let bbox = shape.bounding_box();
        let brush = brush.make_brush(self, || bbox);
        let path = shape.into_path(0.1);
        if path.elements().is_empty() || !path.is_finite() {
            return;
        }

        let mut outline = expand_stroke(
            path.path_elements(0.1),
            &kurbo_stroke(style, width),
            &StrokeOpts::default(),
            0.1,
        );
        if outline.elements().is_empty() || !outline.is_finite() {
            return;
        }

        outline.apply_affine(self.state.transform);
        self.fill_path(&outline, &brush, false);
    }

    fn draw_image_area_impl(
        &mut self,
        image: &TuiImage,
        src_rect: Rect,
        dst_rect: Rect,
        interpolation: InterpolationMode,
    ) {
        if image.width == 0
            || image.height == 0
            || !src_rect.is_finite()
            || !dst_rect.is_finite()
            || src_rect.width() <= 0.0
            || src_rect.height() <= 0.0
            || dst_rect.width() <= 0.0
            || dst_rect.height() <= 0.0
        {
            return;
        }

        let transform = self.state.transform;
        let determinant = transform.determinant();
        if !determinant.is_finite() || determinant.abs() <= f64::EPSILON {
            return;
        }

        let inverse = transform.inverse();
        let bounds = transformed_bounds(dst_rect, transform);
        if !bounds.is_finite() {
            return;
        }
        let (col0, row0, col1, row1) = self.cell_range(bounds);
        let cell_width = self.config.cell_width();
        let cell_height = self.config.cell_height();

        for row in row0..row1 {
            for col in col0..col1 {
                if self.clipped(col, row) {
                    continue;
                }
                let top_point = Point::new(
                    (col as f64 + 0.5) * cell_width,
                    (row as f64 + 0.25) * cell_height,
                );
                let bottom_point = Point::new(
                    (col as f64 + 0.5) * cell_width,
                    (row as f64 + 0.75) * cell_height,
                );
                let top = sample_image(
                    image,
                    inverse * top_point,
                    src_rect,
                    dst_rect,
                    interpolation,
                );
                let bottom = sample_image(
                    image,
                    inverse * bottom_point,
                    src_rect,
                    dst_rect,
                    interpolation,
                );
                if top[3] == 0 && bottom[3] == 0 {
                    continue;
                }

                let Some(cell) = self.cell_mut(col, row) else {
                    continue;
                };
                let background = cell.bg;
                let foreground =
                    blend_over(Color::rgba8(top[0], top[1], top[2], top[3]), background);
                let lower = blend_over(
                    Color::rgba8(bottom[0], bottom[1], bottom[2], bottom[3]),
                    background,
                );
                cell.reset();
                cell.fg = foreground;
                cell.bg = lower;
                cell.set_symbol("▀");
            }
        }
    }

    fn write_text_cell(&mut self, col: usize, row: usize, symbol: &str, style: &TextStyle) -> bool {
        let Some(cell) = self.cell_mut(col, row) else {
            return false;
        };

        if let Some(bg) = style.bg {
            cell.bg = blend_over(bg, cell.bg);
        }
        cell.fg = blend_over(style.fg, cell.bg);
        cell.modifier = modifiers_for(style);
        cell.set_symbol(symbol);
        true
    }

    #[cfg(feature = "big-text")]
    fn draw_big_text(&mut self, data: &LayoutData, origin_col: isize, origin_row: isize) {
        let Some(big_text) = &data.big_text else {
            return;
        };
        let cell_width = self.config.cell_width();
        let glyph_cols = big_text.glyph_cols();
        let glyph_rows = big_text.glyph_rows();

        for (line_index, line) in data.lines.iter().enumerate() {
            let line_row = origin_row + (line_index * glyph_rows) as isize;
            let line_col = origin_col + (line.x_offset / cell_width).round() as isize;

            for grapheme in &data.graphemes[line.grapheme_start..line.grapheme_end] {
                if grapheme.cols == 0 {
                    continue;
                }
                let col = line_col + grapheme.col as isize;
                for row_offset in 0..glyph_rows {
                    let row = line_row + row_offset as isize;
                    if row < 0 {
                        continue;
                    }
                    for col_offset in 0..glyph_cols {
                        let x = col + col_offset as isize;
                        if x < 0 {
                            continue;
                        }
                        let symbol = big_text.symbol(
                            grapheme.col + col_offset,
                            line_index * glyph_rows + row_offset,
                        );
                        let mut encoded = [0; 4];
                        self.write_text_cell(
                            x as usize,
                            row as usize,
                            symbol.encode_utf8(&mut encoded),
                            &grapheme.style,
                        );
                    }
                }
            }
        }
    }
}

impl RenderContext for TuiRenderContext<'_> {
    type Brush = TuiBrush;
    type Text = TuiText;
    type TextLayout = TuiTextLayout;
    type Image = TuiImage;

    fn status(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn solid_brush(&mut self, color: Color) -> Self::Brush {
        TuiBrush::Solid(color)
    }

    fn gradient(&mut self, gradient: impl Into<FixedGradient>) -> Result<Self::Brush, Error> {
        Ok(TuiBrush::Gradient(gradient.into()))
    }

    fn clear(&mut self, region: impl Into<Option<Rect>>, color: Color) {
        let (col0, row0, col1, row1) = match region.into() {
            Some(rect) => self.cell_range(rect),
            None => (0, 0, self.area.width as usize, self.area.height as usize),
        };

        for row in row0..row1 {
            for col in col0..col1 {
                if let Some(cell) = self.buffer_cell_mut(col, row) {
                    let bg = blend_over(color, cell.bg);
                    cell.reset();
                    cell.bg = bg;
                }
            }
        }
    }

    fn stroke(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>, width: f64) {
        self.stroke_impl(shape, brush, width, &StrokeStyle::default());
    }

    fn stroke_styled(
        &mut self,
        shape: impl Shape,
        brush: &impl IntoBrush<Self>,
        width: f64,
        style: &StrokeStyle,
    ) {
        self.stroke_impl(shape, brush, width, style);
    }

    fn fill(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>) {
        self.fill_impl(shape, brush, false);
    }

    fn fill_even_odd(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>) {
        self.fill_impl(shape, brush, true);
    }

    fn clip(&mut self, shape: impl Shape) {
        let mut path = shape.into_path(0.1);
        path.apply_affine(self.state.transform);
        let flattened = FlattenedPath::new(&path);

        let width = self.area.width as usize;
        let height = self.area.height as usize;
        let mut next = ClipMask::empty(width, height);
        let (col0, row0, col1, row1) = self.cell_range(path.bounding_box());
        for row in row0..row1 {
            for col in col0..col1 {
                if self.clipped(col, row) {
                    continue;
                }
                if flattened.winding(self.cell_center(col, row)) != 0 {
                    next.mask[row * width + col] = true;
                }
            }
        }

        match self.state.clip.take() {
            Some(current) => {
                let mut current =
                    Arc::try_unwrap(current).unwrap_or_else(|current| ClipMask::clone(&current));
                current.intersect(&next);
                self.state.clip = Some(Arc::new(current));
            }
            None => self.state.clip = Some(Arc::new(next)),
        }
    }

    fn text(&mut self) -> &mut Self::Text {
        &mut self.text
    }

    fn draw_text(&mut self, layout: &Self::TextLayout, pos: impl Into<Point>) {
        let origin = self.state.transform * pos.into();
        let cell_width = self.config.cell_width();
        let cell_height = self.config.cell_height();
        let origin_col = (origin.x / cell_width).round() as isize;
        let origin_row = (origin.y / cell_height).round() as isize;

        let data = layout.data();
        #[cfg(feature = "big-text")]
        if data.big_text.is_some() {
            self.draw_big_text(data, origin_col, origin_row);
            return;
        }
        let text = data.text.as_str();

        for (line_index, line) in data.lines.iter().enumerate() {
            let row = origin_row + line_index as isize;
            if row < 0 || row as usize >= self.area.height as usize {
                continue;
            }
            let row = row as usize;
            let mut last_cell = None;

            for grapheme in &data.graphemes[line.grapheme_start..line.grapheme_end] {
                let col = origin_col + ((line.x_offset + grapheme.x) / cell_width).round() as isize;
                if col < 0 {
                    continue;
                }
                let col = col as usize;
                let grapheme_text = &text[grapheme.start..grapheme.end];

                if grapheme.cols == 0 {
                    if let Some((last_col, last_row)) = last_cell
                        && let Some(cell) = self.cell_mut(last_col, last_row)
                    {
                        let mut symbol = cell.symbol().to_owned();
                        symbol.push_str(grapheme_text);
                        cell.set_symbol(&symbol);
                    }
                    continue;
                }

                if grapheme_text == "\t" {
                    for offset in 0..grapheme.cols {
                        if self.write_text_cell(col + offset, row, " ", &grapheme.style) {
                            last_cell = Some((col + offset, row));
                        }
                    }
                    continue;
                }

                if grapheme.cols > 1 && self.clipped(col + 1, row) {
                    continue;
                }

                if self.write_text_cell(col, row, grapheme_text, &grapheme.style) {
                    let background = self.cell_mut(col, row).map(|cell| cell.bg);
                    for offset in 1..grapheme.cols {
                        if let Some(cell) = self.cell_mut(col + offset, row) {
                            cell.reset();
                            if let Some(background) = background {
                                cell.bg = background;
                            }
                        }
                    }
                    last_cell = Some((col, row));
                }
            }
        }
    }

    fn save(&mut self) -> Result<(), Error> {
        self.stack.push(self.state.clone());
        Ok(())
    }

    fn restore(&mut self) -> Result<(), Error> {
        self.state = self.stack.pop().ok_or(Error::StackUnbalance)?;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn transform(&mut self, transform: Affine) {
        self.state.transform *= transform;
    }

    fn make_image_with_stride(
        &mut self,
        width: usize,
        height: usize,
        stride: usize,
        buf: &[u8],
        format: ImageFormat,
    ) -> Result<Self::Image, Error> {
        let pixel_count = width.checked_mul(height).ok_or(Error::InvalidInput)?;
        let packed =
            piet::util::image_buffer_to_tightly_packed(buf, width, height, stride, format)?;
        let bytes_per_pixel = format.bytes_per_pixel();
        let mut pixels = Vec::with_capacity(pixel_count);

        for pixel in packed.chunks_exact(bytes_per_pixel) {
            pixels.push(match format {
                ImageFormat::Grayscale => [pixel[0], pixel[0], pixel[0], 255],
                ImageFormat::Rgb => [pixel[0], pixel[1], pixel[2], 255],
                ImageFormat::RgbaSeparate => [pixel[0], pixel[1], pixel[2], pixel[3]],
                ImageFormat::RgbaPremul => {
                    let alpha = pixel[3];
                    [
                        piet::util::unpremul(pixel[0], alpha),
                        piet::util::unpremul(pixel[1], alpha),
                        piet::util::unpremul(pixel[2], alpha),
                        alpha,
                    ]
                }
                _ => return Err(Error::NotSupported),
            });
        }

        Ok(TuiImage {
            width,
            height,
            pixels: pixels.into(),
        })
    }

    fn draw_image(
        &mut self,
        image: &Self::Image,
        dst_rect: impl Into<Rect>,
        interp: InterpolationMode,
    ) {
        let dst_rect = dst_rect.into();
        let src_rect = Rect::new(0.0, 0.0, image.width as f64, image.height as f64);
        self.draw_image_area_impl(image, src_rect, dst_rect, interp);
    }

    fn draw_image_area(
        &mut self,
        image: &Self::Image,
        src_rect: impl Into<Rect>,
        dst_rect: impl Into<Rect>,
        interp: InterpolationMode,
    ) {
        self.draw_image_area_impl(image, src_rect.into(), dst_rect.into(), interp);
    }

    fn capture_image_area(&mut self, _src_rect: impl Into<Rect>) -> Result<Self::Image, Error> {
        Err(Error::NotSupported)
    }

    fn blurred_rect(&mut self, rect: Rect, _blur_radius: f64, brush: &impl IntoBrush<Self>) {
        self.fill(rect, brush);
    }

    fn current_transform(&self) -> Affine {
        self.state.transform
    }
}

/// Render into a buffer with a closure and finish the context automatically.
pub fn render_with<F>(buffer: &mut Buffer, area: TuiRect, config: TuiConfig, draw: F)
where
    F: for<'ctx> FnOnce(&mut TuiRenderContext<'ctx>),
{
    let mut context = TuiRenderContext::with_area(buffer, area, config);
    draw(&mut context);
    let _ = context.finish();
}

fn modifiers_for(style: &TextStyle) -> Modifier {
    let mut modifiers = Modifier::empty();
    if style.weight.to_raw() >= 600 {
        modifiers |= Modifier::BOLD;
    }
    if style.italic {
        modifiers |= Modifier::ITALIC;
    }
    if style.underline {
        modifiers |= Modifier::UNDERLINED;
    }
    if style.strikethrough {
        modifiers |= Modifier::CROSSED_OUT;
    }
    modifiers
}

fn kurbo_stroke(style: &StrokeStyle, width: f64) -> Stroke {
    let join = match style.line_join {
        LineJoin::Miter { .. } => KurboJoin::Miter,
        LineJoin::Round => KurboJoin::Round,
        LineJoin::Bevel => KurboJoin::Bevel,
    };
    let cap = match style.line_cap {
        LineCap::Butt => KurboCap::Butt,
        LineCap::Round => KurboCap::Round,
        LineCap::Square => KurboCap::Square,
    };

    let mut stroke = Stroke::new(width).with_join(join).with_caps(cap);
    if let Some(limit) = style.miter_limit()
        && limit.is_finite()
        && limit > 0.0
    {
        stroke = stroke.with_miter_limit(limit);
    }

    let dashes = style
        .dash_pattern
        .iter()
        .copied()
        .filter(|dash| dash.is_finite() && *dash >= 0.0)
        .collect::<Vec<_>>();
    if dashes.iter().any(|dash| *dash > 0.0) {
        let offset = if style.dash_offset.is_finite() {
            style.dash_offset
        } else {
            0.0
        };
        stroke = stroke.with_dashes(offset, dashes);
    }

    stroke
}

fn gradient_color(gradient: &FixedGradient, point: Point) -> Color {
    match gradient {
        FixedGradient::Linear(linear) => {
            let dx = linear.end.x - linear.start.x;
            let dy = linear.end.y - linear.start.y;
            let length_squared = dx * dx + dy * dy;
            let t = if length_squared == 0.0 {
                0.0
            } else {
                ((point.x - linear.start.x) * dx + (point.y - linear.start.y) * dy) / length_squared
            };
            sample_stops(&linear.stops, t)
        }
        FixedGradient::Radial(radial) => {
            let t = if radial.radius == 0.0 {
                0.0
            } else {
                ((point - radial.center).hypot() / radial.radius).clamp(0.0, 1.0)
            };
            sample_stops(&radial.stops, t)
        }
    }
}

fn sample_stops(stops: &[GradientStop], t: f64) -> Color {
    let Some(first) = stops.first() else {
        return Color::BLACK;
    };
    if stops.len() == 1 {
        return first.color;
    }

    let t = t.clamp(0.0, 1.0) as f32;
    let mut previous = first;
    for stop in &stops[1..] {
        if t <= stop.pos {
            let span = (stop.pos - previous.pos).max(f32::EPSILON);
            let local = ((t - previous.pos) / span).clamp(0.0, 1.0);
            return mix_color(previous.color, stop.color, local);
        }
        previous = stop;
    }
    previous.color
}

fn mix_color(a: Color, b: Color, t: f32) -> Color {
    let (ar, ag, ab, aa) = a.as_rgba8();
    let (br, bg, bb, ba) = b.as_rgba8();
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Color::rgba8(mix(ar, br), mix(ag, bg), mix(ab, bb), mix(aa, ba))
}

fn transformed_bounds(rect: Rect, transform: Affine) -> Rect {
    let p0 = transform * Point::new(rect.x0, rect.y0);
    let p1 = transform * Point::new(rect.x1, rect.y0);
    let p2 = transform * Point::new(rect.x1, rect.y1);
    let p3 = transform * Point::new(rect.x0, rect.y1);

    Rect::new(
        p0.x.min(p1.x).min(p2.x).min(p3.x),
        p0.y.min(p1.y).min(p2.y).min(p3.y),
        p0.x.max(p1.x).max(p2.x).max(p3.x),
        p0.y.max(p1.y).max(p2.y).max(p3.y),
    )
}

fn sample_image(
    image: &TuiImage,
    point: Point,
    src_rect: Rect,
    dst_rect: Rect,
    interpolation: InterpolationMode,
) -> [u8; 4] {
    if !point.x.is_finite()
        || !point.y.is_finite()
        || point.x < dst_rect.x0
        || point.y < dst_rect.y0
        || point.x >= dst_rect.x1
        || point.y >= dst_rect.y1
    {
        return [0; 4];
    }

    let u = (point.x - dst_rect.x0) / dst_rect.width();
    let v = (point.y - dst_rect.y0) / dst_rect.height();
    let x = src_rect.x0 + u * src_rect.width();
    let y = src_rect.y0 + v * src_rect.height();
    image.sample(x, y, interpolation)
}

fn bilinear_samples(samples: [([u8; 4], f64); 4]) -> [u8; 4] {
    let mut red = 0.0;
    let mut green = 0.0;
    let mut blue = 0.0;
    let mut alpha = 0.0;

    for (pixel, weight) in samples {
        let pixel_alpha = pixel[3] as f64 / 255.0;
        let weight = pixel_alpha * weight;
        red += pixel[0] as f64 / 255.0 * weight;
        green += pixel[1] as f64 / 255.0 * weight;
        blue += pixel[2] as f64 / 255.0 * weight;
        alpha += weight;
    }

    if alpha <= 0.0 {
        return [0; 4];
    }

    let channel = |value: f64| (value / alpha * 255.0).round().clamp(0.0, 255.0) as u8;
    [
        channel(red),
        channel(green),
        channel(blue),
        (alpha * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

struct FlattenedPath {
    segments: Vec<(Point, Point)>,
}

impl FlattenedPath {
    fn new(path: &BezPath) -> Self {
        let mut segments = Vec::with_capacity(path.elements().len());
        let mut current = None;
        let mut subpath_start = None;

        flatten(path.path_elements(0.1), 0.1, |element| match element {
            PathEl::MoveTo(p) => {
                current = Some(p);
                subpath_start = Some(p);
            }
            PathEl::LineTo(p) => {
                if let Some(from) = current {
                    segments.push((from, p));
                }
                current = Some(p);
            }
            PathEl::ClosePath => {
                if let (Some(from), Some(to)) = (current, subpath_start)
                    && from != to
                {
                    segments.push((from, to));
                }
                current = subpath_start;
            }
            PathEl::QuadTo(_, _) | PathEl::CurveTo(_, _, _) => {}
        });

        Self { segments }
    }

    fn winding(&self, point: Point) -> i32 {
        self.segments
            .iter()
            .map(|&(from, to)| ray_winding(from, to, point))
            .sum()
    }

    fn even_odd_contains(&self, point: Point) -> bool {
        let mut inside = false;
        for &(from, to) in &self.segments {
            if ray_crosses(from, to, point) {
                inside = !inside;
            }
        }
        inside
    }
}

fn ray_winding(a: Point, b: Point, point: Point) -> i32 {
    if (a.y > point.y) == (b.y > point.y) {
        return 0;
    }
    let x = a.x + (point.y - a.y) * (b.x - a.x) / (b.y - a.y);
    if point.x >= x {
        return 0;
    }
    if b.y > a.y { -1 } else { 1 }
}

fn ray_crosses(a: Point, b: Point, point: Point) -> bool {
    if (a.y > point.y) == (b.y > point.y) {
        return false;
    }
    let x = a.x + (point.y - a.y) * (b.x - a.x) / (b.y - a.y);
    point.x < x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_stroke_style_to_kurbo() {
        let style = StrokeStyle::new()
            .line_join(LineJoin::Miter { limit: 3.0 })
            .line_cap(LineCap::Round)
            .dash_pattern(&[2.0, 1.0])
            .dash_offset(1.0);
        let stroke = kurbo_stroke(&style, 4.0);

        assert_eq!(stroke.width, 4.0);
        assert_eq!(stroke.join, KurboJoin::Miter);
        assert_eq!(stroke.miter_limit, 3.0);
        assert_eq!(stroke.start_cap, KurboCap::Round);
        assert_eq!(stroke.end_cap, KurboCap::Round);
        assert_eq!(stroke.dash_pattern.as_slice(), &[2.0, 1.0]);
        assert_eq!(stroke.dash_offset, 1.0);
    }

    #[test]
    fn invalid_dash_pattern_falls_back_to_solid() {
        let mut style = StrokeStyle::new();
        style.set_dash_pattern([f64::NAN, -1.0, 0.0]);
        let stroke = kurbo_stroke(&style, 1.0);

        assert!(stroke.dash_pattern.is_empty());
    }

    #[test]
    fn bilinear_interpolation_is_alpha_aware() {
        let samples = [
            ([255, 0, 0, 255], 0.5),
            ([0, 0, 0, 0], 0.5),
            ([0, 0, 0, 0], 0.0),
            ([0, 0, 0, 0], 0.0),
        ];

        assert_eq!(bilinear_samples(samples), [255, 0, 0, 128]);
    }
}
