// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use piet::kurbo::{Point, Rect, Size};
use piet::{
    FontStyle, FontWeight, RenderContext, Text as _, TextAffinity, TextAlignment, TextAttribute,
    TextLayout, TextLayoutBuilder,
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
fn text_affinity_disambiguates_soft_wrapped_boundary() {
    let mut buffer = empty_buffer(20, 4);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let layout = context
        .text()
        .new_text_layout("hello world")
        .max_width(5.0)
        .build()
        .unwrap();

    let upstream = layout.hit_test_text_position_with_affinity(6, TextAffinity::Upstream);
    assert_eq!(upstream.point, Point::new(6.0, 0.8));
    assert_eq!(upstream.line, 0);

    let downstream = layout.hit_test_text_position_with_affinity(6, TextAffinity::Downstream);
    assert_eq!(downstream.point, Point::new(0.0, 1.8));
    assert_eq!(downstream.line, 1);

    let default = layout.hit_test_text_position(6);
    assert_eq!(default.point, upstream.point);
    assert_eq!(default.line, upstream.line);
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

#[test]
fn rects_for_range_spans_soft_wrapped_lines() {
    let mut buffer = empty_buffer(20, 4);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let layout = context
        .text()
        .new_text_layout("hello world")
        .max_width(5.0)
        .build()
        .unwrap();

    assert_eq!(
        layout.rects_for_range(2..10),
        vec![Rect::new(2.0, 0.0, 5.0, 1.0), Rect::new(0.0, 1.0, 4.0, 2.0)]
    );
    assert_eq!(
        layout.rects_for_range(7..10),
        vec![Rect::new(1.0, 1.0, 4.0, 2.0)]
    );
    assert_eq!(
        layout.rects_for_range(6..10),
        vec![Rect::new(0.0, 1.0, 4.0, 2.0)]
    );
}

#[test]
fn soft_wrapped_selection_uses_first_line_alignment_offset() {
    let mut buffer = empty_buffer(20, 4);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let centered = context
        .text()
        .new_text_layout("a bc")
        .max_width(3.0)
        .alignment(TextAlignment::Center)
        .build()
        .unwrap();

    assert_eq!(
        centered.rects_for_range(0..4),
        vec![Rect::new(1.0, 0.0, 2.0, 1.0), Rect::new(0.5, 1.0, 2.5, 2.0),]
    );

    let end = context
        .text()
        .new_text_layout("a bc")
        .max_width(3.0)
        .alignment(TextAlignment::End)
        .build()
        .unwrap();
    assert_eq!(
        end.rects_for_range(0..4),
        vec![Rect::new(2.0, 0.0, 3.0, 1.0), Rect::new(1.0, 1.0, 3.0, 2.0)]
    );
}

#[test]
fn text_affinity_preserves_grapheme_and_hard_break_geometry() {
    let mut buffer = empty_buffer(20, 4);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());

    let wide = context
        .text()
        .new_text_layout("你好")
        .max_width(2.0)
        .build()
        .unwrap();
    assert_eq!(
        wide.hit_test_text_position_with_affinity(3, TextAffinity::Upstream)
            .point,
        Point::new(2.0, 0.8)
    );
    assert_eq!(
        wide.hit_test_text_position_with_affinity(3, TextAffinity::Downstream)
            .point,
        Point::new(0.0, 1.8)
    );
    assert_eq!(
        wide.rects_for_range(0..6),
        vec![Rect::new(0.0, 0.0, 2.0, 1.0), Rect::new(0.0, 1.0, 2.0, 2.0)]
    );

    let combining = context
        .text()
        .new_text_layout("e\u{301}e\u{301}")
        .max_width(1.0)
        .build()
        .unwrap();
    assert_eq!(
        combining
            .hit_test_text_position_with_affinity(3, TextAffinity::Downstream)
            .point,
        Point::new(0.0, 1.8)
    );
    assert_eq!(
        combining.rects_for_range(0..6),
        vec![Rect::new(0.0, 0.0, 1.0, 1.0), Rect::new(0.0, 1.0, 1.0, 2.0)]
    );

    let tabbed = context
        .text()
        .new_text_layout("\ta")
        .max_width(2.0)
        .build()
        .unwrap();
    assert_eq!(
        tabbed
            .hit_test_text_position_with_affinity(1, TextAffinity::Upstream)
            .point,
        Point::new(4.0, 0.8)
    );
    assert_eq!(
        tabbed
            .hit_test_text_position_with_affinity(1, TextAffinity::Downstream)
            .point,
        Point::new(0.0, 1.8)
    );

    let hard_break = context.text().new_text_layout("ab\ncd").build().unwrap();
    for affinity in [TextAffinity::Upstream, TextAffinity::Downstream] {
        let position = hard_break.hit_test_text_position_with_affinity(2, affinity);
        assert_eq!(position.point, Point::new(2.0, 0.8));
        assert_eq!(position.line, 0);
    }
}

#[test]
fn layout_origin_snaps_to_cell_grid() {
    let mut buffer = empty_buffer(20, 4);
    let mut context =
        TuiRenderContext::new(&mut buffer, TuiConfig::with_cell_size(Size::new(8.0, 16.0)));
    let layout = context.text().new_text_layout("hello").build().unwrap();

    assert_eq!(
        layout.layout_origin(Point::new(4.0, 8.0)),
        Point::new(8.0, 16.0)
    );
    assert_eq!(
        layout.layout_origin(Point::new(-4.0, -8.0)),
        Point::new(-8.0, -16.0)
    );
}

#[test]
fn layout_origin_in_cell_units() {
    let mut buffer = empty_buffer(20, 4);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::cells());
    let layout = context.text().new_text_layout("hello").build().unwrap();

    assert_eq!(
        layout.layout_origin(Point::new(4.25, 8.75)),
        Point::new(4.0, 9.0)
    );
}

#[test]
fn layout_origin_is_stable_for_multiline_wide_and_cjk_text() {
    let mut buffer = empty_buffer(20, 4);
    let mut context =
        TuiRenderContext::new(&mut buffer, TuiConfig::with_cell_size(Size::new(8.0, 16.0)));
    let layout = context
        .text()
        .new_text_layout("你好\nwide")
        .build()
        .unwrap();
    let reference = context.text().new_text_layout("hello").build().unwrap();
    let requested = Point::new(12.0, 28.0);

    assert_eq!(layout.line_count(), 2);
    assert_eq!(layout.layout_origin(requested), Point::new(16.0, 32.0));
    assert_eq!(
        layout.layout_origin(requested),
        reference.layout_origin(requested)
    );
    assert_eq!(
        layout.rects_for_range(0..3),
        vec![Rect::new(0.0, 0.0, 16.0, 16.0)]
    );
}

#[test]
fn hit_testing_uses_actual_draw_origin_with_half_cell_offset() {
    let mut buffer = empty_buffer(20, 4);
    let mut context =
        TuiRenderContext::new(&mut buffer, TuiConfig::with_cell_size(Size::new(8.0, 16.0)));
    let layout = context.text().new_text_layout("hello").build().unwrap();

    let requested = Point::new(4.0, 8.0);
    let draw_origin = layout.layout_origin(requested);
    let pointer_at_drawn_cell_center = draw_origin + (4.0, 8.0);
    let layout_point = pointer_at_drawn_cell_center - draw_origin.to_vec2();
    let hit = layout.hit_test_point(layout_point);

    assert_eq!(hit.idx, 0);
    assert!(hit.is_inside);
}
