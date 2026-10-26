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
    use std::env;
    use std::io;
    use std::ops::Range;
    use std::path::Path;
    use std::time::Duration;

    use piet::kurbo::{Point, Rect as KurboRect};
    use piet::{
        Color, FontStyle, FontWeight, RenderContext, Text as _, TextAttribute, TextLayout as _,
        TextLayoutBuilder,
    };
    use piet_tui::{TuiConfig, TuiRenderContext, TuiTextLayout};
    use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
    use ratatui::layout::Rect;
    use ratatui::{DefaultTerminal, Frame};

    const BACKGROUND: Color = Color::rgb8(13, 16, 24);
    const FOREGROUND: Color = Color::rgb8(210, 216, 228);
    const MUTED: Color = Color::rgb8(120, 130, 150);
    const ACCENT: Color = Color::rgb8(255, 196, 87);
    const LINK: Color = Color::rgb8(100, 190, 255);
    const CODE_FG: Color = Color::rgb8(255, 180, 120);
    const CODE_BG: Color = Color::rgb8(33, 39, 53);
    const QUOTE: Color = Color::rgb8(150, 170, 190);

    pub(super) fn main() -> io::Result<()> {
        let (name, source) = load_markdown()?;
        let blocks = parse_markdown(&source);
        ratatui::run(|terminal| run(terminal, &name, &blocks))
    }

    fn load_markdown() -> io::Result<(String, String)> {
        match env::args_os().nth(1) {
            Some(path) => {
                let path = Path::new(&path);
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("markdown")
                    .to_owned();
                Ok((name, std::fs::read_to_string(path)?))
            }
            None => Ok((
                "markdown.md".to_owned(),
                include_str!("markdown.md").to_owned(),
            )),
        }
    }

    fn run(terminal: &mut DefaultTerminal, name: &str, blocks: &[Block]) -> io::Result<()> {
        let mut scroll = 0;
        let mut max_scroll = 0;

        loop {
            terminal.draw(|frame| {
                max_scroll = draw_markdown(frame, name, blocks, scroll);
            })?;
            scroll = scroll.min(max_scroll);

            if !event::poll(Duration::from_millis(250))? {
                continue;
            }
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Up | KeyCode::Char('k') => {
                    scroll = scroll.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    scroll = scroll.saturating_add(1).min(max_scroll);
                }
                KeyCode::PageUp => {
                    scroll = scroll.saturating_sub(10);
                }
                KeyCode::PageDown => {
                    scroll = scroll.saturating_add(10).min(max_scroll);
                }
                KeyCode::Home | KeyCode::Char('g') => scroll = 0,
                KeyCode::End | KeyCode::Char('G') => scroll = max_scroll,
                _ => {}
            }
        }
    }

    fn draw_markdown(frame: &mut Frame, name: &str, blocks: &[Block], scroll: usize) -> usize {
        let area = frame.area();
        let content = Rect::new(
            area.x.saturating_add(1),
            area.y.saturating_add(1),
            area.width.saturating_sub(2),
            area.height.saturating_sub(2),
        );
        let mut context =
            TuiRenderContext::with_area(frame.buffer_mut(), area, TuiConfig::default());
        context.clear(None, BACKGROUND);

        let header = context
            .text()
            .new_text_layout(format!(" {name}  ·  Markdown rich-text stress test"))
            .range_attribute(0.., TextAttribute::TextColor(ACCENT))
            .range_attribute(0.., FontWeight::BOLD)
            .build()
            .unwrap();
        context.draw_text(&header, Point::new(0.0, 0.0));

        let footer = context
            .text()
            .new_text_layout("↑/↓ or j/k scroll  ·  PgUp/PgDn  ·  Home/End  ·  q quit")
            .range_attribute(0.., TextAttribute::TextColor(MUTED))
            .build()
            .unwrap();
        let footer_y = area.height.saturating_sub(1) as f64;
        context.draw_text(&footer, Point::new(1.0, footer_y));

        if content.width == 0 || content.height == 0 {
            return 0;
        }

        let clip = KurboRect::new(
            content.x as f64,
            content.y as f64,
            content.right() as f64,
            content.bottom() as f64,
        );
        context.save().unwrap();
        context.clip(clip);

        let mut y = content.y as isize - scroll as isize;
        let mut total_height = 0usize;
        for block in blocks {
            let indent = block.indent.min(content.width.saturating_sub(1) as usize);
            let max_width = content.width.saturating_sub(indent as u16).max(1) as f64;
            let layout = build_layout(&mut context, block, max_width);
            let height = layout.line_count();
            let visible = y + height as isize > content.y as isize && y < content.bottom() as isize;

            if visible {
                context.draw_text(
                    &layout,
                    Point::new((content.x as usize + indent) as f64, y as f64),
                );
            }

            let gap = block.kind.gap_after();
            y += (height + gap) as isize;
            total_height += height + gap;
        }

        context.restore().unwrap();
        total_height.saturating_sub(content.height as usize)
    }

    fn build_layout(
        context: &mut TuiRenderContext<'_>,
        block: &Block,
        max_width: f64,
    ) -> TuiTextLayout {
        let mut builder = context
            .text()
            .new_text_layout(block.text.text.clone())
            .max_width(max_width);

        builder = match block.kind {
            BlockKind::Heading(1) => builder
                .default_attribute(TextAttribute::TextColor(ACCENT))
                .default_attribute(FontWeight::BOLD),
            BlockKind::Heading(2) => builder
                .default_attribute(TextAttribute::TextColor(LINK))
                .default_attribute(FontWeight::BOLD),
            BlockKind::Heading(_) => builder
                .default_attribute(TextAttribute::TextColor(Color::rgb8(160, 220, 180)))
                .default_attribute(FontWeight::BOLD),
            BlockKind::Quote => builder
                .default_attribute(TextAttribute::TextColor(QUOTE))
                .default_attribute(FontStyle::Italic),
            BlockKind::Code => builder
                .default_attribute(TextAttribute::TextColor(CODE_FG))
                .background_color(0..block.text.text.len(), Some(CODE_BG)),
            BlockKind::Rule => builder.default_attribute(TextAttribute::TextColor(MUTED)),
            BlockKind::Paragraph | BlockKind::Bullet | BlockKind::Ordered => {
                builder.default_attribute(TextAttribute::TextColor(FOREGROUND))
            }
        };

        for span in &block.text.spans {
            let range = span.range.clone();
            if span.style.bold {
                builder = builder.range_attribute(range.clone(), FontWeight::BOLD);
            }
            if span.style.italic {
                builder = builder.range_attribute(range.clone(), FontStyle::Italic);
            }
            if span.style.underline {
                builder = builder.range_attribute(range.clone(), TextAttribute::Underline(true));
            }
            if span.style.strikethrough {
                builder =
                    builder.range_attribute(range.clone(), TextAttribute::Strikethrough(true));
            }
            if span.style.code {
                builder = builder
                    .range_attribute(range.clone(), TextAttribute::TextColor(CODE_FG))
                    .background_color(range.clone(), Some(CODE_BG));
            }
            if let Some(color) = span.style.fg {
                builder = builder.range_attribute(range, TextAttribute::TextColor(color));
            }
        }

        builder.build().unwrap()
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum BlockKind {
        Heading(u8),
        Paragraph,
        Bullet,
        Ordered,
        Quote,
        Code,
        Rule,
    }

    impl BlockKind {
        fn gap_after(self) -> usize {
            match self {
                Self::Heading(1) => 1,
                Self::Heading(_) | Self::Paragraph | Self::Rule => 1,
                Self::Bullet | Self::Ordered | Self::Quote => 0,
                Self::Code => 1,
            }
        }
    }

    struct Block {
        kind: BlockKind,
        indent: usize,
        text: StyledText,
    }

    #[derive(Default)]
    struct StyledText {
        text: String,
        spans: Vec<Span>,
    }

    impl StyledText {
        fn push(&mut self, text: &str, style: InlineStyle) {
            if text.is_empty() {
                return;
            }
            let start = self.text.len();
            self.text.push_str(text);
            let end = self.text.len();
            if style == InlineStyle::default() {
                return;
            }
            if let Some(last) = self.spans.last_mut()
                && last.range.end == start
                && last.style == style
            {
                last.range.end = end;
            } else {
                self.spans.push(Span {
                    range: start..end,
                    style,
                });
            }
        }
    }

    struct Span {
        range: Range<usize>,
        style: InlineStyle,
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    struct InlineStyle {
        bold: bool,
        italic: bool,
        underline: bool,
        strikethrough: bool,
        code: bool,
        fg: Option<Color>,
    }

    fn parse_markdown(source: &str) -> Vec<Block> {
        let lines: Vec<&str> = source.lines().collect();
        let mut blocks = Vec::new();
        let mut index = 0;

        while index < lines.len() {
            let line = lines[index];
            let trimmed = line.trim_start();
            let indent = line.len() - trimmed.len();

            if trimmed.is_empty() {
                index += 1;
                continue;
            }

            if trimmed.strip_prefix("```").is_some() {
                index += 1;
                let mut code = Vec::new();
                while index < lines.len() && !lines[index].trim_start().starts_with("```") {
                    code.push(lines[index]);
                    index += 1;
                }
                index += usize::from(index < lines.len());
                let mut text = StyledText::default();
                text.push(
                    &code.join("\n"),
                    InlineStyle {
                        code: true,
                        ..InlineStyle::default()
                    },
                );
                blocks.push(Block {
                    kind: BlockKind::Code,
                    indent: indent + 2,
                    text,
                });
                continue;
            }

            if is_rule(trimmed) {
                blocks.push(Block {
                    kind: BlockKind::Rule,
                    indent,
                    text: StyledText::default(),
                });
                index += 1;
                continue;
            }

            if let Some((level, rest)) = heading(trimmed) {
                blocks.push(styled_block(BlockKind::Heading(level), indent, "", rest));
                index += 1;
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("> ") {
                blocks.push(styled_block(BlockKind::Quote, indent, "│ ", rest));
                index += 1;
                continue;
            }

            if let Some((prefix, rest)) = list_item(trimmed) {
                let kind = if prefix.as_bytes().first().is_some_and(u8::is_ascii_digit) {
                    BlockKind::Ordered
                } else {
                    BlockKind::Bullet
                };
                blocks.push(styled_block(kind, indent, &prefix, rest));
                index += 1;
                continue;
            }

            let mut paragraph = String::new();
            while index < lines.len()
                && !lines[index].trim().is_empty()
                && !is_block_start(lines[index])
            {
                if !paragraph.is_empty() {
                    if lines[index - 1].ends_with("  ") {
                        paragraph.push('\n');
                    } else {
                        paragraph.push(' ');
                    }
                }
                paragraph.push_str(lines[index].trim());
                index += 1;
            }
            blocks.push(styled_block(BlockKind::Paragraph, indent, "", &paragraph));
        }

        blocks
    }

    fn styled_block(kind: BlockKind, indent: usize, prefix: &str, content: &str) -> Block {
        let mut text = StyledText::default();
        text.push(
            prefix,
            InlineStyle {
                bold: true,
                fg: Some(ACCENT),
                ..InlineStyle::default()
            },
        );
        parse_inline_into(content, InlineStyle::default(), &mut text);
        Block { kind, indent, text }
    }

    fn heading(line: &str) -> Option<(u8, &str)> {
        for level in 1..=3 {
            let marker = match level {
                1 => "# ",
                2 => "## ",
                _ => "### ",
            };
            if let Some(rest) = line.strip_prefix(marker) {
                return Some((level, rest));
            }
        }
        None
    }

    fn is_rule(line: &str) -> bool {
        let line = line.trim_end();
        line.len() >= 3
            && (line.chars().all(|ch| ch == '-')
                || line.chars().all(|ch| ch == '*')
                || line.chars().all(|ch| ch == '_'))
    }

    fn list_item(line: &str) -> Option<(String, &str)> {
        for marker in ["- ", "* ", "+ "] {
            if let Some(rest) = line.strip_prefix(marker) {
                return if let Some(rest) = rest.strip_prefix("[x] ") {
                    Some(("☑ ".to_owned(), rest))
                } else if let Some(rest) = rest.strip_prefix("[ ] ") {
                    Some(("☐ ".to_owned(), rest))
                } else {
                    Some(("• ".to_owned(), rest))
                };
            }
        }

        let digits = line.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 && line[digits..].starts_with(". ") {
            let marker = &line[..digits];
            let rest = &line[digits + 2..];
            return Some((format!("{marker}. "), rest));
        }
        None
    }

    fn is_block_start(line: &str) -> bool {
        let line = line.trim_start();
        line.is_empty()
            || line.starts_with("```")
            || line.starts_with("> ")
            || heading(line).is_some()
            || is_rule(line)
            || list_item(line).is_some()
    }

    fn parse_inline_into(input: &str, style: InlineStyle, output: &mut StyledText) {
        let mut index = 0;
        while index < input.len() {
            let rest = &input[index..];

            if let Some((inner, used)) = delimited(rest, "**") {
                parse_inline_into(
                    inner,
                    InlineStyle {
                        bold: true,
                        ..style
                    },
                    output,
                );
                index += used;
                continue;
            }
            if let Some((inner, used)) = delimited(rest, "__") {
                parse_inline_into(
                    inner,
                    InlineStyle {
                        bold: true,
                        ..style
                    },
                    output,
                );
                index += used;
                continue;
            }
            if let Some((inner, used)) = delimited(rest, "~~") {
                parse_inline_into(
                    inner,
                    InlineStyle {
                        strikethrough: true,
                        ..style
                    },
                    output,
                );
                index += used;
                continue;
            }

            if let Some((inner, used)) = delimited(rest, "*") {
                parse_inline_into(
                    inner,
                    InlineStyle {
                        italic: true,
                        ..style
                    },
                    output,
                );
                index += used;
                continue;
            }
            if let Some((inner, used)) = delimited(rest, "_") {
                parse_inline_into(
                    inner,
                    InlineStyle {
                        italic: true,
                        ..style
                    },
                    output,
                );
                index += used;
                continue;
            }

            if let Some(rest) = rest.strip_prefix('`')
                && let Some(end) = rest.find('`')
            {
                output.push(
                    &rest[..end],
                    InlineStyle {
                        code: true,
                        ..style
                    },
                );
                index += end + 2;
                continue;
            }

            if rest.starts_with('[')
                && let Some(label_end) = rest.find("](")
                && let Some(url_end) = rest[label_end + 2..].find(')')
            {
                parse_inline_into(
                    &rest[1..label_end],
                    InlineStyle {
                        underline: true,
                        fg: Some(LINK),
                        ..style
                    },
                    output,
                );
                index += label_end + 2 + url_end + 1;
                continue;
            }

            if rest.starts_with('\\')
                && let Some(ch) = rest[1..].chars().next()
            {
                output.push(&rest[1..1 + ch.len_utf8()], style);
                index += 1 + ch.len_utf8();
                continue;
            }

            let ch = rest.chars().next().unwrap();
            if is_inline_special(ch) {
                output.push(&rest[..ch.len_utf8()], style);
                index += ch.len_utf8();
                continue;
            }

            let mut end = ch.len_utf8();
            for ch in rest[end..].chars() {
                if is_inline_special(ch) {
                    break;
                }
                end += ch.len_utf8();
            }
            output.push(&rest[..end], style);
            index += end;
        }
    }

    fn delimited<'a>(input: &'a str, delimiter: &str) -> Option<(&'a str, usize)> {
        let rest = input.strip_prefix(delimiter)?;
        let end = rest.find(delimiter)?;
        let inner = &rest[..end];
        if inner.is_empty() {
            return None;
        }
        Some((inner, delimiter.len() + end + delimiter.len()))
    }

    fn is_inline_special(ch: char) -> bool {
        matches!(ch, '\\' | '`' | '*' | '_' | '~' | '[')
    }
}
