# 富文本标签语法扩展需求

> 状态：已完成
> 
> 日期：2026-09-19
> 
> 分类：feat

## 文档用途

本文档是 Huozi 富文本标签语法扩展的临时需求基线。后续实现 parser、结构化中间表示、Tiqian 输入适配和渲染接入时，应以本文档为准；已经确认的设计不依赖聊天记录或临时口头约定。

本迭代以 parser 为先：先让标签语法、结构化结果和 Tiqian 输入能够完整表达所需能力。本迭代不补足 renderer 对新增能力的绘制或交互行为；渲染侧暂不支持的富文本 layer、注音、装饰和行内对象维持 no-op。为传递新数据、维持编译或保持现有普通文本输出，可以按需编辑 renderer 相关文件；parser 不得因 renderer 暂不支持而丢弃结构和参数。

## 背景与目标

Huozi 已将段落排版后端切换为 tiqian。当前 Huozi 的标签 parser 仅能表达成对标签和一个可选值，并只将字号、填充色、描边和阴影降低为 `TextRun` 样式。Tiqian 的 `ParagraphBuilder` 还支持局部字体样式、注音、CLREQ 装饰、背景、下划线、链接、技术文本、行内盒、行内对象和自动间距控制等能力。

本迭代扩展富文本标签语法，使调用方可以使用自闭合标签与多属性标签，并让 parser 能表达这些能力。目标如下：

1. 保持现有 `[tag]内容[/tag]` 与 `[tag=value]内容[/tag]` 文本兼容；
2. 支持 `[tag key=value]内容[/tag]` 多属性成对标签；
3. 支持 `[tag /]` 与 `[tag key=value /]` 自闭合标签；
4. 使用 `[br ... /]` 表达段落分隔及后续段落的段落设置；
5. 使用 `[span ...]内容[/span]` 一次声明多个行内样式；
6. 为 background、underline 等富文本对象保留专门标签和参数；
7. 支持单引号和双引号属性值，适用于带空白或特殊字符的文本；
8. 以最大程度继续布局和输出为优先，不因未知 tag、未知属性或单个不可解析属性中止整段游戏文本的处理。

## 范围

### 包含

- 标签头的多属性解析；
- 自闭合标签解析；
- 引号属性值与引号内转义；
- 旧单值标签的兼容降低规则；
- 段落分隔、自闭合位置内容和成对范围内容的结构化表示；
- 现有样式属性和新增 Tiqian 能力的标签设计；
- parser 到 Tiqian 输入的目标映射；
- 当前渲染不支持能力的 no-op 边界。

### 不包含

- 在本需求阶段实现 parser、输入适配或 renderer；
- 更改 tiqian 的 layout、shaping、字体 fallback、断行或绘制模型；
- 将段落标签提前实现为实际多段布局输出；
- 补足 Huozi renderer 对背景、下划线、删除线、ruby、注音、CLREQ 装饰或行内对象的绘制、交互行为；
- 新增竖排、完整 bidi / RTL、分页或多栏能力。

## 当前实现

### 当前语法

当前 parser 支持：

```text
[tag]内容[/tag]
[tag=value]内容[/tag]
[tag="带空格的值"]内容[/tag]
[tag='带空格的值']内容[/tag]
```

标签必须严格嵌套且开始、结束标签名相同。`[[` 和 `]]` 分别表示字面的 `[` 与 `]`。调用方可以通过 `parse_with` 使用其他单字符开闭符号。

当前 AST 为：

```text
Element::Text { start, end, content, segment_id }
Element::Block { start, end, tag, value, inner }
```

当前 `TextStyle` 只保存：

- `font_size`；
- `fill_color`；
- `stroke`；
- `shadow`。

当前已实际生效的样式标签为：

| 标签 | 效果 |
| --- | --- |
| `size` | 字号。 |
| `color`、`fillColor` | 文本填充色。 |
| `stroke`、`strokeColor`、`strokeWidth` | 描边参数。 |
| `shadow`、`shadowColor`、`shadowOffsetX`、`shadowOffsetY`、`shadowBlur`、`shadowWidth` | 阴影参数。 |

无值的未知标签和 `span` 当前仅用于切分 `TextSpan`；可选 `style_prefabs` 可以使无值标签替换为预设 `TextStyle`。带值的未知样式标签会记录 warning 并忽略其样式值，文本仍正常参与布局。

`docs/parser.md` 中的 `bold`、`italic`、`underline`、`fontFamily`、`fontSize`、`opacity`、`lineHeight`、`indent` 等示例不代表当前实现。其中 `fontSize` 不是当前 `size` 的别名；`lineHeight` 和 `indent` 也不会从标签写入段落样式。

### 当前 Tiqian 输入边界

Huozi 当前把整个 `Vec<TextSpan>` 合并为一个 tiqian `ParagraphBuilder`：

- `LayoutStyle` 提供宽高约束、行高和首行缩进；
- 每个 `TextRun` 写入字号和 fill / stroke / shadow paint；
- 当前 `TextRun` 不携带字体族、locale、字重、斜体、基线偏移、背景、链接、注音、装饰或段落分隔；
- 当前输出适配器只生成普通文本 glyph 的 SDF 顶点，ruby、注音、着重号、下划线和删除线几何会被跳过。

## 总体设计

### Parser 优先与数据保留

标签 parser 负责保留输入的结构和声明，不应因当前渲染不支持而丢弃 `background`、`underline`、`ruby`、`link`、`technical`、`box` 或 `object` 等内容。后续输入适配器把这些结构降低为 `ParagraphBuilder` 操作；renderer 可暂时针对未支持对象 no-op。

新的结构化中间表示至少区分：

1. 普通文本；
2. 带内容、可嵌套的成对标签；
3. 无内容、位于当前位置的自闭合标签；
4. 段落分隔及其后续段落参数；
5. 文字样式、paint、富文本 layer 和语义范围。

`TextRun` 只保存局部文字样式。背景、线条、链接、注音、装饰和其他范围语义使用专门声明保存，供后续降低为 Tiqian 的 scope、span 或位置插入操作。

### 推荐 AST 形状

实现可以调整具体 Rust 类型名，但应表达以下信息：

```text
Element::Text {
  start,
  end,
  content,
  segment_id,
}

Element::Block {
  start,
  end,
  tag,
  attributes,
  inner,
}

Element::SelfClosing {
  start,
  end,
  tag,
  attributes,
}

Attribute {
  name,
  value,
}
```

`attributes` 使用按源顺序保存的 `Vec<Attribute>`：

- 属性数量很小，线性读取足够；
- 便于稳定地保留和报告重复属性；
- 后续扩展不丢失源顺序。

重复属性、未知属性和不属于当前 tag 的属性不应使整个解析调用失败。应按源顺序应用可识别属性；同名属性以最后一个可识别值为准，并记录 warning。这样能最大程度保持游戏文本可布局。

## 语法扩展

### 成对标签

保留并支持以下形式：

```text
[tag]内容[/tag]
[tag=value]内容[/tag]
[tag key=value]内容[/tag]
[tag key=value key2=value2]内容[/tag]
[tag key="包含空格的值" key2='另一个值']内容[/tag]
```

开始和结束标签继续严格匹配并按 LIFO 嵌套。带内容标签不能以自闭合形式同时再拥有结束标签。

### 旧单值语法兼容

`[tag=value]` 是旧语法，必须继续支持。其值降低为 tag 自己的主属性：

| 旧写法 | 等价属性声明 |
| --- | --- |
| `[size=24]` | `size="24"` |
| `[color=#FF0000]` | `color="#FF0000"` |
| `[ruby="tí qiàn"]` | `text="tí qiàn"` |
| `[link="https://tiqian.org"]` | `target="https://tiqian.org"` |
| `[background=#FFF3BF]` | `color="#FFF3BF"` |
| `[underline=#1677FF]` | `color="#1677FF"` |

实现中的兼容降低不要求向调用方暴露“标签同名属性”。每种 tag 可有明确的主属性名称。

当旧单值与多属性中同一主属性同时存在时，按属性在标签头中的书写顺序应用，最后一个可识别值覆盖之前的值。例如：

```text
[color=red color=blue]文字[/color]
```

最终使用 `blue`，并记录重复属性 warning。

### 自闭合标签

新增：

```text
[tag /]
[tag key=value /]
[tag key="包含空格的值" /]
```

自闭合标签产生 `Element::SelfClosing`，不创建空内容 `Block`。`/` 是标签头的结束标记，可紧跟标签名或未引用属性值：

```text
[br /]             可解析
[br indent=2 /]    可解析
[br/]              与 `[br /]` 等价
[br indent=2/]     与 `[br indent=2 /]` 等价
```

未引用属性值不含 `/`，因此末尾 `/` 可以无歧义地识别为自闭合标记。语法无法形成完整 token 时，parser 不应 panic；应保留尽可能多的原始文本或将对应结构降级为普通文本，并记录 warning。

### 属性语法与引号

属性基本形式为：

```text
key=value
```

标签名、属性名、等号和属性值之间允许空白。未引用值不得包含空白、当前标签的开闭符号、`=`、`/`、`"` 或 `'`。

属性值支持单双引号：

```text
[font family="Source Han Sans SC, Inter"]文字[/font]
[ruby text='tí qiàn']提椠[/ruby]
[link target="https://example.com/a?x=1&y=2"]链接[/link]
```

引号内支持：

| 引号形式 | 可直接包含 | 转义形式 |
| --- | --- | --- |
| `"..."` | 空白、单引号 | `\"` 表示双引号，`\\` 表示反斜杠。 |
| `'...'` | 空白、双引号 | `\'` 表示单引号，`\\` 表示反斜杠。 |

引号属性值不允许换行。未闭合引号或不完整转义不应导致 panic；parser 记录 warning，并将无法识别的标签头降级为普通文本。

标签内部的 `[[` 与 `]]` 仅用于正文文本转义，不在属性值中解释为方括号转义。属性值需要包含当前开闭符号时，使用引号包裹；后续实现可按需要扩展专门的属性转义，但本迭代不要求为方括号增加转义语义。

## 标签与参数

### 分类说明

- **普通成对标签**：覆盖一个非空或可空的文本范围，支持嵌套。
- **多属性标签**：使用普通成对标签语法，但具有多个可选属性；`span` 是多属性文字样式入口。
- **自闭合标签**：位于文本位置，不包含子内容。
- **段落设置**：只允许出现在 `br` 自闭合标签上，表示后续段落的样式；不得作为 `span` 或其他行内范围的属性。

未列出的 tag 或属性保持宽容处理：记录 warning，忽略其不支持的效果，正文仍参与布局。已知 tag 的未知属性同样只记录 warning 并忽略。一个属性值解析失败时，只忽略该属性，保留同标签内其他有效属性和文本。

### 当前样式标签

以下 tag 与属性当前已支持，扩展后必须保持原有结果：

| 标签 | 主属性或可用属性 | Tiqian 目标 |
| --- | --- | --- |
| `size` | `size` | `TextStyleOverride.font_size`。 |
| `color`、`fillColor` | `color` | 文本 layer 的 `RichTextPaint::Fill`。 |
| `stroke` | `color`、`width`，兼容旧单值组合格式 | 文本 layer 的 `RichTextPaint::Stroke`。 |
| `strokeColor` | `color` | 描边颜色。 |
| `strokeWidth` | `width` | 描边宽度。 |
| `shadow` | `offsetX`、`offsetY`、`blur`、`color`、`width`，兼容旧单值组合格式 | 文本 layer 的 `RichTextPaint::Shadow`。 |
| `shadowColor` | `color` | 阴影颜色。 |
| `shadowOffsetX` | `offsetX` | 阴影 X 偏移。 |
| `shadowOffsetY` | `offsetY` | 阴影 Y 偏移。 |
| `shadowBlur` | `blur` | 阴影 blur 半径。 |
| `shadowWidth` | `width` | 阴影扩张半径。 |
| `span` | 无属性时仅建立普通范围边界 | 多属性范围入口。 |

现有 `style_prefabs` 的无值 tag 行为继续保留。无值 tag 命中预设时，预设仍可作为基础 `TextStyle`；显式写出的可识别属性随后覆盖预设中的相应字段。预设不表达后续新增的范围语义或段落设置。

### 多属性 `span`

`span` 支持所有行内文字样式和文本 paint 属性，以减少嵌套标签：

```text
[span size=28 color="#F59E0B" shadowColor="#0008" shadowOffsetX=1 shadowOffsetY=2 shadowBlur=4 shadowWidth=1]警告[/span]
```

`span` 可支持的属性分组如下：

| 分组 | 属性 | Tiqian 目标 |
| --- | --- | --- |
| 字体 | `font`、`family`、`size`、`weight`、`italic`、`locale`、`baseline`、`attach` | `TextStyleOverride`。 |
| 填充 | `color`、`fillColor` | `Fill` paint。 |
| 描边 | `strokeColor`、`strokeWidth`，以及 `stroke` 的兼容组合值 | `Stroke` paint。 |
| 阴影 | `shadowColor`、`shadowOffsetX`、`shadowOffsetY`、`shadowBlur`、`shadowWidth`，以及 `shadow` 的兼容组合值 | `Shadow` paint。 |

`span` 不接受段落设置：`indent`、`lineHeight`、`blockIndent`、`lastLineAlignment`、`lineLengthGrid`、`rubyLineHeightMode`、`inlineObjectMinimumClearance`、`emphasisDotGap`。遇到时记录 warning 并忽略该属性，继续处理文本和其他可识别属性。

所有独立样式 tag 与 `span` 共享同一套“属性应用到当前样式”的实现，避免为 `[size=24]` 与 `[span size=24]` 分别维护赋值逻辑。

### 新增局部文字样式标签

| 标签 | 主属性 | 额外属性 | Tiqian 目标 | 当前渲染 |
| --- | --- | --- | --- | --- |
| `font` | `family` | 无 | `font_families`，逗号分隔的 fallback 顺序。 | 普通 glyph 绘制继续复用现有 SDF 路径。 |
| `weight` | `weight` | 无 | `font_weight`。 | 普通 glyph 绘制继续复用现有 SDF 路径。 |
| `bold` | 无 | `weight`，默认 `700` | `font_weight`。 | 普通 glyph 绘制继续复用现有 SDF 路径。 |
| `italic` | 无 | `enabled`，默认 `true` | `italic`。 | 普通 glyph 绘制继续复用现有 SDF 路径。 |
| `locale` | `locale` | 无 | `locale`。 | 无额外绘制需求。 |
| `baseline` | `baseline` | 无 | `baseline_shift`，单位 px，正值向下。 | 普通 glyph 位置由 Tiqian 结果决定。 |
| `attach` | `attach` | 无 | `inline_attachment`；当前只支持 `previous` 与 `none`。 | 无额外绘制需求。 |

推荐示例：

```text
[font="Source Han Sans SC, Inter"]混排[/font]
[weight=700]粗体[/weight]
[bold]粗体[/bold]
[italic]斜体[/italic]
[locale=ja]日本語[/locale]
[baseline=-4]上标[/baseline]
[attach=previous]注释号[/attach]
```

`font` 的单值语法 `[font="A, B"]` 使用逗号分隔 fallback 字体族。`family` 是 `span` 中的同义属性。空字体族列表或不能解析的字体族字符串只记录 warning，并继承当前字体族。

### 新增富文本 layer 标签

这些标签分别表达独立的 Tiqian `RichTextLayer`。它们不列为 `span` 的属性；需要与文字样式组合时通过嵌套表达。

| 标签 | 主属性 | 可选属性 | Tiqian 目标 | 当前渲染 |
| --- | --- | --- | --- | --- |
| `background` | `color` | `strokeColor`、`strokeWidth`、`shadowColor`、`shadowOffsetX`、`shadowOffsetY`、`shadowBlur`、`shadowWidth`、`paddingX`、`paddingY`、`radius`、`continuationRadius`、`metricPolicy`、`clearance` | `Background` layer；`paddingX` 大于零时同时生成 `InlineBoxSpan`。 | no-op。 |
| `underline` | `color` | `strokeColor`、`strokeWidth`、`shadowColor`、`shadowOffsetX`、`shadowOffsetY`、`shadowBlur`、`shadowWidth`、`thickness`、`pattern`、`dashLength`、`gapLength`、`clearance` | `Underline` layer。 | no-op。 |
| `lineThrough` | `color` | 与 `underline` 相同 | `LineThrough` layer。 | no-op。 |

`background`、`underline` 和 `lineThrough` 的 `color` 生成对应 layer 的 fill paint。`pattern` 可取 `solid`、`dashed` 或 `dotted`；虚线使用 `dashLength` 与 `gapLength`，点线使用 `gapLength`。`metricPolicy` 可取 `markedFaces`、`uniformTextStyle`、`uniformParagraphStyle`。

推荐示例：

```text
[background color="#FFF3BF" paddingX=6 paddingY=2 radius=4]提示[/background]
[underline color="#1677FF" thickness=1 pattern=dashed dashLength=3 gapLength=2]链接[/underline]
[lineThrough color="#666" thickness=1]过期内容[/lineThrough]
```

### 新增注音、装饰与语义范围标签

| 标签 | 主属性 | 可选属性 | Tiqian 目标 | 当前渲染 |
| --- | --- | --- | --- | --- |
| `ruby` | `text` | `font`、`locale` | `RubySpan`，默认 `Pinyin`。 | no-op。 |
| `bopomofo` | `text` | `font`、`locale` | `RubySpan`，`Bopomofo`；未指定 locale 时使用 `zh-TW`。 | no-op。 |
| `emphasis` | 无 | 无 | `DecorationKind::Emphasis`。 | no-op。 |
| `mourning` | 无 | 无 | `DecorationKind::Mourning`。 | no-op。 |
| `properNoun` | 无 | 无 | `DecorationKind::ProperNoun`。 | no-op。 |
| `bookTitle` | 无 | 无 | `DecorationKind::BookTitle`。 | no-op。 |
| `link` | `target` | 无 | `RichTextSemantic::Link`；可见文字等于地址时采用技术文本断行。 | 只保留语义，导航与专属样式 no-op。 |
| `technical` | 无 | 无 | `TechnicalInline`、`ProgressiveTechnical` 断行与自动间距抑制。 | 无额外绘制需求。 |
| `code` | 无 | `font`、`color`、背景属性 | `InlineCode`：局部文本样式、背景、技术文本语义、技术断行和自动间距抑制。 | 背景 no-op；文本按当前普通 glyph 输出。 |
| `noAutoSpace` | 无 | 无 | 自动间距抑制范围。 | 无额外绘制需求。 |
| `box` | 无 | `start`、`end`、`spacing` | `InlineBoxSpan`。`spacing` 为 `narrow` 或 `source`。 | 无额外绘制需求。 |

推荐示例：

```text
[ruby="tí qiàn"]提椠[/ruby]
[bopomofo="ㄊㄧˊ ㄑㄧㄢˋ"]提椠[/bopomofo]
[emphasis]重点[/emphasis]
[properNoun]北京大学[/properNoun]
[bookTitle]活字[/bookTitle]
[link="https://tiqian.org"]tiqian.org[/link]
[technical]std::sync::Arc[/technical]
[code font="monospace" color="#1E1E23" paddingX=4]cargo test[/code]
[noAutoSpace]CJKAPI混排[/noAutoSpace]
[box start=4 end=4 spacing=narrow]标签[/box]
```

Ruby、bopomofo 与 `box` 的内容为空时，后续降低为 Tiqian 输入时无法产生有效范围。为保持游戏文本可继续处理，parser 保留空范围并记录 warning；输入适配器跳过该条声明，不影响相邻正文。

### 自闭合位置标签

| 标签 | 属性 | 结构化含义 | 后续 Tiqian 目标 | 当前阶段 |
| --- | --- | --- | --- | --- |
| `br` | 见“段落分隔” | 结束当前段落并开始后续段落。 | 为后续段落创建新的 `ParagraphBuilder` / `LayoutInput`。 | 仅 parser 表达，布局待实现。 |
| `object` | `alt`、`width`、`ascent`、`descent`、`leadingBoundary`、`trailingBoundary` | 在当前位置声明一个行内对象。 | `inline_object(replacement_text, metrics)`。 | parser 和输入表达优先；对象绘制 no-op。 |

`object.alt` 是复制、搜索、选择和无障碍使用的非空替代文本。若缺少或为空，记录 warning 并跳过该对象声明，原始标签文本不应导致全段崩溃。`width`、`ascent`、`descent` 是布局单位；边界调整参数以后续 Tiqian `InlineObjectBoundaryAdjustment` 的公开语义为准。

## 段落分隔 `[br /]`

### 语义

`br` 表示段落分隔，不表示同一段落内的硬换行：

```text
第一段[br indent=2 lineHeight=36 blockIndent=1 /]第二段
```

上例产生两个逻辑段落：

1. 第一段使用进入该段落时已有的段落样式；
2. 第二段继承当前段落样式，并用 `br` 属性覆盖指定字段。

调用方可以直接在输入文本中使用换行字符，因此本语法不需要 Tiqian 的 `hard_break()`，也不在本迭代引入 `[lineBreak /]`。

`br` 位于段首、段尾或连续出现时保留空段落结构；后续布局如何呈现空段落由段落输入和布局输出阶段决定。parser 不丢弃这些结构。

### 可用段落参数

| 属性 | 作用 | Tiqian 目标 |
| --- | --- | --- |
| `indent` | 后续段落的首行缩进，单位 `ic`。 | `ParagraphStyle.first_line_indent`。 |
| `lineHeight` | 后续段落的绝对行高，单位 layout px。 | `ParagraphStyle.line_height`。 |
| `blockIndent` | 后续段落所有行的起始缩进，单位 `ic`。 | `ParagraphStyle.block_indent`。 |
| `lastLineAlignment` | 最后一行对齐方式：`start`、`center`、`end`。 | `ParagraphStyle.last_line_alignment`。 |
| `lineLengthGrid` | 是否启用行长字格量化。 | `ParagraphStyle.line_length_grid.enabled`。 |
| `rubyLineHeightMode` | ruby 行高策略：`perLine`、`uniformParagraph`。 | `ParagraphStyle.ruby_line_height_mode`。 |
| `inlineObjectMinimumClearance` | 行内对象与相邻行的最小净空，单位 em。 | `ParagraphStyle.inline_object_minimum_clearance_em`。 |
| `emphasisDotGap` | 着重号与字面的净空，单位 em。 | `ParagraphStyle.emphasis_dot_gap_em`。 |

未指定的参数从前一段继承。`br` 中未识别或值无效的段落参数只记录 warning 并忽略该项，其他参数与后续文本仍正常处理。

## 属性与标签降低原则

### 样式继承与嵌套

成对标签按作用域嵌套。内层可识别属性覆盖外层同类属性，离开内层后恢复外层值。

```text
[span size=20 color=red]
  外层
  [span color=blue weight=700]内层[/span]
  外层
[/span]
```

文本样式与 paint 采用当前有效值生成范围。`background`、`underline`、`ruby`、装饰等范围对象按标签范围与其他对象重叠，不能因已有 `TextRun` 切分而丢失。

### 属性应用顺序

同一标签内的属性按书写顺序处理：

```text
[span color=red color=blue size=20]文字[/span]
```

最终填充色为 `blue`，字号为 `20`。重复 `color` 记录 warning；不因此拒绝该段文本。

预设样式、外层样式和显式属性的优先级为：

```text
初始 TextStyle
  → 无值 tag 命中的 style_prefab
  → 外层成对标签
  → 当前标签内按书写顺序的可识别属性
```

### 宽容处理

Huozi 面向游戏引擎，富文本可能来自动态文本、脚本、编辑器或本地化资源。parser 和输入适配应尽力保留可布局的正文：

| 情况 | 处理 |
| --- | --- |
| 未知 tag | 记录 warning；成对未知 tag 保持为普通范围并继续处理其内容，自闭合未知 tag 跳过其效果。 |
| 已知 tag 的未知属性 | 记录 warning，忽略该属性。 |
| 属性值无效 | 记录 warning，忽略该属性，保留同标签的其他有效属性。 |
| 重复属性 | 记录 warning，最后一个可识别值覆盖前值。 |
| 无法完整识别的标签头、未闭合引号或不完整转义 | 记录 warning，将无法识别部分尽量降级为普通文本。 |
| 未匹配结束标签或不完整成对标签 | 记录 warning，将无法形成完整结构的内容尽量保留为普通文本。 |
| 当前 renderer 不支持的对象 | parser 和输入适配保留其声明；renderer 对对应对象 no-op。 |
| 空 ruby、bopomofo、box 或 object | 记录 warning，跳过无法降低的对象声明，继续布局正文。 |

这套规则的目标是避免单个富文本错误中止整个段落或渲染帧。warning 应包含 tag、属性和可用的源位置，便于资源制作阶段定位问题。

## 目标 Tiqian 映射

| Huozi 分类 | Tiqian `ParagraphBuilder` 能力 |
| --- | --- |
| 文字样式 | `push_text_style(TextStyleOverride)`。 |
| 文本 fill / stroke / shadow | `push_paints(&[RichTextPaint])` 或对应层声明。 |
| 背景 | `push_background(RichTextBackgroundPaint, paints)`。 |
| 下划线 | `push_underline(RichTextLinePaint)`。 |
| 删除线 | `push_line_through(RichTextLinePaint)`。 |
| Ruby / bopomofo | `push_ruby(RubyAnnotation)`。 |
| CLREQ 装饰 | `push_decoration(DecorationKind)`。 |
| 链接 | `push_link(target)`。 |
| 技术文本 | `push_technical()`。 |
| 行内代码 | `push_inline_code(style, background)`。 |
| 自动间距抑制 | `push_auto_space_suppressed()`。 |
| 行内盒 | `push_inline_box(InlineBoxStyle)`。 |
| 行内对象 | `inline_object(replacement_text, metrics)`。 |
| 段落分隔 | 结束当前输出段落；以继承并覆盖后的 `ParagraphStyle` 创建新的 builder。 |

所有 range 型对象继续使用 Unicode scalar 半开范围。原始 `SourceRange` 指向单个输入 `Segment` 中包含标签的源文本；Tiqian 显示范围指向去除标签后拼接的段落文本。两个坐标系必须分别维护，不能直接互换。

## Renderer 边界

本迭代不补足 renderer 对新增能力的绘制或交互行为。当前 Huozi 输出适配器只输出普通文本的 fill、stroke、shadow SDF glyph 顶点。为传递新数据、维持编译或保持现有普通文本输出，可以修改输出适配器、顶点或其他 renderer 相关文件；新增对象的 renderer 行为维持下表所列的 no-op。

| 对象 | Parser / 输入适配 | 当前 renderer |
| --- | --- | --- |
| 字体样式、字号、基线偏移、技术断行、自动间距抑制、行内盒 | 必须写入 Tiqian 输入。 | 继续普通 glyph 输出。 |
| 背景、下划线、删除线 | 必须保留并写入 Tiqian rich-text。 | no-op。 |
| Ruby、bopomofo、着重号、示亡号、专名号、书名号 | 必须写入 Tiqian layout 输入与 rich-text 声明。 | no-op。 |
| 链接 | 必须保留 target 语义。 | 导航与专属视觉 no-op。 |
| 行内代码 | 必须写入技术断行、自动间距抑制、局部样式与背景声明。 | 背景 no-op；普通文本 glyph 继续输出。 |
| 行内对象 | 必须写入对象度量和替代文本。 | 对象图形 no-op。 |
| 段落分隔 | parser 先表达；多段输出适配待实现。 | 待实现。 |

纯绘制参数不得作为 Huozi 的另一套布局算法。Tiqian 是字体选择、shaping、断行、标点空间、行调整和最终 placement 的唯一来源。

## 实施要求

### 实施顺序与当前边界

当前数据流是：

```text
Segment
  → parse / parse_with
  → Vec<Element>
  → to_spans
  → Vec<TextSpan>
  → HuoziTiqianInputAdapter::adapt
  → 一个 ParagraphBuilder
  → LayoutInput
```

这套数据流不能承载 `[br /]`：`TextRun` 只存局部样式和来源范围，`TextSpan` 只是一组 run；而 `ParagraphBuilder` 在追加首段文本后不能再更新 `ParagraphStyle`。因此不能把段落设置加到 `TextRun`，也不能在 `HuoziTiqianInputAdapter::adapt` 的循环中修改当前 builder。

本迭代改为以下数据流：

```text
Segment
  → parse / parse_with
  → Vec<Element>
  → lower_elements
  → ParsedText { paragraphs }
  → ParsedParagraph { nodes, paragraph_style }
  → HuoziTiqianInputAdapter::adapt_paragraph
  → 一个 ParagraphBuilder
  → LayoutInput
```

`ParsedText` 是 parser 的完整输出；`TextSpan` 与 `TextRun` 保留为直接布局 API 和文字叶子使用的类型。`[br /]` 切分 `ParsedText.paragraphs`。本迭代不会把多个 `LayoutInput` 组合成新的公开布局输出。

阶段按上述数据转换安排。Phase 1 和 Phase 2 可以暂时不能编译。Phase 3 完成时 parser 必须可用。Phase 4 完成时单段布局入口必须恢复可用；`[br /]` 只保留在 `ParsedText` 中，当前单段输入适配不读取它。`ParsedText` 完整保留段落边界和设置，后续多段布局迭代以它为输入。

### Phase 1：建立新的 parser 输出模型

涉及文件：`src/parser/parse_elements.rs`、`src/parser/text_style.rs`、`src/parser/text_run.rs`、`src/parser/text_span.rs`、新增 `src/parser/parsed_text.rs`，以及 `src/parser.rs` 的导出。

1. 将 `Element::Block.value: Option<String>` 替换为 `attributes: Vec<Attribute>`；`Attribute` 保存属性名、属性值和属性值在源 segment 中的 scalar 范围，供 warning 定位使用。
2. 在 `Element` 增加 `SelfClosing { start, end, tag, attributes }`；保留 `Text` 和带 `inner` 的 `Block`，使成对标签和位置标签在语法树中不再混用。
3. 新增 `ParsedText { paragraphs: Vec<ParsedParagraph> }`。`ParsedParagraph` 保存从当前段首继承并由 `[br /]` 覆盖的 `ParagraphStyleOverride`，以及按原文顺序排列的 `Vec<InlineNode>`。
4. 定义 `InlineNode` 为 `Text(TextRun)`、`Scope { kind: InlineScopeKind, children }` 和 `Object(InlineObject)`。`InlineScopeKind` 使用具名变体保存背景、线条、ruby、装饰、链接、技术文本、行内代码、自动间距抑制和行内盒；文字样式只写入 `TextRun`，避免同一份样式同时通过叶子和 scope 重复降低。不要用字符串 tag 或通用属性表替代已确定的语义类型。
5. `TextStyle` 扩展为 Huozi 的完整局部文字样式：字体族、字号、locale、字重、斜体、基线偏移和行内附着语义，同时保留现有 fill、stroke、shadow。`TextRun` 不保存背景、链接、ruby、装饰或段落设置。
6. `ParagraphStyleOverride` 的字段使用 `Option<T>` 表示“继承前一段”或“覆盖该字段”；它只存本文档列出的 `[br /]` 参数，不能复用 `LayoutStyle`，因为 `LayoutStyle` 还包含整个布局调用的宽高约束。

此阶段允许 `elements_to_spans.rs`、`layout.rs` 和 adapter 因旧类型签名尚未同步而不能编译。不要在旧 `TextSpan` 上叠加大量可选字段，也不要为临时通过编译复制一套富文本范围。

### Phase 2：重写标签头解析和恢复路径

涉及文件：`src/parser/parse_elements.rs` 与该模块的测试。

1. 将现有只读取一对 `key=value` 的 `tag_head_keypair` 替换为按顺序读取属性的 parser：先读 tag 名，再循环读取旧主值或 `key=value`，最后识别可有可无的自闭合 `/`。
2. 读取 `[tag=value]` 时立即按 tag 的主属性名生成一个 `Attribute`；例如 `size`、`color`、`ruby.text`、`link.target`。之后的命名属性继续追加到同一列表，因此 lowering 可以统一按列表顺序处理覆盖关系。
3. 将 quoted value 改为逐字符扫描：识别 `\\` 和与当前引号相同的转义引号，拒绝换行；输出去除引号和转义后的值，并保留原始 scalar 范围。
4. 允许结尾 `/` 紧贴 tag 名或未引用值，令 `[br/]`、`[br /]`、`[br indent=2/]` 和 `[br indent=2 /]` 都生成相同的 `SelfClosing`。
5. 在匹配结束标签、标签头或引号值无法完整识别时，scanner 不应让整个 `parse` 调用失败。新增“标签状文本”恢复分支，至少消费当前开符号并将无法形成节点的原文作为 `Element::Text` 输出；继续扫描其后的文本。正常成对标签仍只在结束名匹配时生成 `Block`。
6. 保留当前 `[[`、`]]` 正文转义及 `parse_with::<OPEN, CLOSE>`。不要测试 `csscolorparser` 或 tiqian 的内部行为；只断言 Huozi 的节点、属性顺序、值和 scalar 范围。

本阶段结束时，`parse` / `parse_with` 可以独立通过 parser 层测试。`lower_elements`、公开 `parse_text` 和布局入口可以仍未接通。

### Phase 3：将语法树降低为带作用域的段落文档

涉及文件：将 `src/parser/elements_to_spans.rs` 替换为 `src/parser/elements_to_document.rs`，新增样式与 scope 配置辅助模块，修改 `src/parser.rs` 与 `src/layout.rs` 的 parser 调用边界。

1. 用一次深度优先遍历替换当前 `Rc<RefCell<IntoIter<Element>>>` 栈。遍历遇到 `Text` 时创建带完整生效 `TextStyle` 与原始 `SourceRange` 的 `TextRun`；遇到文字样式 `Block` 时复制当前样式、应用属性后直接递归子元素；遇到范围语义 `Block` 时递归降低子元素并保存为一个 `Scope` 节点。
2. 为 `span` 和旧的 `size`、`color`、`stroke`、`shadow` 等样式 tag 调用同一个 `apply_text_attributes(&mut TextStyle, &[Attribute])`。该函数只处理文字样式和文本 paint；背景、线条、ruby、装饰、链接、技术文本、代码、盒与自动间距分别构造对应的 `InlineScopeKind`。
3. 在 `apply_text_attributes` 中按属性出现顺序覆盖字段。无法转换的单个值保留当前字段并输出 warning；未知属性也只输出 warning。不要为每个字段写独立测试；组合样例覆盖这条公共路径即可。
4. 无值 tag 命中 `style_prefabs` 时先复制预设 `TextStyle`，再应用当前标签的显式属性。未命中预设的未知 tag 不改变当前样式，但保留其子节点，保证正文不丢失。
5. 遇到 `SelfClosing::br` 时结束当前 `ParsedParagraph`，根据当前段落的 `ParagraphStyleOverride` 复制并应用其属性后创建下一段。位于开头、结尾或连续的 `br` 都创建空段。遇到 `SelfClosing::object` 时创建 `InlineNode::Object`；无法构造有效替代文本时记录 warning 并不创建对象。
6. 将 `Huozi::parse_text` 和 `parse_text_with` 改为返回 `ParsedText`，由它们依次解析各 `Segment`、拼接 `Element` 后调用 `lower_elements`。`TextSpan` 不再是标签 parser 的返回类型；直接调用 `Huozi::layout` 的用户仍可提供手工 `Vec<TextSpan>`。

完成条件：`parse_text` 与 `parse_text_with` 返回完整 `ParsedText`，其 node 树保留所有新增 tag；`[br /]` 形成段落数组和后续 `ParagraphStyleOverride`。这个阶段是 parser 的可运行验收点。

### Phase 4：把单段 `ParsedParagraph` 写入 Tiqian builder

涉及文件：`src/layout/tiqian_input.rs`、`src/layout.rs`，并按编译需要调整 `src/layout/tiqian_output.rs`、测试与公开文档。

1. 将 `HuoziTiqianInputAdapter::adapt(text_spans, layout_style, initial_text_style)` 改为 `adapt_paragraph(paragraph, layout_style, initial_text_style)`。用 `ParsedParagraph.paragraph_style` 覆盖由 `LayoutStyle` 构造的默认 `ParagraphStyle`，再在追加第一段文本前写入 builder。
2. 新增递归 `append_nodes`：`Text` 调用 `builder.with_text_style`、`with_paints` 和 `push`；`Scope` 分别调用实际的 `with_background`、`with_underline`、`with_line_through`、`with_ruby`、`with_decoration`、`with_link`、`with_technical`、`with_inline_code`、`with_auto_space_suppressed` 或 `with_inline_box`；`Object` 调用 `inline_object`。文字追加时创建 `HuoziSourceMapEntry`，display offset 只在 `Text` 与对象的替代文本追加后递增。
3. 将扩展后的 Huozi `TextStyle` 完整转换为 Tiqian `TextStyle` / `TextStyleOverride`，包括字体族、locale、字重、斜体、基线偏移和 `InlineAttachment`。已有 fill、stroke、shadow 仍由 `tiqian_paints` 生成；背景和线条使用其独立的 Tiqian rich-text 入口。
4. parser lowering 已跳过空 ruby、空 box 和无替代文本对象。`append_nodes` 额外携带当前 scope 种类：对象处在 ruby scope 内时，在调用 `inline_object` 前记录固定 warning 并跳过该对象，因为 Tiqian 将这种位置插入视为 builder 错误；其余节点使用不返回 `Result` 的 `with_*` scope API。不要把 `try_with_*` 当作恢复机制：builder 已记录的错误会在 `build()` 时再次返回，无法在同一个 builder 内清除。
5. `layout_parse` 与 `layout_parse_with` 接收 `ParsedText`。当前只读取第一个 `ParsedParagraph` 并向调用方记录 `[br /]` 尚未接入多段输出的固定 warning；后续段落保留在 parser 输出中，但不进入单段 `LayoutInput`。直接 `layout(Vec<TextSpan>)` 和 `layout_plain` 保持原有单段路径。
6. 输出适配器只为当前普通文本 glyph 路径消费文本层的 fill、stroke、shadow。为编译和数据传递可调整其查询方式，但不得开始绘制背景、线条、ruby、装饰或对象。

完成条件：没有 `[br /]` 的新 tag 可以从 `layout_parse` 到达对应的 Tiqian `LayoutInput`，并完成现有普通 glyph 输出。含 `[br /]` 时，当前布局只消费第一段并记录 warning；多段 `LayoutInput`、跨段来源映射和布局输出留待后续迭代。

### Phase 5：收敛验证和更新文档

涉及文件：`docs/parser.md`、`docs/api-design.md`、`docs/architecture.md`、本迭代文档及受本次公开 API 变化影响的示例。

1. 用实际 tag 表、属性名、自闭合规则、宽容处理和 `[br /]` 的当前退化行为替换 `docs/parser.md` 中未实现的历史示例；
2. 更新 `docs/api-design.md` 的类型关系和 `docs/architecture.md` 的输入数据流，说明 `ParsedText`、`ParsedParagraph` 与直接 `Vec<TextSpan>` 布局入口的边界；
3. 只在公开函数签名已经改变的示例中更新调用代码，不顺手重构 renderer 或无关示例；
4. 更新本文档状态和验证记录，审阅目标 diff 与编辑器诊断。

Phase 4 是本迭代的最终可运行状态。Phase 5 不新增布局或 renderer 能力。

### 完成结果与验证

实现采用手写 Unicode scalar scanner：标签头、空白、token、属性值和单双引号值使用一次顺序扫描；显式 frame 栈处理嵌套、自闭合节点、未闭合标签和恢复边界。公开 `ScalarOffset` 和属性范围均按 Unicode scalar value 计算。

本迭代已完成：

- 多属性、单双引号转义、自闭合标签和自定义开闭符号；
- `ParsedText`、`ParsedParagraph`、行内 scope 和对象的结构化 lowering；
- Tiqian 单段输入适配、局部文字样式和 rich-text scope 映射；
- `[br /]` 的段落结构保留。当前 `layout_parse` 仅消费第一段，并在存在后续段落时输出固定 `warn`；
- 当前普通 SDF glyph 输出，背景、线条、注音、装饰和对象图形维持 no-op。

已运行：

```text
cargo test parser::tests --lib
cargo test --test parser_custom_symbols
cargo test
```

截至 2026-09-19，以上命令均通过。完整测试包含 34 个库测试、5 个集成测试和 3 个 doctest；`parse_elements` 的 doctest 另有 1 个按标记忽略。重复 example target 的 Cargo warning 与本迭代无关。

### 最小必要测试

测试围绕新增语法的语义、结构边界和 Huozi 自己的输入降低编写；不为单个字段赋值、外部库行为或“其他功能未受影响”编写测试。

| 验证目标 | 最小测试 |
| --- | --- |
| 标签头 AST | 在 `parse_elements.rs` 以一个输入覆盖 `[size=24]`、命名属性、单双引号、`\\` / 引号转义和 `[br indent=2/]`；断言 `Block.attributes`、`SelfClosing.attributes` 的书写顺序、解码后的值和关键 scalar 范围。 |
| 语法恢复 | 在同一模块用一个未闭合引号或不匹配结束 tag 的输入，断言该标签状内容及其后的文字都作为有序 `Element::Text` 保留；不要求断言日志内容。 |
| 文字样式降低 | 在 `elements_to_document.rs` 用一个嵌套 `[span]`、旧 `[size]`、`stroke` 与 `shadow` 样例，断言生成的文字叶子拥有最终字号、paint 和 `SourceRange`。这个样例证明所有文字样式 tag 经过同一 `apply_text_attributes` 路径。 |
| 范围 scope 降低 | 用一个背景包裹 ruby、链接或装饰的样例，断言 `InlineNode::Scope` 的嵌套顺序、具名参数与内部文字来源范围；不逐项测试每种 Tiqian 枚举映射。 |
| 段落结构 | 用连续且带属性的 `[br /]` 样例，断言 `ParsedText.paragraphs` 的段数、空段、后续 `ParagraphStyleOverride` 和每段文字；不在本迭代断言多段布局结果。 |
| Tiqian 输入与来源映射 | 在 `tiqian_input.rs` 构造一个单段 `ParsedParagraph`，包含局部文字样式、背景或下划线、链接及对象；断言 `LayoutInput.content.text`、相关 `rich_text` / ruby / decoration / inline object 声明与 `HuoziSourceMap` 的 display range。不要断言 shaping、断行或绘制结果。 |

修改 parser、结构化表示或输入适配后，运行相关单元测试、`cargo test`、编辑器诊断和 `git diff --check`。本迭代不为 renderer no-op 添加“普通 glyph 未受影响”的测试；普通输出路径由现有集成测试和实际构建验证覆盖。

## 兼容性与回滚

本项目处于开发阶段，不承诺稳定的富文本 tag ABI；但本迭代明确保留已有 `[tag]...[/tag]`、`[tag=value]...[/tag]`、单双引号单值和 `[[` / `]]` 正文转义行为。

若新 AST 或输入适配无法保持现有字号、填充、描边和阴影的结果，应撤回该部分实现，恢复现有 parser 与 `TextRun` 降低路径。需求文档继续保留为后续重新实施的设计基线。
