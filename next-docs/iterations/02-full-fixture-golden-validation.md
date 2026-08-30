# 迭代 02：全量 Tiqian Fixture Golden 验证

- 状态：`completed`
- 日期：2026-08-25
- 范围：通过全部 `EarlyLayoutFixtures` 的原始 layout dump golden，完成确定性 stub 下的完整段落级跨语言验收
- 前置迭代：[迭代 01：Tiqian 排版核心翻译与初步验证](01-tiqian-fixture-parity.md)
- 参考实现：Tiqian `eb26f889c57d50e52e41d3a76185cdb6a3bdba45`；`kinsoku-push-in` 的 fixture/grid 修正与更新 golden 在该基线后单独记录
- 总体路线：[Huozi × Tiqian 整合 Roadmap](../roadmap.md)

## 目标

1. 使用迭代 01 建立的单 fixture 验证命令，逐个验证全部 49 个 Tiqian `EarlyLayoutFixtures`。
2. 每个 fixture 都以实际 Kotlin 输入、确定性 stub、greedy、lookahead、paragraph-dp 三种 breaker 和 Tiqian checked-in golden 完成字节级对比。
3. 按能力主题定位并修正 Rust 严格镜像与 Kotlin 的差异；所有算法判断均回查唯一对应的 Kotlin 源码与测试。
4. 盘点不经过 `EarlyLayoutFixtures` 的 Kotlin 直接单元测试，作为后续迭代的工作清单，不在本迭代创建 Rust 测试语料或手写断言预期。

## 已确定边界

- Kotlin `EarlyLayoutFixtures` 是唯一 fixture 输入真值，Tiqian checked-in layout dump 是唯一 golden 真值。
- 继续使用 `:engine:exportLayoutFixture`、`fixture_layout_dump` 与 `tools/verify-fixture.sh`；不复制 fixture/golden，不建立另一套 Rust fixture 或 expected 输出。
- 每次命令只验证一个 fixture；脚本从 Tiqian 读取原 golden，失败时输出 unified diff 与首个不同的 dump 行。
- 当前 adapter 与 runner 只承担输入映射、确定性 stub 调用和 Kotlin dump 格式化；不得在其中按 fixture id、预期结果或差异添加排版规则。
- 平台 shaping、真实字体、Skia、Android、Web、Core Text、benchmark、调参实验、报告 probe 与工具入口不属于本迭代。
- 不经过 `EarlyLayoutFixtures` 的 Kotlin 直接单元测试在本迭代只盘点，不翻写为 Rust `#[test]`、integration test 或独立测试数据。

## 验收链路

```text
Kotlin EarlyLayoutFixture
  -> :engine:exportLayoutFixture -PfixtureId=<id>
  -> JSON（stdout / stdin）
  -> tiqian-rs fixture_layout_dump + deterministic stub
  -> greedy / lookahead / paragraph-dp layout dump
  -> Tiqian checked-in golden 的字节级比较
```

同一 fixture 的 Kotlin 基线继续由原测试验证：

```text
EarlyLayoutFixtures
  -> Kotlin ExplainableStubParagraphLayoutEngine
  -> LayoutDumpGoldenTest
  -> Tiqian checked-in golden
```

## 执行与验收规则

每个主题批次按下列顺序执行：

1. 先运行该批次的原 fixture/golden；不从 golden 文本猜测算法。
2. 差异先核对 Kotlin exporter 的 JSON、breaker、hyphenator、`pinBasicNoHang` 与 deterministic stub 参数。
3. 输入和运行参数一致后，回查触发差异的 Kotlin 测试及唯一对应的 Rust 镜像文件，修复 Rust 实现。
4. 重跑当前 fixture、当前主题批次和受影响的前序批次；通过后记录结果再进入下一批。
5. 每批完成后运行 Tiqian `LayoutDumpGoldenTest`；迭代结束时运行全部 49 个 fixture、`cargo test`、`cargo check` 与两仓库的 `git diff --check`。

发现以下情况时停止当前修复并向负责人确认：Tiqian fixture/golden 与源码、测试或 ADR 明显矛盾；对齐需要有意复制疑似 Kotlin 缺陷；需要改动 Tiqian golden；或需要引入非确定性平台证据。

## Fixture 验收批次

### F1：字体角色、引号与标点几何（11）

- [x] `basic-pause-stop`
- [x] `ellipsis-and-dash`
- [x] `nested-quotes`
- [x] `adjacent-punctuation-spacing`
- [x] `contextual-curly-quotes`
- [x] `mixed-script-quote-paragraph-language`
- [x] `adjacent-curly-quote-list-context`
- [x] `mi10s-adjacent-curly-quote-wrap`
- [x] `unmatched-curly-quotes`
- [x] `fallback-roles`
- [x] `ascii-brackets-in-cjk`

验收重点：font role/fallback、display substitution、引号上下文、标点 body/glue、相邻标点压缩和结构化 punctuation/role decisions。

### F2：断行、禁则与修复（10）

- [x] `mi10s-western-bracket-citation-wrap`
- [x] `bibliographic-numeric-locator-break`
- [x] `greedy-multi-line`
- [x] `kinsoku-carry-previous`
- [x] `kinsoku-push-in`
- [x] `lookahead-future-push-in`
- [x] `lookahead-avoids-repair`
- [x] `ascii-point-mark-in-cjk`
- [x] `ascii-point-mark-impossible-measure`
- [x] `line-end-kinsoku`

验收重点：UAX #14 边界、数字/符号 cohesion、greedy/lookahead/DP 选行、CarryPrevious、CarryNext、PushIn、Hang、repair candidate 与 contextual kinsoku decisions。

### F3：行调整、两端对齐与缩进（7）

- [x] `justify-cjk-paragraph`
- [x] `justify-mixed-paragraph`
- [x] `justify-unbreakable-number-symbol`
- [x] `real-paragraph-1`
- [x] `first-line-indent`
- [x] `adaptive-short-line-indent`
- [x] `indent-opening-quote`

验收重点：CJK/中西边界 stretch、不可拉伸 cohesion、compression、line-length grid、block/first-line indent、opening punctuation 的行首 trim 与行几何。

### F4：Latin、技术文本与强制断行控制（16）

- [x] `latin-word-wrap`
- [x] `latin-camelcase`
- [x] `latin-existing-hyphen`
- [x] `latin-hard-break`
- [x] `latin-opaque-url-token`
- [x] `zero-width-space-soft-break`
- [x] `western-hyphenation`
- [x] `progressive-technical-inline`
- [x] `progressive-technical-hash-fill`
- [x] `progressive-technical-alpha-numeric`
- [x] `progressive-technical-current-line-emergency`
- [x] `mandatory-single-newline`
- [x] `mandatory-blank-lines`
- [x] `mandatory-leading-trailing-newline`
- [x] `mandatory-crlf`
- [x] `mandatory-wraps-long-line`

验收重点：Latin segmentation、existing/synthetic hyphen、English hyphenation、progressive technical tier、emergency tracking、U+200B、single/CRLF/连续/首尾 mandatory break 与非渲染 control cluster。

### F5：装饰、ruby 与行间几何（5）

- [x] `emphasis-marks`
- [x] `ruby-line-height`
- [x] `bopomofo-tone-em-box`
- [x] `interlinear-lines`
- [x] `mourning-frame`

验收重点：decoration decision/segment、pinyin ruby、Bopomofo placement、行高 floor、annotation geometry 与 source-faithful range。

## 命令

从 `tiqian-rs` 仓库根目录执行单个 fixture：

```shell
bash tools/verify-fixture.sh <fixture-id>
```

Tiqian checkout 不在默认同级 `../tiqian` 时：

```shell
TIQIAN_ROOT=/absolute/path/to/tiqian bash tools/verify-fixture.sh <fixture-id>
```

每个主题批次使用其上方明确列出的 fixture id 逐个运行。迭代结束时应实际运行：

```shell
cargo check
cargo test
./gradlew :engine:jvmTest --tests 'org.tiqian.layout.LayoutDumpGoldenTest'
```

README 中的命令与本节保持一致；命令或入口变化时先更新二者并实际执行。

## 后续 TODO：Kotlin 直接单元测试

以下测试不以 `EarlyLayoutFixtures` 作为输入，常直接构造 `Cluster`、`LineSolution`、profile、shaper/metrics fake 或调用内部函数。它们不能由本迭代的 fixture golden 验收替代，但也不能通过手写 Rust fixture、expected 或 `#[test]` 复制测试语料。

后续迭代必须先决定一个通用的“原 Kotlin 测试调用与断言 → Rust 镜像 API”验证入口；Kotlin 测试中的调用、输入和断言预期仍是唯一真值。该入口确定前，不宣称下面项目已获得跨语言等价覆盖。

### `commonTest`：52 个文件

| 主题 | Kotlin 测试文件 |
| --- | --- |
| `core`（6） | `EastAsianSpacingTest`、`LayoutQueriesTest`、`LinkAddressDisplayTest`、`TextRangeTest`、`UnicodeScriptEvidenceTest`、`UnicodeWordCharacterTest` |
| `clreq`（5） | `BopomofoParserTest`、`ClreqPunctuationGlyphSubstitutorTest`、`KinsokuLevelTest`、`NumberSymbolCohesionTest`、`PunctuationGluePlacementTest` |
| `font`（3） | `CjkFontRoleClassifierTest`、`ScriptAwareFontMetricsNormalizerTest`、`UsesLatinFaceTest` |
| `linebreak`（3） | `LiangHyphenatorTest`、`MandatoryBreakTest`、`UnicodePunctuationLineBreakTest` |
| `shaping`（1） | `ExplainableStubTextShaperTest` |
| `layout`（34） | `AsciiPointMarkKinsokuTest`、`AttachedInlineBoundaryRelocationTest`、`AutoSpaceSingleGapTest`、`BaselineAlignmentTest`、`BilingualEmphasisTest`、`BopomofoLayoutTest`、`DecideHyphenBreakTest`、`DisplayGlyphSubstitutionEngineTest`、`EmergencyGraphemeTrackingTest`、`ExplainableStubParagraphLayoutEngineTest`、`FontInstanceMetricsRequestTest`、`GreedyLineBreakerTest`、`InlineBoxLayoutTest`、`InlineObjectLayoutTest`、`JustifierEngineTest`、`JustifierTest`、`KinsokuAndCohesionRepairEngineTest`、`LineBreakRepairEngineTest`、`LookaheadLineBreakerTest`、`ParagraphDpLineBreakerTest`、`ProgressiveTechnicalBreakTest`、`PunctuationAtomBuilderHaltTest`、`PunctuationBodyFloorInvariantTest`、`PunctuationGeometryEngineTest`、`PunctuationSpacingRuleTest`、`PushInLineWideCapacityTest`、`QuoteClassificationEngineTest`、`QuotePairAnalyzerTest`、`RubyLayoutTest`、`SpacingAndLineGeometryEngineTest`、`UnicodePunctuationBoundaryTest`、`VerbatimRangeAutoSpaceTest`、`WidthIndependentAnnotationCacheTest`、`ZeroWidthBreakControlLayoutTest` |

### JVM 直接功能测试：5 个文件

- `linebreak/EnglishHyphenationTest`
- `layout/HyphenationLayoutTest`
- `layout/JustifierCompressionTest`
- `layout/LineAdjustmentPushInTest`
- `layout/OpeningBracketLineStartTest`

`layout/LayoutDumpGoldenTest` 不在上表中：其 49 个 fixture 已由本迭代通过同一份 Kotlin fixture 与 golden 跨语言验证。`KinsokuHangingExperimentProbe`、`LayoutBenchmarkProbe`、`LookaheadWindowProbe`、`ParagraphDpReferenceExperiment`、`ParagraphDpTuningProbe`、`ParagraphScaleBenchmarkProbe`、`LayoutReport*`、`ReadmeSampleMain` 与其他工具入口属于性能、实验、报告或工具工作，不作为后续直接单测跨语言验收项。

## 完成条件

- 全部 49 个 `EarlyLayoutFixtures` 均通过 `tools/verify-fixture.sh` 的原始 Tiqian golden 字节级比较；每个比较均包含 greedy、lookahead、paragraph-dp。
- 五个 fixture 主题批次的结构化差异均已修复，或在停止条件下获得负责人明确决定；没有在 exporter、runner、fixture 或 golden 中引入排版补丁。
- Tiqian `LayoutDumpGoldenTest`、Rust `cargo check` 与 `cargo test` 通过。
- `tiqian-rs` 与 Tiqian 的 `git diff --check` 通过；README 的验证命令已实际执行。
- 本文的 52 个 `commonTest` 文件和 5 个 JVM 直接功能测试文件已作为后续 TODO 保留，且没有被错误表述为本迭代已覆盖。
- 实施记录写明参考 Tiqian commit、每个主题批次的实际结果、执行命令和任何负责人决定。

## 实施记录

- 2026-08-25：F1 完成。11 个 fixture 均通过 `bash tools/verify-fixture.sh <fixture-id>` 的字节级比较；每次比较均覆盖 greedy、lookahead、paragraph-dp。未发现 Rust/Kotlin 行为差异，未修改 Rust 排版实现、fixture、golden、exporter 或 runner。
- F1 实际验证：`cargo check --bin fixture_layout_dump`、`cargo test`、全部 11 个 F1 fixture 命令，以及 Tiqian `./gradlew :engine:jvmTest --tests 'org.tiqian.layout.LayoutDumpGoldenTest'`。Kotlin golden 测试通过；Linux 主机仅报告现有 Apple Kotlin/Native cinterop target 已禁用的 Gradle 警告。
- Rust 当前仍有 6 条既有编译警告（未使用 import/变量、冗余类型括号和未读取字段）；它们与 F1 fixture 输出无关，本批未顺手修改。
- 2026-08-25：F2 完成。10 个 fixture 均通过同一字节级比较，覆盖 UAX #14 西文括号边界、文献数字定位、greedy/lookahead/paragraph-dp、CarryPrevious、PushIn、ASCII 点号禁则与极窄悬挂、行尾禁则；未发现差异或修复项。
- 2026-08-25：F3 完成。7 个 fixture 均通过同一字节级比较，覆盖 CJK/混排行调整、不可拆数字符号、line-length grid、首行缩进及其窄行自适应、开括号行首 trim；未发现差异或修复项。
- 2026-08-25：F4 完成。16 个 fixture 均通过同一字节级比较，覆盖 Latin 分段、既有/合成连字符、U+200B、技术文本分层断点、grapheme emergency tracking 与单行/连续/首尾/CRLF mandatory break。首轮仅 `mandatory-blank-lines` 失败：Rust 空 `f32::sum()` 返回 `-0.0`，而 Kotlin 空 `sumOf` 返回 `+0.0`，使尾随空行的 `adjustedWidth`/`visualWidth` 产生可观察 dump 差异。`LineGeometryStage` 现于每个相关 `f32::sum()` 后加 `0.0`，使空和保持 Kotlin 的正零语义；全 F4 回归通过。
- 2026-08-25：F5 完成。5 个 fixture 均通过同一字节级比较，覆盖着重号、拼音 ruby 的按行加高、注音调号 em box、行间专名号/书名号与示亡号几何；未发现差异或修复项。
- 2026-08-25：迭代完成。最终全量回归中，49 个 fixture 均通过原始 Tiqian golden 的字节级比较，每项覆盖 greedy、lookahead、paragraph-dp。`cargo check --bin fixture_layout_dump`、`cargo test`、Tiqian `./gradlew :engine:jvmTest --tests 'org.tiqian.layout.LayoutDumpGoldenTest'` 与两仓库 `git diff --check` 均通过。Rust 当前没有独立测试用例；`cargo test` 的两个 test target 与 doc-tests 均以 0 个测试成功结束。

## 本迭代之后

下一迭代首先设计并确认通用的 Kotlin 直接单元测试跨语言验证入口，再逐主题处理本文件列出的 57 个直接测试文件。完成该验证后，才进入 Rust 契约与发布准备。