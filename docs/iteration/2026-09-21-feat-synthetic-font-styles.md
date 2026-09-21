# 仿粗体与仿斜体字体实例

> 状态：已完成
>
> 日期：2026-09-21
>
> 分类：feat

## 文档用途

本文定义 Huozi 在真实静态字体面与标准 variable font 轴无法满足字重或斜体请求时，生成仿粗体与仿斜体字体实例的行为、数据模型、开关位置、轮廓与 SDF 实现、缓存身份及验证边界。

本文是本迭代后续实现、测试与验收的依据。实现涉及 Huozi 与 Tiqian 的字体身份和文字样式契约；稳定行为完成后应同步更新两个仓库各自的持续维护文档。历史迭代文档不回写。

## 背景

当前字体候选路径依次处理：

1. 调用方指定的 family 候选域；
2. `FontRole` 对 CJK 与 Latin 来源用途的优先级；
3. 静态 normal、italic、oblique style；
4. CSS Fonts weight 候选顺序；
5. variable font 的 `wght`、`ital`、`slnt` 标准轴；
6. HarfRust 完整 shaping 覆盖检查。

若请求粗体或斜体而候选 family 没有相应静态 face 或标准轴，当前实现选择最接近的非合成实例。结果仍可布局与绘制，但视觉上不能满足请求。

Huozi 使用 SkRifa 读取 metrics、glyph bounds 与 outline，使用 HarfRust shaping，使用 `ab_glyph_rasterizer` 将 outline 栅格化，并生成 CPU SDF atlas。Tiqian 输出的最终 `FontFaceId` 贯穿 shaping、metrics、bounds、glyph replay 与 atlas key。

## 已确认取舍

1. 不为 synthetic font 单独引入 `FontFaceInstance` 层；扩展现有 `FontFaceIdData`，使 `FontFaceId` 表示可完整回放的最终字体实例。
2. `FontFaceId` 中的 synthetic 参数与资源、collection member、variation instance 一同参与相等比较、hash、debug display 与缓存身份。
3. 仿粗体与仿斜体允许分别生成独立的 SDF atlas glyph。即使真实粗体或斜体也会产生另一套 glyph，本迭代接受 synthetic 导致的额外图集占用与栅格化成本。
4. 仿粗体采用现有 SDF 距离阈值外扩的路线，不实现轮廓 offset，不复用用户 stroke 的语义字段。
5. 仿斜体采用 outline shear；raster bounds 与 Tiqian ink bounds 必须复用同一坐标变换。
6. synthetic weight 与 style 分别提供开关。开关位于可继承的 `TextStyle`，默认允许两者；不以 backend 全局开关替代 run/span 级样式语义。
7. 真实静态 face 与标准 variable axis 始终优先；variation 不属于 synthesis，也不受 synthesis 开关影响。
8. synthetic 不改变 glyph id、glyph advance、cluster advance、字体 ascent、descent、leading 或 baseline。

## 数据模型

### 允许策略：`FontSynthesis`

Tiqian 公开一个轻量位标志值：

```text
FontSynthesis
├─ NONE
├─ WEIGHT
└─ STYLE
```

`WEIGHT | STYLE` 表示两者均允许。默认值为 `WEIGHT | STYLE`。

Tiqian `TextStyle` 新增：

```text
font_synthesis: FontSynthesis
```

Tiqian `TextStyleOverride` 新增：

```text
font_synthesis: Option<FontSynthesis>
```

其覆盖语义与 `font_weight`、`italic` 一致：未指定时继承基础样式，指定时整体替换该位标志值。使用一个位标志值而不是两个 bool，可表达 `NONE`、仅 weight、仅 style 与两者允许，同时避免所有 style、override、builder 和序列化结构成对增加字段。

Huozi 的 parser `TextStyle` 保存同一语义的字段，并通过 `HuoziTiqianInputAdapter` 原样传入 Tiqian。serde 使用 camelCase 字段 `fontSynthesis`。Huozi 的默认值同样为 `WEIGHT | STYLE`。

`FontSynthesis` 只表示调用方允许的 fallback 行为，不表示最终选中的字体实际发生了合成。

### 实际实例：`FontFaceIdData.synthesis`

`FontFaceIdData` 新增结构化 synthetic 实例描述：

```text
FontFaceIdData
├─ resource_id
├─ collection_index
├─ variation_instance
└─ synthesis
   ├─ embolden_em
   └─ oblique_degrees
```

没有合成时两个参数均为空，且 `FontFaceId::new`、`with_resource_id` 等现有便捷入口默认构造无合成实例。

参数必须使用可稳定比较与 hash 的位模式存储，做法与 `FontVariationSetting` 一致。`embolden_em` 使用相对于 em 的值，不能存储某次布局字号下的像素值；`oblique_degrees` 保存最终采用的角度。这样同一 `FontFaceId` 可在不同字号下回放相同比例的实例。

允许策略与实际实例使用不同类型和字段，避免把“可以合成”误当成“已经合成”。

### 物理字体目录解析

Huozi 的字体目录仍按以下物理 key 找到 `FontFaceRecord`：

```text
resource_id + collection_index
```

variation 与 synthesis 不改变字体 bytes 的归属。读取 SkRifa/HarfRust 字体资源时先解析物理 key，再分别应用 variation 与 synthetic 参数。

`ReplayableFontFaceDescriptor` 继续描述已注册物理 face。`ReplayableFontCatalog::face(synthetic_id)` 必须能解析到对应物理 descriptor，不能继续仅依赖完整 `FontFaceId` 的 `HashMap` 精确命中；实现可使用物理 key 索引，或构造去除 synthesis 的目录查询 key。descriptor 本身不为每种 synthetic 组合复制一份。

## 选择与触发规则

### 真实能力优先

一个候选 face 的处理顺序为：

1. 按现有 family、role、style 与 weight 规则选择候选。
2. 若候选有符合请求的静态 face，使用静态 face。
3. 若候选有对应标准 axis，使用 `wght`、`ital` 或 `slnt` variation。
4. 若选中的物理实例仍不能满足请求，且对应 `FontSynthesis` 位允许，附加 synthetic 参数。
5. 使用物理 face 与 variation instance 执行 HarfRust shaping。
6. 将包含 synthetic 描述的最终 `FontFaceId` 写入 resolution、cluster、glyph run、glyph replay 与 debug decision。

synthetic 不是新的 cmap/shaping 候选，不重复执行一次 HarfRust shaping。它修饰已经选中的物理候选实例。

### 仿粗体触发

首版只提供单档仿粗体，不把 CSS weight 1–1000 连续映射为多个外扩量。

当请求达到 bold 档、选中的静态/variable 实例仍低于该档，且 `FontSynthesis::WEIGHT` 已启用时，为最终实例设置固定 `embolden_em`。关闭该位时保留当前最近真实 weight 的退化行为。

首版以 `font_weight >= 600` 作为 bold 档，固定单侧 `embolden_em = 1/60`。左右与上下合计各增加 `1/30em`。两者均以具名常量集中维护。

### 仿斜体触发

当 `TextStyle.italic == true`，候选 family 没有可用静态 italic/oblique face，也没有可用 `ital`/`slnt` 标准轴，且 `FontSynthesis::STYLE` 已启用时，设置 `oblique_degrees = 14`。

关闭该位时保留当前 normal face 退化行为。首版继续使用现有 `italic: bool` 输入，不公开任意 oblique angle；14° 是最终实例参数而不是 renderer 推断值。

## 仿粗体实现

仿粗体复用现有 SDF stroke 所证明的距离阈值外扩能力，但使用独立的字体实例语义：

```text
synthetic fill threshold
  = normal fill threshold - embolden logical pixels × threshold_per_logical_pixel
```

要求：

1. synthetic bold 与用户 stroke 可同时存在。
2. synthetic bold 生成 fill 的基准轮廓；用户 stroke 仍使用独立颜色与宽度，并从该轮廓继续外扩。
3. shadow 从 fill 与 stroke 的最终外缘开始，再应用用户的扩张半径、偏移和 `blur`。
4. atlas key 使用包含 synthesis 的完整 `FontFaceId + glyph_id`。即使仿粗体与普通实例的底层 SDF 数据相同，本迭代也允许分别缓存。
5. atlas glyph 或输出适配必须从最终 `FontFaceId` 获得 `embolden_em`，不得根据请求 weight 再次推断。
6. ink bounds 在每侧按字号换算后的 embolden 值扩张。
7. SDF radius 与 buffer 必须覆盖最大固定外扩量；超出表示能力时局部退化并记录固定警告，不为日志执行额外扫描或复杂格式化。

本迭代不实现 outline embolden、bitmap 多次偏移绘制或 alpha morphology。

## 仿斜体 outline shear

### 变换

SkRifa outline 坐标使用 baseline 原点和向上的 y 轴。横排默认右倾角 `θ = 14°`，使用：

```text
x' = x + tan(θ) × y
y' = y
```

因此位于 baseline 上方的点向右移动，baseline 上的点不移动。角度符号由统一 helper 定义并用已知轮廓测试验证，不能直接复用 OpenType `slnt` 的符号约定，因为 OpenType 与 CSS 的正负方向相反。

### 实现形态

新增一个包装任意下游 pen 的 `ShearingPen<P>`：

- `move_to`：变换端点后转发；
- `line_to`：变换端点后转发；
- `quad_to`：变换控制点与端点后转发；
- `curve_to`：变换两个控制点与端点后转发；
- `close`：直接转发。

同一 wrapper 用于两条路径：

1. `draw_outline` 时包裹 `RasterizingPen`，产生实际倾斜 bitmap 与 SDF。
2. `glyph_ink_bounds` 时包裹 `ControlBoundsPen`，产生 Tiqian 消费的倾斜 ink bounds。

这样实际 raster bounds 与布局 ink bounds 使用完全相同的坐标变换，不维护第二套四角估算公式。

### 改造成本评估

outline shear 本身为中低成本改造：

| 范围 | 成本 | 原因 |
| --- | --- | --- |
| `ShearingPen` | 低 | 五个 `OutlinePen` 方法，所有坐标使用同一纯函数。 |
| raster 路径 | 低 | 在现有 `RasterizingPen` 前增加 wrapper，不修改 rasterizer。 |
| ink bounds 路径 | 低 | 在现有 `ControlBoundsPen` 前增加同一 wrapper。 |
| atlas identity | 低 | `FontFaceId` 已作为 atlas key；新增字段自然区分实例。 |
| 字体选择与输出证据 | 中 | 必须正确决定何时合成，并让最终 identity 贯穿 resolution、run、glyph 与 debug。 |
| 回归测试 | 中 | 需要覆盖控制点、bounds、组合样式、缓存隔离与真实 face/axis 优先级。 |

预计 outline shear 的实现与单元测试约半天；连同字体身份、选择、输入开关、SDF bold 和全链路测试，整个迭代预计 3–5 个工作日。

### 颜色 glyph

Huozi 当前不重放字体原生彩色图层，颜色 glyph 会退化到同 face 的 glyph 0 SDF。本迭代不新增彩色 glyph synthetic 支持。outline shear 只应用于实际进入单通道 outline/SDF 的 glyph。

## 开关位置与公开入口

### 推荐层级

开关的权威语义位于 Tiqian `TextStyle.font_synthesis`，原因如下：

1. synthesis 是字体选择 fallback 策略，不是 fill/stroke/shadow paint。
2. 同一段落的不同 run/span 可能需要不同策略。
3. `FontBackendRequest` 已携带 resolved `TextStyle`，Huozi backend 可直接消费，不需要额外参数通道。
4. `TextStyleOverride` 已提供字段级继承，基础样式可设置段落默认值，局部 span 可覆盖。
5. 布局输出可解释最终 synthetic 实例，而不是 renderer 根据全局状态猜测。

Huozi parser `TextStyle.font_synthesis` 是对该语义的公开映射。调用方通过初始 `TextStyle` 设置一次，即可得到整个段落的默认策略；通过 `TextRun.style` 或富文本范围可局部覆盖。

### 不采用 backend 全局开关

不在 `HuoziFontManager` 或 `Huozi::new` 上增加唯一的全局 synthesis bool。全局开关无法表达同段落局部禁用，也会让相同 `LayoutInput` 在不同隐藏 backend 配置下产生不同字体 identity。

如果未来宿主需要强制能力上限，可以独立增加 backend capability mask，并与 `TextStyle.font_synthesis` 取交集；该需求不在本迭代中预留字段。

### Huozi 富文本语法

实现阶段应为 parser 增加一个直接控制 `TextStyle.font_synthesis` 的范围入口，值映射为：

| 输入值 | 结果 |
| --- | --- |
| `none` | `FontSynthesis::NONE` |
| `weight` | `FontSynthesis::WEIGHT` |
| `style` | `FontSynthesis::STYLE` |
| `all` | `FontSynthesis::WEIGHT | FontSynthesis::STYLE` |

标签名为 `fontSynthesis`，可写为 `[fontSynthesis=style]...[/fontSynthesis]` 或 `[span fontSynthesis=style]...[/span]`。它不进入 fill、stroke、bold 或 italic paint 参数。直接结构化 API 使用 serde camelCase 字段 `fontSynthesis`。

## Bounds 与 metrics

### Ink bounds

实际 bounds 计算顺序：

1. 从物理 face 与 variation instance 取得 outline。
2. 若有 synthetic oblique，通过 `ShearingPen` 变换 outline，并由 `ControlBoundsPen` 求 bounds。
3. 若有 synthetic bold，按单侧 `embolden_em × font_size` 在四侧扩张 bounds。
4. 将结果写入 glyph bounds 与对应 shaping decision。

仿粗体不修改 outline，因此其 bounds 扩张必须显式完成；仿斜体的 bounds 由变换后的 outline 直接得到。

### Advance 与纵向 metrics

以下值保持物理 face 与 variation shaping 的结果：

- glyph advance；
- glyph positioning；
- cluster advance；
- ascent、descent、leading；
- baseline。

synthetic 允许 ink 越出 advance。断行与段落高度不会仅因 synthetic fallback 改变。

## 缓存与回放

1. `FontFaceIdData.synthesis` 参与 `Eq` 与 `Hash`。
2. ink bounds LRU 使用完整 `FontFaceId + glyph_id`，普通、仿粗、仿斜与组合实例分别缓存。
3. SDF atlas 使用完整 `FontFaceId + glyph_id`，允许产生独立 atlas glyph。
4. fallback glyph id 缓存同样按完整实例区分。
5. `face_for_id` 只用物理 key 找字体 bytes，再应用 id 中的 variation 与 synthesis。
6. `Display for FontFaceId` 必须输出 synthesis，使 debug dump 能区分真实 face、variation、仿粗、仿斜及组合实例。
7. 用户 stroke 与 shadow 不进入 `FontFaceId`；它们继续属于 paint 和顶点层。

## API 备选与结论

### 两个独立 bool

可在 `TextStyle` 放置 `allow_synthetic_weight` 与 `allow_synthetic_style`。优点是直白；缺点是 `TextStyle`、override、builder、Huozi parser style、serde 与适配层都要成对增长。未采用。

### 三级策略枚举

可定义 `Disabled / WeightOnly / WeightAndStyle`。字段少，但不能表达“只允许 style”，能力不对称。未采用。

### 位标志值

一个 `FontSynthesis` 字段表达四种组合，适合继承和整体覆盖，并与 CSS 的 weight/style 独立允许语义一致。已采用。

## 实施阶段

### Phase 1：Tiqian 契约

1. 新增 `FontSynthesis` 位标志及默认值。
2. 扩展 `TextStyle`、`TextStyleBuilder`、`TextStyleOverride` 与 override builder。
3. 扩展 `FontFaceIdData`，新增 actual synthesis descriptor 与访问器。
4. 更新 `FontFaceId` display、相等、hash 与便捷构造。
5. 更新 replay descriptor 查询，使 synthetic id 解析到物理 face descriptor。
6. 增加样式继承、identity 与 debug display 测试。

### Phase 2：Huozi 候选与 bounds

1. 将 Huozi parser `TextStyle.font_synthesis` 传入 Tiqian。
2. 在真实静态 face 与标准 axis 无法满足时，根据允许位生成 synthetic 参数。
3. HarfRust 继续只消费物理 face 与 variation instance。
4. 新增 `ShearingPen`，接入 raster 与 ink bounds。
5. 对 synthetic bold 的 ink bounds 应用固定 em 外扩。
6. 验证 static、variable、synthetic 的优先级与组合结果。

### Phase 3：SDF 与输出

1. atlas 按完整 synthetic identity 生成和缓存 glyph。
2. synthetic bold 使用最终 fill threshold；stroke 和 shadow 从该轮廓依次外扩，保持用户指定的描边宽度与阴影扩张语义。
3. synthetic oblique 使用 outline shear 后生成的 bitmap/SDF，输出保持轴对齐 quad。
4. 验证 normal、bold、italic、bold italic 的 atlas key、bitmap bounds 与顶点。
5. 验证 stroke、shadow 与 synthetic 可组合。

### Phase 4：parser、示例与文档

1. 为 Huozi 富文本 parser 增加 synthesis 范围控制并更新 `docs/parser.md`。
2. 更新 Huozi `docs/architecture.md`。
3. 在 Tiqian-rs 创建对应迭代记录，并按其规范更新持续维护文档。
4. 运行两个仓库的相关测试、全目标检查和 Huozi render 人工验证。

## 最小必要测试

| 验证目标 | 最小证据 |
| --- | --- |
| 默认策略 | `TextStyle::default()` 允许 weight 与 style。 |
| override 继承 | 未指定 synthesis 的 span 继承基础策略；指定后整体替换。 |
| 禁用 weight | 缺少真实 bold 时保留最近真实实例，不附加 embolden。 |
| 禁用 style | 缺少真实 italic/oblique/axis 时保留 normal，不附加 shear。 |
| 真实 face 优先 | 有静态 bold/italic 时不生成 synthetic。 |
| variable axis 优先 | 有 `wght`、`ital` 或 `slnt` 时不生成对应 synthetic。 |
| 组合实例 | 同时缺少 bold 与 italic 能力时产生 embolden + oblique identity。 |
| shaping 不重复 | synthetic 不增加 candidate attempt 或额外 HarfRust shaping。 |
| shear 控制点 | line、quadratic 与 cubic 的端点和控制点均使用同一变换。 |
| bounds 同源 | 倾斜 bitmap bounds 与 Tiqian glyph bounds 在像素容差内一致。 |
| bold bounds | 四侧按固定 em 外扩，advance 保持不变。 |
| atlas 隔离 | normal、bold、oblique 与组合实例使用不同 atlas key。 |
| paint 独立 | synthetic bold 与用户 stroke 同时存在且各自参数不被覆盖。 |
| replay descriptor | synthetic id 能解析到原物理 face descriptor。 |
| debug evidence | dump 可区分 static、variable 与 synthetic 实例。 |

## 性能与内存

1. candidate 构造仍为相对于候选 face 数量的 $O(n)$，synthetic 不增加字体目录扫描。
2. 一个候选最多执行一次完整 shaping；synthetic 参数在候选被选中时确定。
3. `ShearingPen` 逐 outline point 做常数次乘加，不收集第二份 path。
4. bounds 继续使用固定容量 LRU；完整 identity 会让不同 synthetic 实例分别占用条目。
5. atlas 允许不同 synthetic 实例分别占用网格；该内存换取实现简单、identity 直接与可独立淘汰。
6. 不为日志额外扫描、排序、clone 或格式化完整候选上下文。

## 风险与检查点

| 风险 | 处理方式 |
| --- | --- |
| 将允许策略写进最终 identity | 策略只在 `TextStyle`；identity 只保存实际应用参数。 |
| synthetic id 无法查询 descriptor | descriptor lookup 使用物理 key，不复制 descriptor。 |
| shear 的角度方向错误 | 用 baseline 上方已知点和实际 glyph fixture 验证 14° 右倾。 |
| raster 与 bounds 采用不同算法 | 两者复用同一 `ShearingPen`。 |
| 仿粗与用户 stroke 混淆 | 使用独立 synthetic fill threshold 与 bounds 语义。 |
| SDF 外扩越过 buffer | 校准固定 embolden，增加边界测试与固定警告退化。 |
| CJK 合成斜体不合适 | 默认符合通用 synthesis 行为；调用方可在基础样式或局部 span 清除 `STYLE`。 |
| synthetic 改变断行 | advance 与纵向 metrics 保持真实 shaping 结果。 |

## 完成记录

1. Tiqian 新增默认允许 `WEIGHT | STYLE` 的 `FontSynthesis`，并将其接入 `TextStyle`、builder 和 override 继承。
2. `FontFaceId` 新增按 `f32` 位模式保存的 `FontSynthesisInstance`；display、相等比较、hash、ink bounds cache 与 atlas key 均区分最终实例。
3. Huozi 保持真实 static face、`wght`、`ital`、`slnt` 优先；无法满足时按已确认阈值生成仿粗与仿斜 identity，且每个候选最多 shaping 一次。
4. `ShearingPen` 同时用于 SkRifa outline raster 与 `ControlBoundsPen`；仿粗扩大 ink bounds 与 text paint 的基准 fill threshold，不修改 advance、用户 stroke 宽度或阴影参数。
5. `fontSynthesis` serde 字段和同名富文本标签已实现，接受 `none`、`weight`、`style`、`all`。
6. render 示例默认加载末位的 `SweiGothicCJKsc-Regular.ttf`，并使用 `SweiGothic` alias 展示常规、仿粗体、仿斜体和仿粗斜体；不新增操作面板。
7. 已通过 Tiqian contract 单测、Huozi parser/input、候选、bounds、raster、SDF 输出定向测试及两仓 `cargo check`。
8. 最终验证通过：Huozi `cargo test`（61 个单测、5 个集成测试、3 个 doctest 通过，1 个 doctest 忽略）、`cargo check --all-targets`、`cargo check --all-targets --features woff`、`cargo check --example render`、`cargo check --example render --features woff`、`cargo run --example render --release --features woff`；Tiqian `cargo test`（1341 个测试通过）与 `cargo check --all-targets`；两个仓库的 `git diff --check`。
9. 修正仿粗后的 text paint 轮廓：stroke 从仿粗 fill 外缘按用户宽度外扩，shadow 从 fill 与 stroke 的外缘按用户扩张半径外扩。`tiqian_output_replays_glyphs_paints_and_segment_identity` 回归测试、Huozi `cargo test` 与 `cargo check --all-targets --features woff` 均通过。
10. 仿粗外扩量调整为单侧 `1/60em`；后端 identity、ink bounds 与组合样式测试共享同一实现常量，96px bounds 回归测试验证单侧外扩为 `1.6px`。调整后通过仿粗 bounds、组合样式、text paint 定向测试、Huozi `cargo test`、`cargo check --all-targets --features woff` 与 `cargo run --example render --release --features woff`。

## 回滚

若 outline shear 产生不可接受的 SDF 质量或 bounds 偏差，可暂时关闭 `STYLE` 默认位并保留完整 identity 与开关契约；不得退回 renderer 根据 `italic` 私自倾斜的路径。

若 SDF threshold 仿粗在可用外扩范围内仍不可接受，可在后续迭代替换实际 raster 技术；`FontFaceIdData.synthesis.embolden_em`、开关和 bounds 契约保持不变。
