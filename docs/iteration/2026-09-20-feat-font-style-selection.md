# 字体族、字重与斜体选择

> 状态：已完成
>
> 日期：2026-09-20
>
> 分类：feat

## 文档用途

本文定义 Huozi 普通文字字体选择的独立迭代，使 parser 已支持的 `[font]`、`[weight]`、`[bold]` 和 `[italic]` 能改变最终 shaping、字体度量、glyph bounds 与 SDF 轮廓。

本文是本迭代讨论和实施的唯一依据。当前行为、目标、范围、数据结构、已确认方案、实施阶段和验证要求均记录在本文中，不依赖聊天记录或临时记忆。

富文本布局结果输出迭代 `docs/iteration/2026-09-19-feat-rich-text-layout-output.md` 在本迭代完成前暂停。背景、线条、注音输出、装饰、对象和链接不属于本文范围。

## 背景

Huozi 的 `TextStyle` 已包含：

```text
font_families: Vec<String>
font_size: f64
locale: String
font_weight: i32
italic: bool
baseline_shift: f32
inline_attachment: InlineAttachment
fill_color、stroke、shadow
```

parser 已将：

- `[font]` 写入 `font_families`；
- `[weight]` 写入 `font_weight`；
- 无属性 `[bold]` 写入 `font_weight = 700`；
- `[bold weight=...]` 写入指定字重；
- `[italic]` 写入 `italic = true`；
- `[italic enabled=...]` 写入指定布尔值。

`HuoziTiqianInputAdapter` 会把字体族、字号、locale、字重、斜体、基线偏移和附着关系完整转换为 Tiqian `TextStyle` 或 `TextStyleOverride`。直接 `layout(Vec<TextSpan>)` 与结构化 `layout_parse*` 当前都使用这条完整样式转换。

Tiqian 在每次 `FontBackendRequest` 中向 Huozi 字体后端提供：

- source text、display text 与 shaping range；
- `TextStyle`，包含 `font_families`、`font_weight`、`italic`、`font_size` 和 `locale`；
- `FontRole`；
- Tiqian 决定的 OpenType feature。

尚未完成的部分位于 `HuoziFontManager`：它虽然登记了每个静态 face 的别名、字重和斜体属性，但 `shape()` 仍只按 `FontSource` 注册顺序尝试 face，选择第一个 shaping 后没有 glyph id `0` 的候选。请求中的字体族、字重和斜体不参与候选顺序。所有 face 的 `FontFaceId` 都使用 `FontVariationInstance::default()`，HarfRust shaping、SkRifa glyph bounds 和 SDF 轮廓也都使用默认 variation 位置。

因此当前实际行为是：

| 样式 | 当前数据状态 | 当前最终效果 |
| --- | --- | --- |
| 字体族 | 已进入 `FontBackendRequest.style.font_families`。 | 不改变候选顺序；通常仍选择注册顺序中第一个完整覆盖文字的 face。 |
| 字重 | 已进入 `font_weight`。静态 descriptor 也记录 face 的实际 weight。 | 不匹配静态 face，不设置 variable font `wght` 轴，也不合成加粗。 |
| `bold` | parser 转成字重请求。 | 与其他字重请求相同，通常不会改变最终字形。 |
| 斜体 | 已进入 `italic`。静态 descriptor 也记录 face 是否 italic。 | 不匹配 italic/oblique face，不设置 `ital`/`slnt` 轴，也不合成倾斜。 |
| 字号 | 进入 shaping 与 SDF 四边形缩放。 | 已生效；本迭代保持现有行为。 |
| locale | 进入 HarfRust buffer language。 | 已生效；本迭代保持现有行为。 |

## 当前字体数据与调用链

### `FontSource`

公开输入类型当前为：

```text
FontSource
├─ bytes: Vec<u8>
└─ alias: Option<String>
```

`FontSource::new(bytes)` 没有显式别名；`FontSource::with_alias(bytes, alias)` 提供一个调用方指定的名称。`HuoziFontManager::from_sources` 依次展开每个字体文件或字体集合中的 face。

当前 `alias` 同时承担：

- `FontFaceId.resource_id` 的可读部分；
- `ReplayableFontFaceDescriptor.family_aliases` 中唯一的 family 名称；
- capability report 的 source label。

未提供 alias 时使用 `font-source-{source_index}`。当前不会读取字体 name table 建立字体族名称，也不会把一个来源映射到多个 family alias。

### `FontFaceRecord` 与 descriptor

内部 `FontFaceRecord` 当前保存：

```text
FontFaceRecord
├─ id: FontFaceId
├─ source_index
├─ collection_index
├─ bytes
├─ units_per_em
├─ ascent、descent、leading
└─ typo_ascent、typo_descent
```

`ReplayableFontFaceDescriptor` 另行保存：

```text
id
family_aliases
roles
weight
italic
source_label
```

weight 与 italic 来自 SkRifa `Attributes`。当前候选循环遍历 `faces`，没有把 descriptor 的 family、weight 或 italic 用于排序。`faces` 与 `descriptors` 下标按注册时一一对应，但 `FontFaceRecord` 自身没有保存这些匹配属性。

### 字体选择与完整 shaping

一次 `FontBackend::shape` 请求的当前流程：

```text
FontBackendRequest
  → 按注册顺序遍历 FontFaceRecord
  → 每个候选执行完整 HarfRust shaping
  → 统计 glyph id 0
  → 第一个无缺字候选成为最终 face
  → 全部缺字时保留第一个候选结果
  → 写入 FontFaceId、GlyphRun、FontResolution 和候选尝试
```

Tiqian ADR R0003 固定了以下约束：

1. 字体候选选择与完整 shaping 是一次原子操作；不得先用 cmap 预检查替代最终 shaping。
2. 第一个完整 shaping 后不含 glyph id `0` 的候选获选，随后停止尝试。
3. 全部候选缺字时返回首选候选的 shaping 结果。
4. shaping、metrics、glyph bounds 和 glyph replay 必须使用同一个 `FontFaceId`。
5. `FontFaceId` 必须标识物理 SFNT face、collection index 和 variation instance；字号不属于 face identity。
6. 每个 shaping request 只产生一个最终 face。

本迭代只能改变候选的构造和顺序，以及候选所使用的 variation instance，不改变上述原子选择与 fallback 规则。

### variation 与回放

`FontFaceId` 已包含 `FontVariationInstance`，后者保存排序、去重后的轴 tag 与 `f32` 值。因此 Tiqian 和 Huozi 的 atlas key 已能区分同一物理 face 的不同 variation instance。

但 Huozi 当前始终创建默认 variation instance：

- HarfRust shaper 没有设置 variation 位置；
- `glyph_ink_bounds` 使用默认位置；
- `draw_outline` 使用默认位置；
- color glyph bounds 使用默认位置；
- `raw_metrics` 直接读取默认实例的 `hhea` / `OS/2` 静态字段；
- SDF atlas 以默认实例的 `FontFaceId + glyph_id` 缓存。

如果本迭代接入 variable font，必须让候选身份、shaping、bounds、metrics 与轮廓绘制使用同一组轴值，不能只在其中一个阶段设置轴。

## 目标

1. `[font]` 的字体族顺序实际决定候选 family 顺序。
2. `[weight]` 与 `[bold]` 实际改变最终采用的静态 face 或 variable font instance；缺少对应能力时退化到最近的非合成 face。
3. `[italic]` 实际改变最终采用的静态 face 或 variable font instance；缺少对应能力时退化到最近的非合成 face。
4. 未指定字体族时继续使用调用方提供的 `Vec<FontSource>` 顺序作为 family fallback 顺序。
5. 在同一 family 内按已确定的 weight 与 italic 规则选择候选，再以完整 shaping 结果决定是否 fallback。
6. 最终 `FontFaceId` 精确表示实际物理 face；若使用 variation instance，则同时精确表示实际轴位置，并贯穿 metrics、glyph bounds、SDF 栅格化与 atlas key。
7. 普通正文、技术文本、行内代码以及 Tiqian 发出的其他文字 shaping 请求共用同一字体选择规则。
8. 保留当前静默退化原则：缺少理想样式时选择可用候选，不因字体配置不完整而中止布局。
9. 以自动测试和 `examples/render` 人工样例证明 family、weight、bold 与 italic 会改变最终 face 或 variation instance，并保持 measure 与 draw 一致。

## 范围

### 包含

- Huozi 字体目录所需的 family、weight 与 italic 元数据，以及已确认方案需要的 variation 元数据；
- 根据 `FontBackendRequest.style` 构造有序候选；
- 静态字体文件和字体集合成员的 face 匹配；
- 按已确认方案为现有 `font_weight` 与 `italic` 请求选择静态 face 或 variable font instance；
- 如果使用 variation instance，保证 `FontFaceId`、HarfRust shaping、SkRifa bounds、metrics 和轮廓回放一致；
- 未找到请求 family、理想 weight 或 italic face 时的退化顺序；
- 候选尝试信息与 capability report 的准确性；
- 必要的单元测试、布局验证和示例验证；
- 实施完成后更新 `docs/architecture.md`、`docs/parser.md` 和 `docs/rendering-gap-analysis.md` 的实际行为说明。

### 不包含

- 新增 parser 标签或属性；
- 任意字体轴的公开用户配置 API；
- 字距、词距、文本变换、small caps 或自定义 OpenType feature 标签；
- 字体下载、系统字体发现、按平台名称查询字体或热更新字体目录；
- 修改 Tiqian 的 `FontBackend`、`FontFaceId`、fallback 原子规则或每次请求单 face 的约束；
- 背景、线条、ruby/bopomofo 输出、CLREQ 装饰、对象、链接或多段布局结果；
- 彩色 glyph 图层绘制；
- 因 SDF 以外后端而新增另一套字体选择规则。
- synthetic bold 与 synthetic oblique；该能力作为后续独立迭代的候选工作。

## 后续 TODO：软件合成字体样式

后续可建立独立迭代，评估在 family 缺少合适静态 face 和 variable axis 时生成 synthetic bold 与 synthetic oblique。本迭代只保留最近的非合成 face，不实现软件合成，也不新增控制合成行为的公开样式字段。

后续迭代需要完整解决以下问题：

1. 规定触发 synthetic bold 与 synthetic oblique 的候选顺序和合成参数。
2. 为合成结果建立可回放身份，区分物理 face、variation instance 与软件合成参数。
3. 让 shaping 结果中的 ink bounds、SDF 轮廓、atlas cache key 和最终绘制使用同一组合成参数。
4. 规定 synthetic bold 与现有文字 stroke、SDF buffer 和 glyph advance 的关系。
5. 评估是否需要公开类似 `font-synthesis` 的开关，使调用方可以按文字或语言禁用合成。

## 已确认的设计约束

### 单一候选序列

每个请求只构造一次有序候选序列，`shape()` 按该序列执行完整 shaping。family、weight、italic 与 variation 的选择不能分别建立互相矛盾的路径。

候选至少需要包含：

```text
FontCandidate
├─ face record
├─ FontFaceId（含 variation instance）
├─ family 匹配依据
├─ static weight / italic 属性
└─ 实际用于 shaping 与 replay 的 variation location
```

是否新增具名内部 struct 由实施时按复用程度决定；本文只固定候选必须携带的数据，不要求为一次调用的局部数据过度封装。

### family fallback 与 glyph fallback 的顺序

`font_families` 是调用方声明的首选 family 顺序。每个 family 内可以有多个静态 face 或 variable instance。候选顺序必须先反映 family 意图，再由完整 shaping 检查字符覆盖。

没有请求 family 或请求 family 均未注册时，必须退化到字体来源注册顺序，确保已有调用方仍能布局。family 未命中不能产生空候选或 panic。

family 名称来自字体 name table，至少收集 typographic family、family 与兼容 family 名称；调用方传给 `FontSource::with_alias` 的 alias 作为额外名称。名称在登记和查询时去除首尾空白，并使用 ASCII 不区分大小写的比较。同一名称匹配多个来源或集合成员时，它们按 `FontSource` 与 `collection_index` 的注册顺序组成该 family 的 face 集合。`FontSource::new` 注册的字体也可通过 name table 中的 family 名称被 `[font]` 点名。

### weight 与 italic 属于 face 选择

字重和斜体必须在 shaping 前决定，因为它们可能改变 glyph id、advance、kerning、bounds 和字体度量。不能在 SDF 绘制阶段只扩大轮廓或倾斜顶点来假装完成字体样式。

如果选择静态 face，其 `FontFaceId` 使用默认 variation instance。如果选择 variable font instance，其 `FontFaceId` 必须保存实际轴值。

静态 face 的 weight 使用 CSS Fonts 字体匹配规则。请求值先限制到 `1..=1000`，再按 CSS Fonts 对 400、500 和其他字重规定的方向性顺序寻找可用 face；同一匹配位置有多个 face 时保持注册顺序。

字体目录内部保留 normal、italic、oblique 三种静态 style，不能只用 `ReplayableFontFaceDescriptor.italic` 的布尔值完成候选排序。`italic = true` 时按 `italic → oblique → normal` 搜索；`italic = false` 时按 `normal → oblique → italic` 搜索。style 优先级先于同一 family 内的 weight 排序：先选择当前 style 档的最佳 weight；该 style 没有任何 face 时才进入下一 style 档。

本迭代支持静态 face 与 variable font 的标准样式轴，不生成 synthetic bold 或 synthetic oblique。静态 face 和 variable font instance 都作为所属 family 中的候选，按同一 style 与 weight 规则排序；缺少理想 face 或轴时退化到最近可用的非合成实例。

variable font 的 `wght` 使用限制到 `1..=1000` 后的请求字重，并再次限制到字体声明的轴范围。`italic = true` 时优先使用 `ital=1`，并限制到该轴范围；字体没有 `ital` 但有 `slnt` 时使用 `slnt=-14`，并限制到该轴范围。字体同时具有 `ital` 和 `slnt` 时只设置 `ital`，`slnt` 保持默认值。`italic = false` 时两个轴均使用默认值。没有对应轴时按字体的静态 style 参与既定退化顺序。

### 同一身份贯穿 measure 与 draw

以下操作必须从同一个 `FontFaceId` 解析同一个字体位置：

1. HarfRust shaping；
2. glyph ink bounds；
3. Tiqian `FontMetricsRequest`；
4. outline 或 color glyph bounds；
5. SDF 轮廓栅格化；
6. atlas cache key。

禁止 shaping 使用非默认 variation，而 bounds、metrics 或轮廓仍使用默认位置。

### 性能

- 不对每个 shaping request 重新解析字体文件、name table、axes 或静态 attributes。
- family 与样式匹配使用字体目录建立时缓存的元数据。
- 候选排序不得依赖对所有候选执行额外 cmap 扫描；覆盖判断继续复用完整 shaping 结果。
- 避免在热路径 clone 字体 bytes、family 集合或 variation settings；允许为最终 `FontFaceId` 和 Tiqian 结果进行必要的轻量 clone。
- 候选构造目标复杂度为相对于注册 face 数量的 $O(n)$；不得为每个 family 重复扫描全部 face 形成 $O(f \times n)$。

## 已确认的候选顺序

每个 shaping request 按以下顺序构造候选：

1. 按 `font_families` 的声明顺序查找 family；没有请求 family 或全部未命中时，使用字体来源注册顺序。
2. 在一个 family 内按请求 italic 状态选择 style 档：italic 请求使用 `italic → oblique → normal`，normal 请求使用 `normal → oblique → italic`。
3. 在当前 style 档内，静态 face 按 CSS Fonts weight 规则排序；variable font 为请求生成限制到轴范围的实例，并以该实例的目标 weight 参与同一排序。
4. 同一匹配位置的候选保持 `FontSource` 与 `collection_index` 注册顺序。
5. 按候选顺序执行完整 shaping，第一个没有 glyph id `0` 的候选获选。
6. 当前 family 的候选都缺字时继续下一个请求 family；所有候选都缺字时保留整个候选序列中的第一个 shaping 结果。

候选序列不包含 synthetic style。候选尝试信息保存实际 `FontFaceId`，因此同一物理 variable font 的不同轴位置可以被区分。

## 实施阶段

以下 phase 按依赖顺序推进。中间 phase 不要求程序可编译、可运行或通过测试，只要求其数据结构和调用方向能直接进入下一 phase。不得为了维持中间状态而添加临时适配器、重复字段或兼容分支。完整编译、测试、示例和文档验证统一在最后一个 phase 完成。

具体字段和函数名可以按实现需要调整，但不得改变本文固定的行为边界。实施中若发现字体元数据无法可靠表达既定规则、依赖库 API 与本文假设不符，或必须修改 Tiqian 公共接口，应停止实施，说明原因和建议后确认方案。

### Phase 1：建立物理 face 目录

目标是让 `from_sources` 一次性解析后续选择所需的全部字体事实，不接入新的候选顺序。

1. 为每个字体文件或集合成员保存稳定的物理 face 定位信息，不用包含 variation 的完整 `FontFaceId` 作为物理 face 的唯一查找键。
2. 从 name table 收集 typographic family、family 与兼容 family 名称，并合并 `FontSource::with_alias` 提供的名称。
3. 缓存静态 weight 以及 normal、italic、oblique style。
4. 缓存 `wght`、`ital`、`slnt` 轴的最小值、默认值和最大值。
5. 保留注册顺序，使同名 family、集合成员和默认 fallback 都有稳定次序。

阶段产出：`FontFaceRecord` 或等价内部数据包含候选构造所需的全部元数据；后续热路径不再解析 name table、attributes 或 variation axes。

### Phase 2：建立有序候选构造

目标是把一次 `FontBackendRequest` 转成唯一的有序候选序列，暂不要求所有候选都能完成 variation-aware shaping。

1. 按请求的 `font_families` 顺序选取 family；未请求或全部未命中时使用物理 face 注册顺序。
2. 按 italic 请求选择 style 档，再在档内应用 CSS Fonts weight 顺序。
3. 为 variable face 根据请求构造限制到轴范围的 `wght`、`ital` 或 `slnt` 设置。
4. 为每个候选构造包含实际 `FontVariationInstance` 的 `FontFaceId`。
5. 保持同一匹配位置的注册顺序，并避免为每个 family 重复扫描全部 face。

阶段产出：候选序列已经能完整表达 family、style、weight 和 variation 决策；候选身份不包含 synthetic style。

### Phase 3：接入候选 shaping 与 fallback

目标是让 `FontBackend::shape` 消费 Phase 2 的候选序列，并保留 Tiqian R0003 的完整 shaping 语义。

1. HarfRust 为每个候选使用 `FontFaceId.variation_instance` 构造 `ShaperInstance`。
2. 按候选顺序执行完整 shaping，第一个没有 glyph id `0` 的候选获选。
3. 当前 family 全部缺字时继续下一个 family；全部候选缺字时保留全序列第一个结果。
4. `GlyphRun`、glyph 的 `render_font_face`、`ShapingDecisionInfo` 和 `FontCandidateAttempt` 写入实际候选的 `FontFaceId`。
5. 保持选中候选后的短路行为，不增加 cmap 预扫描。

阶段产出：最终 face 和 glyph 数据已经反映 family、静态样式与 variable instance；此时 bounds、metrics 和 SDF 仍可留待下一 phase 统一接通。

### Phase 4：统一实例解析与字体回放

目标是让 shaping 之后的全部测量和绘制从最终 `FontFaceId` 恢复同一个物理 face 与 variation location。

1. 调整 Huozi 内部 `face_for_id` 或等价查找，使其按物理资源与 collection index 定位 `FontFaceRecord`，并从传入 ID 读取 variation 设置。
2. SkRifa glyph bounds 与 color glyph bounds 使用该 variation location。
3. `raw_metrics` 改用同一 location 下的 SkRifa metrics，应用字体提供的 variation metric delta。
4. outline draw 与 glyph 0 fallback 使用同一 location，SDF 因而来自 shaping 选中的实例。
5. 保持 atlas key 为完整 `FontFaceId + glyph_id`，不同 variation instance 不共享缓存项。

阶段产出：glyph id、advance、bounds、metrics、outline、SDF 和 atlas identity 全部来自同一实例。

### Phase 5：补齐验证资产

目标是集中建立能够证明最终行为的测试与人工样例，不为中间数据搬运编写低价值测试。

1. 补充 family 顺序、family fallback、CSS weight、italic/oblique 顺序和缺样式退化测试。
2. 补充 variable `wght`、`ital`、`slnt` 实例及 shaping、bounds、metrics、outline 一致性测试。
3. 保留并扩展 glyph coverage 与全部缺字时首候选退化测试。
4. 验证 `layout` 与 `layout_parse` 的等价样式请求得到相同字体选择。
5. 在 `examples/render` 增加 family、regular/bold 与 normal/italic 的可视对照，不调整示例 GPU 架构。

阶段产出：最小必要测试矩阵全部有直接证据，示例能人工检查字体提供的 face 或 variation 带来的字形变化。

### Phase 6：最终整合与交付验证

目标是只在完整流程具备后恢复并验证仓库的可运行状态。

1. 清理 phase 之间遗留的旧字段、默认实例路径和重复逻辑，不保留临时兼容代码。
2. 检查 capability report、候选尝试信息和公开文档只描述实际完成的能力。
3. 更新 `docs/architecture.md`、`docs/parser.md` 和 `docs/rendering-gap-analysis.md`。
4. 运行完整测试、全 target 编译、release 示例和文档检查。
5. 人工检查示例中的 family、weight、bold、italic 变化以及布局与 SDF 轮廓一致性。

完成条件：自动测试能证明最终 `FontFaceId` 或 variation instance 随样式请求变化，示例能看到对应字形变化，且 measure 与 draw 使用同一实例；仓库最终可编译、可运行、可测试。

## 最小必要测试

| 验证目标 | 最小证据 |
| --- | --- |
| family 顺序 | 两个都覆盖同一文本的 family，交换 `font_families` 顺序后最终 `FontFaceId` 随之改变。 |
| family fallback | 首选 family 缺字、次选 family 完整覆盖时，候选尝试顺序和最终 face 符合声明顺序。 |
| 未注册 family | 请求 family 均未注册时仍按已确认的默认顺序完成布局，不 panic。 |
| 静态 weight | 同 family 至少两个静态 weight，请求不同 weight 后选择符合规则的 face。 |
| bold | `[bold]` 与等价 `[weight=700]` 得到相同字体候选与最终实例。 |
| italic | 同 family normal 与 italic/oblique face 按已确认规则选择。 |
| 样式缺失退化 | family 缺少请求 weight 或 italic 时选择按已确认顺序得到的最近非合成实例。 |
| variable weight | 同一 variable font 的不同 weight 请求产生不同 `FontVariationInstance`，并改变 shaping 或轮廓证据。 |
| variation replay | glyph bounds、metrics、outline draw 与 atlas key 使用 shaping 选中的同一 variation instance。 |
| glyph coverage | 样式匹配的首选候选缺字时，继续尝试后续候选；所有候选缺字时保留首个候选结果。 |
| 输入路径 | `layout` 与 `layout_parse` 的等价 `TextStyle` 请求得到相同字体选择。 |
| 现有行为 | 默认样式和只提供一个静态 face 时继续产生可重放 glyph；现有字体 fallback 测试继续通过。 |

测试不应只断言样式字段到达 `FontBackendRequest`；必须断言最终 face、variation instance、候选顺序或可观察字形证据发生预期变化。

## 性能与内存验收

1. 字体目录元数据只在 `Huozi::new` 时解析一次。
2. shaping 请求不 clone 字体 bytes，不重新读取 name table 或 variation axes。
3. 候选构造保持 $O(n)$，其中 $n$ 是已注册 face 数量。
4. 第一个完整候选获选后立即停止后续 shaping，保持 Tiqian R0003 的短路行为。
5. atlas 只因不同的 `FontFaceId` 或 glyph id 建立独立项；同一实例重复请求继续命中缓存。
6. 不为日志额外收集、排序或格式化完整字体目录；异常输入继续采用固定 `warn` 与局部退化。

## 兼容性与回滚

项目处于开发阶段，不要求保持错误的字体选择结果。但以下公开行为需要明确：

- `FontSource::with_alias` 的名称语义可能因 family 决策而固定或扩展；
- 同一批字体来源在显式 `[font]`、`[weight]` 或 `[italic]` 下会选择不同 face；
- 未使用这些样式时应尽量保持现有注册顺序 fallback；
- `FontFaceId` 已包含 variation instance，不需要建立第二套 glyph identity；
- variable font 只有在 shaping、metrics、bounds 与 replay 使用同一实例时才可交付；否则撤回该部分。

回滚时可以恢复候选排序或 variation-aware 路径，但不能移除 parser 已公开的样式字段，也不能破坏 Tiqian R0003 的同 face 回放约束。

## 预计验证

待实施完成后至少运行：

```text
cargo test
cargo check --all-targets
cargo run --example render --release
git diff --check
```

人工检查 `examples/render` 中 family、regular/bold 与 normal/italic 对照，确认字形变化、布局位置和 SDF 轮廓一致。若文档更新，另运行项目采用的中文文档措辞检查。

## 相关实现位置

| 文件 | 当前职责 |
| --- | --- |
| `src/parser/text_style.rs` | Huozi `TextStyle` 的 family、weight 与 italic 字段。 |
| `src/parser/elements_to_document.rs` | `[font]`、`[weight]`、`[bold]`、`[italic]` 到 `TextStyle` 的转换。 |
| `src/layout/tiqian_input.rs` | 将完整 Huozi 文字样式写入 Tiqian。 |
| `src/font_backend.rs` | 字体来源登记、候选 shaping、metrics、bounds 和 replay catalog；本迭代主要实现位置。 |
| `src/glyph_rasterizer.rs` | 根据 `FontFaceId` 绘制轮廓并生成 SDF 输入。 |
| `src/huozi.rs` | 以 `FontFaceId + glyph_id` 管理 SDF atlas。 |
| `examples/render/fonts.rs` | 当前受控静态字体和 variable font 资源。 |
| `examples/render/main.rs` | 官方 WGPU 人工验证入口。 |
| `tiqian-rs/src/shaping/font_backend.rs` | `FontBackendRequest`、候选尝试与统一 backend contract。 |
| `tiqian-rs/src/core/font_face.rs` | `FontFaceId` 与 `FontVariationInstance`。 |
| `tiqian-rs/docs/adr/R0003-unified-font-backend.md` | 字体选择、完整 shaping、metrics 与 replay 的既有约束。 |
