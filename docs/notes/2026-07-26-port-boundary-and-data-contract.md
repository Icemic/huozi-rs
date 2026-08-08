# Tiqian-rs 移植边界与数据契约讨论纪要

- 日期：2026-07-26
- 状态：已确认，等待后续阶段性草案汇总
- 关联：[整合 Roadmap](../roadmap.md) · [Tiqian 当前内部结构](../references/tiqian-current-structure.md) · [范围与 Shaping 边界纪要](2026-07-26-scope-and-shaping-boundaries.md)
- 修订说明：本纪要将此前“首阶段”“首版”的表述修订为“一次完整移植、内部可分阶段实施”。

## 会议议题

本次会议围绕以下问题展开：

1. Tiqian 的哪些部分移植到 `tiqian-rs`，Huozi 与 `tiqian-rs` 如何组合；
2. 两者交换的文本索引、grapheme、shaping cluster 与 glyph 数据如何表达；
3. 字体、fallback、shaping 和 metrics 如何由 Huozi 提供给 `tiqian-rs`；
4. `LayoutInput`、`LayoutResult`、font identity 和移植实施方式如何确定。

讨论确认了总体边界和核心语义。具体 Rust 类型、trait 方法签名、模块划分和依赖库留待后续草案定义。

## 最终决策

### 1. `tiqian-rs` 的职责边界

`tiqian-rs` 负责移植 Tiqian 中以下连续的排版部分：

```text
line-break opportunities + CLREQ profile
  -> paragraph layout / repair / adjustment
  -> LayoutResult + LayoutDebugInfo
```

职责包括：

- 断行机会和强制换行；
- CJK/CLREQ profile、标点分类和 display substitution policy；
- 标点 body、ink、glue 与行边处理；
- paragraph layout、禁则和断行修复；
- compression、justification 与其他行调整；
- line box、cluster 和最终 glyph placement；
- decoration、ruby、inline object 等布局及几何；
- 结构化 layout decisions 与 capability issues。

Huozi 负责该部分前后的能力：

```text
Huozi
  source / parser / style preparation
  grapheme segmentation
  font resources and fallback implementation
  shaping and raw font metrics
       ↓
tiqian-rs
  line breaking / CLREQ / layout / repair / adjustment
  LayoutResult / LayoutDebugInfo
       ↓
Huozi
  glyph rasterization
  SDF atlas
  render batches
  vertex/index generation
  WGPU or other renderer
```

`tiqian-rs` 不依赖 Huozi 的 parser、SDF、WGPU 或顶点类型。Huozi 不在 renderer 中重新实现断行、标点空间或行调整。

### 2. Shaping 由 Huozi 实现，由 `tiqian-rs` 按需调用

字体文件、字体实例和 shaping 实现属于 Huozi，但 shaping 不是进入 `tiqian-rs` 前只执行一次的固定预处理。

`tiqian-rs` 在布局过程中可以请求 Huozi 对指定 run 或 display text 执行 shaping。需要按需调用的原因包括：

- CLREQ display substitution 可能改变待 shaping 文本；
- 替代码点缺字时需要回滚 source text 并重新 shaping；
- 西文断词可能插入可见连字符；
- 行边界或 OpenType feature 可能要求重新 shaping 某个片段。

本次仍沿用此前确认的规则：shaping 只保证在 run 内成立，不提供跨 run shaping。

### 3. `tiqian-rs` 提供后端 trait 和默认 layout engine

`tiqian-rs` 定义平台无关的字体与 shaping 后端 trait，由 Huozi 提供 production 实现。trait 至少覆盖以下能力：

- font resolution / fallback evidence；
- text shaping；
- raw font metrics。

`tiqian-rs` 同时提供完整的默认 layout engine 和确定性的测试后端。测试后端用于 Kotlin/Rust fixture 与 golden 对照，不伪装成 production 字体实现。

对外使用方式原则上是：

```text
LayoutEngine
  + LayoutInput
  + Huozi backend implementation
  -> LayoutResult
```

具体采用每次传入 backend、engine 持有 backend，还是 layout session 形式，留待草案阶段确定。

### 4. 文本索引统一使用 UTF-8 byte offset

所有 source text 范围统一使用 UTF-8 byte offset。范围端点必须位于有效 UTF-8 boundary。

需要区分三个索引空间：

| 类型 | 索引空间 | 含义 |
| --- | --- | --- |
| source range | UTF-8 byte offset | 原始文本、style、annotation 和复制语义 |
| grapheme range | grapheme table index | 某结构覆盖输入 grapheme table 中的条目 |
| glyph range | glyph array index | 某 shaping/layout cluster 对应 glyph 数组中的条目 |

Rust 中应使用不同的 range 类型或 newtype，避免三个索引空间都以裸 `Range<usize>` 传递而被误用。

### 5. Grapheme、shaping cluster 与 glyph 分离

输入保留 grapheme table，每个 grapheme 至少包含 source range。

Shaping cluster 用于连接 source、grapheme 和 glyph：

```text
ShapingCluster
  source_range
  grapheme_range
  glyph_range
```

这一结构允许表达：

- 多个 grapheme 形成一个连字 glyph；
- 一个 grapheme 产生 base 与 mark 等多个 glyph；
- 多个 grapheme 对应多个 glyph。

`glyph_range` 只表示 glyph 数组中的索引区间，不包含字体内部的 glyph id。glyph id 属于区间内每个 glyph：

```text
Glyph
  glyph_id
  advance
  offset
  ink_bounds
```

一个 glyph 的 source 映射可以通过其所属 shaping cluster 查询，不要求在每个 glyph 上重复保存完整 source range。

### 6. Font manager 属于 Huozi，跨边界传递不透明 `FontId`

Huozi 后续建立专门的字体管理器，负责：

- 字体文件与 face；
- fallback 配置；
- shaping 和 metrics cache；
- glyph rasterization；
- glyph atlas lookup。

`tiqian-rs` 只持有和传递不透明 `FontId`，不解释其内部值。`FontId` 应能让 Huozi 在 fallback、shaping、metrics、layout result、rasterization 和 atlas 阶段找到同一字体实例。

`FontId` 的具体构成、字号是否属于 identity、生命周期和跨段稳定性尚未确定，不阻塞独立移植工作。

### 7. 移植阶段先确定一版完整的 `LayoutInput` 和 `LayoutResult`

移植开始前先确定一版能够承载 Tiqian 当前完整排版语义的 `LayoutInput` 和 `LayoutResult`。这里的“一版”是移植期间使用的完整契约工作版本，不是按功能拆分的首版交付；接入 Huozi 时可以根据实际使用调整物理结构。

`LayoutInput` 至少需要表达：

- source text 与 UTF-8 ranges；
- grapheme table；
- text/style spans 与 source boundaries；
- paragraph style 和 writing mode；
- constraints 和 layout profile；
- decorations、ruby、inline boxes 与 inline objects；
- 影响断行和精确几何的语义边界。

`LayoutResult` 至少需要表达：

- layout clusters 与 shaping clusters；
- glyph runs、glyph id、advance、offset 和 ink bounds；
- lines、line boxes 和最终 placement；
- source/display text 的关系；
- annotation 与 rich-text 查询所需几何；
- 完整结构化 `LayoutDebugInfo` 和 capability issues。

结果应允许 Huozi 直接取得最终 positioned glyph，再以 `FontId + glyph_id` 读取字形、生成 SDF 和构造顶点。是否保存冗余的最终 glyph 坐标，或通过 iterator 从 cluster pen 与 glyph offset 计算，留待草案阶段确定。

### 8. 移植是一个完整交付，不区分“首版”

`tiqian-rs` 的 port 对外是一次完整移植，不以“首版”“后续补齐”的方式拆分功能。

移植过程可以在内部按数据模型、规则、断行、调整、几何、诊断和测试等部分分阶段实施，但这些阶段：

- 不构成独立对外交付版本；
- 不与 Huozi 字体管理、production shaping、SDF 或渲染接入等其他迭代交叉；
- 只有整个 port 满足验收条件后，才进入 Huozi 接入阶段。

### 9. Port 期间保留 Tiqian 已有能力与扩展点

移植期间尽量保持被移植部分与 Tiqian 当前实现一致。保持一致指领域语义、排版行为、策略、扩展点、诊断和测试证据，不要求复制 Kotlin API 或模块结构。

包括 `writingMode` 在内的已有字段和扩展点应保留。若 Tiqian 当前只有模型扩展点、尚未完成对应行为，`tiqian-rs` 保持同等的显式未实现状态，不在 port 中删除，也不借移植新增原项目没有的完整能力。

以下工作必须放在完整 port 之后单独决策：

- 删除 Huozi 暂时不用的字段或能力；
- 合并模块或精简 API；
- 有意改变 Tiqian 的排版取舍；
- 将完整 debug 数据改为可选或按 feature 裁剪。

## 实施与验收关系

整体顺序确定为：

```text
完整 tiqian-rs port
  -> 与 Kotlin Tiqian 对照验收
  -> Huozi FontManager / production backend
  -> Huozi LayoutInput lowering
  -> Huozi LayoutResult / SDF / renderer 接入
  -> port 后的独立精简与重构
```

Port 完成前，不进入 Huozi production 接入。Port 的内部实施阶段只服务于最终完整交付。

验收需要覆盖：

- 被移植类型和策略的 Rust 对应；
- 当前相关算法和结构化 decisions；
- fixture、golden 与规范化 dump 对照；
- source/display、cluster、glyph、line 和 annotation 几何；
- 已有扩展点及未实现能力的显式状态；
- 独立构建、测试和发布；
- 用于对照的 Kotlin Tiqian 参考 commit。

## 后续仍需形成草案的事项

本纪要确认方向，不给出最终接口。后续阶段性草案需要定义：

1. source、grapheme、shaping cluster、layout cluster 和 glyph 的具体 Rust 类型；
2. backend trait 的方法、错误与 capability issue；
3. `FontId` 的身份、生命周期和管理器边界；
4. `LayoutInput`、`LayoutResult` 与 positioned-glyph 查询接口；
5. 完整 port 的内部实施阶段、参考 commit 和逐项验收矩阵。
