# 段落始端、居中与末端对齐

> 状态：已完成（人工 render 观察入口已提供）
>
> 日期：2026-09-21
>
> 分类：feat

## 文档用途

本文定义 Huozi 段落对齐设置的需求、语义、公开 API、Tiqian 映射、输入路径、边界行为和验证要求。

本文是本迭代后续讨论和实施的唯一依据。已确认的目标、取舍和限制均记录在本文中，不依赖聊天记录或临时记忆。实施中若需要改变本文确定的公开语义或范围，应先讨论并更新本文，再修改代码。

## 实施结果

已实现 `ParagraphAlignment::{Start, Center, End}` 和 `LayoutStyle.align`，默认值为 `Start`。直接布局、普通文本和结构化富文本第一段均通过同一个输入适配映射到 Tiqian `LastLineAlignment`；`[br align=... /]` 会保存后续段落覆盖。输入适配器在无宽度限制或宽度转换后非有限时退化为 `Start`。

`examples/render` 的 Layout 控制区提供 `Start`、`Center`、`End` 选择。自动测试覆盖 `[br align]`、直接和结构化输入映射、覆盖优先级、显式无限宽度与无宽度限制退化、四个公开入口以及自动换行近似行为。已运行完整测试、全 target 检查、render 示例编译、文档检查和空白检查；本次未进行交互式窗口的人工视觉观察。

## 背景

Huozi 使用 Tiqian 完成段落断行、行调整和最终 glyph placement。Tiqian 当前通过 `ParagraphStyle.last_line_alignment` 提供 `Start`、`Center`、`End` 三种对齐值，但 Huozi 没有为普通布局调用公开可用的段落对齐设置：

- `LayoutStyle` 只有宽高约束、行高和首行缩进；
- `layout`、`layout_plain` 和第一段 `layout_parse*` 无法由调用方设置对齐；
- parser 已能把段落对齐保存到后续 `ParsedParagraph`，输入适配器也能转换该字段；
- 当前 `layout_parse*` 只布局第一段，因此 `[br ... /]` 保存的后续段落对齐尚不能进入公开布局结果。

本迭代接受 Tiqian `LastLineAlignment` 作为 Huozi 段落对齐的近似实现，不在 Tiqian 中新增另一套所有视觉行对齐算法。

## 已确认的语义

### 对调用方的语义

Huozi 公开一项段落 `align` 设置，取值为：

| 值 | 公开语义 |
| --- | --- |
| `Start` | 段落内容从行内始端开始。简体中文横排及当前 LTR 范围内表现为居左。 |
| `Center` | 段落内容居中。 |
| `End` | 段落内容靠行内末端。简体中文横排及当前 LTR 范围内表现为居右。 |

默认值为 `Start`，以保持现有 Huozi 输出。

公开命名使用 `Start` 与 `End`，不使用 `Left` 与 `Right`。这与 Tiqian 现有枚举一致，也避免把当前简体中文横排及 LTR 表现固化为方向无关语义。

### Tiqian 映射

Huozi 不在输出适配器中重新移动 glyph，也不维护第二套行宽或对齐计算。输入适配器将 Huozi 段落对齐一一映射为 Tiqian `ParagraphStyle.last_line_alignment`：

| Huozi | Tiqian |
| --- | --- |
| `ParagraphAlignment::Start` | `LastLineAlignment::Start` |
| `ParagraphAlignment::Center` | `LastLineAlignment::Center` |
| `ParagraphAlignment::End` | `LastLineAlignment::End` |

Tiqian 继续是行内几何的唯一真值来源。最终 `draw_x`、行缩进、字格余量和 glyph placement 均消费 Tiqian 的布局结果。

### 接受的近似行为

Tiqian 当前只对结束原因为 `MandatoryBreak` 或 `ParagraphEnd` 的行应用 `last_line_alignment`。结束原因为 `AutoWrap` 的行继续执行 Tiqian 既有的正文行调整，不应用该对齐偏移。

因此 Huozi 的公开段落对齐具有以下实际行为：

| 视觉行类型 | `Start` | `Center` | `End` |
| --- | --- | --- | --- |
| 段落结束行 | 始端对齐 | 居中 | 末端对齐 |
| Tiqian mandatory break 结束行 | 始端对齐 | 居中 | 末端对齐 |
| 自动换行产生的非末行 | 使用 Tiqian 既有正文行调整 | 使用 Tiqian 既有正文行调整 | 使用 Tiqian 既有正文行调整 |

正常连续中文正文的自动换行行通常接近可用行宽，并由 Tiqian 进行两端调整，因此这种差异通常不明显。不可分割的西文 token、URL、技术文本、行内对象或其他断行限制可能使自动换行行在行调整后仍未占满可用宽度；这类行不会像 CSS `text-align: center` 或 `text-align: end` 那样整体平移。

该差异是本迭代明确接受的产品取舍。Huozi 的 `align` 不承诺与 CSS `text-align` 完全等价，也不将“所有自动换行视觉行均居中或靠末端”列入本迭代验收条件。

### Huozi 的换行边界

Huozi 当前的 `[br /]` 表示段落分隔并创建新的 `ParsedParagraph`。同一段落内的 hard break 和 Tiqian `hard_break()` 尚未接入 Huozi 公开输入路径。

本迭代不新增 hard-break 标签或 API，不改变 `[br /]` 的段落分隔语义。上文对 Tiqian mandatory break 的说明只用于完整记录底层映射行为，不表示本迭代新增 Huozi hard-break 能力。

## 目标

1. 让调用方可以通过 `LayoutStyle` 为 `layout`、`layout_plain`、`layout_parse` 和 `layout_parse_with` 设置段落始端、居中或末端对齐。
2. 第一段结构化富文本使用 `LayoutStyle` 提供的对齐值，不再只能固定使用 Tiqian 默认 `Start`。
3. 使用 `[br align=... /]` 保存后续段落覆盖到 `ParagraphStyleOverride.last_line_alignment`，使未来多段布局消费时遵循同一对齐语义。
4. 由 Tiqian 计算最终行位置，Huozi 输出适配器直接消费最终 placement。
5. 默认配置保持当前始端对齐结果。
6. 对无有限宽度时的居中和末端对齐定义稳定的退化行为，避免产生无穷或非有限坐标。
7. 通过输入适配测试和公开布局入口测试证明设置进入 Tiqian，并改变最终 glyph 横向位置。

## 范围

### 包含

- 新增 Huozi 公开段落对齐枚举；
- 在 `LayoutStyle` 中新增段落对齐字段；
- 直接 `TextSpan`、普通文本和结构化富文本第一段的输入转换；
- 现有 `ParagraphStyleOverride.last_line_alignment` 与布局默认值的优先级；
- 有限 `box_width` 下 `Start`、`Center`、`End` 的最终横向位置；
- 无有限 `box_width` 时的退化处理；
- serde 的 camelCase 字段和值表示；
- 必要的单元测试、公开入口回归测试、示例人工验证和持续维护文档更新。

### 不包含

- 修改 Tiqian 的断行、两端调整、`LastLineAlignment` 或 line geometry 算法；
- 新增与 CSS `text-align` 完全等价的所有视觉行对齐算法；
- 让 `Center` 或 `End` 改变自动换行行的既有正文行调整；
- 新增 Huozi hard-break 标签、API 或 Tiqian `hard_break()` 接入；
- 实现 `layout_parse*` 的多段布局结果；
- 改变 `[br /]` 的段落分隔语义；
- 新增竖排、完整 bidi / RTL、分页或多栏能力；
- 重构 `LayoutStyle` 与 `ParagraphStyleOverride` 为新的嵌套样式体系；
- 在 Huozi 输出适配器或 renderer 中二次移动 glyph；
- 改变布局返回元组、glyph 顶点格式、来源映射或 SDF 渲染。

## 公开 API

### `ParagraphAlignment`

在 Huozi 布局模块公开：

```text
ParagraphAlignment
├─ Start
├─ Center
└─ End
```

类型需要满足现有公开样式使用方式，至少派生：

```text
Debug、Clone、Copy、PartialEq、Eq、Serialize、Deserialize
```

serde 使用 `camelCase` 值：

```text
start
center
end
```

`Default` 返回 `Start`。

本类型表达 Huozi 的段落设置。实现使用 Huozi 自有类型并在输入适配器中转换 Tiqian 类型，使 Huozi 的公开 API 不依赖外部 crate 的类型路径，本文记录的近似语义也属于 Huozi 自身约定。

现有 `src/parser/parsed_text.rs` 中的私有语境类型 `LastLineAlignment` 由本类型替换。`ParagraphStyleOverride.last_line_alignment` 改为 `Option<ParagraphAlignment>`，parser、布局样式和输入适配器共用这一个 Huozi 枚举。仓库内不得同时保留语义相同的 `LastLineAlignment` 与 `ParagraphAlignment`。

### `LayoutStyle.align`

`LayoutStyle` 新增：

```text
pub align: ParagraphAlignment
```

完整职责保持为：

```text
LayoutStyle
├─ box_width
├─ box_height
├─ line_height
├─ indent
└─ align
```

serde 继续使用 `#[serde(rename_all = "camelCase", default)]`，因此序列化字段名为 `align`。缺少该字段时使用 `Start`。

本迭代保持 `LayoutStyle` 的现有扁平结构。`line_height`、`indent` 已经是其中的段落默认设置，`align` 沿用这一既有边界。

### 段落覆盖优先级

结构化段落的有效值按以下顺序确定：

```text
ParagraphStyleOverride.last_line_alignment
  → LayoutStyle.align
  → ParagraphAlignment::Start
```

含义如下：

1. 当前段落存在 parser 保存的显式覆盖时，使用覆盖值；
2. 当前段落没有覆盖时，使用本次布局调用的 `LayoutStyle.align`；
3. 默认构造的 `LayoutStyle` 使用 `Start`。

`[br align=start|center|end /]` 是唯一的用户侧段落覆盖语法。parser 将其保存为内部 `ParagraphStyleOverride.last_line_alignment`；不接受 `lastLineAlignment` 或 `alignment` 作为同义属性。

当前 `layout_parse*` 只消费第一段，因此公开入口暂时只能观察 `LayoutStyle.align` 对第一段的效果。后续多段布局实施时必须按上述优先级消费已经保存的覆盖值，无需重新设计段落对齐模型。

## 输入路径

### `layout`

`HuoziTiqianInputAdapter::adapt` 当前直接构造 Tiqian `ParagraphStyle`。实施后它从 `LayoutStyle.align` 设置 `last_line_alignment`。

`layout` 从首个 run 推导基础 `TextStyle` 的现有行为不变。对齐不属于 `TextRun`，不得从首个 run 或 `TextSpan` 推导。

### `layout_plain`

`layout_plain` 继续把每个 `Segment` 转为普通 `TextSpan`，随后通过 `layout` 进入同一输入适配路径。不得为普通文本建立第二套对齐逻辑。

### `layout_parse` 与 `layout_parse_with`

`HuoziTiqianInputAdapter::adapt_paragraph` 计算当前段落的有效对齐：

```text
paragraph override 存在 → 使用 override
paragraph override 不存在 → 使用 LayoutStyle.align
```

两个 parser 入口继续共用 `layout_parsed_text` 和同一结构化输入适配路径。

### 输出适配

`HuoziTiqianOutputAdapter` 不增加对齐参数，也不根据容器宽度重新计算偏移。它继续使用 Tiqian 的 `positioned_clusters`、`LineBox.indent` 和最终 `draw_x` 生成 glyph 顶点。

## 宽度与退化行为

### 有限宽度

`Center` 与 `End` 需要有限的可用行宽作为参照。调用方设置：

```text
box_width = Some(finite_width)
```

时，输入适配器按请求把对齐值传给 Tiqian。

`line_length_grid` 当前默认启用。Tiqian 会先从容器宽度得到字格量化后的 measure，再按现有规则处理字格余量和末行对齐。Huozi 不覆盖或复制该计算。

### 无宽度限制

`LayoutStyle.box_width = None` 当前转换为 `f32::INFINITY`。无限行宽下无法定义有限的居中或末端偏移，直接向 Tiqian 传入 `Center` 或 `End` 可能产生无穷坐标。

本迭代规定：

| `box_width` | 请求对齐 | 传给 Tiqian 的值 |
| --- | --- | --- |
| 转换为 `f32` 后仍有限的宽度 | `Start`、`Center`、`End` | 原值 |
| `None` | `Start` | `Start` |
| `None` | `Center`、`End` | 退化为 `Start` |

退化时允许输出固定 warning，但不得为了日志扫描文本、收集行信息或格式化复杂上下文。布局继续产出结果，不返回错误，不 panic。

`Some` 中的异常浮点值沿用现有 `LayoutStyle` 与 Tiqian 约束处理边界；本迭代不增加一套全面运行时 validate。若现有代码或测试已经对非有限、零或负宽度作出规定，应保持既有行为。

## 布局尺寸与渲染约定

本迭代不改变 Huozi 布局返回值：

```text
(Vec<GlyphVertices>, Vec<SegmentGlyphSpan>, u32, u32)
```

返回宽度继续来自 `Tiqian LayoutResult.size.width`，不改为文字墨迹宽度，也不因居中或末端对齐单独计算包围盒。返回高度、可见行裁切、`row`、`col`、来源映射和 atlas 行为保持不变。

对齐后的 glyph 顶点位置应反映 Tiqian 最终 `draw_x`。renderer 无需知道对齐设置。

## 实施阶段

### Phase 1：公开样式模型

1. 在 `src/layout/layout_style.rs` 定义并公开 `ParagraphAlignment`。
2. 为枚举实现默认值与 serde camelCase 表示。
3. 在 `LayoutStyle` 增加 `align`，默认设为 `Start`。
4. 更新仓库内显式 `LayoutStyle` struct literal，使其继续编译；不顺带重构为 builder。

阶段产出：所有布局入口均能接收同一公开对齐值，默认行为保持不变。

### Phase 2：输入适配

1. 在 `HuoziTiqianInputAdapter::adapt` 中将布局级对齐映射到 Tiqian。
2. 在 `adapt_paragraph` 中按已确认优先级合并段落覆盖与布局默认值。
3. 对 `box_width = None` 的 `Center`、`End` 执行 `Start` 退化。
4. 将 `[br align=... /]` 解析和 `ParagraphStyleOverride` 接到公开的 `ParagraphAlignment`，不新增同义字段或枚举。

阶段产出：直接布局、普通文本和第一段结构化富文本共用一致的 Tiqian 映射。

### Phase 3：验证与文档同步

1. `ParagraphAlignment` 和 `LayoutStyle` 继续使用现有 serde 派生，不为格式序列化测试引入新 crate；默认值由 `LayoutStyle::default()` 的实现直接定义，不增加只复述字段赋值的测试。
2. 已补充输入适配测试，直接检查 Tiqian `LayoutInput.paragraph_style.last_line_alignment`。
3. 已补充公开布局入口测试，检查有限宽度下单行文字的横向位置变化。
4. 已补充结构化段落覆盖优先级与 `[br align]` 解析测试。
5. 已补充无宽度限制和显式无限宽度的稳定退化测试，确保公开顶点位置有限。
6. 已在 `examples/render` 的现有布局控制区域增加 `Start`、`Center`、`End` 选择，不改变示例渲染架构。
7. 已更新 `docs/architecture.md` 和 `docs/parser.md`，使持续维护文档描述实际完成的公开能力和近似语义。

阶段产出：自动测试覆盖 API、映射、优先级与退化行为，示例可以直接观察三种对齐结果。

## 最小必要测试

| 验证目标 | 最小证据 |
| --- | --- |
| 默认兼容 | `LayoutStyle::default()` 将 `align` 定义为 `Start`；公开入口的对齐位置回归覆盖该默认语义依赖的布局路径。 |
| serde | `ParagraphAlignment` 与 `LayoutStyle` 保持现有 camelCase serde 派生；本迭代不为格式往返测试引入额外依赖。 |
| 直接输入映射 | `adapt` 对有限宽度分别生成 Tiqian `Start`、`Center`、`End`。 |
| 结构化输入映射 | 第一段无覆盖时使用 `LayoutStyle.align`。 |
| 覆盖优先级 | 段落覆盖存在时优先于不同的 `LayoutStyle.align`。 |
| 公开入口 | `layout` 与 `layout_plain` 对同一单行文本产生一致的始端、居中、末端横向位置。 |
| parser 入口 | `layout_parse` 与 `layout_parse_with` 对等价第一段产生一致位置。 |
| 有限宽度几何 | 同一短单行文本满足 `Start.x < Center.x < End.x`，并由 Tiqian placement 产生该差异。 |
| 无限宽度退化 | `box_width = None` 且请求 `Center` 或 `End` 时按 `Start` 布局，不产生 NaN 或 infinity 顶点位置。 |
| 自动换行近似 | 至少一个会自动换行的案例记录并断言 Tiqian 现有行为：自动换行行不因 Huozi 设置而获得末行对齐偏移，段落结束行按设置对齐。该测试用于锁定本文接受的近似，不要求新增 CSS 式行为。 |

测试应优先断言 Tiqian 输入字段、`LineBox.indent`、positioned cluster 或最终公开 glyph 横向位置，不只断言 Huozi 枚举赋值。

## 验收标准

1. 调用方可以通过同一个 `LayoutStyle.align` 控制 `layout`、`layout_plain`、`layout_parse` 和 `layout_parse_with` 的第一段对齐。
2. 有限宽度下，短单行文本的 `Start`、`Center`、`End` 产生可观察且正确排序的横向位置。
3. 默认 `Start` 与当前输出一致。
4. 结构化段落覆盖优先级符合本文规定。
5. 无宽度限制时 `Center`、`End` 稳定退化，不产生非有限几何，也不中止布局。
6. Huozi 不复制 Tiqian 行几何算法，输出适配器和 renderer 不新增对齐计算。
7. 自动换行行继续保持 Tiqian 当前正文行调整；该近似行为在测试和持续维护文档中可查。
8. 所有相关测试、全 target 检查和 render 示例编译通过。

## 性能与内存

- 每个段落只进行一次枚举匹配与 Tiqian 枚举映射，时间复杂度为 $O(1)$。
- 不遍历文本、cluster、line 或 glyph 来实现对齐。
- 不新增 heap 分配、缓存或布局结果复制。
- 无限宽度退化只检查布局宽度选项和枚举值，不为 warning 收集上下文。
- renderer 与 SDF atlas 热路径不增加分支。

## 兼容性与回滚

项目处于开发阶段，不要求为 `LayoutStyle` 新字段保持 Rust struct literal 的源码兼容；仓库内调用点在本迭代中统一补齐或使用 `..Default::default()`。

serde 反序列化通过 `default` 保持缺少新字段的数据可用，默认结果为 `Start`。序列化会新增 camelCase 字段 `align`。

`[br align=... /]`、`ParagraphStyleOverride.last_line_alignment` 和 parser 文档在本迭代后保持有效。本迭代不依赖多段布局完成才能交付。

若实施中发现 Tiqian 不能在有限宽度下稳定提供本文定义的单行 `Start`、`Center`、`End` 位置，或必须在 Huozi 输出端二次计算行偏移，应停止实施并重新讨论，不得以 renderer 补丁完成迭代。

回滚时可以移除 `LayoutStyle.align` 和 Huozi 枚举映射，恢复固定 `Start`；不需要修改 Tiqian，也不应删除 parser 已有的后续段落覆盖数据。

## 预计验证

实施完成后至少运行：

```text
cargo test
cargo check --all-targets
cargo check --example render
python <work-as-a-senior-engineer>/scripts/check-doc-style.py docs/iteration/2026-09-21-feat-paragraph-alignment.md
git diff --check
```

文档检查命令中的脚本路径应替换为本机 skill 目录中的实际路径。人工验证在 `examples/render` 中使用有限 `box_width`，依次选择 `Start`、`Center`、`End`，检查短单行文字的位置以及自动换行段落的已接受近似行为。

## 相关实现位置

| 文件 | 当前职责与本迭代关系 |
| --- | --- |
| `src/layout/layout_style.rs` | `LayoutStyle` 定义；新增公开枚举与 `align` 字段。 |
| `src/layout/tiqian_input.rs` | 两条输入适配路径；映射布局默认值、段落覆盖和无限宽度退化。 |
| `src/layout.rs` | `layout`、`layout_plain`、`layout_parse*` 公开入口；保持共用输入适配。 |
| `src/parser/parsed_text.rs` | `ParagraphStyleOverride.last_line_alignment` 改用公开 `ParagraphAlignment`，移除现有重复枚举。 |
| `src/parser/elements_to_document.rs` | `[br align=... /]` 解析；写入内部段落覆盖字段。 |
| `src/layout/tiqian_output.rs` | 消费 Tiqian 最终 placement；本迭代不新增对齐计算。 |
| `examples/render/main.rs`、`examples/render/ui.rs` | 官方人工验证入口；增加对齐选择。 |
| `docs/architecture.md` | 完成后记录公开样式、输入映射和近似语义。 |
| `docs/parser.md` | 更新 `[br align=... /]` 的后续段落覆盖说明。 |
| `tiqian-rs/src/core/text_model.rs` | Tiqian `LastLineAlignment` 与 `ParagraphStyle` 定义。 |
| `tiqian-rs/src/layout/line_geometry_stage.rs` | Tiqian 根据 `LineEndReason` 应用对齐偏移的现有实现。 |
