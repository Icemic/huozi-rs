# Huozi × Tiqian 整合 Roadmap

本文是 Huozi 与 Tiqian 长期整合工作的总入口，用于记录目标、职责边界、阶段划分、验收原则与待决策事项。具体领域分析、架构决策和每个阶段的实施计划将在独立文档中展开。

## 背景与目标

Huozi 当前覆盖从字体读取、字形测量、SDF 图集生成到 WGPU 渲染数据输出的链路，但排版仍以逐字符测量和按宽度换行为主。Tiqian 已实现较完整的简体中文横排流程，包括字体策略、shaping 接口、字体度量归一、标点空间、断行、禁则修复、行调整和可解释的布局结果。

本次整合的目标是：

1. 将 Tiqian 的排版能力整理并移植到独立的 Rust 项目 `tiqian-rs`。
2. 由 `tiqian-rs` 替代 Huozi 现有的段落布局算法。
3. Huozi 继续负责字体资源、字形提取与图集、渲染数据生成，并补齐 shaping 等上游能力。
4. 让测量、排版和绘制共享同一份字体、glyph 与 placement 证据。
5. 通过分阶段交付控制迁移风险，保持 Huozi 的既有使用场景和 WGPU 输出能力可验证。

当前目标仍以横排为主。竖排、日文 JLREQ、韩文 KLREQ、分页、多栏和编辑器能力不在本轮默认范围内；是否纳入后续工作，需要单独决策。

## 当前事实基线

### Huozi

当前链路大致为：

```text
源文本 / Segment
  -> 富文本标签解析
  -> TextSpan / TextRun / SourceRange
  -> 单字符字形读取与测量
  -> 按显式换行或盒宽换行
  -> SDF glyph atlas
  -> GlyphVertices / SegmentGlyphSpan
  -> WGPU 或其他宿主渲染
```

已经具备：

- 字体文件读取与可替换的字形提取后端；
- 字体和 glyph metrics；
- 字形 bitmap、SDF 生成、图集与缓存；
- 填充、描边、阴影所需的顶点和纹理数据；
- 富文本解析及 UTF-8 源范围；
- Windows、macOS、Linux、Android、iOS 与 WebAssembly 的既有目标。

已确认需要补齐或重构的能力包括：

- 生产环境 shaping 流程及 glyph cluster 模型；
- 多字体选择与 fallback；
- CJK 断行机会、禁则与标点空间；
- layout 与渲染数据之间稳定、可回放的中间结果；
- 部分字体后端尚未实现的度量能力。

### Tiqian

Tiqian 当前的核心流程大致为：

```text
source text + style + annotations + constraints
  -> 字体角色与 fallback
  -> 平台 shaping adapter
  -> 字体度量归一
  -> 标点 atom / glue
  -> 断行候选与 mandatory break
  -> line breaking + kinsoku repair
  -> compression / justification / neighbor adjustment
  -> LayoutResult + structured decisions
  -> 平台 renderer
```

Tiqian 的排版核心拥有规则与布局真值；平台适配层只提供字体、shaping、glyph metrics 和绘制证据。其当前实现范围是简体中文横排，并包含 Compose、Android 与 Web 接入验证。

Tiqian 采用 MPL-2.0。`tiqian-rs` 计划按 MPL-2.0 发布，并作为独立 crate 与 Apache-2.0 的 Huozi 组合。在开始移植代码前，仍需核对文件级许可义务、来源标注和发布物中的许可证说明。

## DDD 视角下的职责边界

以下是第一版限界上下文，用于指导调研和讨论，不代表最终 crate 划分。

### 文本与样式上下文

负责源文本、源范围、富文本样式、annotation、inline object 与布局约束。它是宿主输入到排版模型的转换边界。

需要重点解决 Huozi 的 `Segment` / `TextSpan` 模型与 Tiqian 输入模型如何对应，以及 UTF-8 byte range、cluster range、复制和命中测试如何保持一致。

### 字体与 Shaping 上下文

负责字体资源、字体实例标识、fallback 候选、script/language/direction、OpenType shaping、glyph cluster、advance、offset 和 ink bounds。

初步边界为：

- Huozi 管理和读取具体字体资源；
- `tiqian-rs` 定义布局所需的字体选择与 shaping 端口；
- Huozi 或独立 adapter 实现该端口；
- adapter 不拥有断行、标点空间、禁则或两端对齐规则。

字体 fallback 究竟由 `tiqian-rs` 完整决策，还是由 Huozi 提供候选集合后共同完成，需要在领域分析阶段确认。

### 段落排版上下文

负责字符角色、字体度量归一、标点 body/ink/glue、断行候选、强制换行、禁则修复、压缩、拉伸、两端对齐、行盒与 annotation 几何。

这是 `tiqian-rs` 的核心领域，也是从 Tiqian 移植的主要范围。该上下文只消费 shaping 与 metrics 证据，不读取字体文件，不生成 SDF，也不直接生成 WGPU 顶点。

### 字形资产与渲染上下文

负责 glyph bitmap/outline、SDF 图集、缓存、纹理页、绘制批次和 WGPU 顶点。它消费排版结果中的 glyph id、字体实例、placement 与视觉样式，不自行重新 shaping 或修正布局。

该上下文原则上保留在 Huozi。现有的逐字符 glyph 缓存键、图集写入和顶点生成需要升级为可消费 shaped glyph，而不是继续以 Unicode `char` 作为唯一入口。

### 诊断与验证上下文

负责结构化排版决策、布局 dump、fixture、golden、差异报告和跨实现对照。它用于解释每次字体选择、断点、修复和空间调整，并支撑 Kotlin 与 Rust 实现的行为对齐。

该上下文应从第一阶段就建立，不能等移植完成后再补。

## 目标链路

目标架构暂定为：

```text
Huozi source / rich text
  -> 输入转换与 source mapping
  -> tiqian-rs layout input
  -> 字体选择策略
  -> Huozi shaping + metrics adapter
  -> tiqian-rs 段落布局
  -> explainable LayoutResult
  -> Huozi glyph/SDF asset resolution
  -> Huozi render batches / WGPU vertices
```

关键约束：

- source text 不因显示替换、软换行或字体 fallback 而改变；
- shaping 的 cluster、glyph、advance 和 offset 可被绘制阶段原样重放；
- renderer 不持有第二套断行或标点修正规则；
- 每个 heuristic 和降级路径都有名称、结构化决策和 fixture；
- `tiqian-rs` 不依赖 WGPU、SDF 或具体字体读取库；
- Huozi 不复制 `tiqian-rs` 的排版规则。

## 分阶段路线

阶段编号表示依赖关系，不承诺固定版本号或时间。每个阶段开始前应建立独立实施文档，写明范围、fixture、验收命令和退出条件。

### 阶段 0：治理与事实冻结

目标：建立可以安全推进移植的共同事实来源。

主要交付物：

- Tiqian 现有模块、核心类型、pipeline 和 ADR 的能力清单；
- Huozi 当前字体、布局、图集与渲染链路审计；
- MPL-2.0 与 Apache-2.0 组合方式、来源标注和发布要求说明；
- 术语表与领域关系图；
- ADR 模板、阶段文档模板和状态维护规则；
- 代表性 CJK、Latin、中西混排和富文本 fixture 清单。

退出条件：两边当前能力、缺口和不可破坏行为有文档证据；尚未确认的事项被列为显式决策，不隐藏在实现假设中。

### 阶段 1：领域建模与契约设计

目标：在写移植代码前确定稳定的领域边界和最小契约。

主要交付物：

- 文本、style span、source range、annotation、constraints 的输入模型；
- 字体请求、字体实例、fallback 证据和 capability issue 模型；
- shaping / metrics provider 契约；
- cluster、glyph placement、line box、布局查询和结构化 decision 模型；
- `tiqian-rs` 与 Huozi 的上下文映射及防腐层设计；
- crate 边界候选及依赖方向 ADR。

退出条件：可以用 stub adapter 构造完整输入并得到确定性的空或最小布局结果；核心契约不依赖 Huozi、WGPU 或具体字体后端。

### 阶段 2：对照测试基线

目标：先建立 Kotlin Tiqian 与未来 Rust 实现之间的行为对照，再逐步移植算法。

主要交付物：

- 可跨语言读取的 fixture 与规范化 dump 格式；
- Kotlin 侧参考输出；
- Rust 侧 golden test harness；
- 浮点误差、字体证据差异和平台差异的比较规则；
- 至少覆盖 source range、mandatory break、cluster、line box 和 decision 的最小语料。

退出条件：Rust 测试能够读取同一 fixture，并明确报告“模型缺失”“行为差异”或“平台证据差异”，而不是只比较截图。

### 阶段 3：`tiqian-rs` 核心骨架与基础规则

目标：建立平台无关、可解释的 Rust 排版核心。

建议按可独立验收的能力切片推进：

1. source-faithful 文本、强制换行与 cluster 连续性；
2. 字符角色、字体角色、fallback 决策与度量归一；
3. UAX #14 断行候选、Latin 分词和词距；
4. 标点分类、body/ink/glue 与行边处理；
5. line breaking、禁则修复与结构化决策。

退出条件：基础横排 fixture 在 stub shaping 下与 Kotlin 参考结果达到约定的一致性，所有差异都有分类和说明。

### 阶段 4：行调整与正文能力迁移

目标：移植 Tiqian 中依赖基础断行结果的中文正文能力。

候选切片包括：

- 分层挤压与 PushIn / Carry / Hang 等修复；
- 中西混排自动间距；
- 两端对齐与末行对齐；
- 段首缩进、整段缩进与行长量化；
- 邻行均摊；
- 西文音节连字；
- 富文本字号、字重、斜体和字体族对几何的影响。

着重号、专名号、书名号、示亡号、ruby、注音、列表和 inline object 应根据 Huozi 的实际使用优先级另行排序，不在本总文档中默认全部首发。

退出条件：选定的正文能力在同一语料、同一 shaping 证据下与 Kotlin 参考实现达到约定一致性，并产出可检查的布局报告。

### 阶段 5：Huozi 字体与 Shaping 基础设施

目标：让 Huozi 提供 `tiqian-rs` 所需的真实字体证据。

主要交付物：

- 多字体资源与稳定的 font instance identity；
- 字符覆盖检查和 fallback 候选能力；
- 基于 Rust shaping 库的 glyph run、cluster、advance 与 offset；
- raw font metrics、ink bounds 和 OpenType 表读取；
- 字号、字重、斜体、script、language、direction 与 feature 设置；
- 缺失字体、缺失 glyph、不完整 bounds 等具名 capability issue；
- shaping 缓存与字体生命周期策略。

退出条件：Huozi adapter 能向 `tiqian-rs` 提供可回放的真实 shaping 结果，并证明测量与后续绘制使用相同字体实例和 glyph id。

### 阶段 6：Huozi 集成与旧布局替换

目标：以 `tiqian-rs` 输出替换 Huozi 现有逐字符布局，同时保留现有图集和渲染输出能力。

主要交付物：

- `Segment` / `TextSpan` 到 `tiqian-rs` 输入的转换；
- `LayoutResult` 到 glyph/SDF 资产和渲染批次的转换；
- shaped glyph id 进入图集与缓存的路径；
- `SegmentGlyphSpan` 或后继 source mapping API；
- 旧 API 的兼容、迁移或废弃策略；
- 旧布局与新布局的可切换对照入口，仅用于迁移验证；
- WGPU 示例与跨平台构建验证。

退出条件：默认 Huozi 路径使用 `tiqian-rs`；测量和绘制不发生二次 shaping；已有渲染效果及选定排版 fixture 通过自动化和人工检查。

### 阶段 7：收口、性能与发布

目标：完成生产化收口，移除迁移期重复实现。

主要交付物：

- 删除或隔离 Huozi 旧布局算法；
- profiling、benchmark 与内存基线；
- shaping、layout、glyph atlas 各层缓存策略；
- fuzz / property test，重点覆盖 source range、cluster 和极端约束；
- `tiqian-rs` 与 Huozi 的版本兼容策略；
- 发布文档、迁移指南、许可证与第三方声明；
- Desktop、移动端和 WebAssembly 的发布前验证矩阵。

退出条件：公共 API、错误模型、性能预算和支持平台均有明确承诺；不存在 renderer 侧布局补丁或无人维护的双实现。

## Huozi 基础能力补充清单

以下清单用于后续审计，不代表已经选定具体依赖或实现方式。

| 能力 | 当前观察 | 对 Tiqian 排版的意义 | 计划阶段 |
| --- | --- | --- | --- |
| Shaping | 生产布局仍逐 `char` 读取 glyph | 连字、组合字符、glyph offset、cluster 和脚本特性依赖它 | 5 |
| 多字体与 fallback | 当前主入口使用单字体 | 中文、Latin、标点和缺字需要稳定的字体选择证据 | 5 |
| Font identity | 尚未形成跨 shaping/atlas/render 的稳定标识 | 保证测量与绘制同源 | 1、5 |
| Ink bounds | 后端能力不完全一致 | 标点 body、悬挂、压缩和碰撞判断需要真实墨迹边界 | 5 |
| OpenType metrics | 需要系统审计 | CJK baseline、字身框与 `ic` 等模型需要可靠度量 | 5 |
| Glyph-id 图集入口 | 当前核心入口以字符为中心 | shaped glyph 不能总由单个 Unicode 字符表示 | 5、6 |
| Source-to-cluster mapping | 当前主要保留 TextRun byte range | 复制、命中、断行和富文本跨 cluster 需要更细映射 | 1、6 |
| Capability issue | 当前多为 warning 或未实现 | 缺少证据时必须具名降级，不能静默猜测 | 1、5 |
| 可解释布局报告 | 当前缺少统一结构 | 移植对照、回归定位和规则审计依赖它 | 2 |

## 文档体系

后续文档统一放在 `huozi-rs/docs`，建议按用途组织：

```text
docs/
  roadmap.md                 # 本文：总入口与总体状态
  architecture.md            # 整合后的当前架构，形成后再创建
  domain-model.md            # 统一语言、限界上下文与上下文关系
  capability-audit.md        # Huozi / Tiqian 能力与缺口证据
  migration/                 # 各阶段实施文档与对照记录
  adr/                       # 跨项目和 Rust 实现的新架构决策
```

规则：

- roadmap 只维护总目标、阶段状态和入口链接；
- 改变职责边界、公共契约、许可策略或关键排版取舍时记录 ADR；
- 每个实施阶段写清 fixture、验收命令和退出条件；
- Tiqian 的历史 ADR 是移植依据，不直接改写为 Huozi 的当前事实；
- 普通 bug、测试补充和文档修正不单独创建阶段。

## 验证原则

每个影响断行、字体选择、标点空间、行高或 glyph placement 的切片至少包含：

1. 代表性 fixture 与结构化 decision；
2. Rust 单元测试或 golden；
3. 与 Kotlin 参考实现的差异检查，直至完成移植或明确修订取舍；
4. 使用真实字体的集成测试；
5. Huozi 最终渲染结果的人工检查。

不能只以截图相似作为验收。至少还要检查 source range、cluster、glyph id、advance、line break、baseline、visual bounds 和降级原因。

## 主要风险

### 许可与来源追踪

`tiqian-rs` 采用 MPL-2.0，Huozi 采用 Apache-2.0。独立 crate 的组合边界需要在发布前保持清晰，移植文件需要保留必要的许可和来源信息。

### 机械翻译导致边界固化

Kotlin 模块结构服务于 KMP 和现有前端，不应直接等同于 Rust crate 结构。先迁移领域模型和行为，再根据 Rust 的编译边界、feature 与平台需求决定物理拆分。

### Shaping 与绘制分叉

如果布局按一套字体和 glyph 度量计算，而 SDF/renderer 又按字符重新选 glyph，会产生不可修复的几何偏差。glyph id、font identity 和 placement 必须贯穿布局到绘制。

### 平台浮点与字体差异

不同 shaping 后端和字体版本会造成可接受的数值差异，也可能暴露真实规则错误。对照工具需要区分算法差异、字体证据差异和容差内误差。

### 一次迁移范围过大

Tiqian 已包含大量正文、富文本与前端能力。首个可用版本需要按 Huozi 的真实使用需求选择能力，不能以“完整搬完全部 Kotlin 代码”作为唯一交付单位。

## 待决策事项

以下问题必须在对应阶段开始前由项目负责人确认；本 roadmap 不替代这些决定：

1. `tiqian-rs` 首个可用版本需要覆盖哪些 Tiqian 能力，哪些 decoration、ruby、列表或 Web 专属能力延后？
2. 字体 fallback 的最终决策权位于 `tiqian-rs`，还是由 Huozi 提供已排序字体候选并让 `tiqian-rs` 校验？
3. `tiqian-rs` 首版是否只支持横排和从左到右文本，还是契约从一开始就需要可表达 RTL？
4. Huozi 现有 public layout API 与 `GlyphVertices` 输出需要保持到何种兼容级别？
5. Kotlin Tiqian 在 Rust 移植期间继续独立演进，还是需要设定一个参考 commit 并定期同步？
6. `tiqian-rs` 的仓库位置、发布名称、版本策略和维护关系如何安排？
7. 首选 shaping / font parsing / rasterization 依赖组合是什么，现有多个 `GlyphExtractorTrait` 后端是否继续保留？

## 当前状态

| 阶段 | 状态 | 说明 |
| --- | --- | --- |
| 0. 治理与事实冻结 | `todo` | 已有第一轮简要调研；尚未形成正式能力审计、术语表与许可说明 |
| 1. 领域建模与契约设计 | `todo` | 未开始 |
| 2. 对照测试基线 | `todo` | 未开始 |
| 3. 核心骨架与基础规则 | `todo` | 未开始 |
| 4. 行调整与正文能力迁移 | `todo` | 未开始 |
| 5. Huozi 字体与 Shaping 基础设施 | `todo` | 未开始 |
| 6. Huozi 集成与旧布局替换 | `todo` | 未开始 |
| 7. 收口、性能与发布 | `todo` | 未开始 |

状态含义：

- `todo`：尚未开始；
- `wip`：已经开始，但未满足退出条件；
- `blocked`：存在必须由负责人或外部条件解除的阻塞；
- `done`：交付物、测试和人工验收均已完成。

## 下一步

进入阶段 0，先产出 `capability-audit.md`：按同一条端到端 pipeline 对照 Huozi 与 Tiqian 的数据模型、能力、证据来源和缺口。审计完成后，再讨论首版范围和 `domain-model.md`，避免在事实不完整时提前确定 crate 与 API。
