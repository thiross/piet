// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::io::Result<()> {
    native::main()
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::io;
    use std::time::Duration;

    use piet::kurbo::{BezPath, Circle, Line, Point, Rect};
    use piet::{
        Color, FontStyle, FontWeight, ImageFormat, InterpolationMode, RenderContext, StrokeStyle,
        Text as _, TextAttribute, TextLayoutBuilder,
    };
    use piet_tui::{TuiConfig, TuiImage, TuiRenderContext, render_with};
    use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
    use ratatui::{DefaultTerminal, Frame};

    pub(super) fn main() -> io::Result<()> {
        ratatui::run(run)
    }

    fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(draw)?;
            if event::poll(Duration::from_millis(250))? {
                match event::read()? {
                    Event::Key(key)
                        if key.kind == KeyEventKind::Press
                            && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) =>
                    {
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }
    }

    fn draw(frame: &mut Frame) {
        let area = frame.area();
        render_with(frame.buffer_mut(), area, TuiConfig::default(), |context| {
            context.clear(None, Color::rgb8(12, 14, 24));
            context.fill(
                Rect::new(0.0, 0.0, area.width as f64, 1.0),
                &Color::rgb8(40, 44, 80),
            );

            let title_width = area.width.saturating_sub(2) as f64;
            let title = context
                .text()
                .new_text_layout("piet-tui: rich text on a terminal grid")
                .max_width(title_width)
                .range_attribute(0..8, FontWeight::BOLD)
                .range_attribute(0..8, TextAttribute::TextColor(Color::rgb8(255, 200, 80)))
                .build()
                .unwrap();
            context.draw_text(&title, Point::new(1.0, 0.0));

            let body = context
                .text()
                .new_text_layout(
                    "Hello, 世界! Bold, italic, underline and colors all fit in cells.",
                )
                .max_width(title_width)
                .range_attribute(7..10, FontStyle::Italic)
                .range_attribute(12..21, TextAttribute::Underline(true))
                .range_attribute(26..31, TextAttribute::Strikethrough(true))
                .range_attribute(26..31, TextAttribute::TextColor(Color::rgb8(240, 120, 160)))
                .background_color(12..21, Color::rgb8(50, 70, 90))
                .build()
                .unwrap();
            context.draw_text(&body, Point::new(1.0, 2.0));

            if area.width > 8 && area.height >= 14 {
                let image = gradient_image(context, (area.width - 4).min(48) as usize, 8);
                context.draw_image(
                    &image,
                    Rect::new(2.0, 6.0, 2.0 + image.width() as f64, 10.0),
                    InterpolationMode::Bilinear,
                );
            }

            let separator_style = StrokeStyle::new().dash_pattern(&[3.0, 2.0]);
            context.stroke_styled(
                Line::new((1.0, 4.5), (area.width.saturating_sub(1) as f64, 4.5)),
                &Color::rgb8(90, 100, 150),
                1.0,
                &separator_style,
            );

            if area.width >= 60 && area.height >= 23 {
                draw_kurbo_shapes(context);
            }

            let help = context
                .text()
                .new_text_layout("q or Esc: quit")
                .range_attribute(0.., FontStyle::Italic)
                .build()
                .unwrap();
            context.draw_text(&help, Point::new(1.0, area.height.saturating_sub(2) as f64));
        });
    }

    fn draw_kurbo_shapes(context: &mut TuiRenderContext<'_>) {
        let caption = context
            .text()
            .new_text_layout("kurbo shapes, stroke width = 1 cell")
            .range_attribute(0.., FontStyle::Italic)
            .build()
            .unwrap();
        context.draw_text(&caption, Point::new(2.0, 11.0));

        context.stroke(
            Line::new((2.5, 14.5), (22.5, 14.5)),
            &Color::rgb8(100, 180, 255),
            1.0,
        );
        context.stroke(
            Line::new((30.5, 16.5), (50.5, 13.5)),
            &Color::rgb8(255, 200, 80),
            1.0,
        );
        context.stroke(
            Circle::new((68.5, 15.0), 2.5),
            &Color::rgb8(240, 120, 160),
            1.0,
        );
        context.stroke(
            Rect::new(2.5, 18.5, 21.5, 20.5),
            &Color::rgb8(120, 220, 150),
            1.0,
        );

        let mut triangle = BezPath::new();
        triangle.move_to((32.5, 20.5));
        triangle.line_to((41.5, 17.5));
        triangle.line_to((50.5, 20.5));
        triangle.close_path();
        context.stroke(&triangle, &Color::rgb8(180, 150, 255), 1.0);
    }

    fn gradient_image(context: &mut TuiRenderContext<'_>, width: usize, height: usize) -> TuiImage {
        let mut pixels = Vec::with_capacity(width * height * 4);
        for y in 0..height {
            for x in 0..width {
                let red = (x * 255 / width.max(1)) as u8;
                let green = (y * 255 / height.max(1)) as u8;
                let blue = 255 - red;
                pixels.extend_from_slice(&[red, green, blue, 255]);
            }
        }

        context
            .make_image(width, height, &pixels, ImageFormat::RgbaSeparate)
            .unwrap()
    }
}
