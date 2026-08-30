# Huozi × Tiqian 整合 Roadmap

- 状态：当前方案
- 更新日期：2026-08-25
- 当前阶段：Kotlin 直接单元测试跨语言验证（待规划）

本文是 Huozi、Tiqian 与 `tiqian-rs` 整合工作的总入口。它说明整个移植的背景、目标架构、当前实施方案、验收原则和后续工作。阅读本文和当前迭代文档即可了解现行计划；`notes/`、`references/` 与已作废草案仅保留调研和讨论历史。

## 背景

Huozi 已覆盖富文本解析、字体读取、字形栅格化、SDF 图集、顶点生成和 WGPU 渲染，但段落布局仍以逐 `char` 测量和按宽度换行为主。当前链路缺少 production shaping、glyph cluster、多字体 fallback、CJK 标点空间、禁则修复、行调整和可解释布局结果。

Tiqian 已实现较完整的简体中文横排核心，包括：

- source/display text 与范围映射；
- 字体角色、fallback 契约、shaping 与字体度量归一；
- UAX #14、mandatory break 与西文断词；
- CLREQ 标点 body/ink/glue 和显示替换；
- line breaking、PushIn/Hang/Carry 等禁则修复；
- compression、autospace、justification、缩进与行长网格；
- decoration、ruby、注音、inline box/object 几何；
- `LayoutResult`、布局查询和结构化 decisions。

Tiqian 的核心测试使用确定性的 stub shaping 和 stub metrics，因此可以在不引入 Skia、Android、Web 或 Core Text 差异的情况下验证排版规则。现有 `EarlyLayoutFixtures` 与 layout dump golden 是本次 Rust 移植的当前行为基准。

## 最终目标

1. 将 Tiqian 已验证的排版核心移植为独立 Rust crate `tiqian-rs`。
2. 让 `tiqian-rs` 严格翻译 Tiqian 的排版核心：保持 Kotlin 的模块、文件边界、算法阶段、状态模型和调用关系；Rust 语法、所有权和标准库替换只服务等价表达，不借 fixture/golden 自行设计规则。
3. 由 Huozi 提供生产字体资源、fallback、shaping、raw metrics、glyph/SDF 资产和 renderer。
4. 以 `tiqian-rs` 替换 Huozi 当前逐字符段落布局，同时保持 source mapping 和既有渲染能力可验证。
5. 让 shaping、测量、最终 glyph placement 和绘制共享同一份字体与 glyph 证据。

当前范围是简体中文横排及 Tiqian 已实现的相关 LTR 正文能力。竖排、完整 bidi/RTL、JLREQ、KLREQ、分页、多栏和编辑器能力不在本轮目标内；Tiqian 中已有但未实现的扩展点应保留同等的显式状态。

## 项目关系

Tiqian 与 `tiqian-rs` 是两套独立实现：

```text
Kotlin Tiqian
  -> Kotlin LayoutInput
  -> Kotlin 排版核心
  -> Kotlin LayoutResult

Rust tiqian-rs
  -> Rust LayoutInput
  -> Rust 排版核心
  -> Rust LayoutResult
```

两者不建立生产 FFI 或运行时依赖。Tiqian 继续作为独立项目和参考实现；`tiqian-rs` 独立构建、测试和发布。双方通过相同 fixture、相同 golden 语义和明确的参考 commit 对齐行为。

Huozi 在 Rust 移植完成后接入 `tiqian-rs`：

```text
Huozi source / rich text
  -> Huozi 输入转换与 source mapping
  -> tiqian-rs LayoutInput
  -> tiqian-rs 字体策略与段落布局
  -> Huozi shaping / metrics backend（按 tiqian-rs 契约提供证据）
  -> tiqian-rs LayoutResult
  -> Huozi glyph/SDF asset resolution
  -> Huozi render batches / WGPU vertices
```

## 职责边界

### `tiqian-rs`

负责从字体与 shaping 契约之后到最终布局结果的排版真值：

- 平台无关输入、source range、grapheme/cluster/glyph 映射；
- 字体角色、fallback policy 和 metrics normalization；
- 断行机会、CLREQ profile、标点与 display substitution；
- paragraph layout、禁则修复、compression 和 justification；
- line box、最终 glyph placement、annotation geometry；
- 结构化 decisions 与 capability issues。

`tiqian-rs` 不读取字体文件，不生成 bitmap/SDF，不依赖 WGPU，也不直接生成 Huozi 顶点。

### Huozi

负责：

- `Segment`、富文本和宿主样式到 `tiqian-rs` 输入的转换；
- 字体资源、字体实例身份、字符覆盖和 production fallback 能力；
- production shaping、raw font metrics、glyph advance/offset/ink bounds；
- glyph rasterization、SDF、atlas、缓存、绘制批次和 renderer；
- 将 `LayoutResult` 中的 `FontId + glyph_id + placement` 原样重放。

Huozi renderer 不重新 shaping，也不补断行、标点、禁则或两端对齐规则。

### Kotlin Tiqian

在移植期间负责提供当前参考行为：

- `EarlyLayoutFixtures`；
- deterministic stub shaping/metrics；
- checked-in layout dump golden；
- 相关 ADR、规则实现和结构化 decision 语义。

若 Rust 对照暴露疑似 Tiqian bug，不更新 golden、不在 Rust 中有意复制该行为，先停止并由负责人决定。

### 文件来源与 Rust 组织

`tiqian-rs` 严格镜像 Tiqian 核心的 canonical Kotlin 源结构。`engine/src/commonMain/kotlin/org/tiqian/{core,font,linebreak,clreq,layout,shaping}` 中的每个 `.kt` 文件都对应 `tiqian-rs/src/org/tiqian/{core,font,linebreak,clreq,layout,shaping}` 中同名的 `.rs` 文件；目录、模块和文件不合并、不拆分、不重组。

每个 Rust 翻译文件在文件头第一处注释标明唯一对应的 Kotlin 相对路径，例如 `engine/src/commonMain/kotlin/org/tiqian/layout/ParagraphLayoutEngine.kt`。Rust 原生 glue、crate root 或构建文件必须明确标为 Rust 原生；它们不得承载从 Kotlin 翻译而来的排版规则。

翻译前读取对应 Kotlin 文件；翻译后保留相同的阶段职责、具名 heuristic、decision reason 与可观察状态。Rust 类型或控制流只在 Kotlin/JVM 运行时差异要求时作最小等价调整，并在对应 Rust 文件注释中说明。

## 当前测试对齐方案

### 原有 Tiqian 流程

```text
Tiqian fixture
  -> Kotlin Tiqian
  -> layout dump
  -> 与 Tiqian golden 比较
  -> 结论
```

### `tiqian-rs` 验收流程

```text
Kotlin 源文件与对应测试
  -> 完整、严格的 Rust 源码翻译
  -> 首次完整编译
  -> 独立 fixture 验证工具
  -> Kotlin / Rust 结构化结果检查
  -> fixture 与 layout dump golden 验收
  -> 结论
```

当前方案的原则：
- Kotlin 源码与其测试是算法、数据模型和阶段顺序的唯一实现依据；fixture/golden 只验证翻译结果；
- 完整严格翻译完成后才运行 fixture；不得从单个 fixture 或 dump 反推、补写或简化算法；
- fixture 和 golden 暂时以 Tiqian 仓库为唯一行为基准；长期通用格式和共同维护位置以后决定；
- exporter、runner、结果检查和报告工具独立于 `tiqian-rs` production 代码，不包含排版规则或 decision 推导；
- 验证工具优先报告输入、模型、decision 或几何层面的首个差异，layout dump 文本 diff 用于补充和人工审查；
- 初步验证与完整测试验证可以按 fixture 或测试主题分批执行，但失败必须回到对应 Kotlin/Rust 源码核对。

### 确定性 stub

`tiqian-rs` 实现与参考 commit 中 Tiqian 相同的测试 stub，包括 nominal advance、空格、双长破折号、glyph ID、无 ink bounds 和 stub metrics 等行为。因此当前核心对照不需要从 Kotlin 捕获 shaping 运行时数据，也不建设 recording/replay backend。

Tiqian 的平台 shaping 测试不属于本次排版核心移植范围。Huozi 的 production shaping 与真实字体测试在后续集成阶段独立完成。

## 实施路线

整个移植按工作目标划分迭代。翻译批次仅用于管理文件规模和源码审查，不是可编译批次：批次之间缺少符号、未实现调用和编译错误属于预期状态，各批结束不运行编译或测试。只有完整排版核心全部翻译后才进行首次完整编译和 fixture 验证。

### 迭代 01：排版核心翻译与初步验证（已完成）

当前实施文档：[迭代 01：Tiqian 排版核心翻译与初步验证](iterations/01-tiqian-fixture-parity.md)。

本迭代按目录和文件职责分批完成 `engine/src/commonMain/kotlin/org/tiqian/{core,font,linebreak,clreq,layout,shaping}` 中 48 个文件的逐文件、逐类型、逐函数严格镜像翻译。每个翻译批次结束只对照 Kotlin/Rust 源码，不编译、不运行测试。全部翻译完成后进行首次完整编译，恢复独立验证工具，并以 5 个简单或典型 fixture 完成初步验证。

退出条件：约定范围内的 Kotlin 文件已全部镜像翻译并完成逐文件对照；`tiqian-rs` 首次完整编译通过；`basic-pause-stop`、`mandatory-crlf`、`kinsoku-push-in`、`line-end-kinsoku`、`justify-mixed-paragraph` 通过跨语言验收；Tiqian 原有 golden 基线保持有效。

### 迭代 02：全量 Fixture Golden 验证（已完成）

当前实施文档：[迭代 02：全量 Tiqian Fixture Golden 验证](iterations/02-full-fixture-golden-validation.md)。

在迭代 01 的完整源码翻译上，按能力主题验证全部 49 个 `EarlyLayoutFixtures`。每个 fixture 继续直接使用 Kotlin 实际输入、确定性 stub、greedy/lookahead/paragraph-dp 和 Tiqian checked-in layout dump golden；差异回到 Kotlin 源码与唯一 Rust 镜像文件修正，不在 fixture、adapter 或报告工具中补算法。

退出条件：全部 fixture/golden 对照通过；所有差异均已修复或由负责人明确批准；Kotlin `LayoutDumpGoldenTest` 与 Rust 构建检查保持通过。未经过 `EarlyLayoutFixtures` 的 Kotlin 直接单元测试在本迭代只完成盘点，保留为下一迭代工作。

### Kotlin 直接单元测试 Rust 测试树迁移

迭代 02 完成后，将 Kotlin 原测试的调用、输入和断言期望迁移到 `tiqian-rs/tests/org/tiqian/` 的平行 Rust 测试树。该测试树不复制 `EarlyLayoutFixtures` 或 layout dump golden，不建立 fixture-id 行为、第二套 fixture 语料或 Rust-only expected corpus；Kotlin 测试保持唯一真值。

退出条件：迭代 03 文档盘点的 Kotlin 直接单元测试均已按确认的保留、合并或调整规则进入 Rust 测试树，或已由负责人作出明确决定。

### Rust 契约与发布准备

完整测试验证后，收敛 `LayoutInput`、`LayoutResult`、range、font/fallback/shaping/metrics provider、`FontId`、capability issue、错误和布局查询接口；建立独立 CI，并完成 MPL-2.0 来源与发布物许可证核对。

退出条件：crate 可独立构建、测试和发布，production 代码不依赖 Huozi、Kotlin、Node.js、WGPU 或具体字体库。

### Huozi 字体、Shaping 与渲染基础设施

目标：让 Huozi 提供 `tiqian-rs` 所需的真实字体证据，并能按最终 placement 重放 glyph。

主要工作：

- 多字体管理和稳定 font instance identity；
- grapheme segmentation、字符覆盖和 fallback 候选；
- production shaping 的 cluster、glyph ID、advance、offset 和 features；
- raw metrics、ink bounds 和具名 capability issues；
- 以 `FontId + glyph_id` 为核心的 raster/SDF/atlas 路径；
- shaping、layout、raster 和 renderer 之间的字体生命周期与缓存策略。

退出条件：同一字体实例和 glyph ID 能从 shaping 贯穿到 SDF 与绘制，测量与绘制不发生二次选择或二次 shaping。

### Huozi 接入、替换与发布

目标：以 `tiqian-rs` 替换 Huozi 旧布局，并完成生产化收口。

主要工作：

- `Segment` / `TextSpan` 到 Rust layout input 的 lowering；
- Rust layout result 到 glyph assets、render batches 和 source mapping 的转换；
- public API 兼容、迁移或废弃策略；
- Desktop、移动端和 WebAssembly 构建与人工渲染检查；
- 删除或隔离旧逐字符布局；
- benchmark、profiling、fuzz/property tests、版本兼容与发布文档。

退出条件：Huozi 默认使用 `tiqian-rs`；不存在 renderer 侧布局补丁或无人维护的双实现；支持平台、错误模型、性能预算和迁移方式有明确说明。

## 总体验收原则

### 排版核心移植

- 以 Tiqian Kotlin 源码、Kotlin 单元测试、ADR、fixture、golden 和 structured decisions 为当前参考，且前两项决定实现；
- 完整严格翻译并首次编译后，在 deterministic stub 下进行初步 fixture 验证和下一迭代的完整测试验证；
- 比较 source/display text、range、cluster、break、repair、spacing、line geometry 和 decisions；
- 有意改变行为必须先由负责人决定，不能通过自动更新 golden 消除差异。

### Huozi 集成

- 使用真实字体验证 shaping、metrics、glyph ID、ink bounds 和 fallback；
- 检查 `LayoutInput` lowering、`LayoutResult` 和最终 renderer 三个边界；
- glyph placement 必须可重放，renderer 不重新排版；
- 自动化结果与人工渲染检查同时通过。

## 当前状态

| 工作 | 状态 | 说明 |
| --- | --- | --- |
| Huozi/Tiqian 现状调研 | `done` | 已有两侧结构参考与讨论纪要 |
| 移植职责边界 | `decided` | 两套独立实现；Huozi 后续接入 Rust 核心 |
| 对照测试路线 | `decided` | 独立工具复用 Tiqian fixture/golden；结构化检查优先于粗略 diff |
| 迭代 01 | `done` | 完整严格翻译、首次编译和 5 个 fixture 初步验证 |
| 迭代 02 | `done` | 全部 49 个 fixture/golden 已通过；修复 mandatory 空行的负零宽度 |
| Kotlin 直接单元测试 Rust 测试树迁移 | `todo` | 在 `tests/org/tiqian/` 镜像 Kotlin 外部测试；不复制 fixture/golden 语料 |
| Rust 公共契约与发布准备 | `todo` | 完整测试通过后收敛并建立独立 CI |
| Huozi production shaping | `todo` | 核心移植完成后开始 |
| Huozi 布局替换与发布 | `todo` | 依赖 Rust 核心和生产字体链路 |

## 后续 TODO

### 近期

1. 按迭代 03 建立 `tests/org/tiqian/` 平行测试树。
2. 按主题迁移 Kotlin 原测试的调用与断言。
3. 完成直接单元测试验证后，进入 Rust 公共契约与发布准备。

### 完整 port 前

1. 明确完整能力矩阵以及 Tiqian 中哪些前端专属能力不属于 Rust 核心。
2. 确定 fallback policy 与 Huozi production backend 的最终分工。
3. 收敛 Rust 输入、结果、provider trait、FontId、错误和 capability issue。
4. 核对 MPL-2.0 移植文件的来源和发布要求。

### Huozi 接入前

1. 选择 production shaping、font parsing 和 rasterization 依赖组合。
2. 设计 Huozi FontManager、字体实例生命周期和缓存。
3. 决定 Huozi 现有 public layout API 与 `GlyphVertices` 的兼容策略。
4. 建立真实字体、SDF/WGPU 和跨平台验收矩阵。

## 尚待决策

以下事项在对应工作开始前决定，不阻塞迭代 01：

- fixture/golden 的长期通用格式、仓库归属和共同维护方式；
- `tiqian-rs` 完整移植的最终能力矩阵；
- fallback 最终由核心决策到何种程度、Huozi 提供何种候选与证据；
- `FontId` 是否包含字号/variation、生命周期和跨段稳定性；
- Huozi public API 兼容范围；
- production shaping/font/raster 依赖；
- `tiqian-rs` 发布名称、版本策略和维护关系。

## 文档状态

### 当前事实来源

- 本文：总体目标、现行方案、工作路径和状态；
- [迭代 01](iterations/01-tiqian-fixture-parity.md)：当前可执行计划；
- [术语表](glossary.md)：当前用语与待 review 名称；
- `references/`：带日期的 Huozi/Tiqian 现状快照。

### 历史资料

- `notes/` 记录此前讨论与当时结论，用于追溯背景，不再单独决定现行方案；
- `drafts/port-testing-tools.md` 已作废；其固定字体、record/replay backend、完整 corpus、schema 和截图工具方案不再实施；
- 其他早期设计只有在本文或当前迭代明确引用时才构成实施依据。
