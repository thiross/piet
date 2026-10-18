# piet-tui

`piet-tui` is a terminal backend for [`piet`](https://crates.io/crates/piet).
It renders Piet drawing commands into a
[`ratatui`](https://crates.io/crates/ratatui-core) `Buffer`, which can then be
flushed by any ratatui backend, including `ratatui-crossterm`.

This crate is intentionally independent of `piet-common`: depending on
`piet-tui` directly does not pull in Cairo, CoreGraphics, Direct2D, or web
dependencies.

The current implementation covers text layout/rendering, `fill`, `fill_even_odd`,
`stroke`, `stroke_styled`, `clear`, clipping, images, and basic transform state.
Text is rendered on the terminal cell grid; font family and font size are
accepted by the Piet API but cannot be represented by a terminal.

Images support grayscale, RGB, separate-alpha RGBA, and premultiplied RGBA
formats. Sampling can be nearest-neighbor or bilinear, and `draw_image_area`
supports source cropping with scaling. Each terminal cell has only a foreground
and background color, so the backend renders two vertical image samples per cell
with an upper-half block (`▀`). `capture_image_area` is not supported.

The optional `big-text` feature adds fixed-resolution display fonts backed by
[`tui-big-text`](https://crates.io/crates/tui-big-text). Big text is selected
explicitly per layout; `FontFamily` and `FontSize` do not select or scale it.

```rust
use piet::{RenderContext, Text as _, TextLayoutBuilder};
use piet_tui::BigTextSize;

let title = context
    .text()
    .new_text_layout("PIET TUI")
    .big_text(BigTextSize::Quadrant)
    .build()
    .unwrap();
```

`BigTextSize::Full`, `HalfHeight`, `HalfWidth`, and `Quadrant` provide portable
8x8, 8x4, 4x8, and 4x4 cell footprints. Big text uses `font8x8`, so Latin,
Greek, Hiragana, and a few other scripts are available, but CJK Han characters
are not. Unsupported non-whitespace glyphs make layout construction fail with
`Error::NotSupported`.

Text, including big text, uses the current transform only to position its
origin. Terminal cells cannot scale, rotate, or shear glyphs, so those parts of
the transform are ignored for text. Canvas fills, strokes, clips, and images
support the full affine transform.

## Why not a feature on `piet`?

`piet` itself is backend-neutral and does not depend on Cairo, CoreGraphics,
Direct2D, or the web APIs; the native backends are selected by `piet-common`.
Adding a `tui` feature to `piet` therefore would not remove any GUI dependency.
Keeping `piet-tui` as a separate backend crate also avoids dependency cycles and
lets applications opt into only the terminal renderer they need.

Cargo features are additive, so a `tui` feature on `piet-common` could not
automatically suppress its default native backend. Applications that want TUI
rendering should depend on `piet-tui` directly.

## Example

```rust,no_run
use piet::kurbo::Point;
use piet::{FontWeight, RenderContext, Text as _, TextLayoutBuilder};
use piet_tui::{TuiConfig, TuiRenderContext};

fn draw(buffer: &mut ratatui_core::buffer::Buffer, area: ratatui_core::layout::Rect) {
    let mut context = TuiRenderContext::with_area(buffer, area, TuiConfig::default());
    let layout = context
        .text()
        .new_text_layout("Hello, 世界")
        .max_width(area.width as f64)
        .range_attribute(0..5, FontWeight::BOLD)
        .build()
        .unwrap();
    context.draw_text(&layout, Point::ZERO);
}
```

Run the buffer-level example with:

```sh
cargo run -p piet-tui --example basic
```

Run the interactive `ratatui` + `crossterm` example with:

```sh
cargo run -p piet-tui --example crossterm
```

Run the buffer-level big-text example with:

```sh
cargo run -p piet-tui --features big-text --example big_text
```

Run the scrollable Markdown viewer with the bundled sample, or pass another
Markdown file:

```sh
cargo run -p piet-tui --example markdown
cargo run -p piet-tui --example markdown -- path/to/document.md
```

`crossterm` is a dev-dependency of the example only. The library itself renders
to a `ratatui-core::buffer::Buffer` and does not perform terminal I/O.
