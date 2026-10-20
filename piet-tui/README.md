# piet-tui

`piet-tui` is a terminal backend for [`piet`](https://crates.io/crates/piet).
It renders Piet drawing commands into a
[`ratatui`](https://crates.io/crates/ratatui-core) `Buffer`, which can then be
flushed by any ratatui backend, including `ratatui-crossterm`.

This crate can be used directly or selected through the `tui` feature of
`piet-common`. Depending on `piet-tui` directly does not pull in Cairo,
CoreGraphics, Direct2D, or web dependencies.

The current implementation covers text layout/rendering, `fill`, `fill_even_odd`,
`stroke`, `stroke_styled`, `clear`, clipping, images, and basic transform state.
Text is rendered on the terminal cell grid; font family and font size are
accepted by the Piet API but cannot be represented by a terminal.

Use `Color::ansi` for one of the 16 terminal-theme slots or `Color::indexed` for
the full xterm 0–255 palette. Opaque indexed colors remain
`ratatui_core::style::Color::Indexed` in the buffer, allowing the terminal theme
to control their RGB values. Other backends and alpha blending use a
deterministic xterm RGB fallback.

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

Terminal text origins are snapped to the cell grid. `TextLayout::layout_origin`
returns the cell-aligned origin that `draw_text` will use:

```rust
let requested = Point::new(4.0, 8.0);
let draw_origin = layout.layout_origin(requested);
// With 8x16 cells, draw_origin is Point::new(8.0, 16.0).
```

The layout's hit-test methods and `TextLayout::rects_for_range` continue to use
layout coordinates relative to the requested origin. A widget that draws text at
`requested` should translate hit-test points into layout coordinates and
translate selection rectangles by `draw_origin - requested` before painting
them:

```rust
let requested = layout_rect.origin();
let draw_origin = layout.layout_origin(requested);
let layout_point = pointer - draw_origin.to_vec2();
let hit = layout.hit_test_point(layout_point);
for rect in layout.rects_for_range(selection.clone()) {
    context.fill(rect + draw_origin.to_vec2(), &selection_color);
}
```

Apply any active render transform to `requested` before calling
`layout_origin`; the method only describes the terminal's origin quantization.

For terminal-specific UI primitives, `TuiCellExt` provides low-level cell
drawing without adding widget semantics to Piet:

```rust
use piet::{Color, RenderContext};
use piet_tui::TuiCellExt;

context.set_cell(0, 0, "┌", Color::WHITE, Color::BLACK);
context.fill_cells(
    ratatui_core::layout::Rect::new(1, 0, 3, 1),
    Color::WHITE,
    Color::BLACK,
);
```

Cell coordinates are relative to the render context area. These operations
ignore the current transform, but respect the context area and active clip.
Borders and other widget-level behavior should be implemented by the
application or UI framework on top of these primitives.

## Using `piet-common`

`piet-common` exposes the same render context and associated types behind its
`tui` feature. Disable default features so the native backend and its GUI
dependencies are not enabled:

```toml
piet-common = { version = "0.8", default-features = false, features = ["tui"] }
```

The `piet-common` version also provides `Device` and `BitmapTarget` wrappers
around a ratatui `Buffer`. Pixel extraction and PNG export are not supported by
the terminal backend; inspect or display the buffer instead. Enable the
`tui-big-text` feature on `piet-common` to forward to this crate's `big-text`
feature.

`piet` itself is backend-neutral and does not depend on Cairo, CoreGraphics,
Direct2D, or the web APIs; the native backends are selected by `piet-common`.
Adding a `tui` feature to `piet` therefore would not remove any GUI dependency.
Keeping `piet-tui` as a separate backend crate also avoids dependency cycles and
lets applications opt into only the terminal renderer they need.

Cargo features are additive. `piet-common` selects the TUI backend whenever its
`tui` feature is enabled, but the default `native` feature still enables the
platform GUI dependencies unless `default-features = false` is specified.

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
