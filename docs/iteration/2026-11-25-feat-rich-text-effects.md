# 富文本内置动效

> 状态：已完成
>
> 日期：2026-11-25
>
> 分类：feat

## 文档用途

本文既记录本迭代的方案（为什么这样做、契约是什么），也记录实施结果（代码落在哪里、验证到什么程度）。后续讨论、验收与回滚以本文为依据；改变公开语义或范围前先更新本文，再改代码。

本文只涉及 Huozi 引擎与 `examples/render`。调用方（Moyu / Unity / Godot）如何驱动时间轴、如何在引擎里重建顶点不在本文范围内。

## 实施结果

本迭代在 main（`Release huozi v0.27.0`）之上新增两块能力：**逐元素来源数据**（渲染方按字符推进与求值的基础）与**内置动效**（标签 + 无分配外观求值）。

净变化（相对 main）：

| 文件 | 内容 |
| --- | --- |
| `src/effects.rs` | 新增。`Effect` / `EffectContext` / `GlyphAppearance` 与纯外观求值 |
| `src/layout/layout_output.rs` | `RichTextLayoutOutput` 新增 `clusters` / `text_length`；新增 `GlyphCluster { source, glyph_range, element_sources, effects }` |
| `src/layout.rs` | `DocumentBuilder` 跨段落累积并重映射 `clusters` 与 `text_length` |
| `src/layout/tiqian_input.rs` | 来源区间携带 `effects`，新增 `HuoziSourceMap::entry_at` |
| `src/layout/tiqian_output.rs` | 生成簇、回填 `glyph_range` 与逐元素来源；效果颜色按输出色彩空间换算；注音段落 `y_offset` 修正 |
| `src/parser/text_style.rs` | `TextStyle` 新增 `effects: Vec<Effect>` |
| `src/parser/elements_to_document.rs` | 效果标签注册、属性解析、`set_effect` / `set_glow_color` |
| `examples/render/*` | 逐帧外观重建、按元素级来源推进进度、动画帧节流、减少动态效果开关 |
| `examples/layout_flamegraph.rs` | 基准文本换成含效果与新标签的内容 |
| `tests/line_break.rs` | 新增。断行粒度回归（拉丁按词、CJK 按字） |
| `tests/glyph_effects.rs`、`tests/rich_text_effects.rs` | 新增（13 / 6 项） |
| `tests/rich_text_output.rs` | 新增 2 项注音几何用例 |
| `docs/architecture.md`、`docs/parser.md` | 输出契约与标签文档同步 |

### 已决策

1. **排版簇粒度保持 Tiqian 的语义：一次 shaping 请求一个簇。** 动效需要的是"每个绘制元素覆盖哪些字符"，不是"把簇拆细"。拆簇会让 Tiqian 把词内每个字母边界都当成断行机会，见「被否决的方案」。
2. 动效求值留在 Huozi 内部（`Effect::apply`），但**只改外观**：不改进宽、不建布局、不动图集与交互矩形。
3. 效果按簇归属，不按字形归属；跨标签的连字与落在词内部的标签都采用**首个显示 scalar** 的状态。
4. `RichTextLayoutOutput` 只增加数据（`clusters`、`text_length`、`element_sources`），不提供查询方法（不实现 `caret_at` / `selection_rects` 之类）：插入点、选区矩形、行导航属于调用方。
5. `cluster.source` / `element_sources` / `text_length` 使用送入排版的**连续显示文本 scalar 坐标**，包含对象占位、按段落累积，不与含标签的 `SourceRange` 互换。
6. 效果开关用布尔值而不是枚举取值，因此与其他枚举取值统一为 kebab-case 的解析路径无交。
7. `examples/render` 只从缓存顶点重建外观，效果与逐字进度都不触发重新排版。

## 背景

Huozi 的产出是绘制顶点。调用方做逐字显示、打字机效果、抖动标题这类"按时间改外观"的事时，只有两个选择：重新排版，或者自己解析文本再猜每个字形属于哪段样式。前者代价过高（每次排版都要重新 shape），后者需要重复实现标签解析与簇切分。

同时 `RichTextLayoutOutput` 只给了 `glyphs`，渲染方无从知道某个绘制元素覆盖了哪几个输入字符。而 Tiqian 的簇粒度是**排版原子**：CJK 一字一簇，拉丁整词一簇，连字与组合序列整簇。所以"按字符推进"和"按字符求值动效"都缺数据。

本迭代补的就是这份数据，并在其上实现动效。

## 被否决的方案：把排版簇拆到 HarfRust 粒度

早期实现让 shaper 按 HarfRust 的真正簇输出（`ffi` 一簇、`hello` 五簇），`glyph.cluster_range` 与 `glyph.x` 都随之改成簇内坐标。它是可行的，但会改变排版行为，因此被否决。

依据（`tiqian-0.9.0`）：

- **簇边界就是断行候选**：`LineMeasurement::boundary_allowed` 只排除 hidden / hard / Preserve 模式空格序列，其余簇边界一律允许断行。
- Tiqian 自己把「切分单位」准备好再交给 shaper：`shape_paragraph` 按 `ResolvedClusterRange` 逐段调用 backend（CJK 一字、拉丁一词、空白单元、连字分段），并要求 shaper 原样回显；它自己的三个 shaper 实现（stub、测试 backend、空白 ToSpace 路径）全部返回恰好一个簇。
- Tiqian 已经为「簇内含多个源单位」准备了通道：`PositionedCluster.source_stops` 在 `range.length() > 1 && 字形数 == length`（注释写明的 typical Latin）时给出簇内每个 scalar 的 x，`x_for_offset` / `offset_for_x` 用它做簇内映射。拆簇会让这个条件永不成立。

实测差异（同一字体 `SourceHanSansSC-VF`、默认样式；对照组为 main）：

| 文本 @ 宽度 | 保持单簇（main） | 拆到字形簇 |
| --- | --- | --- |
| `hello world typography` @200 | 10 / 10 行 | 11 / 9（`hello world t` + `ypography`） |
| `aaaa bbbbbbbbbbbb ccc` @260 | 4 / 12 / 3 | 12 / 7（词中断开，无连字符） |
| `internationalization` @100 | 6/3/7/4/4 行，24 字形（含 4 个行尾连字符） | 6/6/7/1 行，20 字形（无连字符） |
| `中文排版测试换行` @100 | 3 / 3 / 2 | 3 / 3 / 2（不受影响） |

拉丁文会在词中间被断开且不插入连字符，这是排版回归而不是风格取舍。`tests/line_break.rs` 把上表钉成回归测试。

## 方案总览

```text
富文本标记
  → parser：TextStyle.effects: Vec<Effect>（与颜色、描边同一条样式继承路径）
  → tiqian_input：每个来源区间（run）连同样式打包成 HuoziSourceMapEntry { display_range, source_range, effects }
  → tiqian：照常排版，标签不产生簇边界
  → font_backend：一次 shaping 请求一个排版簇（覆盖 request.range），标签不切分
  → tiqian_output：每个 positioned cluster 输出 GlyphCluster { source, glyph_range, element_sources, effects }
  → layout：跨段落重映射 source / glyph_range，累积 text_length
  → 调用方：按元素级 source 推进进度与求值外观
```

三层分离：

| 层 | 概念 | 职责 |
| --- | --- | --- |
| 排版原子 | Tiqian cluster | 断行、度量、注音锚定、连字点、行结构 |
| 渲染原子 | 绘制元素 | 顶点、逐字进度、动效外观 |
| 文本坐标 | scalar 范围 | `cluster.source`、`element_sources`、`text_length` |

## 数据模型

### `RichTextLayoutOutput`

```rust
pub struct RichTextLayoutOutput {
    pub glyphs: Vec<UnitVertices>,
    /// Tiqian 排版簇：一次 shaping 请求一个，供逐字显示与动效消费。
    pub clusters: Vec<GlyphCluster>,
    /// 已送入排版的正文 scalar 总数；不含标签，包含对象占位，不重复计算注音。
    pub text_length: usize,
    pub segment_glyph_spans: Vec<SegmentGlyphSpan>,
    pub interactions: Vec<Interaction>,
    pub width: u32,
    pub height: u32,
}

pub struct GlyphCluster {
    /// 簇的 scalar 半开范围（文档连续显示文本坐标）。
    pub source: Range<usize>,
    /// `glyphs` 中的半开范围；空格等可没有绘制元素。
    pub glyph_range: Range<usize>,
    /// 与 `glyph_range` 等长的逐元素 scalar 子范围；`None` 表示簇不可再分。
    pub element_sources: Option<Arc<[Range<usize>]>>,
    /// 按声明顺序组合的绘制效果；跨标签的连字采用首 scalar 的状态。
    pub effects: Arc<[Effect]>,
}
```

契约：

- `clusters` 按 `(row, col)` 递增，`glyph_range` 首尾相接且覆盖 `glyphs` 的全部下标（`adapt` 末尾有 `debug_assert` 校验）；
- `glyph_range` 可能为空：空格有进宽但不产生绘制元素；被高度裁剪的行不产生 positioned cluster，因此不出现空簇；
- `element_sources` 只在簇内有多个元素时存在；正文元素按 scalar 推进，背景 / 线条 / 装饰 / 注音 / 行内对象不承载逐字进度，统一使用整个簇范围；
- 正文元素数与簇的 scalar 数相等时按一一对应分配（对应 Tiqian `source_stops` 成立的情形）；否则按比例切分，与 Tiqian 在缺少 `source_stops` 时对簇内偏移线性插值的做法一致；
- `text_length` 只统计**实际送入排版**的 scalar：剩余高度为 0 的段落不会进入布局，因此不计入；已送入但整段被裁掉的段落仍计入。

### `TextStyle.effects`

`Vec<Effect>` 与 `fill_color`、`stroke` 走同一条样式继承路径：作用域进入时被修改，离开时随样式副本回退。

### `Effect` / `EffectContext` / `GlyphAppearance`

```rust
pub enum Effect {
    Cluster,
    Wave,
    Shake,
    /// 目标 RGBA 颜色；alpha 控制混色强度，不改变字形透明度。
    Glow { color: [f32; 4] },
    Glitch,
}

pub struct EffectContext {
    /// 元素覆盖的 scalar 半开范围；起点决定动画相位。
    pub source: Range<usize>,
    pub elapsed_seconds: f64,
    /// `None` 显示全文；`Some(n)` 只显示前 `n` 个 scalar 覆盖的完整元素。
    pub visible_characters: Option<usize>,
    /// 停止运动与颜色脉动，保留静态 glow 颜色。
    pub reduce_motion: bool,
}

pub struct GlyphAppearance {
    pub offset: [f32; 2],
    pub color: [f32; 4],
    pub visible: bool,
}
```

三者都是 `Copy` 级别的小结构，求值不分配内存。

## 标签语法

| 标签 | 语义 | 写法 |
| --- | --- | --- |
| `cluster` | 语义标记：该范围按整簇显示；本身不移动不变色 | `[cluster]…[/cluster]` |
| `wave` | 纵向正弦浮动，固定 2px 幅度 | `[wave]`、`[wave=false]` |
| `shake` | 12Hz 抖动的整像素位移 | `[shake]`、`[shake=false]` |
| `glow` | 填充色 RGB 脉动混色 | `[glow]`、`[glow=false]`、`[glow=red]`、`[glow color="#FFD580"]` |
| `glitch` | 偶发的水平跳变与冷色混色 | `[glitch]`、`[glitch=false]` |

解析规则：

- 无属性的效果标签名 = 开启。`apply_text_attributes` 在属性为空时先看标签名是否为效果名；
- 属性名为效果名时按布尔解析：`=true` 开启，`=false` 关闭；`glow` 的值不是布尔时按 CSS 颜色解析（`[glow=red]`）；
- `glow` 额外接受 `color=` 属性；
- `span` 与其他文字样式标签同样可以带效果属性：`[span wave=true glow=red]`；
- 效果列表按**首次启用顺序**排列，重复启用不会产生第二项（用 `std::mem::discriminant` 判定同类），关闭时移除同类项；
- 非法布尔值与非法的 glow 颜色只输出固定文本 `warn`，保留当前状态，不中断解析；
- 效果标签不写入 `display_text`、不生成 `content.spans`、不增加 `source_boundaries`：`f[wave]fi[/wave]` 的输入仍是 `ffi` 且只有一个富文本段。

```text
[wave]波动[wave=false]静止[/wave]恢复波动[/wave]
[shake][glow color="rgba(255, 128, 32, 0.5)"]抖动与颜色脉动[/glow][/shake]
[stroke color=white width=2][glitch]描边随文字移动[/glitch][/stroke]
```

## 效果语义

设 `phase = source.start * 1.7`、`clock = elapsed_seconds`。相位只取决于元素的起始 scalar。

| 效果 | 未启用 `reduce_motion` | 启用 `reduce_motion` |
| --- | --- | --- |
| `Cluster` | 不动 | 不动 |
| `Wave` | `offset.y += round(sin(clock*3 + phase) * 2)` | 不动 |
| `Shake` | `tick = floor(clock*12)`，`offset.x += round(sin(tick*7 + phase))`，`offset.y += round(sin(tick*11 + phase))` | 不动 |
| `Glow` | `emphasis = 0.9 + 0.1*sin(clock*2 + phase)` | `emphasis = 0.9` |
| `Glitch` | `tick = floor(clock*8)`；当 `sin(tick*13 + phase) > 0.94` 时 `offset.x += ±2` 并把 RGB 向 `[0.55, 0.95, 1.0]` 混 0.6 | 不动 |

统一规则：

- 逐字进度不足时（`visible_characters` 存在且 `source.end > visible`）直接 `visible = false` 并返回，不改任何绘制属性；
- `Glow` 的混色权重是 `emphasis * 0.4 * color[3]`，只混 RGB，alpha 始终等于原填充色；
- 多个效果按 `Vec` 顺序叠加：位移相加，RGB 依次混色；
- `Shake` / `Wave` 的位移取整到像素，避开亚像素位移带来的 SDF 边缘糊化；
- 求值全部是整数运算与 `sin`，不分配、不查表、不持有时钟。

## 输出侧接线

### 效果快照按样式区间缓存

`adapt` 先把 `source_map.entries` 一次性转换成 `Vec<Arc<[Effect]>>`：

- 不需要换算时直接 `Arc::clone(&entry.effects)`，同一 run 的所有簇共享一份；
- 需要换算时（`ColorSpace::Linear` 且含 `Glow`）为每个 entry 生成一份转换后的 `Arc`，因此颜色换算次数是 **run 数**而不是簇数。

遍历 positioned cluster 时用单调游标（`while entry.display_range.end() <= cluster.start { cursor += 1 }`）取簇首 scalar 所属 entry，复杂度 O(簇数 + run 数)。

### 元素来源回填

`adapt` 在元素全部追加完成、`sort_into_display_order` 排好逐字顺序之后，用一次线性扫描按 `(row, col)` 回填每个簇的 `glyph_range`，同时按元素算 `element_sources`（正文元素按位置切分，非正文元素用整簇范围；元素不足两个或没有正文字形时返回 `None`）。`debug_assert` 校验游标正好走完 `glyphs`。

### 色彩空间

`Glow` 的目标颜色按输出色彩空间换算：`ColorSpace::Linear` 下转成线性值，`ColorSpace::SRGB` 保持原值。

### 逐字显示

进度单位是输入 scalar 数，判定单位是**元素**：`element_source(i).end <= visible` 的元素整体显示。`[cluster]` 标记的簇改用簇范围判定，因此整簇同时出现。连字与组合序列在簇内只有一个元素，天然整簇显示。

## 渲染方契约

调用方需要：

1. 用 `clusters` 分组、按元素遍历（`glyphs[cluster.glyph_range]` 与 `element_source(i)` 一一对应）；
2. 每个元素求值一次外观，只作用于该元素的**文字**顶点；
3. 背景、线条、装饰、行内对象按簇归属但不参与位移与混色；
4. 可见性按元素级 `source` 判定；`visible = false` 时该元素不提交顶点；
5. 只有「当前可见 + 含会动的效果（`Cluster` 除外、alpha 为 0 的 `Glow` 除外）+ 含文字元素」时才需要持续推进帧。

`examples/render` 是参考实现：

- `assemble` 外层遍历绘制层、内层遍历簇内元素，按元素级 source 判定可见并构造 `EffectContext`；文字元素的顶点统一加偏移，只有 `TextFill` 层使用混色后的颜色；
- `has_animated_effects` 过滤 `reduce_motion`、`Effect::Cluster`、alpha 为 0 的 `Glow` 与无文字元素的簇；
- 帧节奏：有动画时最多每 `16ms` 请求一帧，用 `ControlFlow::WaitUntil`；`WindowEvent::RedrawRequested` 不再触发新的重绘请求（egui 对该事件也返回 `repaint`，否则会自旋）；
- 控制面板新增「减少动态效果」复选框，与逐字进度共用 `appearance_changed` 路径，只重建顶点缓冲、不重新排版；
- 进度滑条的值是 `text_length`。

## 性能与内存

| 关注点 | 处理 |
| --- | --- |
| 效果列表分配 | `Arc<[Effect]>` 按 run 共享，簇只克隆引用 |
| 颜色换算 | 每个 run 一次，不是每个簇一次 |
| 元素来源 | 只在多元素簇上分配；单元素簇（CJK 常态）为 `None` |
| 簇查找 / 回填 | 单调游标 + 一次线性扫描，无额外排序 |
| 每帧求值 | 只写顶点副本，不重新 shaping、不重新生成 SDF |
| 空闲时 | 无动画帧请求，回到 `ControlFlow::Wait` |

## 非目标

- 不提供光标 / 选区查询方法；
- 不做 UAX #29 字素簇切分：`clusters` 是 Tiqian 排版簇，`element_sources` 是 scalar 子范围，都不承担字素语义；
- 不提供可配置的幅度、频率、速度或曲线；
- 不改 `TextVertices.row` / `col` 的既有语义，`col` 仍是 positioned cluster（= 排版簇）序号；
- 不保留 `LayoutResult`，不做跨布局缓存；
- 不涉及调用方的节点 props 与时间轴。

## 验证

自动化（`cargo test` 全绿）：

| 文件 / 用例 | 覆盖 |
| --- | --- |
| `tests/line_break.rs`（4 项） | 拉丁按词换行、长词整行下移、窄行下走连字符、CJK 按字换行 |
| `tests/glyph_effects.rs`（13 项） | 名称解析、未完整显示不泄露、alpha 保持、wave 幅度、shake 12Hz、glitch 偶发、glow 只混 RGB、alpha 作为混色权重、`reduce_motion`、嵌套叠加、相位只依赖起点、重复求值确定性 |
| `tests/rich_text_effects.rs`（6 项） | 簇粒度与元素来源、标签落在连字内部不改变 shaping 与 segment、跨段落与行内对象的 `text_length`、簇元素集合含背景 / 线条 / 注音、高度裁剪、`glow` 颜色随色彩空间 |
| `tests/rich_text_output.rs`（新增 2 项） | 注音跨簇与组合序列的间距、注音随基文在后续段落一起下移 |
| `src/parser/elements_to_document.rs`（新增 6 项） | glow 颜色覆盖与恢复、span 属性形式、描边别名不受影响、开关与作用域恢复、顺序与去重、非法布尔保留状态 |
| `src/layout/tiqian_input.rs`（新增 2 项） | 效果标签不新增 Tiqian 边界、直接 run 的效果元数据与空区间跳过 |
| `src/huozi.rs` | 一次 shaping 请求一个簇、同簇多个字形共享 `col` |
| `examples/render/main.rs`（6 项） | scalar 进度整簇显示、词内逐元素推进、动效移动文字三层且不改缓存布局 / alpha、背景装饰不动、只有可见会动效果才请求帧、透明 glow 不请求帧 |

## 已知限制

1. `reduce_motion` 只停止运动，不做额外的可访问性降级。
2. 帧节流固定 16ms，未与显示刷新率对齐。
3. 落在同一个拉丁词内部的多个效果标签共享首个字符的状态（例如 `f[wave]fi[/wave]` 整词波动）；要单独处理只能自行切分文本。
4. 含连字的拉丁词（如 `office`）簇内字形数与 scalar 数不等，`element_sources` 使用比例切分，逐字进度是近似值。
5. 显示替换（`display_text` 与源文本不一致）时簇内无法按字符细分，动效相位退化为按请求起点计算。
6. `Effect::Cluster` 目前不改外观，只是语义标记：整簇显示规则由渲染方按该效果实现。

## 回滚方案

改动集中在 `src/effects.rs`、`src/layout*.rs`、`src/parser/text_style.rs`、`src/parser/elements_to_document.rs`、`examples/render` 与 `tests/`。回滚即删除 `effects` 模块、移除 `RichTextLayoutOutput` 的 `clusters` / `text_length` 与 `GlyphCluster`、`TextStyle.effects`，并把逐字进度改回 `glyphs` 下标。不涉及数据迁移或持久化格式。
