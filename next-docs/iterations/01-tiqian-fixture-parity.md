# 迭代 01：Tiqian 排版核心翻译与初步验证

- 状态：`completed`
- 日期：2026-08-24
- 范围：梳理并完整严格翻译 Tiqian 排版相关 Kotlin 源码，完成首次 Rust 编译，并通过 5 个简单或典型 fixture 的初步验证
- 参考实现：Tiqian `eb26f889c57d50e52e41d3a76185cdb6a3bdba45`；`kinsoku-push-in` 的 fixture/grid 修正与更新 golden 在该基线后单独记录
- 总体路线：[Huozi × Tiqian 整合 Roadmap](../roadmap.md)

## 背景与位置

Huozi 已具备字体读取、SDF 图集和 WGPU 渲染，但仍使用逐字符布局；Tiqian 已实现简体中文横排的断行、CLREQ 标点、禁则修复、行调整和可解释布局。整合工作的最终目标是把这部分排版核心移植到独立的 `tiqian-rs`，再由 Huozi 提供 production shaping、字体资产和渲染。

Tiqian 与 `tiqian-rs` 保持为两套独立实现，不通过生产 FFI 互相调用。Kotlin 源码与 Kotlin 测试决定 Rust 的算法、数据模型和阶段顺序；fixture 和 checked-in golden 只用于完整翻译后的验证。

本迭代按文件或目录组织翻译批次，完成约定范围内全部排版源码的严格镜像翻译。翻译批次只管理工作量和逐文件源码审查，不是可编译批次：批次之间缺少符号、未实现调用和编译错误属于预期状态，各批结束不运行编译或测试。全部翻译完成后才进行首次完整编译，随后使用独立 fixture 验证命令完成 5 个 fixture 的初步验证。

## 目标

1. 按已列出的 48 个 Kotlin 文件及其唯一 Rust 镜像路径完成严格翻译。
2. 按翻译批次逐文件、逐类型、逐函数完成严格翻译；每批只做 Kotlin/Rust 源码对照。
3. 全部翻译完成后进行首次完整编译，修复 Kotlin→Rust 语言映射产生的编译问题。
4. 建立独立 fixture 验证命令，依次通过 5 个初步验收用例。

本迭代不要求完成全部 Kotlin 测试的 Rust 等价覆盖；该工作属于下一迭代。本迭代不能以初步 fixture 通过替代源码完整性检查。

## 已确定边界

- Tiqian 与 `tiqian-rs` 是两套独立实现，不建立生产 FFI 或运行时依赖。
- 当前直接使用 Tiqian 的 `EarlyLayoutFixtures` 和 checked-in layout dump golden。
- Tiqian 继续使用 `ExplainableStubTextShaper` 与 stub font metrics；Rust 只能在逐文件翻译 Kotlin stub 与其依赖后采用等价后端。
- 平台 shaping、真实字体、Skia、Android、Web 和 Core Text 测试不属于本迭代。
- fixture exporter、Rust fixture runner、结果检查与报告工具独立于 production 代码；它们复用原有 Tiqian fixture/golden，不包含排版规则，且不决定翻译文件的边界。
- `tiqian-rs` 镜像 `engine/src/commonMain/kotlin/org/tiqian/{core,font,linebreak,clreq,layout,shaping}`：每个 Kotlin `.kt` 文件对应同路径、同名 Rust `.rs` 文件；不得一对多、多对一、拆分、合并或重组。
- 每个翻译文件在文件头第一处注释唯一对应的 Kotlin 相对路径；Rust 原生 glue 文件明确标注为原生，且不承载排版规则。

## 翻译与验收链路

```text
Kotlin 源文件
  -> 同模块 / 同目录 / 同名 Rust 文件
  -> 完整 Rust 排版核心
  -> 首次完整编译

单 fixture 验证命令
  -> Kotlin fixture adapter
  -> JSON（标准输出 / 标准输入）
  -> Rust fixture runner + deterministic stub
  -> Kotlin 格式 layout dump
  -> 直接比较 Tiqian checked-in golden
  -> 初步 fixture 验收
```

Tiqian 原有测试链路保持不变：

```text
Tiqian fixture
  -> Kotlin layout engine
  -> layout dump
  -> Tiqian checked-in golden
```

## 范围确认

状态：`confirmed`（2026-08-24）。五个 fixture 都从 `ExplainableStubParagraphLayoutEngine.layout` 进入相同的完整 pipeline；它们共同需要 width-independent annotation、paragraph shaping、font role/fallback、metrics normalization、CLREQ punctuation、line-break planning、三种 breaker、repair、compression/justification、line geometry、structured debug assembly 和 annotation geometry。

仅已确认的 12 个共享 `layout` 关键文件（主入口、annotation cache、shaping、punctuation geometry/ledger、line-break planning、breaker、repair、adjustment、justifier、line geometry、debug assembly）合计 8,141 行；`layout` 目录共有 24 个 Kotlin 文件。它们还依赖 `core` 的模型与文本文件、`font` 的两个策略文件、`clreq`、`linebreak` 与 `shaping` 模块。`justify-mixed-paragraph` 会把 autospace、UAX #14、Latin segmentation 和 justification 纳入同一闭包。

因此，这五个 fixture 不能作为小型、相互独立的翻译切片。严格镜像翻译必须覆盖完整排版核心；不再把 fixture 用作翻译范围或中途完成条件。

## 翻译批次

以下 6 个批次覆盖 48 个 `commonMain` Kotlin 文件。每个文件完成严格翻译和源码对照后，将对应条目标为 `[x]`。批次用于控制审查范围，不表示对应模块能够编译或独立运行。

### T1：`core` 基础模型与 Unicode 证据（12 文件）

- [x] `core/Units.kt`
- [x] `core/Geometry.kt`
- [x] `core/TextModel.kt`
- [x] `core/LayoutModel.kt`
- [x] `core/LayoutQueries.kt`
- [x] `core/SourceInteractionBoundaries.kt`
- [x] `core/EastAsianSpacing.kt`
- [x] `core/EastAsianSpacingData.kt`
- [x] `core/UnicodeScriptEvidence.kt`
- [x] `core/UnicodeScriptEvidenceData.kt`
- [x] `core/UnicodeWordCharacter.kt`
- [x] `core/UnicodeWordCharacterData.kt`

重点核对 range 与索引语义、source/display text、输入输出模型、cluster/glyph/line/debug 字段、Unicode 数据和判断顺序。

### T2：CLREQ、字体与 shaping 契约（7 文件）

- [x] `clreq/ClreqProfile.kt`
- [x] `clreq/NumberSymbolCohesion.kt`
- [x] `clreq/BopomofoReading.kt`
- [x] `font/FontPolicy.kt`
- [x] `font/FontMetrics.kt`
- [x] `shaping/TextShaper.kt`
- [x] `shaping/ReplayableFontBackend.kt`

重点核对 profile 默认值、字体角色与 fallback、metrics normalization、shaping 证据、capability issues 和 deterministic stub 契约。

### T3：断行与断词基础（5 文件）

- [x] `linebreak/LineBreak.kt`
- [x] `linebreak/UnicodePunctuationLineBreak.kt`
- [x] `linebreak/UnicodePunctuationLineBreakData.kt`
- [x] `linebreak/Hyphenation.kt`
- [x] `linebreak/EnglishHyphenation.kt`

重点核对 UAX #14、mandatory break、CRLF、零宽 break、Latin 断词、hyphenation、Unicode 数据和 source range 语义。

### T4：角色、shaping 与标点准备（12 文件）

- [x] `layout/ClusterRoleResolution.kt`
- [x] `layout/ContextualQuoteRoleResolver.kt`
- [x] `layout/QuotePairAnalyzer.kt`
- [x] `layout/UnicodePunctuationBoundaryResolver.kt`
- [x] `layout/KinsokuRule.kt`
- [x] `layout/PunctuationModel.kt`
- [x] `layout/PunctuationGeometryLedger.kt`
- [x] `layout/PunctuationGeometryStage.kt`
- [x] `layout/ProgressiveBreakDecisions.kt`
- [x] `layout/PreparedParagraph.kt`
- [x] `layout/WidthIndependentAnnotationCache.kt`
- [x] `layout/ParagraphShapingStage.kt`

重点核对具名 heuristic、decision reason、cache key、display substitution、font/metrics evidence、标点 body/ink/glue 及 cluster/glyph 关系。

### T5：断行规划与修复（6 文件）

- [x] `layout/DefaultHyphenator.kt`
- [x] `layout/LineOptimization.kt`
- [x] `layout/LineRepair.kt`
- [x] `layout/LineBreaker.kt`
- [x] `layout/ParagraphDpLineBreaker.kt`
- [x] `layout/LineBreakPlanningStage.kt`

重点核对 greedy、lookahead、paragraph DP、candidate 评分与 tie-break，以及 CarryPrevious、CarryNext、PushIn、Hang 和 progressive technical tier。

### T6：行调整、最终几何与主入口（6 文件）

- [x] `layout/Justifier.kt`
- [x] `layout/LineAdjustmentStage.kt`
- [x] `layout/LineGeometryStage.kt`
- [x] `layout/AnnotationGeometryStage.kt`
- [x] `layout/LayoutDebugAssembly.kt`
- [x] `layout/ParagraphLayoutEngine.kt`

重点核对 compression、autospace、justification、line/glyph placement、annotation geometry、structured debug、`LayoutResult` 组装和主 pipeline 调用顺序。

### 每批结束检查

1. Kotlin 与 Rust 文件路径、文件名和来源注释一一对应。
2. 类型、函数、常量、枚举、extension function、默认值与可观察字段没有遗漏。
3. 分支、循环、调用顺序、具名 heuristic 与 decision reason 保持等价。
4. 必要的 Kotlin→Rust 语言映射已记录，未借语言转换重构算法或模块。
5. 在本文件的批次清单中将已完成文件标为 `[x]`；不运行 `cargo check`、`cargo test` 或 fixture，不为中间编译错误添加占位实现。

已确认的通用语言映射：Kotlin `data class` 默认映射为可 `Clone` 的公开字段 Rust `struct`，选择性复制后以可变副本修改字段；需要构造校验以维持不变量的类型使用私有字段和 `new(...)`，修改时重新构造。默认参数的省略形式不超过三种时使用具名构造函数，超过三种时使用 builder。`sealed interface` 直接映射为 Rust `enum`；不能直接等价映射时暂停确认。无状态 Kotlin `object` 映射为 Rust 模块函数。Kotlin `IntRange` 映射为 Rust 原生 `IntRange.rs`，两端均包含，`IntRange::EMPTY` 以 `start = 1`、`end_inclusive = 0` 表示。含浮点字段的数据类使用 Rust 标准 `PartialEq`，接受与 Kotlin `Float.equals` 在 `NaN` 和带符号零上的边缘差异。所有 source offset 保持 Kotlin 的 UTF-16 code-unit 语义；Rust 原生 `TextIndex.rs` 仅负责与 UTF-8 byte index 的转换。FIXME（可选后续工作）：在完整测试验证阶段评估是否补充 source offset 转换的边界测试与性能基准。

## 初步验证批次

### 1. `basic-pause-stop`

验收覆盖：普通 CJK 文本、逗号和句号的标点 body/glue 输出。

完成条件：三个 breaker 的 line、cluster、font、punctuation、geometry 与 layout dump golden 一致。

### 2. `mandatory-crlf`

验收覆盖：CRLF、mandatory break、source range 连续性和 line end reason。

完成条件：三个 breaker 的 CRLF cluster、source range、mandatory break decision 和 line end reason 一致，source text 未被改写。

### 3. `kinsoku-push-in`

验收覆盖：固定 Basic kinsoku profile、行首禁则、PushIn 候选及标点行边压缩。

完成条件：三个 breaker 的 repair candidate、selected repair、行范围、调整后宽度、line-edge trim 和相关 decisions 一致。已确认 Tiqian fixture 的默认 line-length grid 会改变该用例的语义；经用户决定，fixture 已禁用 grid 并更新 golden，使其真实覆盖成功 PushIn。

### 4. `line-end-kinsoku`

验收覆盖：行尾禁则及对应断点/修复路径。

完成条件：三个 breaker 的断点、CarryNext、line end reason 和相关 decisions 一致。

### 5. `justify-mixed-paragraph`

验收覆盖：中西混排、autospace 和 justification allocation。

完成条件：三个 breaker 的 cluster 顺序、字体角色、spacing 类型、natural/adjusted width 和 justification decisions 一致。

## 验证工具职责

### 已决定的开发期验证命令

- 验证一次只接收一个 fixture id，并按本节列出的初步验证顺序逐个通过；每个 fixture 都必须覆盖 greedy、lookahead 与 paragraph-dp 三个 breaker。
- Kotlin JVM fixture adapter 直接读取 `EarlyLayoutFixtures`，将该 fixture 的实际输入和运行参数输出为 JSON 到标准输出。它必须包含 `LayoutInput` 的全部有效字段，以及 breaker、`useEnglishHyphenation`、`pinBasicNoHang` 等不属于 `LayoutInput` 的运行参数；adapter 不解析 Kotlin 源码，也不推导或复制 fixture 配置。
- Rust fixture runner 从标准输入读取该 JSON，使用已严格翻译的 `ExplainableStubParagraphLayoutEngine`、stub metrics 与 deterministic stub shaping 运行完整 pipeline。它不读取 Kotlin、fixture 或 golden 路径，也不包含任何排版规则。
- Rust result adapter 将三个 breaker 的 `LayoutResult` 输出为与 Kotlin `LayoutDumpGoldenTest` 完全相同的 layout dump 文本。该 adapter 只做字段格式化、顺序与转义，不推导 decision 或几何。
- 验证命令直接读取 Tiqian 仓库内对应的 checked-in golden 路径并作字节比较；fixture 和 golden 不复制到 `tiqian-rs`。失败时除保留标准 diff 外，额外打印首个不同的 dump 文本行，便于回到 Kotlin/Rust 对应源文件定位。

### 不变量

- Kotlin fixture 是唯一输入真值，Tiqian checked-in golden 是唯一 golden 真值；不得在 Rust 中按 fixture id 写配置、提交 fixture 导出副本或更新 Tiqian golden 以消除差异。
- 当前 adapter 使用 JSON 仅作为单 fixture 的开发期数据交换格式，不构成 production API、长期 schema 或跨语言运行时依赖。
- 验证失败时先确认 JSON 输入、breaker 与 stub 参数一致，再回到唯一对应的 Kotlin/Rust 文件核对；不在 adapter、fixture 或报告器中添加排版规则。

## 验收

本迭代完成需同时满足：

- 已列出的 48 个 Kotlin 文件均有同路径、同名 Rust 镜像文件，并完成逐文件源码对照；
- 48 个文件均已严格镜像翻译，并完成逐文件源码对照；
- 翻译批次期间未以编译或 fixture 结果替代源码完整性检查；
- 全部翻译完成后的首次 `cargo check` 通过；
- 5 个初步 fixture 均通过三个 breaker 的 Kotlin/Rust 结构化结果与 layout dump golden 验收；
- Tiqian 原有 `LayoutDumpGoldenTest` 保持通过，已确认的 `kinsoku-push-in` fixture/grid 修正及其 golden 变更已单独审查；
- `tiqian-rs` production 代码不依赖 Kotlin、Node.js、Tiqian 仓库或 golden；
- 实施记录写明参考 Tiqian commit、必要语言映射和实际验证命令。

## 停止条件

出现以下情况时停止当前工作并向负责人确认：

- 现有 Tiqian golden 与 fixture、ADR 或当前实现明显矛盾；
- 对齐必须有意复制疑似 Tiqian bug；
- 严格翻译要求改变已经确认的移植职责边界或文件镜像规则；
- 需要更新 Tiqian golden；
- 需要引入平台 shaping 或真实字体才能继续。

验证工具的字段或报告不足不改变翻译范围；应补充工具的观察能力，再根据 Kotlin/Rust 源码定位差异。

## 实施记录

- 参考 Tiqian 基线为 `eb26f889c57d50e52e41d3a76185cdb6a3bdba45`；`kinsoku-push-in` 的 fixture/grid 修正及更新 golden 已单独审查，确认该 fixture 关闭默认 grid 后覆盖成功 PushIn。
- 已完成 48 个约定 `commonMain` Kotlin 文件的同路径、同名 Rust 镜像翻译与源码对照。Kotlin `data class` 映射为带公开字段的可 `Clone` Rust `struct`；带构造不变量的类型通过私有字段和构造函数保留校验；默认参数按简单具名构造或 builder 映射；source offset 保持 UTF-16 code-unit 语义。
- 开发期验证命令由 Tiqian `:engine:exportLayoutFixture`、Rust `fixture_layout_dump` binary 和 `tiqian-rs/tools/verify-fixture.sh` 组成。它直接读取 Tiqian fixture/golden，不向 Rust 复制 fixture 或 golden。
- `basic-pause-stop`、`mandatory-crlf`、`kinsoku-push-in`、`line-end-kinsoku` 与 `justify-mixed-paragraph` 均已通过 greedy、lookahead、paragraph-dp 三种 breaker 的字节级 layout dump golden 对比。
- 实际验证已通过：`cargo check --bin fixture_layout_dump`、`cargo test`、五个 `bash tools/verify-fixture.sh <fixture-id>` 命令，以及 Tiqian `./gradlew :engine:jvmTest --tests 'org.tiqian.layout.LayoutDumpGoldenTest'`。两仓库的 `git diff --check` 均通过。

## 本迭代之后

下一迭代完成所有测试验证：逐主题翻译和运行 Kotlin 对应单元测试，覆盖全部 `EarlyLayoutFixtures`、layout dump golden、核心不变量和 structured decisions。完整测试通过后，才进入 Rust 契约与发布准备、Huozi production shaping、字体身份链路和旧布局替换。详细路线见总体 roadmap。
