# 富文本内置动效

## 目标

将已有 `Effect` 接入富文本正文和 `examples/render`。

## 标签与行为

- 支持 `[cluster]`、`[wave]`、`[shake]`、`[glow]`、`[glitch]`；
- 按外层到内层的顺序组合效果。

## 数据与绘制

- `TextStyle` 增加 `effects: Vec<Effect>`；
- `RichTextLayoutOutput` 增加 `clusters: Vec<GlyphCluster>` `text_length: usize`‘
- `GlyphCluster` 包含 `source: Range<usize>`、`glyph_range: Range<usize>` `effects: Arc<[Effect]>`’
- `source` 采用送入 Tiqian 的文档连续文本 scalar 范围；
- `text_length` 是包括当前实际送入排版的输入长度；高度耗尽后未送入排版的后续段落不计；
- `cluster` 来自 Tiqian 实际输出。

## 其他

无。