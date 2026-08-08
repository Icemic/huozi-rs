# Huozi × Tiqian 术语表

- 状态：草案，等待 review
- 日期：2026-07-26
- 范围：Huozi、Tiqian、`tiqian-rs` 的排版、字体、测试和集成文档

本文统一三个项目之间的用词。它先记录现有事实，不为尚未定名的概念强行造词。

## 使用规则

1. **代码标识符服从代码。** 文档第一次提到公开类型时，同时写中文含义和代码名，例如“布局结果（`LayoutResult`）”；后文可按上下文使用其中一种。
2. **标准术语优先。** Unicode、OpenType、CLREQ 和 Rust 等已有术语沿用标准或项目既有译法。
3. **项目术语需要来源。** 只有已经出现在当前实现、架构文档或 ADR 中，并且含义稳定的名称，才标为“已采用”。
4. **普通机制不用命名。** 能用一句话直接说明的实现方式，不包装成新的首字母缩写、复合名词或“某某模型”。
5. **待定项不提前扩散。** 标为“待定”的词可以用于讨论，但在确定前不应写进公共 API、JSON 字段、crate 名或正式架构文档。
6. **中英文不机械一一对应。** `font`、`typeface`、`glyph`、`shaping` 等词先保证领域含义准确，再决定是否全部翻译。

状态含义：

- **已采用**：可以在新文档中直接使用。
- **工作用语**：当前讨论需要，但最终名称或边界仍可能调整。
- **待定**：请在 review 时决定名称或是否保留该概念。
- **不采用**：不作为项目术语；出现时改成已有术语或直白描述。

## 排版与文本

| 中文用语 | 代码或英文 | 含义 | 状态与来源 |
| --- | --- | --- | --- |
| 源文本 | source text | 调用者输入的原始文本。显示替换、软换行和断词不得改写它。 | 已采用；Tiqian 架构与 ADR 0003 |
| 显示文本 | display text / `displayText` | 送入字体成形或绘制的文本，可以按 profile 使用不同码点；始终保留到源文本的映射。 | 已采用；Tiqian ADR 0003、0008 |
| 源范围 | source range | 原始文本中的半开范围。`tiqian-rs` 约定以 UTF-8 字节偏移表示。 | 已采用；Tiqian、Huozi 现有模型及移植纪要 |
| UTF-8 字节偏移 | UTF-8 byte offset | 从 UTF-8 编码后文本起点计算的字节位置。不得与字符序号混用。 | 已采用；Huozi `SourceRange` 与移植纪要 |
| 字素簇 | grapheme cluster | Unicode 用户感知字符边界，用于断行安全、编辑语义和字体选择边界；不能替代字体成形。 | 工作用语；Unicode 概念，“字素簇”译名待 review |
| 字体成形 | shaping | 根据字体、文字、语言、方向和 OpenType 特性，把文本转换为字形及其位置。代码、模块名和较密集的技术段落可保留 `shaping`。 | 已采用；Tiqian ADR 0008 标题使用“字体成形” |
| 成形段 | shaping run | 共享字体实例、字号、文字、语言、方向和 OpenType 特性的连续成形输入。成形只保证在段内成立。 | 工作用语；移植纪要，中文名称待 review |
| 成形簇 | shaping cluster | 连接源范围、字素范围和字形范围的一组映射。 | 工作用语；移植数据契约，尚未进入 Rust API |
| 布局簇 | layout cluster | 排版核心用于断行、标点空间和行调整的基本单位。它不一定与字素簇或成形簇一一对应。 | 工作用语；移植数据契约，边界仍待领域模型确定 |
| 字形 | glyph | 字体中的可绘制图形单元，由 glyph ID 标识；它不等同于 Unicode 字符。 | 已采用；Tiqian 与 Huozi |
| 字形段 | glyph run / `GlyphRun` | 共享字体及成形属性的一组有序字形。 | 工作用语；`GlyphRun` 已采用，中文名称待 review |
| 字形 ID | glyph ID | 字体内部的字形编号。只有与字体身份一起才可唯一定位字形。 | 已采用；Tiqian、Huozi 目标链路 |
| 字形范围 | glyph range | 字形数组中的半开索引范围，不包含 glyph ID 本身。 | 工作用语；移植数据契约 |
| 前进量 | advance | 排下一个字形或布局单位前，笔位置沿行内方向移动的距离。成形前进量与布局调整后的前进量必须区分。 | 工作用语；项目当前多直接写 `advance`，中文译名待 review |
| 偏移 | offset | 字形相对笔位置的平移，通常由字体成形给出。 | 已采用；Tiqian shaping 模型 |
| 字形位置 | glyph placement | 字形 ID、字体身份、笔位置和偏移共同确定的最终绘制位置。 | 已采用；Tiqian 架构与 Huozi Roadmap |
| 墨迹边界 | ink bounds | 字形实际可见墨迹的外接边界，不等同于前进量或字身范围。 | 已采用；Tiqian 架构与 ADR |
| 基线 | baseline | 行内字形对齐所依据的参考线。 | 已采用；Tiqian |
| 行盒 | line box / `LineBox` | 一行的范围、基线、上下边界、宽度、缩进和结束原因等布局结果。 | 已采用；Tiqian `LineBox` |
| 版心 | measure | 一行正文可使用的布局宽度。当前文档也会出现 `maxWidth`，两者不应与最终视觉宽度混用。 | 已采用；Tiqian 架构与 ADR |
| 断行机会 | break opportunity | 某个文本边界是否允许、禁止或要求断行，以及相应原因和代价。 | 已采用；Tiqian `linebreak` |
| 强制换行 | mandatory break | 必须结束当前行的文本边界或控制字符。 | 已采用；Tiqian |
| 断行 | line breaking | 根据断行机会、版心和约束确定各行范围。 | 已采用；Tiqian |
| 避头尾 | kinsoku | 防止特定字符出现在行首或行尾的排版规则。代码和既有决策名可保留 `kinsoku`。 | 已采用；Tiqian、CLREQ 审计 |
| 断行修复 | line repair / repair | 初始断点违反避头尾或版心约束时，对断点或行边空间进行的具名处理，例如 PushIn、Hang、CarryPrevious、CarryNext。 | 已采用；Tiqian ADR |
| 行调整 | line adjustment / adjustment | 在断行合法后调整行内空间，包括压缩、拉伸和邻行均摊。 | 已采用；Tiqian 架构 |
| 两端对齐 | justification | 分配可调整空间，使正文行达到目标行宽。 | 已采用；Tiqian 架构 |
| 标点原子 | punctuation atom / `PunctuationAtom` | 排版核心中同时携带标点墨迹、主体宽度和两侧可调整空间的结构。 | 已采用；Tiqian ADR 0004 |
| 标点主体 | punctuation body / body | 标点占据的主体空间，与墨迹边界及两侧可调整空间分开。 | 工作用语；项目当前写 `body`，中文名称待 review |
| 可调整空间 | glue | 标点或字间可压缩、可拉伸的空间及其上下限。 | 工作用语；项目当前使用 `glue`，是否译为“间隔”“伸缩空白”或保留英文待 review |
| 字间自动间距 | autospace | 中西文字等边界按 profile 插入或调整的间距。 | 已采用；Tiqian ADR 0009，中文简称待 review |
| 显示替换 | display substitution | 不改变源文本和源范围，只改变送入成形与绘制的显示码点。 | 已采用；Tiqian ADR 0003 |
| 西文断词 | hyphenation | 在允许的位置拆分西文单词，并按需要显示连字符。 | 已采用；Tiqian ADR 0029 |
| 行内对象 | inline object | 由宿主提供 advance、ascent 和 descent，不经过普通文本成形的行内内容。 | 已采用；Tiqian |
| 注文 | ruby | 附在基文旁的注音或注释文字。模型名保留 `RubySpan`。 | 已采用；Tiqian |
| 注音 | Bopomofo | 使用注音符号的注文类型。 | 已采用；Tiqian |
| 标注 | annotation | decoration、ruby 等依附于源范围并产生语义或几何的内容的统称。 | 工作用语；现有文档用法较宽，边界待 review |
| 书写模式 | writing mode | 横排、竖排等文字行进和行堆叠方式。当前只实现简体中文横排。 | 已采用；Tiqian |
| 排版配置 | profile / `ClreqProfile` | 一组地区、严格度、标点、避头尾和行调整策略。 | 工作用语；“profile”是否统一译为“排版配置”待 review |

## 字体与平台边界

| 中文用语 | 代码或英文 | 含义 | 状态与来源 |
| --- | --- | --- | --- |
| 字体 | font | 项目目前对字体家族、具体字面和带参数字体实例的宽泛称呼。 | 已有用语，但边界待定；见下方 `font/typeface/font instance` 问题 |
| 字体家族 | font family | 由一组相关字面组成的家族名称，例如 Source Han Sans SC。 | 已采用；Tiqian `fontFamilies` |
| 字面 | typeface | 字体家族中的具体常规、粗体或斜体字面。 | 待定；中文字体行业译法及是否需要单独建模待 review |
| 字体实例 | font instance | 具体字体资源与 face、字号、变体轴等参数形成的可用于成形和绘制的实例。 | 工作用语；Huozi 后续 `FontManager` 设计尚未确定其身份边界 |
| 字体身份 | font identity | 用于证明成形、度量、栅格化和绘制使用同一字体的标识。 | 工作用语；具体由 `FontId`、稳定测试名称或平台句柄承载 |
| 字体 ID | `FontId` | Huozi 与 `tiqian-rs` 边界上传递的不透明字体标识。其构成和生命周期尚未确定。 | 工作用语；移植数据契约 |
| 字体选择 | font selection | 根据文字角色、样式和可用字体确定候选字体的过程。 | 已采用；Tiqian ADR 0001 |
| 字体回退 | font fallback / fallback | 首选字体不能满足请求时选择其他字体。 | 工作用语；项目当前通常直接写 `fallback`，中文译名待 review |
| 字体角色 | font role / `FontRole` | CJK 正文、CJK 标点、Latin、符号、Emoji 等用于字体策略的文本角色。 | 已采用；Tiqian `font` 模块 |
| 字体度量 | font metrics | ascent、descent、line gap 等字体提供的数值。 | 已采用；Tiqian、Huozi |
| 原始字体度量 | raw font metrics / `RawFontMetrics` | 平台或字体文件直接给出的度量，尚未按 CJK/Latin 混排规则归一化。 | 已采用；Tiqian |
| 布局字体度量 | layout font metrics / `LayoutFontMetrics` | 排版核心实际使用的归一化 ascent、descent 和基线信息。 | 已采用；Tiqian |
| 平台适配器 | platform adapter | 向核心提供字体成形、度量或绘制能力的平台实现。它不拥有断行、标点或对齐规则。 | 已采用；Tiqian 架构 |
| 后端 | backend | 实现字体选择、成形、度量等端口的一组能力。 | 工作用语；与“适配器”的边界待 review |
| 字体成形器 | text shaper / `TextShaper` | 接收成形输入并返回成形簇、字形段和诊断的接口或实现。 | 已采用；Tiqian shaping API |
| 渲染器 | renderer | 消费布局结果并绘制字形及标注几何的组件，不重新排版。 | 已采用；Tiqian、Huozi |
| 栅格化 | rasterization | 把字形轮廓转换为位图覆盖率的过程。 | 已采用；Huozi |
| 有向距离场 | SDF / signed distance field | Huozi 用于保存和绘制字形边界距离的纹理表示。文档中可直接使用 `SDF`。 | 已采用；Huozi |
| 字形图集 | glyph atlas | 存放多个栅格化字形或 SDF 的共享纹理。 | 已采用；Huozi |
| 绘制批次 | render batch | 可用同一组渲染状态提交的一组绘制数据。 | 工作用语；Huozi 目标架构，具体类型尚未确定 |
| 降级原因 | capability issue | 平台无法提供某项证据或能力时输出的结构化说明。 | 工作用语；代码名尚未确定，Tiqian 现有文档也使用 capability issue/report |

## 输入、输出与诊断

| 中文用语 | 代码或英文 | 含义 | 状态与来源 |
| --- | --- | --- | --- |
| 布局输入 | `LayoutInput` | 交给段落布局核心的文本、样式、段落配置、约束和行内语义。 | 已采用；Tiqian |
| 布局结果 | `LayoutResult` | 行、簇、字形位置、标注几何和诊断的唯一布局真值。 | 已采用；Tiqian |
| 布局调试信息 | `LayoutDebugInfo` | 字体、成形、标点、断行、修复和行调整等结构化决策。 | 已采用；Tiqian |
| 结构化决策 | structured decision | 具有明确字段和具名原因、可由程序读取的布局决策，不是拼接日志字符串。 | 已采用；Tiqian ADR 0005 |
| 布局转储 | layout dump | `LayoutResult` 和结构化决策的稳定文本表示，用于人工检查和测试差异。 | 已采用；Tiqian 测试体系 |
| 源映射 | source mapping | 从样式、簇、字形、行或绘制结果追溯到源文本范围的关系。 | 已采用；Tiqian、Huozi Roadmap |
| 输入转换 | lowering | 把 Compose、DOM、Huozi `Segment` 等宿主模型转换为布局输入。 | 工作用语；现有 Tiqian 文档直接使用 `lowering`，中文名称待 review |
| 重放 | replay | 下游按布局结果中的字形和几何绘制，或测试代码按已保存响应回答同一请求。具体对象必须写明，避免单独使用。 | 已采用动作词；Tiqian 架构和 ADR 0040 |
| 可解释布局 | explainable layout | 布局结果同时给出选择结果和结构化原因，使差异可定位到具体阶段。 | 已采用；Tiqian 项目原则 |

## 测试与移植

| 中文用语 | 代码或英文 | 含义 | 状态与来源 |
| --- | --- | --- | --- |
| 测试用例 | fixture | 一组固定输入以及验证该输入所需的预期数据。代码、目录名可保留 `fixture`，正文优先写“测试用例”。 | 已采用；Tiqian 测试体系 |
| 测试集 | corpus | 按清单组织、共同用于某项验收的一组测试用例。正文优先写“测试集”。 | 工作用语；测试工具草案，英文是否保留待 review |
| 预期结果 | golden | 经 review 接受、由自动化测试用于回归比较的预期输出。`golden` 可以用于文件或测试类型名称，不与 `corpus` 拼成新术语。 | 已采用；Tiqian `LayoutDumpGoldenTest` |
| 参考实现 | reference implementation | 移植期间用于确认既有行为的 Kotlin Tiqian 实现。参考 commit 必须记录。 | 已采用；移植纪要 |
| 参考输出 | reference output | 由参考实现生成并经 review 的布局结果或图片。 | 已采用；移植纪要 |
| 规范化输入 | normalized layout input | 从 Kotlin 与 Rust 内部类型转换成共同 JSON 结构的布局输入。 | 工作用语；移植纪要 |
| 规范化结果 | normalized layout result | 从 Kotlin 与 Rust 内部类型转换成共同 JSON 结构的布局结果和调试信息。 | 工作用语；移植纪要 |
| 后端请求与响应记录 | backend evidence | 布局期间发生的字体选择、成形和度量请求及其响应。正文优先使用完整中文，不把 `evidence` 扩展成更多复合术语。 | 工作用语；移植测试纪要 |
| 记录 | record / recording | 运行参考实现时保存后端请求及响应的动作。 | 普通动作词，不建立独立架构概念 |
| 按记录响应 | replay backend | 测试实现按已保存的完整请求查找并返回响应。正文应直接描述行为。 | 待定；接口名称和中文名称留待 backend trait 草案确定 |
| 自校验 | replay self-check | Kotlin 生成测试用例后，改用保存的请求与响应再次布局，并精确比较结果。 | 工作用语；移植测试纪要 |
| 对照测试 | comparison test | 在同一规范化输入和同一后端响应下比较 Kotlin 与 Rust 布局结果。 | 已采用；Huozi Roadmap |
| 差异报告 | diff report | 指出测试用例、字段路径、期望值、实际值和误差的报告。 | 已采用；Huozi Roadmap |
| 固定字体 | bundled test font | 随测试资产提供、以文件 hash 和 face index 锁定的字体，不依赖本机字体选择。 | 工作用语；测试工具讨论 |
| 能力覆盖表 | capability matrix | 列出需要验证的能力及其对应测试用例或单元测试。 | 工作用语；移植测试纪要，中文名称待 review |

## 待定名称与边界

以下内容需要 review 后再决定。当前只描述问题，不把候选名称当成正式术语。

### 1. `font`、`typeface` 与 `font instance`

需要确定三个项目是否严格区分：

- 字体家族；
- 字体文件中的具体 face；
- 带字号、变体轴和合成样式的运行时实例。

这会直接影响 `FontId`、fallback、缓存和测试 JSON 的命名。确定前不应使用“字体”之外的新中文词去暗示尚未建立的类型边界。

### 2. `cluster` 的中文译法

候选包括“簇”“群”“字群”，当前表暂用“簇”以贴近 Unicode 常见译法。需要确认：

- grapheme cluster；
- shaping cluster；
- layout cluster；
- Kotlin 当前 `Cluster`。

这些对象不是同一层次，不能都简称为“字符”。

### 3. `advance` 的中文译法

候选包括“前进量”“进宽”“推进量”或保留 `advance`。需要同时区分：

- shaping-time glyph advance；
- cluster natural advance；
- layout-adjusted advance；
- visual width。

### 4. `glue` 与 punctuation `body`

Tiqian 当前采用 `PunctuationAtom`、`body`、`leadingGlue`、`trailingGlue`。需要确认中文正文使用：

- “可调整空间”还是保留 `glue`；
- `body` 是“主体”“字身”还是其他名称；
- 是否需要在排版术语和代码标识之间维持固定对应。

### 5. `fallback`、`adapter` 与 `backend`

这三个词在现有文档中边界不完全统一：

- `fallback` 既指策略，也指实际改用另一字体；
- `adapter` 通常指平台接口实现；
- `backend` 有时指字体/成形能力集合，有时指具体测试实现。

需要在领域模型和 backend trait 草案中确定边界。术语表暂不替接口设计作决定。

### 6. 测试 JSON 的“规范化”与“确定性写法”

当前讨论里至少有两件事：

1. Kotlin/Rust 类型转换成共同数据结构；
2. 同一数据结构稳定地写成 JSON bytes。

两者不应都简称为 `canonical`。建议第一件继续叫“规范化输入/结果”，第二件暂写成“固定 JSON 写法”，最终名称待 review。

### 7. 后端请求的匹配方式

测试工具需要按“操作类型 + 完整规范化请求内容”查找保存的响应，不依赖全局调用顺序。这里是匹配规则，不建议命名为 `semantic-key backend replay`。

待确定内容：

- 接口和 JSON 字段使用 `requestId`、`requestKey` 还是其他名称；
- 是否需要给“按记录响应的测试后端”单独命名；
- 请求重复、冲突和未使用记录如何进入错误模型。

### 8. 测试集完成条件

测试集需要标明完整移植验收所要求的用例和能力，避免通过禁用困难用例绕过验收。当前草案曾使用 `completion profile`，不建议把它作为项目术语。

待确定是直接在测试集清单中保存：

- 必须执行的测试用例；
- 必须覆盖的能力；
- 或一组具名验收范围。

### 9. 固定字体的 Skia 使用方式

测试生成器必须从指定字体文件创建 typeface，并让成形、度量和绘制使用它，不访问系统字体。当前草案曾将其称为 `file-backed font session`，这只是实现要求，不建议建立新术语。

## 不采用的表达

| 表达 | 原因 | 应如何写 |
| --- | --- | --- |
| golden corpus | 把两个已有测试词拼成了没有额外含义的新概念。 | “测试集”“预期结果”或“用于回归测试的测试集” |
| semantic-key backend replay | 把一条请求匹配规则包装成架构名词。 | “按操作类型和完整请求内容查找已保存响应” |
| authoritative success corpus | “权威”“成功”没有给测试集增加可验证定义。 | “完整移植对照测试集”或直接说明由 Kotlin 参考实现生成 |
| completion profile | 只是测试集清单中的验收条件。 | “完整移植验收所需的用例和能力” |
| file-backed font session | 当前只是生成器如何加载固定字体的实现要求。 | “从指定字体文件加载，并供成形、度量和绘制共同使用” |
| backend contract test | 容易误解为独立测试类别或框架。 | “使用确定性测试后端的单元测试”，并写明验证对象 |
| canonical request | 容易与规范化输入、标准语义混淆。 | “按固定 JSON 写法序列化的完整后端请求” |
| canonical pretty JSON | 同时混合语义规范化、字段顺序和排版格式。 | 分别写“固定 JSON 字段顺序”和“2 空格缩进的 JSON” |

## Review 清单

请优先确认以下内容：

1. 项目总称使用“排版”“排印”还是按语境区分。
2. `grapheme cluster`、`shaping cluster`、`layout cluster` 的中文名称。
3. `advance`、`glue`、punctuation `body` 的中文名称。
4. `font`、`typeface`、`font instance` 是否建立严格区分。
5. `fallback`、`adapter`、`backend` 的中文译法和职责边界。
6. 测试工具中待定的请求匹配、固定 JSON 写法和测试集验收字段如何命名。

在以上项目 review 完成前，其他文档不根据本术语表批量改写。
