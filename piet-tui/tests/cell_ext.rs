// Copyright 2026 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use piet::kurbo::{Affine, Rect};
use piet::{Color, RenderContext};
use piet_tui::{TuiCellExt, TuiConfig, TuiRenderContext};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect as TuiRect;
use ratatui_core::style::Color as TuiColor;

fn empty_buffer(width: u16, height: u16) -> Buffer {
    Buffer::empty(TuiRect::new(0, 0, width, height))
}

#[test]
fn set_cell_sets_symbol_and_colors() {
    let mut buffer = empty_buffer(4, 2);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        context.set_cell(1, 1, "┌", Color::rgb8(10, 20, 30), Color::rgb8(40, 50, 60));
    }

    let cell = buffer.cell((1, 1)).unwrap();
    assert_eq!(cell.symbol(), "┌");
    assert_eq!(cell.fg, TuiColor::Rgb(10, 20, 30));
    assert_eq!(cell.bg, TuiColor::Rgb(40, 50, 60));
}

#[test]
fn set_cell_respects_clip_and_context_area() {
    let mut buffer = empty_buffer(4, 2);
    {
        let mut context = TuiRenderContext::with_area(
            &mut buffer,
            TuiRect::new(1, 0, 2, 2),
            TuiConfig::default(),
        );
        context.clip(Rect::new(0.0, 0.0, 1.0, 2.0));
        context.set_cell(0, 0, "a", Color::WHITE, Color::BLACK);
        context.set_cell(1, 0, "b", Color::WHITE, Color::BLACK);
        context.set_cell(2, 0, "c", Color::WHITE, Color::BLACK);
    }

    assert_eq!(buffer.cell((1, 0)).unwrap().symbol(), "a");
    assert_eq!(buffer.cell((2, 0)).unwrap().symbol(), " ");
    assert_eq!(buffer.cell((0, 0)).unwrap().symbol(), " ");
}

#[test]
fn fill_cells_uses_local_coordinates_and_clips() {
    let mut buffer = empty_buffer(6, 4);
    {
        let mut context = TuiRenderContext::with_area(
            &mut buffer,
            TuiRect::new(1, 1, 4, 2),
            TuiConfig::default(),
        );
        context.clip(Rect::new(0.0, 0.0, 3.0, 2.0));
        context.fill_cells(
            TuiRect::new(0, 0, 4, 2),
            Color::rgb8(1, 2, 3),
            Color::rgb8(4, 5, 6),
        );
    }

    for y in 1..3 {
        for x in 1..4 {
            let cell = buffer.cell((x, y)).unwrap();
            assert_eq!(cell.symbol(), " ");
            assert_eq!(cell.fg, TuiColor::Rgb(1, 2, 3));
            assert_eq!(cell.bg, TuiColor::Rgb(4, 5, 6));
        }
    }
    assert_eq!(buffer.cell((4, 1)).unwrap().bg, TuiColor::Reset);
    assert_eq!(buffer.cell((1, 3)).unwrap().bg, TuiColor::Reset);
}

#[test]
fn cell_operations_ignore_transform() {
    let mut buffer = empty_buffer(3, 3);
    {
        let mut context = TuiRenderContext::new(&mut buffer, TuiConfig::default());
        context.transform(Affine::translate((2.0, 2.0)));
        context.set_cell(0, 0, "x", Color::WHITE, Color::BLACK);
    }

    assert_eq!(buffer.cell((0, 0)).unwrap().symbol(), "x");
    assert_eq!(buffer.cell((2, 2)).unwrap().symbol(), " ");
}
