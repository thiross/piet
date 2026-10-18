// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use piet::kurbo::{Point, Rect};
use piet::{Color, FontWeight, RenderContext, Text as _, TextAttribute, TextLayoutBuilder};
use piet_tui::{TuiConfig, TuiRenderContext};
use ratatui_core::backend::TestBackend;
use ratatui_core::terminal::Terminal;

fn main() {
    let backend = TestBackend::new(60, 8);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let buffer = frame.buffer_mut();
            let mut context = TuiRenderContext::with_area(buffer, area, TuiConfig::default());

            context.fill(
                Rect::new(0.0, 0.0, area.width as f64, area.height as f64),
                &Color::rgb8(20, 20, 30),
            );

            let layout = context
                .text()
                .new_text_layout("piet-tui: hello 世界")
                .max_width(area.width as f64 - 2.0)
                .range_attribute(0..8, FontWeight::BOLD)
                .range_attribute(0..8, TextAttribute::TextColor(Color::rgb8(255, 200, 80)))
                .build()
                .unwrap();
            context.draw_text(&layout, Point::new(1.0, 1.0));
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
