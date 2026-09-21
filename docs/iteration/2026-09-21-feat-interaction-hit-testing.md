# 交互元素命中区域

> 状态：已完成
>
> 日期：2026-09-21
>
> 分类：feat

## 文档用途

本文定义 Huozi 为链接和行内对象输出命中区域的迭代。后续讨论、实施和验收以本文为依据；形成新结论后，应先更新本文，再据此修改代码。

本文记录当前基础、已经确认的设计、实施范围和验证要求。

## 目标

本迭代让调用方能够用一个文档坐标点找到对应的交互元素，并取得该元素的 `id`。

交互元素统一表示为：

```text
Interaction
├─ id
└─ areas
```

每个区域对应一个最终排版单元，并保存它在逐字显示中的位置：

```text
InteractionArea
├─ rect
├─ row
└─ col
```

调用方负责把鼠标或触摸位置转换到 Huozi 文档坐标，检查命中的区域，再用 `id` 查询自己的业务数据和执行相应行为。

## 当前基础

### Huozi

Huozi 当前已经能够：

- 解析 `[link target=...]...[/link]`；
- 将链接范围和 `target` 写入 Tiqian；
- 将行内对象及其尺寸写入 Tiqian；
- 从 Tiqian 最终结果取得排版单元、行信息和对象位置；
- 在 `GlyphVertices` 中保存供逐字显示使用的 `row`、`col`，但当前 `col` 仍按 glyph 递增，相关富文本输出迭代计划将其改为按 Tiqian 最终排版单元递增。

当前公开布局结果没有交互元素集合。布局完成后，链接范围和对象位置没有形成统一的命中输出。

当前 `InlineObject` 只携带 `source_range.segment_id`，该字段标识输入 segment 的来源。对象资源键由本迭代新增的独立 `id` 表达。链接当前只携带 `target`，其独立 `id` 也由本迭代新增。

### Tiqian 0.4

Huozi 当前依赖 `tiqian = "0.4"`。该版本提供：

- `positioned_clusters`：每个最终排版单元的占用矩形；
- `get_bounding_boxes`：一个文字范围覆盖的占用矩形；
- `RichTextSemantic::Link { id, target }`：链接范围、调用方标识和链接元数据；
- 行内对象的范围、最终位置和行信息。

链接在 Tiqian 0.4 中是语义数据，可以没有绘制 layer。因此链接区域不能只从 `positioned_rich_text_segments()` 读取。输出适配器应先找到带 `RichTextSemantic::Link` 的范围，再根据该范围覆盖的 `positioned_clusters` 生成区域。

命中矩形使用 Tiqian 最终的占用矩形。它反映断行、标点压缩、中西间距和行调整后的结果，并使用行盒的纵向范围。

## 已确认设计

### 1. 采用逐排版单元区域

本迭代采用方案 B：一个交互元素包含多个区域，每个区域对应一个 Tiqian 最终排版单元。

选择这一粒度是为了与逐字显示一致。调用方只使用当前已经显示到 `(row, col)` 的区域，尚未显示的文字不会提前响应命中。

一个交互元素跨行时，仍然只有一个 `Interaction`；它的 `areas` 分布在不同 `row`。同一个排版单元产生多个 glyph 时，这些 glyph 只对应一个区域。

### 2. `id` 是对外识别信息

`Interaction` 不包含 `kind`。链接和行内对象使用同一种输出结构：

```text
Interaction { id, areas }
```

调用方根据 `id` 查询链接目标、对象资源、点击行为或其他业务数据。Huozi 不根据 `id` 判断交互类型。

链接的 `target` 是链接元数据，不承担识别职责。它继续进入 Tiqian，因为 Tiqian 会在链接文字本身显示地址时应用对应的断行策略；它不进入 `Interaction`。

`id` 使用字符串。parser 中链接和行内对象的 `id` 使用 `Option<String>`，因此没有 `id` 也不影响解析和布局；公开的 `Interaction` 只表示具有非空 `id` 的交互元素，其 `id` 类型为 `String`。

`id` 不要求唯一。Huozi 不合并、去重、校验或警告重复 `id`，每个输入元素独立产生自己的 `Interaction`。

### 3. 链接和行内对象可以声明 `id`

链接语法扩展为同时声明 `id` 和现有 `target`：

```text
[link id="entry-42" target="https://example.com/42"]链接文字[/link]
```

行内对象使用其 `id`：

```text
[object id="portrait-42" alt="角色头像" width=32 ascent=26 descent=6 /]
```

声明了非空 `id` 的元素在布局后进入同一个 `interactions` 集合。缺少或为空的 `id` 不改变链接文字、链接 `target` 语义或对象占位，只是不产生对应的 `Interaction`。

行内对象绘制结果中的 `id` 使用 `Option<String>`，保证没有 `id` 的对象仍可按现有规则完成布局和输出。

### 4. 区域使用文档坐标

`InteractionArea.rect` 直接使用 `tiqian::core::geometry::Rect`，与 Huozi 绘制结果使用同一文档坐标系，并保留浮点精度。多段文档组合时，后续段落的区域使用累积后的纵向坐标和文档级 `row`。

调用方自行处理窗口位置、滚动、缩放、相机和屏幕 DPI，将输入位置转换到文档坐标后再检查区域。

### 5. 区域跟随可见布局结果

区域与正文、对象和其他布局输出使用相同的可见行筛选。受 `box_height` 限制而未输出的行不产生交互区域；跨越可见和不可见行的交互元素只保留可见行中的区域。

若一个交互元素在当前结果中没有任何可见区域，则不需要输出空的 `Interaction`。

## 目标公开结果

正在讨论的富文本布局结果应包含交互元素集合：

```text
RichTextLayoutOutput
├─ glyphs
├─ segment_glyph_spans
├─ interactions: Vec<Interaction>
├─ width
└─ height
```

`Interaction` 的确定语义是：

| 字段 | 语义 |
| --- | --- |
| `id: String` | 调用方声明的交互元素识别符。 |
| `areas: Vec<InteractionArea>` | 该元素在当前可见布局中按排版顺序排列的区域。 |

`InteractionArea` 的确定语义是：

| 字段 | 语义 |
| --- | --- |
| `rect: tiqian::core::geometry::Rect` | Tiqian 最终占用矩形，使用 Huozi 文档坐标和浮点数值。 |
| `row` | 文档级视觉行号。 |
| `col` | 当前行内按 Tiqian 最终排版单元递增的位置。 |

## 输入与输出流程

### 链接

```text
[link id=... target=...]文字[/link]
  → parser 保存 id 和 target
  → Huozi 输入适配将 id、target 和文字范围写入 Tiqian
  → Tiqian 完成 shaping、断行和行调整
  → Huozi 根据链接范围覆盖的 positioned clusters 生成 areas
  → Interaction { id, areas }
```

链接的 `id` 由 Tiqian 的链接 semantic 与 range 随 `LayoutResult.input` 保留。输出适配从该既有数据读取 `id` 和范围，再匹配最终排版单元。

### 行内对象

```text
[object id=... ... /]
  → parser 保存 id、alt 和布局尺寸
  → Huozi 输入适配将 id、对象文字范围和尺寸写入 Tiqian
  → Tiqian 给出对象最终位置和行信息
  → Huozi 生成对象对应的 area
  → Interaction { id, areas }
```

一个行内对象沿用其 positioned cluster 的 `row`、`col`，通常产生一个区域。对象绘制结果仍按富文本输出迭代的设计保存可选 `id`、`alt` 和矩形；统一交互集合让调用方通过一条路径完成链接和对象命中。

## 逐字显示规则

调用方给定当前显示进度 `(visible_row, visible_col)` 后，区域可见条件为：

```text
area.row < visible_row
或
area.row == visible_row 且 area.col <= visible_col
```

完整显示时使用全部区域。

区域的 `col` 由最终排版单元决定，即使该单元没有可绘制 glyph，也沿用富文本输出迭代最终确定的 `col` 规则。本迭代不建立第二套逐字编号。

## 命中规则

调用方使用文档坐标点检查 `InteractionArea.rect`。点与矩形的边界判断、多个命中结果的选择以及输入手势处理均由调用方决定。

本迭代只输出各交互元素对应的区域，不在 Huozi 中增加命中函数或输入状态。

## 与富文本布局输出迭代的关系

`docs/iteration/2026-09-21-feat-rich-text-layout-and-sdf-rendering.md` 定义统一富文本布局结果、逐字显示、多段组合和行内对象输出。本迭代复用其中的：

- `RichTextLayoutOutput`；
- 文档级 `row`、`col`；
- 多段文档坐标；
- 可见行和高度限制；
- 行内对象 `id`。

本迭代替换了该文档中的旧交互设计：

```text
links: Vec<LinkRegion>
LinkRegion { target, rect }
```

替换为：

```text
interactions: Vec<Interaction>
Interaction { id, areas }
```

富文本布局输出迭代已同步采用本文的 `Interaction` 设计。后续若两份文档出现偏差，应记录并报告，由对应迭代负责修正。

两份迭代的实施顺序可以结合安排，但本迭代的交互语义由本文负责；SDF 图形、背景、线条、注音、装饰和 GPU 提交仍由富文本布局输出迭代负责。

## 实施范围

### 包含

- 为 `[link]` 增加 `id`；
- 让行内对象的 `id` 进入统一交互输出；
- 定义公开 `Interaction` 和 `InteractionArea`；
- 在统一布局结果中增加 `interactions`；
- 将链接和对象 `id` 写入 Tiqian 的既有链接与对象模型；
- 从 Tiqian 最终排版单元生成链接和对象区域；
- 为区域附加文档级 `row`、`col`；
- 对多段纵向偏移和 `box_height` 可见行筛选应用相同规则；
- 更新 parser、API、架构和富文本输出相关文档；
- 增加必要的结构化测试。

### 不包含

以下能力由调用方或后续迭代负责：

- hover、按下、抬起和拖动状态；
- URL 打开、页面跳转、对象回调和业务数据表；
- 屏幕坐标到文档坐标的变换；
- 文字光标、选择、选词、复制和无障碍查询；
- 通用空间索引或超长文档虚拟化；
- 新增链接和行内对象之外的交互标签。

## 实施阶段

以下 phase 共同组成一次交付。中间 phase 允许暂时无法编译或功能尚未接通，实施时按最终结构继续推进。Phase 5 完成时必须形成可运行、可测试的结果。

### Phase 1：输入身份

目标：让链接和行内对象在 Huozi 内部保存调用方提供的可选 `id`。

1. 扩展 `InlineScopeKind::Link`，分别保存可选 `id` 和现有 `target`。
2. 扩展 `InlineObject`，保存可选 `id`。
3. 更新 parser lowering，读取链接和对象的 `id`；缺少或为空时保存为 `None`。
4. 保持链接 `target`、对象尺寸和对象占位进入 Tiqian 的现有路径不变。

阶段结束状态：parser 数据已经能够表达交互身份；下游代码可以暂时尚未适配新增字段。

### Phase 2：输出结构与范围关联

目标：建立交互输出所需的数据结构，并关联输入身份与最终布局范围。

1. 定义公开 `Interaction` 和 `InteractionArea`，矩形复用 `tiqian::core::geometry::Rect`。
2. 让 `Interaction.areas` 直接保存 `Vec<InteractionArea>`。
3. 在 `RichTextLayoutOutput` 中增加 `interactions: Vec<Interaction>`。
4. 将链接 `id` 写入 Tiqian 的 `RichTextSemantic::Link`。
5. 将对象 `id` 写入 Tiqian 的 `InlineObjectSpan`。

阶段结束状态：公开结构和内部关联方式已经确定；`interactions` 可以暂时为空，布局入口也可以暂时尚未完成全部连接。

### Phase 3：区域生成与文档组合

目标：从 Tiqian 的最终布局结果生成交互区域。

1. 复用每段已经建立的 positioned cluster 与 `row`、`col` 映射。
2. 按链接显示范围收集对应排版单元的 occupied `Rect`，生成一个链接 `Interaction` 的 `areas`。
3. 按对象对应 positioned cluster 的 occupied `Rect` 生成对象 `Interaction`。
4. 过滤没有非空 `id` 或没有可见区域的元素。
5. 将段落纵向偏移和文档级 `row` 应用于每个区域。
6. 对 `box_height` 使用与其他布局输出相同的可见行筛选。
7. 将各段生成的交互结果并入 `RichTextLayoutOutput.interactions`：同层元素保持输入顺序，嵌套元素先于其外层元素。

阶段结束状态：链接和对象的交互数据已经完整接入布局结果；测试和 demo 可以留到后续 phase 统一完成。

### Phase 4：消费示例与结构化测试

目标：证明公开结果能被直接消费，并覆盖本文规定的行为。

1. 在官方 WGPU demo 中增加交互区域可视化和点击样例。
2. 增加 parser、范围关联和输出转换的结构化测试。
3. 覆盖链接基本区域、跨行、semantic-only、多 glyph 单元、相同 `target` 和重复 `id`。
4. 覆盖对象区域、可选 `id`、逐字 `row`/`col`、多段坐标和高度限制。
5. 确认 demo 的命中选择与输入状态只存在于调用方示例中，没有进入 Huozi 布局 API。

阶段结束状态：功能具备完整消费示例和回归证据；尚未要求持续维护文档已经更新。

### WGPU demo 点击验证

`examples/render/main.rs` 的默认文本包含：

```text
[link id="demo-interaction" target="https://example.com/interaction"]点击这里显示交互提示[/link]
```

demo 在每次布局后保存 `RichTextLayoutOutput.interactions`。未被 egui 消费的鼠标左键按下事件会将物理鼠标坐标转换为窗口逻辑坐标，按 `interactions` 顺序检查 `InteractionArea.rect`；命中后由 egui 显示包含交互 `id` 的提示窗口。嵌套对象已排在外层链接之前，因此这种从前向后的首次命中策略会优先选择对象。

手工验收时运行 `cargo run --example render --release --features woff`，点击默认文本中的“点击这里显示交互提示”，应出现“已点击交互元素：demo-interaction”窗口。点击底部编辑器或控制面板不会触发 Huozi 的 demo 命中处理。

### Phase 5：文档与最终验证

目标：完成实现、文档和最终验证。

1. 更新 `docs/parser.md`、`docs/api-design.md`、`docs/architecture.md` 和 `docs/rendering-gap-analysis.md`。
2. 检查相关迭代文档与实际实现是否一致；发现其他迭代文档存在偏差时报告，不在本迭代中直接编辑。
3. 运行本文“预计验证”中的全部命令，并检查编辑器诊断。
4. 对照“最小必要测试”“性能要求”和“已确认的实施边界”复核实现。
5. 审阅目标 diff，确认没有修改 Tiqian 排版行为，也没有引入本迭代范围外的命中或输入系统。
6. 在本文记录实际实施差异和验证结果，将状态改为“已完成”。

阶段结束状态：项目可运行、全部相关测试通过、demo 可验证交互区域，持续维护文档与实现一致。

## 最小必要测试

| 验证目标 | 最小证据 |
| --- | --- |
| 链接输入 | `[link id=... target=...]` 同时保留 `id` 与 `target`；`target` 仍进入 Tiqian。 |
| 基本区域 | 一个包含多个排版单元的链接产生一个 `Interaction` 和多个 `InteractionArea`。 |
| 跨行链接 | 一个链接跨两行时仍只有一个 `Interaction`，区域分布在两个 `row`。 |
| 多 glyph 单元 | 一个排版单元产生多个 glyph 时只产生一个区域。 |
| 逐字显示 | 过滤到指定 `(row, col)` 后，后续区域不参与命中。 |
| 同 target 链接 | 两个 `id` 不同、`target` 相同的链接产生两个独立 `Interaction`。 |
| 行内对象 | 有 `id` 的对象产生带同一 `id` 的绘制结果和交互区域，并沿用其 positioned cluster 已有的 `row`、`col`。 |
| 可选 ID | 没有 `id` 或 `id` 为空的链接和对象保持原有布局结果，不产生 `Interaction`。 |
| 重复 ID | 两个使用相同 `id` 的元素各自产生独立 `Interaction`，不合并或去重。 |
| 多段坐标 | 后续段落的区域包含正确纵向偏移，`row` 不在段落边界重置。 |
| 高度限制 | 被 `box_height` 截去的行不产生区域，跨界交互只保留可见区域。 |
| Tiqian semantic-only 链接 | 没有视觉 layer 的链接仍能从 semantic range 和 positioned clusters 生成区域。 |

测试验证 Huozi 的输入保存、范围关联和输出转换，不复制 Tiqian 已有的断行与排版规则测试。

## 性能要求

1. 区域生成复用富文本输出阶段已经取得的 positioned clusters 与 `row`、`col` 映射。
2. 一个链接只扫描其显示范围覆盖的排版单元，不为每个链接从头遍历全部正文。
3. 输出内存与交互元素覆盖的排版单元数量线性相关。
4. 构造输出时不 clone 字体数据、完整 `LayoutResult` 或 parser 树。

## 缺少交互 ID

链接或对象缺少 `id`，或者 `id` 为空时，照常进入现有排版和绘制流程，不产生对应的 `Interaction`，也不为此记录 warning。链接的 `target` 仍按现有方式进入 Tiqian，行内对象仍保留布局占位。

交互输出只是现有布局结果的附加数据，不改变 Tiqian 的 shaping、断行、行调整、对象放置和 positioned cluster 生成行为。

## 兼容性与回滚

项目处于开发阶段，本迭代允许调整富文本 parser 数据和统一布局结果，不提供旧 `LinkRegion` 设计的兼容层。

回滚本迭代时，可以删除 `Interaction` 输出和链接 `id` 对应关系，而不改变 Tiqian 的文字 shaping、断行和最终 placement。行内对象的 `id` 同时服务对象资源查找，是否随交互迭代一起回滚应以富文本布局输出迭代的状态为准。

## 风险与检查点

| 风险 | 检查方式 |
| --- | --- |
| 把 `target` 当成身份 | 测试两个相同 `target`、不同 `id` 的链接。 |
| 从绘制 layer 读取纯链接 | 使用只有 semantic 的链接测试区域输出。 |
| glyph 和排版单元混淆 | 使用一个排版单元产生多个 glyph 的样例。 |
| 未显示文字提前可点击 | 按 `(row, col)` 过滤区域并验证命中。 |
| 多段坐标未累加 | 用两个段落检查第二段的 Y 坐标和 `row`。 |
| 裁剪后保留不可见区域 | 使用跨越 `box_height` 的链接和对象。 |
| 重复扫描正文 | review 输出适配算法，确认按范围窗口线性处理。 |

## 已确认的实施边界

1. `Interaction.id` 使用 `String`；parser 中链接和对象的 `id` 使用 `Option<String>`。
2. `InteractionArea.rect` 复用 `tiqian::core::geometry::Rect`。
3. `Interaction.areas` 直接保存 `Vec<InteractionArea>`。
4. 重复 `id` 不触发合并、去重、校验或 warning。
5. 缺少或为空的 `id` 不影响链接和对象的现有布局行为，只不产生交互输出。
6. `row`、`col` 完全复用富文本布局输出建立的 positioned cluster 映射，行内对象和零宽单元是否占用 `col` 均沿用该迭代已经规定的行为。
7. 区域完整使用 Tiqian positioned cluster 的 occupied `Rect`，不另行计算其他矩形，也不修改 Tiqian 几何。
8. 官方 WGPU demo 增加交互区域可视化和点击样例，用于验证公开数据可以直接消费；命中选择和手势策略仍属于 demo 与调用方逻辑。
9. `interactions` 采用深度优先的后序：调用方从前向后选择第一个命中项时，嵌套对象优先于覆盖它的外层链接。

## 文档更新

实施完成时同步更新：

| 文件 | 更新内容 |
| --- | --- |
| `docs/iteration/2026-09-21-feat-rich-text-layout-and-sdf-rendering.md` | 检查统一 `Interaction` 设计是否一致；发现偏差时报告。 |
| `docs/parser.md` | 链接和对象的 `id` 语法与错误处理。 |
| `docs/api-design.md` | `Interaction`、`InteractionArea` 和统一布局结果。 |
| `docs/architecture.md` | 交互数据路径、坐标系、逐字显示和调用方职责。 |
| `docs/rendering-gap-analysis.md` | 完成后更新链接与对象命中能力状态。 |
| 本文 | 写入实际实施差异和验证结果，并将状态改为“已完成”。 |

## 预计验证

实现完成后至少运行：

```text
cargo test
cargo check --all-targets
cargo check --all-targets --features woff
git diff --check
```

还需要：

- 检查编辑器诊断；
- 对修改的中文文档运行项目采用的文档措辞检查；
- 审阅目标 diff，确认没有修改 Tiqian 排版规则或扩大到文字选择能力。

## 实施结果

- 链接和行内对象的 `id` 已由 Tiqian 的既有链接语义和对象模型保存；
- Huozi 输出适配器从 `LayoutResult.input` 读取标识和范围，并匹配最终排版单元生成区域；
- 同层元素按输入顺序输出，嵌套对象位于外层链接之前；
- 已运行 `cargo test`、`cargo check --all-targets`、`cargo check --all-targets --features woff` 与 `git diff --check`；
- Huozi 测试全部通过，其中布局入口测试共 9 项；当前文档风格检查无命中。

## 相关文件

| 文件 | 本迭代职责 |
| --- | --- |
| `src/parser/parsed_text.rs` | 保存链接和对象的 `id`。 |
| `src/parser/elements_to_document.rs` | 读取链接和对象的 `id`。 |
| `src/layout/tiqian_input.rs` | 将交互 `id` 和范围写入 Tiqian 的既有链接与对象模型。 |
| `src/layout/tiqian_output.rs` | 从最终排版单元和对象位置生成交互区域。 |
| `src/layout.rs` | 将交互集合并入统一布局结果，并处理多段坐标。 |
| `src/layout/layout_output.rs` | 定义完整 `RichTextLayoutOutput`、`Interaction` 和 `InteractionArea`，并复用 Tiqian `Rect`。 |
| 富文本输出与持续维护文档 | 保持公开 API、架构和能力状态与实现一致。 |
