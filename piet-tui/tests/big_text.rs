// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg(feature = "big-text")]

use piet::kurbo::{Point, Rect, Size};
use piet::{
    Color, Error, FontFamily, RenderContext, Text as _, TextAttribute, TextLayout,
    TextLayoutBuilder,
};
use piet_tui::{BigTextSize, TuiConfig, TuiRenderContext};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect as TuiRect;
use ratatui_core::style::Color as TuiColor;
use ratatui_core::text::Line;
use ratatui_core::widgets::Widget;
use tui_big_text::{BigText, PixelSize};

fn empty_buffer(width: u16, height: u16) -> Buffer {
    Buffer::empty(TuiRect::new(0, 0, width, height))
}

fn expected_symbols(text: &str, pixel_size: PixelSize, width: u16, height: u16) -> Buffer {
    let area = TuiRect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    BigText::builder()
        .pixel_size(pixel_size)
        .lines(vec![Line::from(text)])
        .build()
        .render(area, &mut buffer);
    buffer
}

#[test]
fn layout_uses_discrete_glyph_footprints() {
    let mut buffer = empty_buffer(8, 8);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());

    let full = context
        .text()
        .new_text_layout("A")
        .big_text(BigTextSize::Full)
        .build()
        .unwrap();
    assert_eq!(full.size(), Size::new(8.0, 8.0));
    assert_eq!(full.line_metric(0).unwrap().height, 8.0);

    let half_height = context
        .text()
        .new_text_layout("A")
        .big_text(BigTextSize::HalfHeight)
        .build()
        .unwrap();
    assert_eq!(half_height.size(), Size::new(8.0, 4.0));

    let half_width = context
        .text()
        .new_text_layout("A")
        .big_text(BigTextSize::HalfWidth)
        .build()
        .unwrap();
    assert_eq!(half_width.size(), Size::new(4.0, 8.0));

    let quadrant = context
        .text()
        .new_text_layout("A")
        .big_text(BigTextSize::Quadrant)
        .build()
        .unwrap();
    assert_eq!(quadrant.size(), Size::new(4.0, 4.0));
    assert_eq!(quadrant.line_metric(0).unwrap().height, 4.0);
}

#[test]
fn draws_big_text_glyph_grid() {
    let mut buffer = empty_buffer(8, 8);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let layout = context
            .text()
            .new_text_layout("A")
            .big_text(BigTextSize::Full)
            .build()
            .unwrap();
        context.draw_text(&layout, Point::ZERO);
    }

    let expected = expected_symbols("A", PixelSize::Full, 8, 8);
    for y in 0..8 {
        for x in 0..8 {
            assert_eq!(
                buffer.cell((x, y)).unwrap().symbol(),
                expected.cell((x, y)).unwrap().symbol(),
                "cell ({x}, {y})"
            );
        }
    }
    assert!(buffer.content().iter().any(|cell| cell.symbol() == "█"));
}

#[test]
fn applies_span_styles_and_background_to_glyph_cells() {
    let mut buffer = empty_buffer(8, 4);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let layout = context
            .text()
            .new_text_layout("AB")
            .big_text(BigTextSize::Quadrant)
            .range_attribute(0..1, TextAttribute::TextColor(Color::rgb8(255, 0, 0)))
            .background_color(0..1, Color::rgb8(10, 20, 30))
            .build()
            .unwrap();
        context.draw_text(&layout, Point::ZERO);
    }

    for y in 0..4 {
        for x in 0..4 {
            let cell = buffer.cell((x, y)).unwrap();
            assert_eq!(cell.fg, TuiColor::Rgb(255, 0, 0));
            assert_eq!(cell.bg, TuiColor::Rgb(10, 20, 30));
        }
    }
    assert_ne!(buffer.cell((4, 0)).unwrap().fg, TuiColor::Rgb(255, 0, 0));
}

#[test]
fn wraps_and_aligns_on_the_glyph_grid() {
    let mut buffer = empty_buffer(12, 8);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());

    let wrapped = context
        .text()
        .new_text_layout("AB")
        .big_text(BigTextSize::Quadrant)
        .max_width(4.0)
        .build()
        .unwrap();
    assert_eq!(wrapped.size(), Size::new(4.0, 8.0));
    assert_eq!(wrapped.line_count(), 2);
    assert_eq!(wrapped.line_text(0), Some("A"));
    assert_eq!(wrapped.line_text(1), Some("B"));
    let hit = wrapped.hit_test_point(Point::new(0.0, 5.0));
    assert_eq!(hit.idx, 1);
    let rects = wrapped.rects_for_range(0..2);
    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0].y0, 0.0);
    assert_eq!(rects[1].y0, 4.0);

    let centered = context
        .text()
        .new_text_layout("A")
        .big_text(BigTextSize::Quadrant)
        .max_width(12.0)
        .alignment(piet::TextAlignment::Center)
        .build()
        .unwrap();
    assert_eq!(centered.hit_test_text_position(0).point.x, 4.0);
}

#[test]
fn big_text_respects_translation_and_clip() {
    let mut buffer = empty_buffer(12, 8);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let layout = context
            .text()
            .new_text_layout("A")
            .big_text(BigTextSize::Quadrant)
            .background_color(.., Color::rgb8(10, 20, 30))
            .build()
            .unwrap();
        context.clip(Rect::new(2.0, 1.0, 4.0, 5.0));
        context.draw_text(&layout, Point::new(2.0, 1.0));
    }

    assert_eq!(buffer.cell((2, 1)).unwrap().bg, TuiColor::Rgb(10, 20, 30));
    assert_eq!(buffer.cell((3, 4)).unwrap().bg, TuiColor::Rgb(10, 20, 30));
    assert_eq!(buffer.cell((4, 1)).unwrap().bg, TuiColor::Reset);
}

#[test]
fn font_attributes_do_not_change_big_text_resolution() {
    let mut buffer = empty_buffer(8, 8);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let layout = context
        .text()
        .new_text_layout("A")
        .font(FontFamily::MONOSPACE, 100.0)
        .big_text(BigTextSize::Quadrant)
        .build()
        .unwrap();

    assert_eq!(layout.size(), Size::new(4.0, 4.0));
}

#[test]
fn unsupported_glyphs_are_rejected() {
    let mut buffer = empty_buffer(8, 8);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let error = context
        .text()
        .new_text_layout("世")
        .big_text(BigTextSize::Full)
        .build()
        .unwrap_err();

    assert!(matches!(error, Error::NotSupported));
}

#[test]
fn empty_big_text_has_one_empty_line() {
    let mut buffer = empty_buffer(8, 8);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let layout = context
        .text()
        .new_text_layout("")
        .big_text(BigTextSize::Quadrant)
        .build()
        .unwrap();

    assert_eq!(layout.size(), Size::new(0.0, 4.0));
    assert_eq!(layout.line_count(), 1);
    assert_eq!(layout.line_text(0), Some(""));
}
