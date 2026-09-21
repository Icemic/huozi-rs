# Huozi 富文本标签语法

Huozi 使用标签将输入文本转换为 `ParsedText`。标签可以表达局部文字样式、富文本范围、行内对象和段落分隔；之后由 Tiqian 负责 shaping、断行与段落几何。

本文档描述当前实际接受的语法和 lowering 结果。新标签应同时更新本文件、`docs/architecture.md` 与相关测试。

## 目录

- [输入、入口与输出](#输入入口与输出)
- [标签头语法](#标签头语法)
- [属性值、重复项与继承](#属性值重复项与继承)
- [文字样式标签](#文字样式标签)
- [范围标签](#范围标签)
- [位置标签和段落](#位置标签和段落)
- [来源范围与 Segment](#来源范围与-segment)
- [恢复规则](#恢复规则)
- [渲染边界](#渲染边界)

## 输入、入口与输出

默认标签符号为 `[` 和 `]`。以下 API 解释标签：

| API | 输入 | 输出或行为 |
| --- | --- | --- |
| `parse` | 一个 `Segment` | `Vec<Element>` 语法树。 |
| `parse_with::<OPEN, CLOSE>` | 一个 `Segment` 和自定义单字符符号 | `Vec<Element>` 语法树。 |
| `Huozi::parse_text` | `Vec<Segment>` | `ParsedText`，保留全部逻辑段落。 |
| `Huozi::parse_text_with` | `Vec<Segment>` 和自定义符号 | `ParsedText`，保留全部逻辑段落。 |
| `Huozi::layout_parse` | `Vec<Segment>` | 解析后仅布局第一段。 |
| `Huozi::layout_parse_with` | `Vec<Segment>` 和自定义符号 | 解析后仅布局第一段。 |
| `Huozi::layout_plain` | `Vec<Segment>` | 不解释标签，原样布局。 |

`parse_with` 的 `OPEN` 和 `CLOSE` 是编译期常量字符，例如 `parse_with::<'<', '>'>(...)`。同一程序应固定使用一组标签符号。正文中的双开符号和双闭符号分别表示字面量开闭符号；例如使用 `<`、`>` 时，`<<` 输出 `<`，`>>` 输出 `>`。

`ParsedText` 由 `Vec<ParsedParagraph>` 组成。每个段落包含按输入顺序排列的 `InlineNode`：

- `Text(TextRun)`：可直接显示的文本及其局部 `TextStyle` 和 `SourceRange`；
- `Scope { kind, children }`：背景、线条、注音、装饰、链接等覆盖子节点的范围语义；
- `Object(InlineObject)`：在当前位置插入的替代文本和对象度量。

`TextSpan` / `TextRun` 仍用于直接 `Huozi::layout` 的手工输入，但不再是标签 parser 的公开结果。

## 标签头语法

### 成对标签

```text
[tag]内容[/tag]
[tag=value]内容[/tag]
[tag key=value key2="带空格的值"]内容[/tag]
```

开始标签与结束标签必须完全相同，并按后开先闭的顺序嵌套。标签名和属性名区分大小写；例如 `lineThrough`、`fillColor`、`shadowOffsetX` 必须按此拼写。

### 自闭合标签

```text
[tag /]
[tag key=value /]
[tag/]
[tag key=value/]
```

自闭合标签只在当前位置插入一个节点，不包含内容。当前仅 `br` 和 `object` 有语义；其他自闭合标签会记录 `warn` 后跳过。

### 兼容单值形式

`[tag=value]` 会先转换为标签的主属性，再和后续命名属性按书写顺序处理。

| 标签 | 单值对应的属性 |
| --- | --- |
| `size` | `size` |
| `color`、`fillColor`、`background`、`underline`、`lineThrough` | `color` |
| `stroke` | `stroke` |
| `strokeColor` | `strokeColor` |
| `strokeWidth` | `width` |
| `shadow` | `shadow` |
| `shadowColor` | `shadowColor` |
| `shadowOffsetX` | `offsetX` |
| `shadowOffsetY` | `offsetY` |
| `shadowBlur` | `blur` |
| `shadowWidth` | `width` |
| `font` | `family` |
| `weight` | `weight` |
| `fontSynthesis` | `fontSynthesis` |
| `locale` | `locale` |
| `baseline` | `baseline` |
| `attach` | `attach` |
| `ruby`、`bopomofo` | `text` |
| `link` | `target` |
| 其他标签 | 与标签同名 |

因此 `[color=red]` 与 `[color color=red]` 有相同效果；`[ruby="tí qiàn"]` 与 `[ruby text="tí qiàn"]` 有相同效果。

### 正文转义

正文中：

| 输入 | 显示文本 |
| --- | --- |
| `[[` | `[` |
| `]]` | `]` |
| `[[bold]]` | `[bold]` |

转义只在正文中生效。属性值内的标签符号按普通字符处理；需要空白、引号或标签符号时应使用引号值。

## 属性值、重复项与继承

属性格式为 `name=value`。标签名、属性名、等号和属性值前后允许 ASCII 空白字符：空格、制表符、回车和换行。引号值本身不能换行。

### 未引用值

未引用值不能包含空白、当前开闭符号、`=`、`/`、单引号或双引号：

```text
[span size=24 locale=zh-Hans]文本[/span]
```

### 引号值和转义

单、双引号都可用：

```text
[font family="Source Han Sans SC, Inter"]文本[/font]
[ruby text='tí qiàn']提椠[/ruby]
```

在双引号值中，`\"` 表示双引号，`\\` 表示反斜杠；在单引号值中，`\'` 表示单引号，`\\` 表示反斜杠。其他反斜杠转义和未闭合引号会使整个标签头降级为正文。

### 重复属性和样式覆盖

同一标签中的属性按书写顺序应用；同一个可识别字段后来者覆盖前者：

```text
[span size=20 size=28 color=red color=blue]文本[/span]
```

上例最终使用字号 `28` 和填充色 `blue`。嵌套时，内层样式仅覆盖其内容；离开内层后恢复外层样式：

```text
[span size=20 color=red]外层[span color=blue]内层[/span]外层[/span]
```

`style_prefabs` 仅对非内建的成对标签生效。若标签名在预设表中，预设 `TextStyle` 作为该标签内容的完整样式；内建标签始终优先按本文件的规则处理。

未知属性、无法转换的属性值和未识别的标签会记录 `warn`，不会中止其他属性或正文的处理。

## 文字样式标签

文字样式标签直接影响子文本的 `TextStyle`。`span` 可组合下表列出的全部文字样式属性；其他文字样式标签也可接收其适用属性。

| 标签 | 主要属性 | 实际规则 |
| --- | --- | --- |
| `span` | 下表全部文字样式属性 | 组合多个局部样式。 |
| `size` | `size` | `f64` 字号。 |
| `color`、`fillColor` | `color` | 填充色，使用 `csscolorparser` 接受的颜色字符串。 |
| `stroke` | 单值 `stroke`；`color`、`width` | 单值使用 `<color>`、`<width>` 或 `<color> <width>`。 |
| `strokeColor` | 单值 `strokeColor`；`color` | 创建或覆盖描边颜色。 |
| `strokeWidth` | 单值/`width` | 创建或覆盖描边宽度。 |
| `shadow` | 单值 `shadow`；`color`、`offsetX`、`offsetY`、`blur`、`width` | 单值使用 `<x> <y> [blur] [color] [width]`。 |
| `shadowColor` | 单值 `shadowColor`；`color` | 创建或覆盖阴影颜色。 |
| `shadowOffsetX` | 单值/`offsetX` | 创建或覆盖阴影横向偏移。 |
| `shadowOffsetY` | 单值/`offsetY` | 创建或覆盖阴影纵向偏移。 |
| `shadowBlur` | 单值/`blur` | 创建或覆盖阴影 blur 半径。 |
| `shadowWidth` | 单值/`width` | 创建或覆盖阴影扩张半径。 |
| `font` | 单值 `family`；`font`、`family` | 逗号分隔字体族列表，空项忽略；已注册 family 构成候选域。CJK 与 Latin 文本角色会在该列表内部优先尝试调用方声明的对应 `FontSourceKind`，不会选择列表外 family。 |
| `weight` | 单值 `weight`；`weight` | `i32` 字重；布局时限制到 `1..=1000`，匹配静态 face 或 variable `wght`。 |
| `bold` | 可选 `weight` | 无属性时设为 `700`；带属性时仅处理 `weight`，后续选择规则与 `weight` 相同。 |
| `italic` | 可选 `italic` 或 `enabled` | 无属性时设为 `true`；带属性时解析 `bool`，匹配 italic/oblique face 或标准斜体轴。 |
| `fontSynthesis` | 单值 `fontSynthesis`；`fontSynthesis` | `none`、`weight`、`style` 或 `all`。控制缺少真实能力时允许的仿粗/仿斜 fallback；默认 `all`，局部值整体覆盖继承值。 |
| `locale` | 单值 `locale`；`locale` | 设置 locale。 |
| `baseline` | 单值 `baseline`；`baseline` | `f32` 基线偏移；负值向上。 |
| `attach` | 单值 `attach`；`attach` | `none` 或 `previous`。 |

文字样式示例：

```text
[span size=28 color="#F59E0B" shadowColor="#0008" shadowOffsetX=1 shadowOffsetY=2 shadowBlur=4]警告[/span]
[font="Source Han Sans SC, Inter"][weight=700]混排[/weight][/font]
[fontSynthesis=none]保持最近真实字体[/fontSynthesis]
[span fontSynthesis=style][italic]仅允许仿斜[/italic][/span]
[stroke color=white width=2]描边[/stroke]
[shadow offsetX=1 offsetY=2 blur=4 color="#0008"]阴影[/shadow]
[baseline=-4]上标位置[/baseline]
```

`stroke` 的宽度、`shadow` 的 blur 半径和扩张半径要求有限且非负；单独的 `shadow` 横纵偏移允许负值。颜色、数字和布尔值无效时保留当前字段值。

## 范围标签

范围标签在 `InlineNode::Scope` 中保留语义；可与文字样式标签任意嵌套。

### 背景、下划线和删除线

| 标签 | 属性 | 值与默认值 |
| --- | --- | --- |
| `background` | `color` | 背景填充色，默认黑色。 |
|  | `strokeColor`、`strokeWidth` | 可选背景描边。 |
|  | `shadowColor`、`shadowOffsetX`、`shadowOffsetY`、`shadowBlur`、`shadowWidth` | 可选背景阴影。 |
|  | `paddingX`、`paddingY`、`radius`、`continuationRadius`、`clearance` | `f32` 几何参数；除 `continuationRadius` 外默认 `0`。 |
|  | `metricPolicy` | `markedFaces`、`uniformTextStyle` 或 `uniformParagraphStyle`；默认 `markedFaces`。 |
| `underline`、`lineThrough` | `color` | 线条填充色，默认黑色。 |
|  | `strokeColor`、`strokeWidth`、`shadowColor`、`shadowOffsetX`、`shadowOffsetY`、`shadowBlur`、`shadowWidth` | 可选线条 paint。 |
|  | `thickness`、`clearance` | `f32`；厚度默认 `1`，净空默认 `0`。 |
|  | `pattern` | `solid`、`dashed` 或 `dotted`；默认 `solid`。 |
|  | `dashLength`、`gapLength` | 虚线或点线长度；未指定时为 `1`。 |

`dashLength`、`gapLength` 可以写在 `pattern` 之前或之后。属性全部读取后再确定最终线条模式。

```text
[background color="#FFF3BF" paddingX=6 paddingY=2 radius=4]提示[/background]
[underline color="#1677FF" thickness=1 pattern=dashed dashLength=3 gapLength=2]链接[/underline]
[lineThrough color="#666" thickness=1]过期内容[/lineThrough]
```

### 注音、装饰和语义范围

| 标签 | 属性 | 规则 |
| --- | --- | --- |
| `ruby` | `text`、`font`、`locale` | `text` 必填且非空；默认 pinyin。 |
| `bopomofo` | `text`、`font`、`locale` | `text` 必填且非空；使用 bopomofo。 |
| `emphasis` | 无 | 着重号范围。 |
| `mourning` | 无 | 示亡号范围。 |
| `properNoun` | 无 | 专名号范围。 |
| `bookTitle` | 无 | 书名号范围。 |
| `link` | `target` | `target` 必填；保留链接语义。 |
| `technical` | 无 | 技术文本断行范围。 |
| `noAutoSpace` | 无 | 抑制自动间距。 |
| `box` | `start`、`end`、`spacing` | 两端附加空间；`spacing` 为 `narrow` 或 `source`，默认 `narrow`。 |

`ruby`、`bopomofo` 缺少或提供空 `text` 时不创建 scope。`link` 缺少 `target` 时不创建链接 scope；空字符串 `target=""` 仍作为链接语义保留。`box` 的属性值无效时保留对应字段的默认值，并继续创建行内盒 scope。

### 行内代码

`code` 同时创建 `InlineCode` scope，并将**所有属性**按 `span` 规则应用到代码文本样式、按 `background` 规则应用到代码背景。因此相同的属性名称可能分别影响文字和背景。

```text
[code font="monospace" color="#1E1E23" paddingX=4 paddingY=2 radius=3]cargo test[/code]
```

在上例中，`font` 和 `color` 设置代码文字；`color`、`paddingX`、`paddingY`、`radius` 也设置代码背景。`code` 的文字和自身背景当前共享 `color` 属性；需要不同视觉效果时，应使用嵌套的文字样式或额外 `background` 范围，并注意当前 renderer 不绘制背景几何。

## 位置标签和段落

### `[br /]`

`br` 结束当前逻辑段落并创建下一段。段首、段尾和连续 `br` 都会保留空段落。`br` 属性覆盖后续段落的样式，未指定字段继承前一段：

| 属性 | 类型和值 |
| --- | --- |
| `indent` | `f32`，首行缩进。 |
| `lineHeight` | `f32`，绝对行高。 |
| `blockIndent` | `f32`，所有行的起始缩进。 |
| `lastLineAlignment` | `start`、`center`、`end`。 |
| `lineLengthGrid` | `bool`。 |
| `rubyLineHeightMode` | `perLine`、`uniformParagraph`。 |
| `inlineObjectMinimumClearance` | `f32`。 |
| `emphasisDotGap` | `f32`。 |

```text
第一段[br indent=2 lineHeight=36 blockIndent=1 /]第二段
```

`parse_text` 与 `parse_text_with` 保留全部 `ParsedParagraph`。当前 `layout_parse` 和 `layout_parse_with` 只把第一段交给 Tiqian，并在存在后续段落时输出固定 `warn`；多段布局输出尚未提供。

### `[object /]`

`object` 需要下列属性：

| 属性 | 语义 |
| --- | --- |
| `alt` | 非空替代文本；用于布局、复制、搜索和无障碍语义。 |
| `width` | `f32` advance。 |
| `ascent` | `f32` ascent。 |
| `descent` | `f32` descent。 |

```text
[object alt="图标" width=16 ascent=12 descent=4 /]
```

缺少任一必填属性、属性无法解析或 `alt` 为空时，当前对象会跳过。对象的 `leadingBoundary` 与 `trailingBoundary` 当前固定为 `Fixed`；标签尚未提供对应属性。

## 来源范围与 Segment

`SourceRange` 的 `start` 与 `end` 是相对于单个 `Segment.content` 的 Unicode scalar 半开范围 `[start, end)`。它们指向原始输入坐标；显示文本使用另一套坐标。

```text
Segment.content: [color=red]甲[/color]
显示文本:      甲
SourceRange:   [11, 12)
```

`Segment.id` 会复制到每个由该 segment 产生的 `Element`、`TextRun` 和 `InlineObject.source_range`。`layout_parse` 进一步将显示文本范围映射到 `SegmentGlyphSpan`。

`Huozi::parse_text` 对每个 `Segment` 独立执行 `parse`，然后才将元素按输入顺序交给 lowering。因此开始标签和结束标签必须在同一个 segment 内；不能把 `[color=red]` 放在一个 segment，再把 `[/color]` 放在下一个 segment。`[br /]` 生成的段落结构可以跨 segment 继续。

## 恢复规则

parser 采用一次从左至右的 Unicode scalar 扫描和显式 frame 栈。其复杂度为 $O(n)$，不会因标签嵌套使用 Rust 递归调用栈。

### 语法恢复

| 输入情况 | 结果 |
| --- | --- |
| 未知成对标签 | 保留子内容；lowering 记录 `warn`，并继续降低子内容。 |
| 未知自闭合标签 | 记录 `warn`，不产生节点。 |
| 错配结束标签 | 结束标签作为正文文本。 |
| 未闭合外层标签 | 外层开始标签和已收集正文作为文本；其中完整闭合的内层 block 仍保留。 |
| 标签头缺少属性值或属性名 | 无法形成标签头的部分作为正文；在下一个候选开符号前继续。 |
| 引号未闭合或转义无效 | 当前标签头到输入结束作为正文；其中内容不再重新解释为标签。 |
| 正文的双开/闭符号 | 转换为一个字面量符号。 |

例如：

```text
[a]前[good]后[/good]
```

产生正文 `[a]前` 和已闭合的 `good` block。又如：

```text
[bad attr][good]后[/good]
```

产生正文 `[bad attr]` 和已闭合的 `good` block。

### lowering 恢复

语法树已形成后，lowering 对每项无效属性记录 `warn` 并保留可用部分：

- 重复可识别属性按顺序覆盖；
- 单项颜色、数字、布尔值或枚举值无效时保留已有字段；
- 空 ruby/bopomofo、缺少 `target` 的 link、无有效对象度量的 object 不创建相应范围或对象；
- 当前 renderer 未消费的范围语义仍保留到 Tiqian 输入。

## 渲染边界

背景、下划线、删除线、注音、CLREQ 装饰、链接、技术文本、自动间距、行内代码、行内盒和对象都会保留并传给 Tiqian。当前 Huozi 输出适配器只为普通文本生成 fill、stroke 和 shadow 的 SDF glyph 顶点。

因此当前效果如下：

| 功能 | Tiqian 布局 | Huozi 顶点输出 |
| --- | --- | --- |
| 字号、字体族、字重、斜体、fontSynthesis、locale、基线、附着、技术断行、自动间距、行内盒 | 生效 | 普通文字 glyph。 |
| 背景、下划线、删除线 | 参与 rich-text layout | 不生成对应几何。 |
| 注音和 CLREQ 装饰 | 参与 rich-text layout | 不生成对应几何。 |
| 链接 | 保留语义 | 不处理导航或专属视觉。 |
| 行内代码 | 文字样式和技术断行生效 | 背景不生成几何，文字按普通 glyph 输出。 |
| 行内对象 | 参与布局 | 不生成对象图形。 |
