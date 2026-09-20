# 排版热点分析与优化

**日期**：2026-09-21  
**分类**：perf  
**状态**：已完成

## 目标

降低 `examples/render` 默认例文在字体与字形缓存已建立后的重复排版耗时。

## 范围

1. 新增无 GUI、GPU 上传和图片导出的排版采样入口。
2. 复用 render demo 的默认例文、字体顺序、布局参数和文字样式。
3. 使用火焰图定位稳态排版热点，再针对热点实施和验证优化。

## 测量设计

`examples/layout_flamegraph.rs` 创建并复用同一个 `Huozi`，先执行一次预热，再按命令行参数重复调用 `layout_parse`。`fixed` 模式重复使用相同文本；`edit` 模式每轮继续添加一个 `a`，确保每次文本和 tiqian annotation cache key 都不同，与 GUI 中持续输入新字符一致。`profile` 模式预先生成带固定长度唯一后缀的输入，保持段落长度稳定并持续触发 annotation cache 未命中，用于火焰图采样。测量范围包括：

- 富文本解析与样式展开；
- Huozi 到 tiqian 的输入适配；
- shaping、断行和行调整；
- tiqian 结果到 glyph 顶点的输出适配；
- 已缓存字形的 SDF 图集查询。

字体文件读取、字体目录初始化和首次 SDF 生成位于预热阶段，不计入稳态循环。程序仅在循环结束后输出总耗时和平均耗时，循环内不执行日志、GUI、GPU 或文件 I/O。

程序在每轮创建与 GUI 一致的单元素 `Segment` 容器。程序在计时外比较预热与末次排版的字形数量、来源区间数量和布局尺寸，确认偶数轮重复排版后结果稳定。

默认字体与未启用 `woff` feature 的 render demo 一致：

1. `InterVariable.ttf`，Latin；
2. `InterVariable-Italic.ttf`，Latin；
3. `SourceHanSansSC-VF.otf`，CJK。

启用 `woff` feature 时还会加载 `SourceHanSerif-VF.otf.woff2`，与相同 feature 下的 render demo 一致。

## 验证结果

- `cargo test` 通过：53 个单元测试、5 个集成测试、3 个 doctest；
- `cargo check --example render --release --features woff` 通过；
- `cargo run --example layout_flamegraph --release --features woff -- 1000 profile` 的末次结果为 256 个字形、0 个来源区间和 `992 × 360` 布局尺寸；
- 使用 `cargo flamegraph` 对 release `profile` 模式采集两轮 3000 次样本，确认 Huozi 自有帧均低于 2%。

2026-09-21 的 Windows release 对照结果如下，每组执行 1000 次：

| 字体配置 | 模式 | 平均耗时 |
| --- | --- | ---: |
| 默认三字体 | `fixed` | 468.611 µs |
| 默认三字体 | 旧 `edit` | 479.032 µs |
| 启用 `woff` 的四字体 | `fixed` | 466.761 µs |
| 启用 `woff` 的四字体 | 旧 `edit` | 474.511 µs |

旧 `edit` 模式只在原文和原文加一个 `a` 之间交替。前两轮后，两份输入均命中 tiqian annotation cache，不能代表 GUI 持续追加字符的缓存未命中路径，因此这些数据不作为编辑延迟基线。

GUI 文本编辑触发的一次实测结果如下：

| 阶段 | 耗时 | 占 `layout_parse` 比例 |
| --- | ---: | ---: |
| 富文本解析 | 23 µs | 0.3% |
| 输入适配 | 23.1 µs | 0.3% |
| tiqian 段落布局 | 7.823 ms | 97.3% |
| 输出适配 | 135.5 µs | 1.7% |
| `layout_parse` 总计 | 8.0391 ms | 100% |

该次调用没有初始化 `Huozi`，SDF 图集也没有更新。后续火焰图与优化集中在 `ParagraphLayoutEngine::layout` 内部。

修正后的 `edit` 模式已复现 GUI 延迟：连续追加 10 个 `a` 时平均每次 7.82281 ms。旧模式只在两份文本之间交替，前两轮后持续命中 `ParagraphLayoutEngine` 的 width-independent annotation cache，导致约 0.47 ms 的错误基线。执行 100 次连续追加时，文本增长使平均耗时升至 9.435203 ms，因此火焰图改用固定长度、每轮唯一的 `profile` 模式。

`profile` 模式执行 100 次时平均 7.97414 ms，执行 1000 次时平均 8.405855 ms，最后一次输出均保持 256 个字形和 992 × 360 布局尺寸。该模式稳定复现 GUI 延迟。使用旧 `edit` 模式生成的火焰图不作为优化依据。

## 火焰图结论与优化

有效火焰图使用 `profile` 模式采集 3000 次，共取得 204580 个样本。主要热点为：

| 调用 | inclusive 样本占比 |
| --- | ---: |
| `prepare_width_independent_annotation` | 89.98% |
| `shape_paragraph` | 88.93% |
| `HuoziFontManager::shape` | 74.79% |
| `HuoziFontManager::candidates` | 41.14% |
| `append_candidates_by_weight` | 36.85% |
| `HuoziFontManager::shape_one_face` | 29.68% |
| HarfRust shaping | 26.17% |
| HarfRust OpenType lookup cache 初始化 | 18.48% |

第一轮优化将字体权重候选从每次创建并扫描 1001 个 `Vec`，改为对实际候选执行稳定排序。字体数量通常为 1 至 4，CSS 权重优先顺序保持不变。1000 次 `profile` 平均耗时从 8.405855 ms 降至 4.211402 ms。

第二轮优化按字体 face 缓存 HarfRust `ShaperData`。该类型保存单个字体的 OpenType lookup 等缓存；variation 仍由每次请求的 `ShaperInstance` 独立表示。1000 次 `profile` 平均耗时降至 1.889177 ms，清理诊断日志后的最终复测为 1.913668 ms。与初始基线相比，耗时减少约 77%，速度约为原来的 4.4 倍。

两项优化均通过字体后端测试、完整测试和固定输出摘要验证。后续两轮 3000 次火焰图采集确认了优化后的热点占比。

第三轮优化为 Huozi 字体后端增加 1024 项未缩放 glyph ink bounds LRU。它缓存 `(FontFaceId, glyph id)` 到 bounds，包括无 bounds 的结果，排版时按字号缩放，避免同一字形在每次 shaping 时重复用 SkRifa 回放 outline。缓存条目数固定，连续加入新字符达到容量后按 LRU 淘汰旧项。调整缓存键后的 1000 次 `profile` 复测平均耗时为 1.635966 ms。

随后使用 `profile` 模式采集 3000 次火焰图，平均耗时为 1.725384 ms。`HuoziFontManager::add_ink_bounds` 的 inclusive 样本占比从 12.63% 降至 1.06%；输入适配为 0.68%，输出适配为 0.25%。主导帧转为 HarfRust 的 OpenType plan、GSUB 与 GPOS 处理，以及 tiqian `ShapingResult` 的克隆和释放。

火焰图中 `font_candidate` 占 2.06%。默认 variation 候选此前每次都会创建空设置容器和与注册实例等价的 `FontFaceId`。现在默认路径复用 `FontFaceRecord` 的已注册 id；仅在实际设置 `wght`、`ital` 或 `slnt` 时构造新的 variation 实例。完整回归通过，1000 次 `profile` 复测为 1.613698 ms。该短基准与前一次的差异按测量波动处理；最终火焰图确认了该优化后的占比。

最终火焰图确认 `font_candidate` 降至 1.32%。其余 Huozi 自有帧均低于 2%：glyph ink bounds 批量处理为 1.07%，bounds LRU 查询为 0.32%，输入适配为 0.65%，输出适配为 0.24%。本次 Huozi 侧优化到此结束，后续热点分析应转向 tiqian 或 HarfRust。

## 回滚

性能采样入口与生产代码分离。若后续优化没有稳定收益，可单独回滚对应生产代码，保留采样入口用于回归分析。