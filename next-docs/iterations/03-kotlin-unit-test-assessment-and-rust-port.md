# 迭代 03：Kotlin 直接单元测试盘点与 Rust 测试树迁移

- 状态：`completed`
- 日期：2026-08-27
- 范围：盘点 Tiqian 57 个不通过 `EarlyLayoutFixtures` 的直接单元测试，并按本文件的保留、合并和调整规则，将必要测试迁移到 `tiqian-rs/tests/` 下的 Rust 平行测试树
- 前置迭代：[迭代 02：全量 Tiqian Fixture Golden 验证](02-full-fixture-golden-validation.md)
- 参考实现：Tiqian `eb26f889c57d50e52e41d3a76185cdb6a3bdba45`；`kinsoku-push-in` 的 fixture/grid 修正与更新 golden 在该基线后单独记录
- 总体路线：[Huozi × Tiqian 整合 Roadmap](../roadmap.md)

## 目标

1. 将 Kotlin `commonTest` 的 52 个直接测试文件和 `jvmTest` 的 5 个直接功能测试文件，按实际保护价值分类，而不是逐文件、逐案例机械翻译。
2. 把保留或合并后保留的测试写入 `tiqian-rs/tests/org/tiqian/` 下与 Kotlin `commonTest` / `jvmTest` 对应的平行目录和文件；不创建第二套 fixture 或 Rust-only expected corpus。
3. 保持 Kotlin 测试中的规则、输入和断言期望为唯一真值；Rust 仅因 API 形态、UTF-16 偏移表示或 JVM 资源加载机制差异调整测试搭建。
4. 在 49 个跨语言 fixture/golden 已验证的完整 pipeline 之外，为局部规则、失败路径、结构化 decision、查询 API 与缓存语义建立可定位的回归保护。

## 已确定边界

- 本迭代只处理下文列出的 57 个 Kotlin 直接测试文件；`LayoutDumpGoldenTest` 的 49 个 fixture 已由迭代 02 的原始 Tiqian fixture/golden 链路覆盖，不在本迭代复制。
- Tiqian Kotlin 源码及其测试决定 Rust 的算法、数据模型、输入和预期；Rust 测试不反向定义规则。
- 每个 Kotlin 外部测试文件对应 `tests/org/tiqian/` 下同包路径、同名的 Rust 测试文件。production 镜像文件不承载本迭代新增的测试正文，也不因测试而重组。
- 现有 `tiqian-rs` 中 `FontPolicy.rs` 和 `ClusterRoleResolution.rs` 的内联测试在对应主题迁移时与测试树去重；凡其行为来自 Kotlin 外部测试，迁移后以测试树版本为准。
- 49 个 fixture golden 继续承担完整段落、三种 breaker 与最终 dump 的跨语言验收；测试树中的单测只保护其无法精确定位的局部契约。
- 不新增 fixture id 特判、测试专用排版分支、Kotlin/Rust 运行时依赖、复制 fixture/golden，或基于 golden 文本反推规则。这里的禁止“独立 Rust fixture 语料”不限制从 Kotlin 外部测试迁移而来的 Rust 单测文件。
- 平台 shaping、真实字体、Skia、Android、Web、Core Text、Huozi production 接入、benchmark、调参实验、报告 probe 与 fuzz/property testing 基础设施不属于本迭代。
- `KinsokuHangingExperimentProbe`、`LayoutBenchmarkProbe`、`LookaheadWindowProbe`、`ParagraphDpReferenceExperiment`、`ParagraphDpTuningProbe`、`ParagraphScaleBenchmarkProbe` 等实验或性能入口不迁移。

## 现状与盘点依据

- Kotlin 直接测试来源：
  - `engine/src/commonTest/kotlin/org/tiqian/`：52 个文件；
  - `engine/src/jvmTest/kotlin/org/tiqian/{linebreak,layout}/`：5 个直接功能测试文件。
- Rust 已有 6 个内联测试，位于 `src/org/tiqian/font/FontPolicy.rs` 与 `src/org/tiqian/layout/ClusterRoleResolution.rs`。
- 迭代 02 已逐一通过 49 个 `EarlyLayoutFixtures` 的原始 Tiqian layout dump golden。该结果验证最终 pipeline，不替代以下各类局部验证：
  - 表驱动 Unicode、CLREQ、字体角色与连字符规则；
  - breaker、repair 和行调整的候选优先级、拒绝原因及前进保证；
  - 标点 body/ink/glue 的局部几何下限；
  - `LayoutResult` 查询、复制、selection 与 source range 语义；
  - cache key、命中与 LRU 淘汰语义。

## 测试迁移方法

### 分类

测试按以下处理方式执行；分类描述的是迁移粒度，不改变 Kotlin 测试作为真值的地位。

| 分类 | 处理 | 适用内容 |
| --- | --- | --- |
| 必须迁移 | 在对应 Rust 平行测试文件保留 Kotlin 的核心断言；纯规则与关键边界通常完整保留。 | 规则层、分行/修复、标点几何、查询 API。 |
| 合并后迁移 | 保留能区分规则分支的代表性场景，删除只更换文案或重复验证同一最终布局的案例。 | 段落引擎总体、autospace、inline、annotation 与行高。 |
| 调整后迁移 | 保留语义契约，改写 JVM 资源或大规模扫描的测试搭建为确定的小规模测试。 | cache 与 English hyphenation。 |

### Rust 测试树形态

测试树使用一个 Cargo integration-test 根文件和与 Kotlin 测试包相同的模块目录：

```text
tiqian-rs/tests/
├── tiqian.rs
└── org/
    ├── mod.rs
    └── tiqian/
        ├── mod.rs
        ├── core/
        │   ├── mod.rs
        │   └── LayoutQueriesTest.rs
        ├── clreq/
        │   ├── mod.rs
        │   └── KinsokuLevelTest.rs
        ├── linebreak/
        ├── font/
        ├── shaping/
        └── layout/
```

- `tests/tiqian.rs` 是唯一由 Cargo 发现的 integration-test crate root；它声明 `mod org;`。各级 `mod.rs` 只声明下一层模块或对应 `*Test.rs`，不承载测试规则。
- Kotlin 外部测试 `engine/src/{commonTest,jvmTest}/kotlin/org/tiqian/<package>/<Name>Test.kt` 对应 `tests/org/tiqian/<package>/<Name>Test.rs`。同名 Kotlin 测试若来自 `commonTest` 与 `jvmTest`，合并在同一个 Rust 测试文件，并按本文件的保留或调整规则分组。
- 测试使用 crate `tiqian` 的公开 API、production 类型与现有 deterministic stub；不增加仅供测试使用的第二套排版实现，也不为外部测试扩大 production API。
- 测试名称表达 Kotlin 所保护的行为或具名 heuristic，不按 Kotlin 序号命名；测试 helper 仅放在使用它的 `*Test.rs` 文件中。
- 所有 source offset 保持 Kotlin UTF-16 code-unit 语义；emoji、代理对、组合音标、字典连字符与 range 断言必须使用相同语义。
- Kotlin 测试若只能通过 Rust 私有实现表达其断言，不以 `pub`、`pub(crate)` 或测试专用包装扩大 production 可见性；先停止该项迁移并记录需要的负责人决定。

### 每批执行规则

1. 先读唯一对应 Kotlin 源文件和 Kotlin 测试，确认 production API、输入和断言语义。
2. 只迁移本文件明确要求的测试或代表性场景；遇到规则矛盾、缺少 Rust 等价 API、需要修改 Kotlin 预期或需要新增测试基础设施时停止并记录决定。
3. 在 `tests/org/tiqian/` 的对应 `*Test.rs` 文件内添加最小测试，优先复用同文件 helper；不得为单处使用创建无意义的测试框架或转发层。
4. 运行受影响的 Rust 测试，再运行完整 `cargo test`。
5. 每个主题批次完成后运行 `bash tools/verify-all-fixtures.sh`，确认局部测试没有改变跨语言 layout dump。

## 必须迁移：基础规则

以下测试不依赖平台，且是后续布局 pipeline 的直接输入；应近乎完整保留 Kotlin 断言和 Unicode 样本。

### `core`（6）

- [x] `EastAsianSpacingTest`
- [x] `LayoutQueriesTest`（具体保留范围见“必须迁移：LayoutResult 查询”）
- [x] `LinkAddressDisplayTest`
- [x] `TextRangeTest`
- [x] `UnicodeScriptEvidenceTest`
- [x] `UnicodeWordCharacterTest`

### `clreq`（5）

- [x] `BopomofoParserTest`
- [x] `ClreqPunctuationGlyphSubstitutorTest`
- [x] `KinsokuLevelTest`
- [x] `NumberSymbolCohesionTest`
- [x] `PunctuationGluePlacementTest`

`KinsokuLevelTest` 必须保留四档禁则递进、ASCII point mark 边界、CJK 括号分类及 measure-adaptive 阈值。`NumberSymbolCohesionTest` 必须保留数字与单位、符号、货币、decimal/thousands separator 及独立数字的范围断言。

### `font`（3）

- [x] `CjkFontRoleClassifierTest`
- [x] `ScriptAwareFontMetricsNormalizerTest`
- [x] `UsesLatinFaceTest`

`CjkFontRoleClassifierTest` 与 Rust 已有 `FontPolicy.rs` 的 emoji 分类测试合并，避免重复测试同一分类分支。

### `linebreak`（3）

- [x] `LiangHyphenatorTest`
- [x] `MandatoryBreakTest`
- [x] `UnicodePunctuationLineBreakTest`

`LiangHyphenatorTest` 保留 no-op、奇偶 level、max-level 覆盖、margin、exception、TeX pattern/exception block 解析。

### `shaping`（1）

- [x] `ExplainableStubTextShaperTest`

## 必须迁移：分行、修复与行调整

以下测试保护最终 golden 难以定位的中间选择、候选顺序及失败路径；应保留绝大多数测试，且断言 range、repair kind、候选 accepted/rejection、badness 或 geometry 等结构化结果。

### `layout` commonTest（11）

- [x] `DecideHyphenBreakTest`
- [x] `EmergencyGraphemeTrackingTest`
- [x] `GreedyLineBreakerTest`
- [x] `JustifierEngineTest`
- [x] `JustifierTest`
- [x] `KinsokuAndCohesionRepairEngineTest`
- [x] `LineBreakRepairEngineTest`
- [x] `LookaheadLineBreakerTest`
- [x] `ParagraphDpLineBreakerTest`
- [x] `ProgressiveTechnicalBreakTest`
- [x] `PushInLineWideCapacityTest`

`GreedyLineBreakerTest` 必须保留空输入、正常填充、natural/adjusted width 分离、超宽 cluster 前进、`PushIn → CarryPrevious → LeaveRagged → Hang` 的选择及拒绝原因、行尾禁则退避、mandatory break 对 repair 的边界。`LookaheadLineBreakerTest` 与 `ParagraphDpLineBreakerTest` 必须保留空输入、回退/窗口行为、mandatory break、unbreakable range、禁则绕行、compression repair 与超宽单元前进。

`JustifierTest` 与 `JustifierEngineTest` 必须保护 stretch tier、不可分数字/单位、connector、CJK/Western boundary、last-line alignment、punctuation glue 与 uniform tracking 的决策优先级。

### `layout` jvmTest（5）

- [x] `EnglishHyphenationTest`（具体调整见“调整后迁移”）
- [x] `HyphenationLayoutTest`
- [x] `JustifierCompressionTest`
- [x] `LineAdjustmentPushInTest`
- [x] `OpeningBracketLineStartTest`

`JustifierCompressionTest` 的 tier 顺序、同 tier 按容量等比例分配、容量耗尽和零 surplus 四项均保留。`OpeningBracketLineStartTest` 保留开括号行首 half-width trim 及结构化 `lineEdgeTrimDecisions`。`HyphenationLayoutTest` 与 `LineAdjustmentPushInTest` 保留其 hyphen reserve、glue squeeze、CJK stretch 选择、PushIn 最小拉入组及 tier promotion 的行为断言。

### 去重边界

`GreedyLineBreakerTest` 与 `KinsokuAndCohesionRepairEngineTest` 的重叠按层次处理：前者只验证 `LineBreaker.rs` 的裸 cluster 输入、repair 候选和 breaker 行为；后者只验证经规则解析后的 repair 编排。两者不得以删除其中一方替代分层覆盖。

`LineBreakRepairEngineTest` 的 Latin/technical token 例子按下列五类保留代表性断言，不逐一复制仅改变文本措辞的案例：

1. camelCase structural break；
2. existing hyphen；
3. URL separator；
4. opaque token emergency；
5. progressive technical tier fallback。

## 必须迁移：标点、引号与局部几何

以下内容直接实现 CLREQ 标点和 source/display 规则；fixture 只覆盖精选组合，不能替代局部条件矩阵。

- [x] `AsciiPointMarkKinsokuTest`
- [x] `DisplayGlyphSubstitutionEngineTest`
- [x] `PunctuationAtomBuilderHaltTest`
- [x] `PunctuationBodyFloorInvariantTest`
- [x] `PunctuationGeometryEngineTest`
- [x] `PunctuationSpacingRuleTest`
- [x] `QuoteClassificationEngineTest`
- [x] `QuotePairAnalyzerTest`
- [x] `UnicodePunctuationBoundaryTest`

`PunctuationBodyFloorInvariantTest` 与 halt placement 测试必须保护压缩不切入 glyph ink 的几何下限。`QuotePairAnalyzerTest` 和 `QuoteClassificationEngineTest` 放入 `QuotePairAnalyzer.rs` / `ContextualQuoteRoleResolver.rs`；与 Rust 现有 `ClusterRoleResolution.rs` 的 emoji/role-range 测试协作，不建立第二份完整 emoji 测试集合。

### 几何案例合并规则

`PunctuationGeometryEngineTest` 的多组区域、字体定位案例保留以下五种 geometry source，每类至少一项：

1. profile fallback；
2. font-provided halt placement；
3. ink-bound safety cap；
4. paired compression；
5. PushIn interaction。

`AsciiPointMarkKinsokuTest` 保留“附着 ASCII 点号”与“不拆 Latin run”；`UnicodePunctuationBoundaryTest` 保留 boundary 分类矩阵。

## 必须迁移：`LayoutResult` 查询与 source 语义

以下 API 是 Rust 对外可观察语义，49 个 layout dump 无法验证其全部行为。测试应放入 `tests/org/tiqian/core/LayoutQueriesTest.rs`，必要时使用最小手工 `LayoutResult`；这种构造是查询 API 的合适测试输入，不构成第二套 fixture。

- [x] `LayoutQueriesTest`
- [x] `ZeroWidthBreakControlLayoutTest`
- [x] `InlineObjectLayoutTest` 中 source-range、selection 相关项
- [x] `DisplayGlyphSubstitutionEngineTest` 中 source-preservation 项

`LayoutQueriesTest` 不逐函数照搬，固定保留下列七类各一个最小代表性场景：

1. copy 恢复 source text，完整选择 ruby/注音 base 时附加 reading；
2. `positionedClusters` 的 occupied box 与 `drawX` 分离；
3. glyph ink bounds 与 selection/occupied geometry 分离；
4. 跨行 range box，以及多 UTF-16 unit cluster 的 range 切分；
5. hit test、caret、emoji/组合音标 selection 不返回非法 source 边界；
6. inline object 是一个 selection unit；
7. rich-text decoration 只裁去外侧 punctuation glue。

## 合并后迁移：段落总体、autospace、inline 与 annotation

以下 Kotlin 测试的最终结果大部分已经由 fixture golden 覆盖，但其中的局部边界仍应以代表性单测保留。每个主题只保留能区分规则分支的场景，不逐项复制相近文本。

- [x] `AttachedInlineBoundaryRelocationTest`
- [x] `AutoSpaceSingleGapTest`
- [x] `BaselineAlignmentTest`
- [x] `BilingualEmphasisTest`
- [x] `BopomofoLayoutTest`
- [x] `ExplainableStubParagraphLayoutEngineTest`
- [x] `FontInstanceMetricsRequestTest`
- [x] `InlineBoxLayoutTest`
- [x] `InlineObjectLayoutTest` 中非 source-range/selection 项
- [x] `RubyLayoutTest`
- [x] `SpacingAndLineGeometryEngineTest`
- [x] `VerbatimRangeAutoSpaceTest`
- [x] `WidthIndependentAnnotationCacheTest`（具体调整见“调整后迁移”）

### 段落引擎总体

`ExplainableStubParagraphLayoutEngineTest` 压缩为六项：

1. 单行结果与 debug breaker 名称；
2. mandatory break、CRLF 与尾随空行；
3. fallback 与 shaper range 覆盖；
4. glyph bounds 进入结果；
5. combining mark / complex emoji 不被错误拆分；
6. style span 边界可以合法拆分 emoji shaping。

其中 complex emoji 相关断言优先扩展 Rust 已有 `ClusterRoleResolution.rs` 测试；不得在段落入口和 cluster resolution 两处复制完整案例集。

### autospace、inline 与 metrics

下列规则每类保留 2–3 个代表性场景：

- CJK–Latin virtual gap 与 authored space 不混淆；
- suppress range 阻止 autospace；
- narrow inline box 的外侧 gap；
- attached inline 不虚构 prose boundary；
- span weight/italic 传入 metrics 与 shaper；
- baseline shift 进入 line geometry。

### ruby、注音、装饰与行高

下列每个不变量保留一个代表性场景：

- ruby 不需要扩行时复用既有 interline space；
- ruby 真发生碰撞时只影响关联 line gap；
- Bopomofo tone / em-box placement；
- inline object 仅在实际碰撞时扩大行高；
- emphasis 的 CJK/Latin 混排 baseline 对齐。

完整段落行为继续由 `ruby-line-height`、`bopomofo-tone-em-box`、`interlinear-lines`、`emphasis-marks` fixture 的跨语言 golden 保护。

## 调整后迁移：cache 与 English hyphenation

### `WidthIndependentAnnotationCacheTest`

保留 cache 语义，不复制大规模宽度 sweep 或伪随机拖动序列：

- [x] 宽度变化命中 cache 且不重新调用 shaper；
- [x] key 区分 text、style、decoration、ruby 与 inline box；
- [x] LRU 容量满时淘汰最旧 entry；
- [x] cached / uncached 输出使用三种确定宽度：窄、正常、宽。

Kotlin 的连续 `80..650` 宽度 sweep 和 pseudo-random drag sequence 不迁移；本迭代不引入 property testing 或 fuzzing 基础设施。

### `EnglishHyphenationTest`

保留三类行为，改为验证 Rust `EnglishHyphenation.rs` 的同源模式和例外数据，而非 Kotlin/JVM resource 读取路径：

- [x] `hyphenation`、`computer` 的稳定 syllable offset；
- [x] 短词与 left/right margin；
- [x] `project`、`present` 的 exception list。

## 实施批次

### U1：基础规则与 Unicode 证据

范围：`core`、`clreq`、`font`、`linebreak`、`shaping` 的“必须迁移：基础规则”清单，以及调整后的 `EnglishHyphenationTest`。

验收重点：UTF-16 offset、Unicode 表、CLREQ profile、数字 cohesion、字体角色、stub shaping、mandatory break 与 hyphenation。

### U2：分行、repair 与行调整

范围：“必须迁移：分行、修复与行调整”清单。

验收重点：greedy/lookahead/paragraph-dp、repair candidate、badness、mandatory boundary、unbreakable range、compression/stretch tier 与 line-edge trim。

### U3：标点、引号与查询 API

范围：“必须迁移：标点、引号与局部几何”及“必须迁移：`LayoutResult` 查询与 source 语义”清单。

验收重点：source/display text、quote role、punctuation body/ink/glue、hit testing、selection、copy、rich-text geometry。

### U4：合并后迁移与 cache

范围：“合并后迁移：段落总体、autospace、inline 与 annotation”及“调整后迁移：cache”。

验收重点：代表性分支覆盖、已有 Rust 内联测试去重、annotation/line-height 不变量、cache 命中与 LRU 语义。

## 验证命令

从 `tiqian-rs` 根目录执行：

```shell
cargo test
bash tools/verify-all-fixtures.sh
```

执行某个正在迁移的 Rust 测试文件时，使用实际 test module/path 过滤 `cargo test`；过滤命令必须在加入文档前实际运行。每个 U 批次完成后运行完整 `cargo test` 与全量 fixture 对照。若改动影响 fixture 输入、golden、断行、字体选择、标点空间、行高或行内几何，按迭代 02 的规则同时核对原始 Tiqian golden；本迭代不更新 golden。

## 完成条件

- 本文列出的 57 个 Kotlin 直接测试文件均已按“必须迁移”“合并后迁移”或“调整后迁移”的明确规则处理；每一项都有 `tests/org/tiqian/` 下对应 Rust 测试，或被本文明确的代表性场景、去重关系或不迁移决定覆盖。
- Rust 测试仅使用 Kotlin 测试中的规则、输入与期望；没有 Rust-only fixture、expected corpus、fixture-id 分支或测试专用排版规则。
- 已有 `FontPolicy.rs`、`ClusterRoleResolution.rs` 内联测试与测试树中的新增测试不重复保护相同分支。
- 每个 U 批次完成后，`cargo test` 和 `bash tools/verify-all-fixtures.sh` 均通过。
- Kotlin 原 `LayoutDumpGoldenTest` 维持通过；本迭代不修改 Tiqian fixture 或 golden。
- 若需要改变 Kotlin 测试预期、Rust 镜像边界、当前测试分类或本迭代范围，先由负责人明确决定并更新本文。

## 停止条件

出现以下情况时停止当前主题并记录负责人决定：

- Kotlin 测试、Kotlin 源码、ADR、fixture 或 golden 明显矛盾；
- Rust 对齐必须有意复制疑似 Kotlin 缺陷；
- 测试需要新增独立 Rust fixture/expected corpus、测试专用 production 分支或跨仓库运行时依赖；
- Kotlin 预期、Tiqian fixture 或 golden 需要更新；
- 对齐需要平台 shaping、真实字体或本迭代范围外的测试基础设施。

## 实施记录

- 2026-08-27：完成盘点并建立本迭代文档，尚未开始迁移 Rust 单测。
- 盘点确认 Kotlin 直接测试为 `commonTest` 52 个文件与 JVM 直接功能测试 5 个文件；`LayoutDumpGoldenTest` 的 49 个 fixture 已在迭代 02 完成跨语言字节级对照。
- 盘点确认 Rust 当前仅有 6 个内联测试，位于 `FontPolicy.rs` 与 `ClusterRoleResolution.rs`；后续在平行测试树中迁移对应行为并去重。
- 2026-08-27：完成 U1。新增 `tests/org/tiqian/{core,clreq,font,linebreak,shaping}/` 的 19 个 Kotlin 对应测试文件及模块声明；`LayoutQueriesTest` 仅保留文中规定的七类代表性场景。`CjkFontRoleClassifierTest` 接管原 `FontPolicy.rs` 的 emoji 分类分支后，删除该重复内联测试。
- U1 验收：`cargo test` 通过（5 个保留内联测试、79 个 integration 测试）；`bash tools/verify-all-fixtures.sh` 输出 49/49 golden matched。该脚本在输出成功结果后返回 130，未报告 fixture mismatch。
- 2026-08-27：完成 U2。新增 `tests/org/tiqian/layout/` 的 11 个 Kotlin 对应测试文件：8 个来自 commonTest 的 `EmergencyGraphemeTrackingTest`、`JustifierEngineTest`、`JustifierTest`、`KinsokuAndCohesionRepairEngineTest`、`LineBreakRepairEngineTest`、`ParagraphDpLineBreakerTest`、`ProgressiveTechnicalBreakTest` 与 `PushInLineWideCapacityTest`，以及 3 个来自 jvmTest 的 `HyphenationLayoutTest`、`LineAdjustmentPushInTest` 与 `OpeningBracketLineStartTest`；既有的 `DecideHyphenBreakTest`、`GreedyLineBreakerTest`、`JustifierCompressionTest`、`LookaheadLineBreakerTest` 覆盖同批其余对应 Kotlin 文件。每个测试仅使用 Rust 公开 API，并按本文定义的代表场景范围收敛。
- U2 验收：`cargo test --manifest-path /home/icemic/workspace/tiqian-rs/Cargo.toml -- --test-threads=1` 通过（5 个保留内联测试、152 个 integration 测试）；`bash /home/icemic/workspace/tiqian-rs/tools/verify-all-fixtures.sh` 输出 49/49 golden matched。
- 2026-08-27：完成 U3。新增 `tests/org/tiqian/layout/` 的 10 个 Kotlin 对应测试文件：`AsciiPointMarkKinsokuTest`、`DisplayGlyphSubstitutionEngineTest`、`PunctuationAtomBuilderHaltTest`、`PunctuationBodyFloorInvariantTest`、`PunctuationGeometryEngineTest`、`PunctuationSpacingRuleTest`、`QuoteClassificationEngineTest`、`QuotePairAnalyzerTest`、`UnicodePunctuationBoundaryTest` 与 `ZeroWidthBreakControlLayoutTest`。`LayoutQueriesTest` 的七类代表性 source/selection/query 场景已在 U1 的对应测试树文件中保留；本批新增 source/display、zero-width control 与标点/引号的 engine 级验证。所有测试只消费 Rust 公开 API 和 deterministic stub，没有新增 production 可见性、fixture 或测试专用逻辑。
- U3 验收：`cargo test --manifest-path /home/icemic/workspace/tiqian-rs/Cargo.toml -- --test-threads=1 --nocapture` 通过（5 个保留内联测试、197 个 integration 测试）；`bash /home/icemic/workspace/tiqian-rs/tools/verify-all-fixtures.sh` 输出 49/49 golden matched。
- 2026-08-27：完成 U4。新增 `tests/org/tiqian/layout/` 的 13 个 Kotlin 对应测试文件：`AttachedInlineBoundaryRelocationTest`、`AutoSpaceSingleGapTest`、`BaselineAlignmentTest`、`BilingualEmphasisTest`、`BopomofoLayoutTest`、`ExplainableStubParagraphLayoutEngineTest`、`FontInstanceMetricsRequestTest`、`InlineBoxLayoutTest`、`InlineObjectLayoutTest`、`RubyLayoutTest`、`SpacingAndLineGeometryEngineTest`、`VerbatimRangeAutoSpaceTest` 与 `WidthIndependentAnnotationCacheTest`。该批按本文合并规则保留 autospace、baseline、ruby/注音/行内对象、attached inline、paragraph entry、metrics instance 与 width-independent cache 的代表性分支；cache 仅验证三种确定宽度、key、命中与 LRU，不迁移宽度扫掠或伪随机拖动。所有新增测试仅使用 Rust 公开 API、deterministic stub 及真实 engine cache pipeline。
- U4 验收：`cargo test --manifest-path /home/icemic/workspace/tiqian-rs/Cargo.toml -- --test-threads=1 --nocapture` 通过（5 个保留内联测试、235 个 integration 测试）；`bash /home/icemic/workspace/tiqian-rs/tools/verify-all-fixtures.sh` 输出 49/49 golden matched。
- 2026-08-28：逐项复核本文件的 57 个 Kotlin 测试文件、查询/cache/hyphenation 子清单及 Rust 平行测试树；所有 checkbox 均有对应可执行断言。补充 `ExplainableStubParagraphLayoutEngineTest` 的 fallback/display shaping range 与 combining-mark shaping-run 断言；`WidthIndependentAnnotationCacheTest` 的 LRU 场景改为访问第一项后插入第三项，明确验证第二项被淘汰，以区分 LRU 与 FIFO。`ClusterRoleResolution.rs` 负责 emoji role range 与 font decision 矩阵；paragraph entry 保留一个互补的 `shaping_decisions` 断言，验证 geometry-only boundary 保持完整 emoji、style boundary 使 shaping range 分裂。复验 `cargo test --manifest-path /home/icemic/workspace/tiqian-rs/Cargo.toml -- --test-threads=1 --nocapture` 通过（5 个保留内联测试、235 个 integration 测试），并复验 `bash /home/icemic/workspace/tiqian-rs/tools/verify-all-fixtures.sh` 输出 49/49 golden matched。

## 本迭代之后

完成 U1–U4 的 Rust 平行测试树迁移和全量 fixture 回归后，进入 Rust 契约与发布准备。