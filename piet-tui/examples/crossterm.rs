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

    use piet::kurbo::{Point, Rect};
    use piet::{
        Color, FontStyle, FontWeight, RenderContext, Text as _, TextAttribute, TextLayoutBuilder,
    };
    use piet_tui::{TuiConfig, render_with};
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

            let help = context
                .text()
                .new_text_layout("q or Esc: quit")
                .range_attribute(0.., FontStyle::Italic)
                .build()
                .unwrap();
            context.draw_text(&help, Point::new(1.0, area.height.saturating_sub(2) as f64));
        });
    }
}
