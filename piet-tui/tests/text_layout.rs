// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use piet::kurbo::{Point, Rect, Size};
use piet::{
    FontStyle, FontWeight, RenderContext, Text as _, TextAlignment, TextAttribute, TextLayout,
    TextLayoutBuilder,
};
use piet_tui::{TuiConfig, TuiRenderContext};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect as TuiRect;

fn empty_buffer(width: u16, height: u16) -> Buffer {
    Buffer::empty(TuiRect::new(0, 0, width, height))
}

#[test]
fn ascii_layout_metrics() {
    let mut buffer = empty_buffer(20, 4);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let layout = context.text().new_text_layout("hello").build().unwrap();

    assert_eq!(layout.size(), Size::new(5.0, 1.0));
    assert_eq!(layout.line_count(), 1);
    assert_eq!(layout.line_text(0), Some("hello"));
    let metric = layout.line_metric(0).unwrap();
    assert_eq!(metric.start_offset, 0);
    assert_eq!(metric.end_offset, 5);
    assert_eq!(metric.trailing_whitespace, 0);
    assert_eq!(metric.height, 1.0);
    assert_eq!(metric.baseline, 0.8);
    assert_eq!(metric.y_offset, 0.0);
}

#[test]
fn empty_text_has_one_line() {
    let mut buffer = empty_buffer(20, 4);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let layout = context.text().new_text_layout("").build().unwrap();

    assert_eq!(layout.line_count(), 1);
    assert_eq!(layout.size(), Size::new(0.0, 1.0));
    assert_eq!(layout.line_text(0), Some(""));
    assert_eq!(layout.hit_test_text_position(0).point, Point::new(0.0, 0.8));
}

#[test]
fn word_wrap_assigns_trailing_space_to_previous_line() {
    let mut buffer = empty_buffer(20, 4);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let layout = context
        .text()
        .new_text_layout("hello world")
        .max_width(5.0)
        .build()
        .unwrap();

    assert_eq!(layout.line_count(), 2);
    assert_eq!(layout.line_text(0), Some("hello "));
    assert_eq!(layout.line_text(1), Some("world"));
    assert_eq!(layout.size().width, 5.0);
    assert_eq!(layout.line_metric(0).unwrap().trailing_whitespace, 1);
    assert_eq!(layout.line_metric(0).unwrap().end_offset, 6);
}

#[test]
fn long_words_break_at_cell_boundary() {
    let mut buffer = empty_buffer(20, 4);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let layout = context
        .text()
        .new_text_layout("abcdefgh")
        .max_width(3.0)
        .build()
        .unwrap();

    assert_eq!(layout.line_count(), 3);
    assert_eq!(layout.line_text(0), Some("abc"));
    assert_eq!(layout.line_text(1), Some("def"));
    assert_eq!(layout.line_text(2), Some("gh"));
}

#[test]
fn wide_and_combining_graphemes() {
    let mut buffer = empty_buffer(20, 4);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let wide = context.text().new_text_layout("你好").build().unwrap();
    assert_eq!(wide.size(), Size::new(4.0, 1.0));

    let combining = context.text().new_text_layout("e\u{301}").build().unwrap();
    assert_eq!(combining.size(), Size::new(1.0, 1.0));
    assert_eq!(combining.line_text(0), Some("e\u{301}"));
}

#[test]
fn alignment_offsets_layout_content() {
    let mut buffer = empty_buffer(20, 4);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let end = context
        .text()
        .new_text_layout("hi")
        .max_width(10.0)
        .alignment(TextAlignment::End)
        .build()
        .unwrap();
    assert_eq!(end.size().width, 10.0);
    assert_eq!(end.hit_test_text_position(0).point.x, 8.0);

    let center = context
        .text()
        .new_text_layout("hi")
        .max_width(10.0)
        .alignment(TextAlignment::Center)
        .build()
        .unwrap();
    assert_eq!(center.hit_test_text_position(0).point.x, 4.0);
}

#[test]
fn style_ranges_are_resolved_per_grapheme() {
    let mut buffer = empty_buffer(20, 4);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let layout = context
        .text()
        .new_text_layout("Hello world")
        .range_attribute(0..5, TextAttribute::TextColor(piet::Color::rgb8(255, 0, 0)))
        .range_attribute(0..5, FontWeight::BOLD)
        .range_attribute(6..11, FontStyle::Italic)
        .range_attribute(6..11, TextAttribute::Underline(true))
        .range_attribute(6..11, TextAttribute::Strikethrough(true))
        .build()
        .unwrap();
    assert_eq!(layout.size(), Size::new(11.0, 1.0));
}

#[test]
fn hit_testing_and_text_positions() {
    let mut buffer = empty_buffer(20, 4);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let layout = context.text().new_text_layout("hello").build().unwrap();

    assert_eq!(layout.hit_test_point(Point::new(2.2, 0.5)).idx, 2);
    assert!(layout.hit_test_point(Point::new(2.2, 0.5)).is_inside);
    assert_eq!(layout.hit_test_point(Point::new(10.0, 0.5)).idx, 5);
    assert!(!layout.hit_test_point(Point::new(10.0, 0.5)).is_inside);

    assert_eq!(layout.hit_test_text_position(3).point, Point::new(3.0, 0.8));
    assert_eq!(layout.hit_test_text_position(5).point, Point::new(5.0, 0.8));
}

#[test]
fn point_mode_scales_text_metrics() {
    let mut buffer = empty_buffer(20, 4);
    let mut context =
        TuiRenderContext::new(&mut buffer, TuiConfig::with_cell_size(Size::new(2.0, 4.0)));
    let layout = context.text().new_text_layout("hi").build().unwrap();

    assert_eq!(layout.size(), Size::new(4.0, 4.0));
    assert_eq!(layout.line_metric(0).unwrap().baseline, 3.2);
}

#[test]
fn newlines_and_trailing_newline_create_empty_line() {
    let mut buffer = empty_buffer(20, 4);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let layout = context.text().new_text_layout("a\nbc\n").build().unwrap();

    assert_eq!(layout.line_count(), 3);
    assert_eq!(layout.line_text(0), Some("a"));
    assert_eq!(layout.line_text(1), Some("bc"));
    assert_eq!(layout.line_text(2), Some(""));
    assert_eq!(layout.size(), Size::new(2.0, 3.0));
}

#[test]
fn crlf_is_treated_as_one_line_break() {
    let mut buffer = empty_buffer(20, 4);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let layout = context.text().new_text_layout("a\r\nb").build().unwrap();

    assert_eq!(layout.line_count(), 2);
    assert_eq!(layout.line_text(0), Some("a"));
    assert_eq!(layout.line_text(1), Some("b"));
}

#[test]
fn tabs_expand_to_next_tab_stop() {
    let mut buffer = empty_buffer(20, 4);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let layout = context.text().new_text_layout("a\tb").build().unwrap();

    assert_eq!(layout.size(), Size::new(5.0, 1.0));
    assert_eq!(layout.line_text(0), Some("a\tb"));
}

#[test]
fn rects_for_range_spans_lines() {
    let mut buffer = empty_buffer(20, 4);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let layout = context.text().new_text_layout("ab\ncd").build().unwrap();

    assert_eq!(
        layout.rects_for_range(0..5),
        vec![Rect::new(0.0, 0.0, 2.0, 1.0), Rect::new(0.0, 1.0, 2.0, 2.0)]
    );
}
