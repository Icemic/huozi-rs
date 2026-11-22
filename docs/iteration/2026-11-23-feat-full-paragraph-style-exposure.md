# 段落样式字段完整暴露

> 状态：已完成
>
> 日期：2026-11-23
>
> 分类：feat

## 文档用途

本文定义 Huozi 把 Tiqian `ParagraphStyle` 中尚未暴露的字段补齐到 `LayoutStyle` 与 `ParagraphStyleOverride` 的方案。后续实施与验收以本文为准；实施中若需改变公开语义或范围，先更新本文再改代码。

本文只涉及 Huozi。Moyu `text` 节点的 props 接入另行讨论。

## 实施结果

`LayoutStyle` 新增 `first_line_indent_policy`、`block_indent`、`writing_mode`、`line_length_grid`、`ruby_line_height_mode`、`inline_object_minimum_clearance`、`emphasis_dot_gap`，`indent` 由 `f64` 改为 `FirstLineIndent`。`ParagraphStyleOverride` 同步新增对应覆盖字段，`line_length_grid_body_alignment` 支持 `Some(None)` 重置。四条布局入口共用 `adapt` / `paragraph_style` 两条映射路径，已写入 Tiqian `ParagraphStyle` 的全部 13 个字段（`writing_mode` 仅透传）。

`[br /]` 新增 `firstLineIndentPolicy`、`lineLengthGridBodyAlignment`、`writingMode`，`indent` 接受数值或 `auto`。`RubyLineHeightMode` 从 `parser` 迁移到 `layout`，`parser` 继续 `pub use`。

新增 `tests/paragraph_style_config.rs` 共 5 项，覆盖 `block_indent` 逐行位移、固定/自适应缩进、`lineLengthGridBodyAlignment` 对量化余量的影响、`[br]` 覆盖与重置；`tiqian_input.rs` 增加两条字段映射单测。`cargo test` 全部通过，`cargo check --all-targets` 无报错。

### 已决策

1. `writing_mode` 纳入本轮：即使 Tiqian 尚无竖排消费点，也允许配置并原样透传。
2. `indent` 使用 `FirstLineIndent::{Auto, Fixed}`，但序列化保持向后兼容：`Fixed` 写成数值，`Auto` 写成 `"auto"`，数值配置可直接反序列化。
3. 标签属性使用 `lineLengthGridBodyAlignment`。
4. `CHANGELOG.md` 由发布流程根据提交生成，本迭代不手工编辑。

## 背景

Huozi 通过 `HuoziTiqianInputAdapter` 把两份配置折算成 Tiqian `ParagraphStyle`：

- `LayoutStyle`：整次布局调用的默认值；
- `ParagraphStyleOverride`：`[br ... /]` 为后续段落保存的覆盖值。

两条构造路径（`adapt`、`paragraph_style`）目前只写入 Tiqian 13 个字段中的 6–9 个，其余字段只能使用 Tiqian 默认值。调用方因此无法通过任何公开入口表达这些排版意图，而它们都已有确定的 Tiqian 语义与被测试覆盖的实现。

## 现状差异

Tiqian `ParagraphStyle` 共 13 个字段，改动前的暴露情况如下（`—` 表示 Huozi 不给该字段传值）：

| Tiqian 字段 | `LayoutStyle` | 富文本标签 |
| --- | --- | --- |
| `white_space` | `white_space` | `whiteSpace` |
| `text_wrap` | `text_wrap` | `textWrap` |
| `tab_size` | `tab_size` | `tabSize` |
| `last_line_alignment` | `align` | `align` |
| `writing_mode` | — | — |
| `line_height` | `line_height`（倍率） | `lineHeight`（绝对值） |
| `first_line_indent` | `indent`（恒为 `Some`） | `indent` |
| `block_indent` | — | `blockIndent` |
| `first_line_indent_policy` | — | — |
| `line_length_grid` | — | `lineLengthGrid`（只能设 `enabled`） |
| `ruby_line_height_mode` | — | `rubyLineHeightMode` |
| `inline_object_minimum_clearance_em` | — | `inlineObjectMinimumClearance` |
| `emphasis_dot_gap_em` | — | `emphasisDotGap` |

本次要补的缺口：

1. `block_indent`、`line_length_grid`、`ruby_line_height_mode`、`inline_object_minimum_clearance_em`、`emphasis_dot_gap_em` 在 `LayoutStyle` 中没有默认值入口；
2. `line_length_grid.body_alignment` 在两条路径上都无法设置；
3. `first_line_indent_policy` 完全没有入口，且现有实现在任何情况下都传 `Some(indent)`，即使加了字段也不会生效；
4. `writing_mode` 没有入口。

## 已确认设计

### 1. `writing_mode` 同样暴露

Tiqian 目前只在 `ParagraphStyle` 中保存 `WritingMode::VerticalRl`，布局管线没有任何竖排消费点。经确认仍按原样暴露并透传：配置暂时不改变输出是预期行为，待 Tiqian 打通竖排后自动生效。

Huozi 定义自己的 `WritingMode::{HorizontalTb, VerticalRl}`（带 serde），适配器一对一转换。

### 2. 类型由 Huozi 定义

沿用 `WhiteSpaceMode`、`TabSize` 的做法：需要序列化的配置类型放在 `src/layout/layout_style.rs`，在适配器中一对一转换到 Tiqian 类型。Tiqian 对应类型没有 serde 支持，直接重导出会破坏 `LayoutStyle` 的 `Serialize` / `Deserialize`。

新增公开类型：

| Huozi 类型 | 取值 | 默认值 |
| --- | --- | --- |
| `LineLengthGrid` | `enabled: bool`、`body_alignment: Option<ParagraphAlignment>` | `{ enabled: true, body_alignment: None }` |
| `MeasureAdaptiveFirstLineIndent` | `short_below_em`、`short_em`、`long_em`，单位 em | `{ 14.0, 1.0, 2.0 }` |
| `FirstLineIndent` | `Auto`、`Fixed(f64)` | `Fixed(0.0)` |
| `WritingMode` | `HorizontalTb`、`VerticalRl` | `HorizontalTb` |

`RubyLineHeightMode` 目前定义在 `parser::parsed_text`，与 `layout` 层的 `WhiteSpaceMode` 分层不一致。本次把它移到 `layout_style.rs`（补 `Serialize` / `Deserialize`），`parser` 保留 `pub use` 以免破坏既有引用路径。

所有默认值与 Tiqian `ParagraphStyle::default()` 一致。

### 3. `LayoutStyle` 新增字段

| 字段 | 类型 | 默认值 | 映射 |
| --- | --- | --- | --- |
| `block_indent` | `f64`（`ic`） | `0.0` | `Ic { count }` |
| `line_length_grid` | `LineLengthGrid` | 见上 | `TiqianLineLengthGrid` |
| `ruby_line_height_mode` | `RubyLineHeightMode` | `PerLine` | 一对一 |
| `inline_object_minimum_clearance` | `f64`（em） | `0.1` | 一对一（`f32`） |
| `emphasis_dot_gap` | `f64`（em） | `0.1` | 一对一（`f32`） |
| `first_line_indent_policy` | `MeasureAdaptiveFirstLineIndent` | Tiqian 默认 | 一对一 |
| `indent` | `FirstLineIndent`（原为 `f64`） | `Fixed(0.0)` | `Fixed` → `Some(Ic)`；`Auto` → `None` |

`inline_object_minimum_clearance` / `emphasis_dot_gap` 的字段名沿用既有标签属性名，去掉 Tiqian 的 `_em` 后缀，单位在文档中说明。

### 4. `ParagraphStyleOverride` 新增字段

| 字段 | 类型 | 覆盖语义 |
| --- | --- | --- |
| `block_indent` | 已有 | — |
| `line_length_grid` | `Option<bool>` | 覆盖 `enabled` |
| `line_length_grid_body_alignment` | `Option<Option<ParagraphAlignment>>` | 覆盖 `body_alignment`；`Some(None)` 重置为跟随末行对齐 |
| `writing_mode` | `Option<WritingMode>` | 覆盖书写方向 |
| `first_line_indent_policy` | `Option<MeasureAdaptiveFirstLineIndent>` | 覆盖整份策略 |
| `indent` | `Option<FirstLineIndent>`（原为 `Option<f32>`） | `Auto` 时可重置前段固定缩进 |
| `ruby_line_height_mode` | 已有 | — |
| `inline_object_minimum_clearance` | 已有 | — |
| `emphasis_dot_gap` | 已有 | — |

不从 `LayoutStyle` 继承、只存在于 `ParagraphStyleOverride` 的字段保持为空，与现有其他字段一致。

### 5. 标签属性

`[br ... /]` 新增四个属性，取值风格与既有属性一致：

| 属性 | 取值 | 说明 |
| --- | --- | --- |
| `firstLineIndentPolicy` | `shortBelow,short,long` 三个数值，逗号分隔 | 覆盖后续段落的策略 |
| `lineLengthGridBodyAlignment` | `default`、`start`、`center`、`end` | `default` 表示跟随 `align`（`body_alignment = None`）；其余覆盖 |
| `writingMode` | `horizontalTb`、`verticalRl` | 覆盖书写方向 |
| `indent` | 数值或 `auto` | `auto` 表示改用策略决定缩进 |

`lineLengthGridBodyAlignment=default` 与 `indent=auto` 需要能显式重置继承值，因此这两个字段的覆盖状态必须是三态。`lineLengthGridBodyAlignment` 用 `Option<Option<ParagraphAlignment>>` 表达“未设置 / 重置为 None / 指定值”；`indent` 用 `Option<FirstLineIndent>` 表达（`Auto` 本身即三态之一），无需嵌套。

### 6. 映射与合并

- 段落覆盖优先，未覆盖时取 `LayoutStyle`；
- `indent: Auto` 时把 `first_line_indent` 传 `None`，策略才生效；`Fixed` 时传 `Some(Ic)`，策略不参与（与 Tiqian 语义一致）；
- `line_length_grid` 的 `enabled` 与 `body_alignment` 独立合并，便于只改其中一项；
- 非法属性值沿用既有做法：`warn` 并保留前一段的值，不中断解析。

### 7. 空段落高度

`DocumentBuilder::push_empty_paragraph` 目前只读取 `line_height`。本次不改变空段落高度算法，但需确认新字段不会让空段落与前一段的高度语义产生新的不一致（`block_indent`、缩进策略不影响空段高度）。

## 范围

### 包含

- `layout_style.rs` 新增公开类型与字段；
- `ParagraphStyleOverride` 字段扩展与 `RubyLineHeightMode` 迁移；
- `adapt` 与 `paragraph_style` 两条路径写入全部新字段；
- `[br]` 新增与调整的属性解析；
- 验证新字段确实改变布局结果的集成测试（`writing_mode` 只验证透传）；
- `docs/architecture.md`、`docs/parser.md` 同步。

### 不包含

- `writing_mode` / 竖排；
- Moyu `text` 节点 props；
- `LayoutStyle.indent` 与标签 `indent` 的单位统一（前者 `ic`、后者同为 `ic`，但 `lineHeight` 的倍率/绝对值不一致问题不在本轮）；
- Tiqian 内部任何算法或校验逻辑；
- 向后兼容：项目处于开发阶段，允许破坏 `LayoutStyle.indent` 与 `ParagraphStyleOverride.indent` 的既有类型。

## 待决策

已全部解决，结论见「实施结果 / 已决策」。

## 验证要求

- 默认输出与改动前逐顶点一致（未设置新字段时行为不变）；
- `block_indent` 改变所有行的起始位置，且与 `indent` 叠加符合 Tiqian 语义；
- `indent=auto` 在窄行（< 14 字）缩 1 字、宽行缩 2 字；
- `lineLengthGrid.enabled=false` 时行长不再量化到字号整数倍；
- `bodyAlignment` 改变量化余量的摆放方向；
- `rubyLineHeightMode` 在行高不足时分别只加高注文行 / 整段所有行；
- `inlineObjectMinimumClearance` 与 `emphasisDotGap` 改变对应行高或装饰位置；
- `[br ... /]` 覆盖只作用于后续段落，非法值不中断解析；
- 四个公开布局入口（`layout`、`layout_plain`、`layout_parse`、`layout_parse_with`）都能吃到 `LayoutStyle` 的新字段。

## 回滚方案

改动集中在 `layout_style.rs`、`parser/parsed_text.rs`、`parser/elements_to_document.rs`、`layout/tiqian_input.rs` 与测试。回滚即还原这些文件的公开字段与映射；不涉及数据迁移或持久化格式。
