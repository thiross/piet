# piet-tui Markdown rendering

This example exercises **rich text layout** with *italic*, ~~struck-through~~,
`inline code`, and a [link](https://github.com/linebender/piet). It also mixes
**bold with _nested italic_** and an escaped \*asterisk\*.

## 中文、Unicode 与换行

这是一段较长的中文文本，用来测试 **中文粗体**、*中文斜体*、`终端代码`、组合字符
é、全角标点，以及 emoji 🚀 的混排效果。Unicode 宽度和换行经常比纯英文更容易暴露问题。

A very long unbroken token follows to exercise grapheme-boundary breaking:
`piet_tui_terminal_rich_text_layout_stress_test_without_any_spaces_or_break_points`.

> A block quote can contain **bold text**, *italics*, and `code`. This line is
> intentionally long enough to wrap across multiple terminal rows while keeping
> the quote marker visible on every source line.

## Lists and task items

- A **bold** bullet with an inline `code` span.
- Another bullet with a [link](https://ratatui.rs) and ~~deleted text~~.
  - Nested items test indentation and continued wrapping across several cells.
- [x] Completed task
- [ ] Pending task

1. First ordered item
2. Second ordered item with *emphasis*
3. Third ordered item with **strong emphasis**

---

## Code block

```rust
fn main() {
    let message = "Markdown **markers** stay literal inside code blocks";
    println!("{message}");
}
```

### Final section

The last paragraph stresses line wrapping again: `alpha beta gamma delta epsilon
zeta eta theta iota kappa lambda mu` should flow through the available width
without losing style ranges when the layout produces multiple lines.
