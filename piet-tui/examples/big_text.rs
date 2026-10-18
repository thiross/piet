// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use piet::kurbo::Point;
use piet::{Color, RenderContext, Text as _, TextAttribute, TextLayoutBuilder};
use piet_tui::{BigTextSize, TuiConfig, TuiRenderContext};
use ratatui_core::backend::TestBackend;
use ratatui_core::terminal::Terminal;

fn main() {
    let backend = TestBackend::new(48, 14);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let buffer = frame.buffer_mut();
            let mut context = TuiRenderContext::with_area(buffer, area, TuiConfig::default());
            context.clear(None, Color::rgb8(12, 14, 24));

            let title = context
                .text()
                .new_text_layout("PIET")
                .big_text(BigTextSize::Quadrant)
                .range_attribute(0..4, TextAttribute::TextColor(Color::rgb8(255, 200, 80)))
                .build()
                .unwrap();
            context.draw_text(&title, Point::new(2.0, 1.0));

            let body = context
                .text()
                .new_text_layout("Big text title, normal rich text body.")
                .build()
                .unwrap();
            context.draw_text(&body, Point::new(2.0, 6.0));
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            print!("{}", buffer.cell((x, y)).unwrap().symbol());
        }
        println!();
    }
}
