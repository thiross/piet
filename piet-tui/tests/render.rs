// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use piet::kurbo::{Affine, Line, Point, Rect, Shape};
use piet::{
    Color, FontStyle, FontWeight, LineCap, RenderContext, StrokeStyle, Text as _, TextAttribute,
    TextLayoutBuilder,
};
use piet_tui::{TuiConfig, TuiRenderContext};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect as TuiRect;
use ratatui_core::style::{Color as TuiColor, Modifier};

fn empty_buffer(width: u16, height: u16) -> Buffer {
    Buffer::empty(TuiRect::new(0, 0, width, height))
}

#[test]
fn draws_text_with_color_weight_and_background() {
    let mut buffer = empty_buffer(20, 4);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let layout = context
            .text()
            .new_text_layout("Hello")
            .range_attribute(0..5, TextAttribute::TextColor(Color::rgb8(255, 0, 0)))
            .range_attribute(0..5, FontWeight::BOLD)
            .background_color(0..5, Color::rgb8(10, 20, 30))
            .build()
            .unwrap();
        context.draw_text(&layout, Point::ZERO);
    }

    let cell = buffer.cell((0, 0)).unwrap();
    assert_eq!(cell.symbol(), "H");
    assert_eq!(cell.fg, TuiColor::Rgb(255, 0, 0));
    assert_eq!(cell.bg, TuiColor::Rgb(10, 20, 30));
    assert!(cell.modifier.contains(Modifier::BOLD));
}

#[test]
fn draws_italic_underline_and_strikethrough() {
    let mut buffer = empty_buffer(20, 4);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let layout = context
            .text()
            .new_text_layout("styled")
            .range_attribute(0..6, FontStyle::Italic)
            .range_attribute(0..6, TextAttribute::Underline(true))
            .range_attribute(0..6, TextAttribute::Strikethrough(true))
            .build()
            .unwrap();
        context.draw_text(&layout, Point::ZERO);
    }

    let cell = buffer.cell((0, 0)).unwrap();
    assert!(cell.modifier.contains(Modifier::ITALIC));
    assert!(cell.modifier.contains(Modifier::UNDERLINED));
    assert!(cell.modifier.contains(Modifier::CROSSED_OUT));
}

#[test]
fn wide_grapheme_occupies_two_cells() {
    let mut buffer = empty_buffer(20, 4);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let layout = context.text().new_text_layout("你好").build().unwrap();
        context.draw_text(&layout, Point::ZERO);
    }

    assert_eq!(buffer.cell((0, 0)).unwrap().symbol(), "你");
    assert_eq!(buffer.cell((1, 0)).unwrap().symbol(), " ");
    assert_eq!(buffer.cell((2, 0)).unwrap().symbol(), "好");
    assert_eq!(buffer.cell((3, 0)).unwrap().symbol(), " ");
}

#[test]
fn wide_grapheme_background_covers_both_cells() {
    let mut buffer = empty_buffer(20, 4);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let layout = context
            .text()
            .new_text_layout("你")
            .background_color(0..3, Color::rgb8(4, 5, 6))
            .build()
            .unwrap();
        context.draw_text(&layout, Point::ZERO);
    }

    assert_eq!(buffer.cell((0, 0)).unwrap().bg, TuiColor::Rgb(4, 5, 6));
    assert_eq!(buffer.cell((1, 0)).unwrap().bg, TuiColor::Rgb(4, 5, 6));
}

#[test]
fn combining_grapheme_stays_in_one_cell() {
    let mut buffer = empty_buffer(20, 4);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let layout = context.text().new_text_layout("e\u{301}").build().unwrap();
        context.draw_text(&layout, Point::ZERO);
    }

    assert_eq!(buffer.cell((0, 0)).unwrap().symbol(), "e\u{301}");
}

#[test]
fn alignment_is_applied_when_drawing() {
    let mut buffer = empty_buffer(20, 4);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let layout = context
            .text()
            .new_text_layout("hi")
            .max_width(10.0)
            .alignment(piet::TextAlignment::End)
            .build()
            .unwrap();
        context.draw_text(&layout, Point::ZERO);
    }

    assert_eq!(buffer.cell((8, 0)).unwrap().symbol(), "h");
    assert_eq!(buffer.cell((9, 0)).unwrap().symbol(), "i");
}

#[test]
fn translation_moves_text_origin() {
    let mut buffer = empty_buffer(20, 4);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        context.transform(Affine::translate((2.0, 1.0)));
        let layout = context.text().new_text_layout("hi").build().unwrap();
        context.draw_text(&layout, Point::ZERO);
    }

    assert_eq!(buffer.cell((2, 1)).unwrap().symbol(), "h");
    assert_eq!(buffer.cell((3, 1)).unwrap().symbol(), "i");
}

#[test]
fn fill_paints_cell_background() {
    let mut buffer = empty_buffer(6, 4);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        context.fill(Rect::new(0.0, 0.0, 3.0, 2.0), &Color::rgb8(1, 2, 3));
    }

    for y in 0..2 {
        for x in 0..3 {
            let cell = buffer.cell((x, y)).unwrap();
            assert_eq!(cell.bg, TuiColor::Rgb(1, 2, 3));
            assert_eq!(cell.symbol(), " ");
        }
    }
    assert_eq!(buffer.cell((3, 0)).unwrap().bg, TuiColor::Reset);
    assert_eq!(buffer.cell((0, 2)).unwrap().bg, TuiColor::Reset);
}

#[test]
fn even_odd_fill_leaves_hole() {
    let mut buffer = empty_buffer(6, 6);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let mut path = Rect::new(0.0, 0.0, 4.0, 4.0).to_path(0.1);
        path.extend(Rect::new(1.0, 1.0, 3.0, 3.0).to_path(0.1));
        context.fill_even_odd(&path, &Color::rgb8(1, 2, 3));
    }

    assert_eq!(buffer.cell((0, 0)).unwrap().bg, TuiColor::Rgb(1, 2, 3));
    assert_eq!(buffer.cell((2, 2)).unwrap().bg, TuiColor::Reset);
}

#[test]
fn clip_restricts_text_and_restore_resets_it() {
    let mut buffer = empty_buffer(20, 4);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let layout = context.text().new_text_layout("hello").build().unwrap();

        context.save().unwrap();
        context.clip(Rect::new(0.0, 0.0, 2.0, 1.0));
        context.draw_text(&layout, Point::ZERO);
        context.restore().unwrap();
    }

    assert_eq!(buffer.cell((0, 0)).unwrap().symbol(), "h");
    assert_eq!(buffer.cell((1, 0)).unwrap().symbol(), "e");
    assert_eq!(buffer.cell((2, 0)).unwrap().symbol(), " ");
}

#[test]
fn clear_replaces_region() {
    let mut buffer = empty_buffer(4, 2);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        context.clear(None, Color::rgb8(9, 8, 7));
    }

    assert_eq!(buffer.cell((0, 0)).unwrap().bg, TuiColor::Rgb(9, 8, 7));
    assert_eq!(buffer.cell((3, 1)).unwrap().bg, TuiColor::Rgb(9, 8, 7));
}

#[test]
fn restore_without_save_errors() {
    let mut buffer = empty_buffer(4, 2);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    assert!(context.restore().is_err());
}

#[test]
fn later_overlapping_weight_range_only_overrides_its_span() {
    let mut buffer = empty_buffer(20, 4);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let layout = context
            .text()
            .new_text_layout("abcdef")
            .range_attribute(0..6, FontWeight::BOLD)
            .range_attribute(2..4, FontWeight::REGULAR)
            .build()
            .unwrap();
        context.draw_text(&layout, Point::ZERO);
    }

    assert!(
        buffer
            .cell((0, 0))
            .unwrap()
            .modifier
            .contains(Modifier::BOLD)
    );
    assert!(
        !buffer
            .cell((2, 0))
            .unwrap()
            .modifier
            .contains(Modifier::BOLD)
    );
    assert!(
        buffer
            .cell((4, 0))
            .unwrap()
            .modifier
            .contains(Modifier::BOLD)
    );
    assert!(
        buffer
            .cell((5, 0))
            .unwrap()
            .modifier
            .contains(Modifier::BOLD)
    );
}

#[test]
fn later_overlapping_background_range_only_overrides_its_span() {
    let mut buffer = empty_buffer(20, 4);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let layout = context
            .text()
            .new_text_layout("abcdef")
            .background_color(0..6, Color::rgb8(1, 2, 3))
            .background_color(2..4, Color::rgb8(4, 5, 6))
            .build()
            .unwrap();
        context.draw_text(&layout, Point::ZERO);
    }

    assert_eq!(buffer.cell((1, 0)).unwrap().bg, TuiColor::Rgb(1, 2, 3));
    assert_eq!(buffer.cell((2, 0)).unwrap().bg, TuiColor::Rgb(4, 5, 6));
    assert_eq!(buffer.cell((3, 0)).unwrap().bg, TuiColor::Rgb(4, 5, 6));
    assert_eq!(buffer.cell((4, 0)).unwrap().bg, TuiColor::Rgb(1, 2, 3));
}

#[test]
fn tab_cells_are_rendered_as_spaces() {
    let mut buffer = empty_buffer(20, 4);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let layout = context.text().new_text_layout("a\tb").build().unwrap();
        context.draw_text(&layout, Point::ZERO);
    }

    assert_eq!(buffer.cell((0, 0)).unwrap().symbol(), "a");
    assert_eq!(buffer.cell((1, 0)).unwrap().symbol(), " ");
    assert_eq!(buffer.cell((2, 0)).unwrap().symbol(), " ");
    assert_eq!(buffer.cell((3, 0)).unwrap().symbol(), " ");
    assert_eq!(buffer.cell((4, 0)).unwrap().symbol(), "b");
}

#[test]
fn stroke_draws_horizontal_line_on_the_cell_grid() {
    let mut buffer = empty_buffer(8, 3);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        context.stroke(
            Line::new((0.0, 0.5), (4.0, 0.5)),
            &Color::rgb8(10, 20, 30),
            1.0,
        );
    }

    for x in 0..4 {
        assert_eq!(buffer.cell((x, 0)).unwrap().bg, TuiColor::Rgb(10, 20, 30));
    }
    assert_eq!(buffer.cell((4, 0)).unwrap().bg, TuiColor::Reset);
    assert_eq!(buffer.cell((0, 1)).unwrap().bg, TuiColor::Reset);
}

#[test]
fn wider_stroke_covers_multiple_rows() {
    let mut buffer = empty_buffer(8, 4);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        context.stroke(
            Line::new((0.0, 1.0), (4.0, 1.0)),
            &Color::rgb8(1, 2, 3),
            2.0,
        );
    }

    assert_eq!(buffer.cell((1, 0)).unwrap().bg, TuiColor::Rgb(1, 2, 3));
    assert_eq!(buffer.cell((1, 1)).unwrap().bg, TuiColor::Rgb(1, 2, 3));
    assert_eq!(buffer.cell((1, 2)).unwrap().bg, TuiColor::Reset);
}

#[test]
fn stroke_applies_the_current_transform() {
    let mut buffer = empty_buffer(8, 4);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        context.transform(Affine::translate((1.0, 1.0)));
        context.stroke(
            Line::new((0.0, 0.5), (2.0, 0.5)),
            &Color::rgb8(1, 2, 3),
            1.0,
        );
    }

    assert_eq!(buffer.cell((2, 1)).unwrap().bg, TuiColor::Rgb(1, 2, 3));
    assert_eq!(buffer.cell((0, 1)).unwrap().bg, TuiColor::Reset);
}

#[test]
fn stroke_respects_clip() {
    let mut buffer = empty_buffer(8, 3);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        context.clip(Rect::new(0.0, 0.0, 2.0, 1.0));
        context.stroke(
            Line::new((0.0, 0.5), (5.0, 0.5)),
            &Color::rgb8(1, 2, 3),
            1.0,
        );
    }

    assert_eq!(buffer.cell((1, 0)).unwrap().bg, TuiColor::Rgb(1, 2, 3));
    assert_eq!(buffer.cell((2, 0)).unwrap().bg, TuiColor::Reset);
}

#[test]
fn stroke_styled_square_caps_extend_the_endpoints() {
    let mut buffer = empty_buffer(8, 3);
    let style = StrokeStyle::new().line_cap(LineCap::Square);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        context.stroke_styled(
            Line::new((0.75, 0.5), (3.25, 0.5)),
            &Color::rgb8(1, 2, 3),
            1.0,
            &style,
        );
    }

    assert_eq!(buffer.cell((0, 0)).unwrap().bg, TuiColor::Rgb(1, 2, 3));
    assert_eq!(buffer.cell((3, 0)).unwrap().bg, TuiColor::Rgb(1, 2, 3));
}

#[test]
fn stroke_styled_dashes_leave_gaps() {
    let mut buffer = empty_buffer(10, 3);
    let style = StrokeStyle::new().dash_pattern(&[2.0, 2.0]);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        context.stroke_styled(
            Line::new((0.0, 0.5), (8.0, 0.5)),
            &Color::rgb8(1, 2, 3),
            1.0,
            &style,
        );
    }

    assert_eq!(buffer.cell((1, 0)).unwrap().bg, TuiColor::Rgb(1, 2, 3));
    assert_eq!(buffer.cell((2, 0)).unwrap().bg, TuiColor::Reset);
    assert_eq!(buffer.cell((4, 0)).unwrap().bg, TuiColor::Rgb(1, 2, 3));
    assert_eq!(buffer.cell((6, 0)).unwrap().bg, TuiColor::Reset);
}

#[test]
fn closed_shape_stroke_does_not_fill_the_interior() {
    let mut buffer = empty_buffer(8, 6);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        context.stroke(Rect::new(0.0, 0.0, 5.0, 4.0), &Color::rgb8(1, 2, 3), 2.0);
    }

    assert_eq!(buffer.cell((0, 0)).unwrap().bg, TuiColor::Rgb(1, 2, 3));
    assert_eq!(buffer.cell((4, 3)).unwrap().bg, TuiColor::Rgb(1, 2, 3));
    assert_eq!(buffer.cell((2, 2)).unwrap().bg, TuiColor::Reset);
}

#[test]
fn non_positive_stroke_width_draws_nothing() {
    let mut buffer = empty_buffer(8, 3);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let line = Line::new((0.0, 0.5), (5.0, 0.5));
        context.stroke(line, &Color::rgb8(1, 2, 3), 0.0);
        context.stroke(line, &Color::rgb8(1, 2, 3), -1.0);
    }

    for x in 0..5 {
        assert_eq!(buffer.cell((x, 0)).unwrap().bg, TuiColor::Reset);
    }
}
