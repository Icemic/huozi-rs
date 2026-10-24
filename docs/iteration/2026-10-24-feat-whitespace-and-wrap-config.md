# 空白处理与换行控制配置接入

> 状态：已完成
>
> 日期：2026-10-24
>
> 分类：feat

## 文档用途

本文定义 Huozi 将 Tiqian 的空白处理、换行控制与 Tab stop 配置接入段落层的过程。后续讨论、实施和验收以本文为依据；形成新结论后，应先更新本文，再据此修改代码。

## 实施结果

已新增 `WhiteSpaceMode`、`TextWrapMode`、`TabSize` 三个公开配置类型，并以 `white_space`、`text_wrap`、`tab_size` 字段加入 `LayoutStyle`；`ParagraphStyleOverride` 保存同名字段的段落覆盖。`tiqian_input.rs` 的两条 `ParagraphStyle` 构造路径均写入这三个值，`[br /]` 新增 `whiteSpace`、`textWrap`、`tabSize` 属性。

新增集成测试 `tests/white_space_config.rs` 共 8 项，覆盖默认值、折叠与保留的宽度差异、`NoWrap` 单行、Tab 倍数影响后续 glyph 位置、`[br]` 覆盖、非法取值降级和 `tabSize` 数值解析。`cargo test` 全部通过，`git diff --check` 仅报告环境原有的换行提示。

## 背景

Huozi 使用 Tiqian 完成空白投影、断行、行调整和最终 glyph placement。Tiqian 0.6 已在 `ParagraphStyle` 提供：

- `white_space: WhiteSpaceMode`：四种空白模式；
- `text_wrap: TextWrapMode`：`Wrap` 与 `NoWrap`；
- `tab_size: TabSize`：Tab stop 的重复间距。

Huozi 此前未把这三项写入 `ParagraphStyle`，布局一律使用 Tiqian 默认值。调用方因此无法表达输入框语义：保留行尾空格占位、让每个空格拥有交互宽度，以及单行不换行。

## 已确认设计

### 1. 配置类型由 Huozi 定义

Huozi 在 `layout_style.rs` 定义三个公开类型，取值与语义和 Tiqian 一致：

| Huozi 类型 | 取值 |
| --- | --- |
| `WhiteSpaceMode` | `Collapse`、`PreserveBreaks`、`Preserve`、`BreakSpaces` |
| `TextWrapMode` | `Wrap`、`NoWrap` |
| `TabSize` | `SpaceAdvanceMultiple(f64)`、`LayoutUnits(f64)` |

`LayoutStyle` 与 `ParagraphStyleOverride` 派生 `Serialize` 和 `Deserialize`，而 Tiqian 的对应类型没有 serde 支持。直接重导出会破坏配置序列化，或迫使 Tiqian 新增依赖，因此由 Huozi 定义同名类型，并在输入适配器中一对一转换。

默认值与 Tiqian `ParagraphStyle::default()` 一致：`PreserveBreaks`、`Wrap`、`SpaceAdvanceMultiple(8.0)`。

### 2. 两级配置

- `LayoutStyle` 增加三个字段，作为整次布局调用的默认值；
- `ParagraphStyleOverride` 增加三个 `Option` 字段，由 `[br /]` 覆盖它之后的段落。

优先级为段落覆盖优先，未覆盖时使用 `LayoutStyle` 的值。这与既有 `indent`、`line_height`、`block_indent` 等字段的处理方式相同。

### 3. 映射到 Tiqian

Huozi 不重新实现空白折叠、Tab stop 或断行，只在输入适配器中转换配置：

| Huozi | Tiqian |
| --- | --- |
| `white_space` | `ParagraphStyle.white_space` |
| `text_wrap` | `ParagraphStyle.text_wrap` |
| `tab_size` | `ParagraphStyle.tab_size` |

两条构造路径均写入这三个值：`adapt` 处理直接 `TextSpan` 输入，`adapt_paragraph` 处理结构化段落。最终几何继续由 Tiqian 决定，输出适配器直接消费其结果。

### 4. `[br /]` 属性

| 属性 | 取值 |
| --- | --- |
| `whiteSpace` | `collapse`、`preserveBreaks`、`preserve`、`breakSpaces` |
| `textWrap` | `wrap`、`noWrap` |
| `tabSize` | 数值，表示空格 advance 的倍数 |

`tabSize` 标签属性只接受倍数形式。`LayoutUnits` 的单位是引擎像素，写进富文本标签会混淆作者语义，因此只通过 `LayoutStyle` 暴露。

无法解析的取值沿用既有做法：记录 `warn`，保留前一段的值，并继续解析其余属性。

### 5. 默认行为随 Tiqian 更新

Tiqian 0.6 的 `PreserveBreaks` 会折叠行内连续空格，旧 Huozi 只在行边折叠。这一变化是 Tiqian 对本轮空白规则的统一实现，Huozi 不保留旧行为，也不为此增加兼容开关。

## 目标

1. 调用方可通过 `LayoutStyle` 为四个公开布局入口设置空白模式、换行控制和 Tab stop。
2. 结构化段落可通过 `[br /]` 单独覆盖这三项配置。
3. 默认配置与 Tiqian 段落默认值一致，未配置时行为由 Tiqian 决定。
4. 空白投影、Tab 求宽和断行全部由 Tiqian 计算，Huozi 不维护第二套实现。

## 范围

### 包含

- 新增三个公开配置类型及其默认值；
- `LayoutStyle` 与 `ParagraphStyleOverride` 的字段接入；
- `tiqian_input.rs` 两条构造路径的转换；
- `[br /]` 三个属性的解析；
- 配置到达 Tiqian 并改变最终布局结果的测试。

### 不包含

- 修改 Tiqian 的空白扫描、Tab 求宽、断行或行调整算法；
- 全局自动间距策略（`Insert` 与 `Replace`）：Tiqian 默认 `Insert` 已生效，当前无切换需求；
- `occupied_bounds`、光标、选择、命中区域的查询几何；
- `TabSize::LayoutUnits` 的标签属性形式；
- 改变 glyph 顶点格式、来源映射或 SDF 渲染。

## 验证要求

- 默认值三项与 Tiqian 一致；
- 折叠与保留模式对行尾空格宽度的差异进入公开结果；
- `NoWrap` 在超过 `box_width` 时保持单行；
- 不同 `tabSize` 倍数改变 Tab 之后 glyph 的横向位置；
- `[br whiteSpace]`、`[br textWrap]` 覆盖只作用于后续段落；
- 非法属性值不中断解析，并保留前一段配置；
- `[br tabSize]` 接受数值并生效。

## 后续

`occupied_bounds` 与光标、选择、命中查询属于既有查询几何待实现项，与本次配置接入无关，另行处理。
