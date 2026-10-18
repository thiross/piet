// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The Piet render context implementation backed by a ratatui `Buffer`.

use std::borrow::Cow;
use std::sync::Arc;

use piet::kurbo::{Affine, BezPath, PathEl, Point, Rect, Shape, Size, flatten};
use piet::{
    Color, Error, FixedGradient, GradientStop, Image as PietImage, ImageFormat, InterpolationMode,
    IntoBrush, RenderContext, StrokeStyle,
};
use ratatui_core::buffer::{Buffer, Cell};
use ratatui_core::layout::Rect as TuiRect;
use ratatui_core::style::Modifier;

use crate::color::blend_over;
use crate::config::TuiConfig;
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
/// Image creation is not implemented in the phase 0-2 backend yet; this type
/// exists so the associated type is concrete and can be extended later.
#[derive(Clone, Debug)]
pub struct TuiImage {
    size: Size,
}

impl PietImage for TuiImage {
    fn size(&self) -> Size {
        self.size
    }
}

#[derive(Clone)]
struct State {
    transform: Affine,
    clip: Option<ClipMask>,
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
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            mask: vec![true; width * height],
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

    fn cell_mut(&mut self, col: usize, row: usize) -> Option<&mut Cell> {
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

        let (col0, row0, col1, row1) = self.cell_range(path.bounding_box());
        for row in row0..row1 {
            for col in col0..col1 {
                let point = self.cell_center(col, row);
                let inside = if even_odd {
                    even_odd_contains(&path, point)
                } else {
                    path.winding(point) != 0
                };
                if inside {
                    let color = brush.color_at(point);
                    self.paint_background(col, row, color);
                }
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

    fn stroke(&mut self, _shape: impl Shape, _brush: &impl IntoBrush<Self>, _width: f64) {
        // Canvas strokes are implemented in the next phase.
    }

    fn stroke_styled(
        &mut self,
        _shape: impl Shape,
        _brush: &impl IntoBrush<Self>,
        _width: f64,
        _style: &StrokeStyle,
    ) {
        // Canvas strokes are implemented in the next phase.
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

        let width = self.area.width as usize;
        let height = self.area.height as usize;
        let mut next = ClipMask::new(width, height);
        for row in 0..height {
            for col in 0..width {
                if path.winding(self.cell_center(col, row)) == 0 {
                    next.mask[row * width + col] = false;
                }
            }
        }

        match self.state.clip.take() {
            Some(mut current) => {
                current.intersect(&next);
                self.state.clip = Some(current);
            }
            None => self.state.clip = Some(next),
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
        _width: usize,
        _height: usize,
        _stride: usize,
        _buf: &[u8],
        _format: ImageFormat,
    ) -> Result<Self::Image, Error> {
        Err(Error::NotSupported)
    }

    fn draw_image(
        &mut self,
        _image: &Self::Image,
        _dst_rect: impl Into<Rect>,
        _interp: InterpolationMode,
    ) {
    }

    fn draw_image_area(
        &mut self,
        _image: &Self::Image,
        _src_rect: impl Into<Rect>,
        _dst_rect: impl Into<Rect>,
        _interp: InterpolationMode,
    ) {
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

fn even_odd_contains(path: &BezPath, point: Point) -> bool {
    let mut inside = false;
    let mut current = None;
    let mut subpath_start = None;

    flatten(path.path_elements(0.1), 0.1, |element| match element {
        PathEl::MoveTo(p) => {
            if let (Some(from), Some(to)) = (current, subpath_start)
                && ray_crosses(from, to, point)
            {
                inside = !inside;
            }
            current = Some(p);
            subpath_start = Some(p);
        }
        PathEl::LineTo(p) => {
            if let Some(from) = current
                && ray_crosses(from, p, point)
            {
                inside = !inside;
            }
            current = Some(p);
        }
        PathEl::ClosePath => {
            if let (Some(from), Some(to)) = (current, subpath_start)
                && ray_crosses(from, to, point)
            {
                inside = !inside;
            }
            current = subpath_start;
        }
        PathEl::QuadTo(_, _) | PathEl::CurveTo(_, _, _) => {}
    });

    inside
}

fn ray_crosses(a: Point, b: Point, point: Point) -> bool {
    if (a.y > point.y) == (b.y > point.y) {
        return false;
    }
    let x = a.x + (point.y - a.y) * (b.x - a.x) / (b.y - a.y);
    point.x < x
}
