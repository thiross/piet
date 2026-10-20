// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Support for the Piet terminal back-end.
//!
//! The terminal backend renders into a [`ratatui_core::buffer::Buffer`].
//! Unlike the native bitmap backends, the dimensions passed to
//! [`Device::bitmap_target`] are measured in terminal cells, and `pix_scale` is
//! ignored.

use std::marker::PhantomData;
use std::path::Path;

use piet::{Error, ImageBuf, ImageFormat};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;

#[doc(hidden)]
pub use piet_tui::*;
#[doc(hidden)]
pub use ratatui_core;

/// The `RenderContext` for the terminal backend, which is selected.
pub type Piet<'a> = TuiRenderContext<'a>;

/// The associated brush type for this backend.
///
/// This type matches `RenderContext::Brush`.
pub type Brush = TuiBrush;

/// The associated text factory for this backend.
///
/// This type matches `RenderContext::Text`.
pub type PietText = TuiText;

/// The associated text layout type for this backend.
///
/// This type matches `RenderContext::Text::TextLayout`.
pub type PietTextLayout = TuiTextLayout;

/// The associated text layout builder for this backend.
///
/// This type matches `RenderContext::Text::TextLayoutBuilder`.
pub type PietTextLayoutBuilder = TuiTextLayoutBuilder;

/// The associated image type for this backend.
///
/// This type matches `RenderContext::Image`.
pub type PietImage = TuiImage;

/// A device that creates terminal buffers and render targets.
pub struct Device {
    config: TuiConfig,
    // Since not all backends can support `Device: Sync`, make it non-Sync here to, for fewer
    // portability surprises.
    marker: PhantomData<*const ()>,
}

unsafe impl Send for Device {}

/// A terminal render target backed by a ratatui buffer.
///
/// The buffer owns the terminal cell grid. A render context borrowed from this
/// target writes directly into that grid.
pub struct BitmapTarget<'a> {
    buffer: Buffer,
    config: TuiConfig,
    phantom: PhantomData<&'a ()>,
}

impl Device {
    /// Create a new device using the default terminal configuration.
    pub fn new() -> Result<Device, Error> {
        Self::with_config(TuiConfig::default())
    }

    /// Create a new device with an explicit terminal configuration.
    pub fn with_config(config: TuiConfig) -> Result<Device, Error> {
        Ok(Device {
            config,
            marker: PhantomData,
        })
    }

    /// The configuration used by targets created from this device.
    pub fn config(&self) -> &TuiConfig {
        &self.config
    }

    /// Create a new terminal target.
    ///
    /// `width` and `height` are measured in terminal cells. `pix_scale` is
    /// accepted for compatibility with the native backends and is ignored.
    pub fn bitmap_target(
        &mut self,
        width: usize,
        height: usize,
        _pix_scale: f64,
    ) -> Result<BitmapTarget<'_>, Error> {
        let width = u16::try_from(width).map_err(|_| Error::InvalidInput)?;
        let height = u16::try_from(height).map_err(|_| Error::InvalidInput)?;
        let buffer = Buffer::empty(Rect::new(0, 0, width, height));
        Ok(BitmapTarget {
            buffer,
            config: self.config.clone(),
            phantom: PhantomData,
        })
    }
}

impl<'a> BitmapTarget<'a> {
    /// Create a target from an existing buffer and terminal configuration.
    pub fn new(buffer: Buffer, config: TuiConfig) -> Self {
        Self {
            buffer,
            config,
            phantom: PhantomData,
        }
    }

    /// Get a Piet render context for the whole buffer.
    ///
    /// Note: the caller is responsible for calling `finish` on the render
    /// context at the end of rendering.
    pub fn render_context(&mut self) -> TuiRenderContext<'_> {
        TuiRenderContext::new(&mut self.buffer, self.config.clone())
    }

    /// The buffer area in terminal cells.
    pub fn area(&self) -> Rect {
        self.buffer.area
    }

    /// The terminal configuration used by render contexts from this target.
    pub fn config(&self) -> &TuiConfig {
        &self.config
    }

    /// Borrow the underlying ratatui buffer.
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Mutably borrow the underlying ratatui buffer.
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    /// Consume the target and return its ratatui buffer.
    pub fn into_buffer(self) -> Buffer {
        self.buffer
    }

    /// Terminal output cannot be extracted as a pixel buffer.
    ///
    /// Use [`buffer`](Self::buffer) to inspect or display the rendered cells.
    #[allow(clippy::wrong_self_convention)]
    pub fn to_image_buf(&mut self, _fmt: ImageFormat) -> Result<ImageBuf, Error> {
        Err(Error::NotSupported)
    }

    /// Terminal output cannot be extracted as raw RGBA pixels.
    pub fn copy_raw_pixels(&mut self, _fmt: ImageFormat, _buf: &mut [u8]) -> Result<usize, Error> {
        Err(Error::NotSupported)
    }

    /// Terminal output cannot be saved as an RGBA PNG.
    pub fn save_to_file<P: AsRef<Path>>(self, _path: P) -> Result<(), Error> {
        Err(Error::NotSupported)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use piet::kurbo::{Point, Rect as KurboRect, Size};
    use piet::{
        AnsiColor, Color, Error, RenderContext, Text as _, TextAttribute, TextLayoutBuilder,
    };
    use ratatui_core::style::Color as TuiColor;

    #[test]
    fn bitmap_target_renders_into_buffer() {
        let mut device = Device::new().unwrap();
        let mut target = device.bitmap_target(12, 3, 1.0).unwrap();
        assert_eq!(target.area(), Rect::new(0, 0, 12, 3));

        {
            let mut context = target.render_context();
            context.clear(None, Color::rgb8(1, 2, 3));
            context.fill(KurboRect::new(0.0, 2.0, 12.0, 3.0), &Color::rgb8(4, 5, 6));
            let layout = context.text().new_text_layout("OK").build().unwrap();
            context.draw_text(&layout, Point::ZERO);
            context.finish().unwrap();
        }

        let first = target.buffer().cell((0, 0)).unwrap();
        assert_eq!(first.symbol(), "O");
        assert_eq!(first.bg, TuiColor::Rgb(1, 2, 3));
        assert_eq!(
            target.buffer().cell((0, 2)).unwrap().bg,
            TuiColor::Rgb(4, 5, 6)
        );
    }

    #[test]
    fn indexed_colors_survive_common_backend_paths() {
        let mut device = Device::new().unwrap();
        let mut target = device.bitmap_target(4, 2, 1.0).unwrap();

        {
            let mut context = target.render_context();
            context.clear(None, Color::ansi(AnsiColor::Blue));
            context.fill(
                KurboRect::new(0.0, 0.0, 2.0, 1.0),
                &Color::ansi(AnsiColor::Green),
            );
            let layout = context
                .text()
                .new_text_layout("A")
                .range_attribute(
                    0..1,
                    TextAttribute::TextColor(Color::ansi(AnsiColor::Yellow)),
                )
                .build()
                .unwrap();
            context.draw_text(&layout, Point::ZERO);
            context.finish().unwrap();
        }

        let cell = target.buffer().cell((0, 0)).unwrap();
        assert_eq!(cell.fg, TuiColor::Indexed(3));
        assert_eq!(cell.bg, TuiColor::Indexed(2));
        assert_eq!(
            target.buffer().cell((2, 0)).unwrap().bg,
            TuiColor::Indexed(4)
        );
    }

    #[test]
    fn bitmap_target_reuses_existing_buffer() {
        let buffer = Buffer::empty(Rect::new(0, 0, 4, 1));
        let mut target = BitmapTarget::new(buffer, TuiConfig::default());
        target
            .buffer_mut()
            .cell_mut((1, 0))
            .unwrap()
            .set_symbol("x");

        assert_eq!(target.buffer().cell((1, 0)).unwrap().symbol(), "x");
        assert_eq!(target.into_buffer().area, Rect::new(0, 0, 4, 1));
    }

    #[test]
    fn text_layout_origin_uses_cell_grid() {
        use piet::TextLayout as _;

        let mut device =
            Device::with_config(TuiConfig::with_cell_size(Size::new(8.0, 16.0))).unwrap();
        let mut target = device.bitmap_target(20, 4, 1.0).unwrap();
        let mut context = target.render_context();
        let layout = context.text().new_text_layout("hello").build().unwrap();

        assert_eq!(
            layout.layout_origin(Point::new(4.0, 8.0)),
            Point::new(8.0, 16.0)
        );
    }

    #[test]
    fn oversized_terminal_dimensions_are_rejected() {
        let mut device = Device::new().unwrap();
        let too_wide = usize::from(u16::MAX) + 1;
        assert!(matches!(
            device.bitmap_target(too_wide, 1, 1.0),
            Err(Error::InvalidInput)
        ));
    }

    #[test]
    fn terminal_target_does_not_support_pixel_export() {
        let mut device = Device::new().unwrap();
        let mut target = device.bitmap_target(2, 2, 1.0).unwrap();
        let mut pixels = [0; 16];

        assert!(matches!(
            target.to_image_buf(ImageFormat::RgbaPremul),
            Err(Error::NotSupported)
        ));
        assert!(matches!(
            target.copy_raw_pixels(ImageFormat::RgbaPremul, &mut pixels),
            Err(Error::NotSupported)
        ));
        assert!(matches!(
            target.save_to_file("unused.png"),
            Err(Error::NotSupported)
        ));
    }

    #[cfg(feature = "tui-big-text")]
    #[test]
    fn big_text_feature_is_forwarded() {
        use piet::TextLayout as _;

        let mut device = Device::new().unwrap();
        let mut target = device.bitmap_target(16, 4, 1.0).unwrap();
        let mut context = target.render_context();
        let layout = context
            .text()
            .new_text_layout("A")
            .big_text(BigTextSize::Quadrant)
            .build()
            .unwrap();

        assert_eq!(layout.line_count(), 1);
        assert!(layout.size().width > 1.0);
        assert!(layout.size().height > 1.0);
    }
}
