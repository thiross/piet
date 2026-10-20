# Piet 文本绘制原点：Windows / macOS GUI 平台核查

本文档对应根目录 `druid-piet-selection-handoff.md` 中的 Piet 任务。公共契约和
TUI 后端已在当前 Linux 工作区完成；Windows 和 macOS 的 GUI 后端尚未在对应系统上
运行验证，因此把需要核查的项目集中记录在这里。

## 已完成的公共契约

`piet::TextLayout` 新增默认方法：

```rust
fn layout_origin(&self, requested: Point) -> Point {
    requested
}
```

参数 `requested` 是调用方传给 `draw_text` 的绘制原点，并且已经应用当前 render
transform。返回值表示后端实际放置 glyph origin 的位置。

- 不吸附文本原点的后端不需要覆盖该方法，默认实现保持 identity。
- `hit_test_point`、`hit_test_text_position` 和 `rects_for_range` 仍使用相对于
  `requested` 的布局坐标，不改变语义。
- 调用方如果要把命中点或选区矩形对齐到实际绘制位置，应使用
  `layout.layout_origin(requested) - requested` 作为偏移。

`piet-tui::TuiTextLayout` 已覆盖该方法。它与 `draw_text` 共用同一个
`origin_cell` 计算，按 `TuiConfig::cell_width()` / `cell_height()` 对 origin
做 `round()`。当前测试覆盖 8x16 cell 的半 cell 吸附、1x1 cells 模式、负数坐标、
多行文本、CJK、宽字符、真实 buffer 中的绘制位置，以及半 cell 偏移下用实际
绘制 cell 中心做命中测试的回归场景。

## 当前 GUI 后端结论

Linux 的 `piet-cairo` 已核查：`draw_text` 将传入位置转换为 Pango 的绘制偏移，
不会把 origin 吸附到额外网格，因此沿用默认 identity 是正确行为，本仓库没有为
Cairo 增加 override。

其余 GUI 后端从源码看也按传入位置绘制，预期同样沿用 identity：

- `piet-direct2d`：`D2DTextLayout::draw(pos, ctx)` 将 `pos` 直接传给
  `DrawTextLayout`。
- `piet-coregraphics`：`draw_text` 通过 `translate(pos.x, frame_size.height + pos.y)`
  定位文本；内部 y 轴翻转不改变公开 origin 语义。
- `piet-web` 和 `piet-svg` 也没有 cell 网格吸附。

因此 Windows / macOS 的默认目标不是新增吸附逻辑，而是确认后端确实在请求位置
绘制文本。只有实测发现后端内部会量化 glyph origin 时，才需要覆盖
`layout_origin`。

`piet-common/tests/text.rs` 已新增跨 GUI 后端的
`layout_origin_is_identity_for_gui` 测试。Linux 上已用 Cairo 通过；在 Windows
或 macOS 上运行同一测试时，会自动选择 Direct2D 或 CoreGraphics 后端。

## Windows 核查清单（piet-direct2d）

1. 在 Windows 上运行 `cargo test -p piet-direct2d`，确认现有测试全部通过。
2. 运行 `cargo test -p piet-common --test text layout_origin_is_identity_for_gui`，
   确认 Direct2D 对非整数 origin 返回 identity。
3. 使用非整数坐标创建 bitmap target，确认 `draw_text` 的实际 glyph 位置与请求
   位置一致。重点检查高 DPI 下 `DIP -> pixel` 转换是否引入 origin 吸附。
4. 如果 DirectWrite 在实际渲染时吸附 origin，先在 `D2DTextLayout` 上实现
   `layout_origin`，并让 `draw_text` 和该方法共用同一个吸附 helper。不要修改
   `hit_test_point` 或 `rects_for_range` 的布局坐标。
5. 补一个回归测试：文本请求原点位于半 cell/半像素偏移时，
   `layout_origin(requested)` 与 `draw_text` 的最终视觉起点一致。

## macOS 核查清单（piet-coregraphics）

1. 在 macOS 上运行 `cargo test -p piet-coregraphics`，确认现有测试全部通过。
2. 运行 `cargo test -p piet-common --test text layout_origin_is_identity_for_gui`，
   确认 CoreGraphics 对非整数 origin 返回 identity。
3. 检查 Retina scale factor 和字体 overhang：确认 `draw_text` 的 translate
   使用的位置与公开 `pos` 一致，`frame_size.height` 只用于坐标翻转。
4. 如果 CoreText/CoreGraphics 在最终绘制时量化 origin，在
   `CoreGraphicsTextLayout` 上覆盖 `layout_origin`，并让 `draw_text` 复用同一
   计算。保持 `rects_for_range` 和 hit test 的坐标空间不变。
5. 用非整数 origin 做一次视觉或 bitmap 回归测试，确认文字基线与选区矩形对齐。

## 验收标准

- 非 TUI 后端默认 `layout_origin(requested) == requested`。
- TUI 后端的 `layout_origin` 与 `draw_text` 使用同一个吸附规则。
- `rects_for_range` 仍返回布局坐标；调用方可以用
  `layout_origin(requested) - requested` 对齐选区。
- Windows / macOS 若未发现吸附，只需保留 identity 测试，不新增平台专用代码。
- Windows / macOS 若发现吸附，必须同时验证绘制位置、命中位置和选区矩形三者一致。
