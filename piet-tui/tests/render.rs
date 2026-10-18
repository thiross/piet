// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use piet::kurbo::{Affine, Line, Point, Rect, Shape};
use piet::{
    Color, Error, FontStyle, FontWeight, ImageFormat, InterpolationMode, LineCap, RenderContext,
    StrokeStyle, Text as _, TextAttribute, TextLayoutBuilder,
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

#[test]
fn draws_two_row_image_with_half_blocks() {
    let mut buffer = empty_buffer(2, 2);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let image = context
            .make_image(
                1,
                2,
                &[255, 0, 0, 255, 0, 0, 255, 255],
                ImageFormat::RgbaSeparate,
            )
            .unwrap();
        assert_eq!(image.width(), 1);
        assert_eq!(image.height(), 2);
        context.draw_image(
            &image,
            Rect::new(0.0, 0.0, 1.0, 1.0),
            InterpolationMode::NearestNeighbor,
        );
    }

    let cell = buffer.cell((0, 0)).unwrap();
    assert_eq!(cell.symbol(), "▀");
    assert_eq!(cell.fg, TuiColor::Rgb(255, 0, 0));
    assert_eq!(cell.bg, TuiColor::Rgb(0, 0, 255));
    assert_eq!(buffer.cell((1, 0)).unwrap().symbol(), " ");
}

#[test]
fn supports_grayscale_rgb_and_rgba_image_formats() {
    let mut buffer = empty_buffer(1, 3);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let grayscale = context
            .make_image(1, 2, &[0, 255], ImageFormat::Grayscale)
            .unwrap();
        context.draw_image(
            &grayscale,
            Rect::new(0.0, 0.0, 1.0, 1.0),
            InterpolationMode::NearestNeighbor,
        );

        let rgb = context
            .make_image(1, 2, &[255, 0, 0, 0, 255, 0], ImageFormat::Rgb)
            .unwrap();
        context.draw_image(
            &rgb,
            Rect::new(0.0, 1.0, 1.0, 2.0),
            InterpolationMode::NearestNeighbor,
        );

        let premultiplied = context
            .make_image(
                1,
                2,
                &[128, 0, 0, 128, 0, 0, 255, 255],
                ImageFormat::RgbaPremul,
            )
            .unwrap();
        context.draw_image(
            &premultiplied,
            Rect::new(0.0, 2.0, 1.0, 3.0),
            InterpolationMode::NearestNeighbor,
        );
    }

    let grayscale = buffer.cell((0, 0)).unwrap();
    assert_eq!(grayscale.fg, TuiColor::Rgb(0, 0, 0));
    assert_eq!(grayscale.bg, TuiColor::Rgb(255, 255, 255));

    let rgb = buffer.cell((0, 1)).unwrap();
    assert_eq!(rgb.fg, TuiColor::Rgb(255, 0, 0));
    assert_eq!(rgb.bg, TuiColor::Rgb(0, 255, 0));

    let premultiplied = buffer.cell((0, 2)).unwrap();
    assert_eq!(premultiplied.fg, TuiColor::Rgb(128, 0, 0));
    assert_eq!(premultiplied.bg, TuiColor::Rgb(0, 0, 255));
}

#[test]
fn image_stride_selects_the_correct_pixels() {
    let mut buffer = empty_buffer(2, 1);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let image = context
            .make_image_with_stride(
                2,
                2,
                8,
                &[
                    255, 0, 0, 0, 255, 0, 9, 9, // red, green, padding
                    0, 0, 255, 255, 255, 255, 9, 9, // blue, white, padding
                ],
                ImageFormat::Rgb,
            )
            .unwrap();
        context.draw_image(
            &image,
            Rect::new(0.0, 0.0, 2.0, 1.0),
            InterpolationMode::NearestNeighbor,
        );
    }

    assert_eq!(buffer.cell((0, 0)).unwrap().fg, TuiColor::Rgb(255, 0, 0));
    assert_eq!(buffer.cell((0, 0)).unwrap().bg, TuiColor::Rgb(0, 0, 255));
    assert_eq!(buffer.cell((1, 0)).unwrap().fg, TuiColor::Rgb(0, 255, 0));
    assert_eq!(
        buffer.cell((1, 0)).unwrap().bg,
        TuiColor::Rgb(255, 255, 255)
    );
}

#[test]
fn too_short_image_buffer_is_rejected() {
    let mut buffer = empty_buffer(1, 1);
    let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
    let error = context
        .make_image_with_stride(2, 2, 4, &[0; 8], ImageFormat::Rgb)
        .unwrap_err();

    assert!(matches!(error, Error::InvalidInput));
}

#[test]
fn draw_image_area_crops_the_source_rect() {
    let mut buffer = empty_buffer(1, 1);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let image = context
            .make_image(
                2,
                2,
                &[
                    255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
                ],
                ImageFormat::RgbaSeparate,
            )
            .unwrap();
        context.draw_image_area(
            &image,
            Rect::new(1.0, 0.0, 2.0, 2.0),
            Rect::new(0.0, 0.0, 1.0, 1.0),
            InterpolationMode::NearestNeighbor,
        );
    }

    assert_eq!(buffer.cell((0, 0)).unwrap().fg, TuiColor::Rgb(0, 255, 0));
    assert_eq!(
        buffer.cell((0, 0)).unwrap().bg,
        TuiColor::Rgb(255, 255, 255)
    );
}

#[test]
fn bilinear_interpolation_blends_neighboring_pixels() {
    let pixels = [
        0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255,
    ];

    let mut nearest_buffer = empty_buffer(2, 1);
    {
        let mut context = TuiRenderContext::new(&mut nearest_buffer, TuiConfig::default());
        let image = context
            .make_image(2, 2, &pixels, ImageFormat::RgbaSeparate)
            .unwrap();
        context.draw_image(
            &image,
            Rect::new(0.0, 0.0, 2.0, 1.0),
            InterpolationMode::NearestNeighbor,
        );
    }
    assert_eq!(
        nearest_buffer.cell((0, 0)).unwrap().fg,
        TuiColor::Rgb(0, 0, 0)
    );

    let mut bilinear_buffer = empty_buffer(2, 1);
    {
        let mut context = TuiRenderContext::new(&mut bilinear_buffer, TuiConfig::default());
        let image = context
            .make_image(2, 2, &pixels, ImageFormat::RgbaSeparate)
            .unwrap();
        context.draw_image(
            &image,
            Rect::new(0.0, 0.0, 2.0, 1.0),
            InterpolationMode::Bilinear,
        );
    }
    assert_eq!(
        bilinear_buffer.cell((0, 0)).unwrap().fg,
        TuiColor::Rgb(128, 128, 128)
    );
    assert_eq!(
        bilinear_buffer.cell((1, 0)).unwrap().fg,
        TuiColor::Rgb(255, 255, 255)
    );
}

#[test]
fn translucent_image_pixels_blend_over_the_cell_background() {
    let mut buffer = empty_buffer(1, 1);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        context.clear(Some(Rect::new(0.0, 0.0, 1.0, 1.0)), Color::BLACK);
        let image = context
            .make_image(
                1,
                2,
                &[255, 0, 0, 128, 0, 255, 0, 0],
                ImageFormat::RgbaSeparate,
            )
            .unwrap();
        context.draw_image(
            &image,
            Rect::new(0.0, 0.0, 1.0, 1.0),
            InterpolationMode::NearestNeighbor,
        );
    }

    let cell = buffer.cell((0, 0)).unwrap();
    assert_eq!(cell.fg, TuiColor::Rgb(128, 0, 0));
    assert_eq!(cell.bg, TuiColor::Rgb(0, 0, 0));
}

#[test]
fn image_respects_transform_and_clip() {
    let mut buffer = empty_buffer(4, 1);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let image = context
            .make_image(
                1,
                2,
                &[255, 0, 0, 255, 0, 0, 255, 255],
                ImageFormat::RgbaSeparate,
            )
            .unwrap();
        context.transform(Affine::translate((1.0, 0.0)));
        context.clip(Rect::new(0.0, 0.0, 1.0, 1.0));
        context.draw_image(
            &image,
            Rect::new(0.0, 0.0, 2.0, 1.0),
            InterpolationMode::NearestNeighbor,
        );
    }

    assert_eq!(buffer.cell((1, 0)).unwrap().symbol(), "▀");
    assert_eq!(buffer.cell((2, 0)).unwrap().symbol(), " ");
}

#[test]
fn degenerate_transforms_draw_no_image() {
    let mut buffer = empty_buffer(2, 1);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let image = context
            .make_image(
                1,
                2,
                &[255, 0, 0, 255, 0, 0, 255, 255],
                ImageFormat::RgbaSeparate,
            )
            .unwrap();
        context.transform(Affine::scale(0.0));
        context.draw_image(
            &image,
            Rect::new(0.0, 0.0, 1.0, 1.0),
            InterpolationMode::NearestNeighbor,
        );
    }

    assert_eq!(buffer.cell((0, 0)).unwrap().symbol(), " ");
    assert_eq!(buffer.cell((0, 0)).unwrap().fg, TuiColor::Reset);
}

#[test]
fn empty_and_invalid_images_draw_nothing() {
    let mut buffer = empty_buffer(2, 1);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        let empty = context
            .make_image(0, 0, &[], ImageFormat::RgbaSeparate)
            .unwrap();
        context.draw_image(
            &empty,
            Rect::new(0.0, 0.0, 1.0, 1.0),
            InterpolationMode::NearestNeighbor,
        );

        let image = context
            .make_image(
                1,
                2,
                &[255, 0, 0, 255, 0, 0, 255, 255],
                ImageFormat::RgbaSeparate,
            )
            .unwrap();
        context.draw_image_area(
            &image,
            Rect::new(0.0, 0.0, -1.0, 1.0),
            Rect::new(0.0, 0.0, 1.0, 1.0),
            InterpolationMode::NearestNeighbor,
        );
        context.draw_image(
            &image,
            Rect::new(0.0, 0.0, 0.0, 0.0),
            InterpolationMode::NearestNeighbor,
        );
        context.draw_image(
            &image,
            Rect::new(f64::NAN, 0.0, 1.0, 1.0),
            InterpolationMode::NearestNeighbor,
        );
    }

    for x in 0..2 {
        assert_eq!(buffer.cell((x, 0)).unwrap().symbol(), " ");
        assert_eq!(buffer.cell((x, 0)).unwrap().fg, TuiColor::Reset);
    }
}
