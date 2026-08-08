# Tiqian-rs 范围与 Shaping 边界讨论纪要

- 日期：2026-07-26
- 状态：已确认，等待后续阶段性草案汇总
- 关联：[整合 Roadmap](../roadmap.md)

## 讨论背景

本轮讨论用于收窄 `tiqian-rs` 的目标和 Huozi 的 shaping 范围。结论作为后续领域建模与阶段性草案的输入，不在本纪要中确定具体 Rust API、crate 划分或依赖库。

## 已确认原则

### 1. 不兼容 Kotlin Tiqian 的接口

`tiqian-rs` 不追求复刻 Kotlin Tiqian 的接口、输入格式或模块结构。移植对象是已经验证的排版规则、领域模型、架构决策与测试证据。

`tiqian-rs` 的输入输出应围绕 Huozi 的实际需求重新设计，但排版核心仍保持独立：

- 不依赖 Huozi 的 parser 类型；
- 不依赖 SDF、WGPU 或顶点类型；
- 不直接生成 Huozi 渲染数据；
- 通过明确的输入和输出转换与 Huozi 集成。

目标关系为：

```text
Huozi source / rich text
  -> 输入转换
  -> tiqian-rs layout input
  -> tiqian-rs layout result
  -> 输出转换
  -> Huozi glyph atlas / render data
```

Kotlin Tiqian 在迁移期间作为算法、ADR、fixture 和行为对照的参考实现。Rust 与 Kotlin 的公开类型无需逐字段等价，只需要能规范化为共同的测试表示。

### 2. 支持范围以 CJK 和 LTR 字母文字为主

首阶段主要支持：

- CJK 文字；
- 排版行为接近 CJK 的文字，例如彝文；
- 拉丁字母；
- 排版行为接近拉丁字母的从左到右文字，例如西里尔字母。

暂不支持：

- 阿拉伯文、希伯来文等 RTL 文字；
- RTL 与 LTR 双向混排；
- 婆罗米系文字及其他依赖复杂音节重排的文字。

这一范围允许首阶段不实现完整 bidi、视觉顺序与逻辑顺序映射、RTL 命中测试，以及复杂脚本专用的音节重排和行边界处理。

“排版行为接近 CJK”只表示可以复用逐字排布等基础机制，不表示自动套用 CLREQ 的中文标点、禁则或段落规则。具体文字采用哪些规则，仍需由 profile 或明确策略决定。

### 3. Grapheme cluster 不替代 shaping

Huozi 不能只把逐 `char` 排版改成逐 grapheme cluster 排版。Grapheme cluster 适合表达用户感知边界，可用于：

- 避免在组合字符或 emoji 序列内部断行；
- 光标、选择、删除和截断；
- source range 映射；
- 尽量以完整 grapheme cluster 进行字体 fallback。

连字、kerning、组合标记定位和字形替换仍需要 shaping。一个 grapheme cluster 可以产生多个 glyph，多个 grapheme cluster 也可以形成一个 glyph。

目标链路应从：

```text
char -> glyph
```

改为：

```text
source range
  -> shaping run
  -> shaping clusters
  -> positioned glyphs
  -> glyph-id atlas
  -> render data
```

### 4. Shaping 只在 run 内进行

连字、组合定位、kerning 和上下文字形替换只保证在同一个 shaping run 内成立，不提供跨 run shaping。

如果潜在连字或组合序列因调用者输入或必要 shaping 属性变化而跨越两个 run，两侧分别 shaping。由此缺失的跨边界连字或组合效果不视为 bug。

影响 shaping、可以形成 run 边界的属性包括：

- font instance；
- 字号、字重和斜体；
- script、language 和 direction；
- OpenType feature；
- variation axes；
- inline object、mandatory break 或显式隔离要求。

run 不得仅因内部存储、缓存分块、parser 节点、普通 grapheme 边界或逐字符图集处理而切分。如果没有必要属性变化，却由引擎拆开潜在连字或组合序列，则仍然属于实现问题。

### 5. Parser run 与 shaping run 分离

Huozi 当前 parser 产生的 `TextRun` 表达源文本和富文本样式，不直接等于 shaping run。

可能存在：

- 多个 parser run 合并为一个 shaping run，例如只有不影响 shaping 的视觉属性发生变化；
- 一个 parser run 拆分为多个 shaping run，例如 script、字体 fallback 或 mandatory break 发生变化。

如果一个 glyph 跨越多个视觉样式区间，首版允许将无法唯一确定 glyph 渲染样式的边界提升为 shaping run 边界。更复杂的多色 glyph 裁剪或重复绘制不作为默认要求。

## 字体 Fallback 约束

字体 fallback 应优先尝试为整个 grapheme cluster 找到同一字体，避免把 base character 和 combining mark 分配到不同字体。

如果没有字体覆盖完整 grapheme cluster，首版允许拆分到不同字体和 shaping run，但需要输出具名 capability issue，不能静默假定组合结果正确。

fallback 的最终决策权属于 `tiqian-rs` 还是 Huozi，仍是后续领域建模阶段的待决策事项。

## 对数据模型的影响

后续模型需要同时保留不同层次的信息：

| 层次 | 用途 |
| --- | --- |
| UTF-8 byte range | 对接 Rust 字符串、富文本 span 和 source mapping |
| Unicode scalar | 字符属性与规则分类 |
| Grapheme cluster | 用户感知边界、fallback 原子性和断行安全 |
| Shaping cluster | glyph 与源文本的映射及 shaping 结果边界 |
| Glyph | 字形测量、图集和绘制 |
| Glyph run | 一组共享字体和 shaping 属性的 glyph |

Shaping 输出至少需要携带：

- font instance identity；
- glyph id；
- source cluster mapping；
- advance；
- offset；
- run 的 script、language、direction 和 feature 信息。

Huozi 的图集与缓存入口后续应以 font instance 和 glyph id 为核心，而不是继续以 Unicode `char` 作为唯一键。绘制阶段直接重放 shaping 与 layout 给出的 placement，不再根据源字符重新选择 glyph。

## 仍待讨论

以下事项没有在本轮确定：

1. `tiqian-rs` 与 Huozi 各自负责字体 fallback 的哪一部分；
2. shaping run 的最终输入输出结构；
3. 哪些视觉样式边界必须切断 shaping；
4. emoji 是否属于首版正式支持范围；
5. CJK、彝文、假名、谚文、西里尔文分别采用哪些 layout profile；
6. 行边界确定后，哪些首版场景需要重新 shaping；
7. 首选 shaping、Unicode segmentation、font parsing 和 rasterization 依赖。

## 后续使用方式

后续讨论继续在 `docs/notes` 下按日期和主题新增纪要。纪要保留讨论当时的结论与未决问题，不直接改写为最终架构。

当积累了足够的输入后，再根据多份纪要整理阶段性草案。草案需要：

- 引用相关纪要；
- 区分已确认原则、暂定方案和待决策事项；
- 处理不同纪要之间的修订或冲突；
- 明确下一阶段的领域模型、接口候选与验收范围。
