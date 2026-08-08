# Tiqian-rs 移植测试与证据链讨论纪要

- 日期：2026-07-26
- 状态：已确认，作为后续测试与工具方案草案的直接输入
- 关联：[整合 Roadmap](../roadmap.md) · [移植边界与数据契约纪要](2026-07-26-port-boundary-and-data-contract.md) · [Tiqian 当前内部结构](../references/tiqian-current-structure.md)

## 会议议题

本次会议讨论如何在完整移植 Tiqian 排版能力的过程中验证 `tiqian-rs` 与 Kotlin 原版的一致性，以及这些测试数据如何继续用于未来的 Huozi 集成验收。

为方便描述，将当前链路分成三部分：

```text
原始宿主输入
  -> A：宿主 lowering、字体、fallback、shaping、raw metrics
  -> B：断行、CLREQ、paragraph layout、repair、adjustment
  -> C：glyph replay、SDF/图集、顶点和渲染
```

`tiqian-rs` 移植 B。未来 Huozi 负责 A、实现 B 所需的 backend，并负责 C。

会议重点讨论：

1. 每组测试数据需要保存哪些边界数据；
2. 如何隔离布局算法与字体、平台 shaping 的差异；
3. 原版截图与 Huozi 截图分别验证什么；
4. 如何组织数据集、比较规则和 golden 更新流程。

## 最终决策

### 1. 每组 fixture 保存五类证据

每组 fixture 包含：

```text
1. 原始输入
2. 规范化 LayoutInput
2.5. Backend 请求与响应证据
3. 规范化 LayoutResult + LayoutDebugInfo
4. 原版渲染参考产物
```

这些数据分别承担不同责任，不能只保留最终截图。

#### 1. 原始输入

保存直接交给 A 的宿主数据，包括：

- source text；
- 富文本与样式范围；
- paragraph style 和 constraints；
- annotations 与 inline objects；
- 宿主字体和其他影响 lowering 的设置。

它用于未来验证 Huozi 从调用者输入到最终渲染的端到端链路。

#### 2. 规范化 `LayoutInput`

保存 A 输出、B 输入的完整领域数据。它不是 Kotlin `LayoutInput` 的直接序列化，而是跨 Kotlin/Rust 的规范化 fixture 格式。

规范化输入至少包括：

- UTF-8 source text 与 ranges；
- grapheme table；
- style ranges 和 source boundaries；
- paragraph style、writing mode 与 constraints；
- profile；
- decorations、ruby、inline boxes 和 inline objects。

Kotlin exporter 负责把 Kotlin 使用的 UTF-16 offsets 转为 UTF-8 byte offsets。Rust 测试直接读取规范化格式。

#### 2.5. Backend 请求与响应证据

B 运行时仍会向 A 请求 font resolution、fallback、shaping 和 raw metrics，因此 `LayoutInput` 本身不足以确定 `LayoutResult`。

fixture 需要保存：

- font resolution 请求与响应；
- shaping 请求与 `ShapedRun`；
- raw font metrics 请求与响应；
- missing glyph、missing ink bounds 和 capability issues；
- display substitution、source rollback、插入连字符等路径需要的证据。

Rust 测试使用 recorded/replay backend，根据请求的语义 key 返回原版记录的响应，使 Kotlin 与 Rust 在相同字体和 shaping 证据上运行。

#### 3. 规范化 `LayoutResult` 与 `LayoutDebugInfo`

这是完整 port 的核心 golden，至少包括：

- source/display text 和 UTF-8 ranges；
- grapheme、shaping cluster、layout cluster 与 glyph range 映射；
- font fixture key、glyph id、advance、offset 和 ink bounds；
- line ranges、baseline 和 line boxes；
- natural、adjusted 与 visual width；
- break、end reason 和 repair；
- 标点 body/glue、compression 和 justification；
- decoration、ruby、inline object 等几何；
- 结构化 layout decisions 与 capability issues。

结构化 decisions 用于定位差异属于 font、shaping、标点、断行、repair 还是行调整，不能只比较最终坐标。

#### 4. 原版渲染参考产物

保存 Kotlin Tiqian 原版在 C 后的参考截图或其他可视产物，用于视觉对照：

- 断行；
- baseline；
- 标点位置；
- 行距；
- glyph 重叠或错位；
- decoration 和 ruby 几何。

原版与 Huozi 的 rasterizer、抗锯齿、hinting、gamma 和 shader 不同，因此原版截图不作为 Huozi 的逐像素 golden。

### 2. Backend 以语义 key 回放，不固化调用顺序

主测试通过 request key 查找记录响应，不要求 Rust 与 Kotlin 以完全相同的顺序调用 backend。

request key 应包含与结果相关的语义信息，例如：

- operation；
- source/display text；
- source range；
- 稳定的 fixture font key；
- 字号、字重、斜体和 language；
- OpenType features 和 variation axes。

测试允许：

- 调用顺序不同；
- 重复请求；
- cache 策略不同。

少数必须发生的行为通过独立交互断言验证，例如 display substitution 缺字后发生 source rollback，或断词插入连字符后请求对应 shaping。

### 3. Kotlin 侧必须验证 recorded backend 可以重放

fixture exporter 不能只记录一次真实运行。导出后，Kotlin B 必须使用 recorded backend 重跑，并验证仍能得到同一规范化 `LayoutResult`。

```text
规范化 LayoutInput
  + recorded backend evidence
  -> Kotlin B replay
  -> 规范化 LayoutResult
  -> 与导出结果比较
```

只有通过该自校验，才能证明 backend evidence 已覆盖 B 依赖的外部状态。

### 4. 建立三类数据集

#### 算法数据集

使用 Tiqian 的 deterministic backend，保存：

- 规范化 `LayoutInput`；
- deterministic backend evidence；
- 规范化 `LayoutResult`；
- 完整 `LayoutDebugInfo`。

该数据集不依赖机器字体，数值稳定，是 port 过程的主要验收集。现有 `test-support` fixtures 和 `LayoutDumpGoldenTest` 是重要来源。

#### 固定字体数据集

使用仓库内可合法再分发的固定字体和明确的 shaping backend，验证：

- ligature 与 combining mark；
- font fallback；
- glyph id、advance 和 offset；
- ink bounds；
- `halt` 等 OpenType feature；
- 测量与绘制同源。

每组数据记录字体文件、SHA-256、face index、shaper 和版本、features、variation axes 与字号。

#### 平台验收数据集

使用 Compose Desktop、Android 或 Web 真实路径，验证：

- 宿主 lowering；
- 平台 adapter；
- renderer replay；
- capability issues；
- 真实截图和平台行为。

该数据集需要记录平台、OS、字体、density 和 renderer，不要求不同平台逐像素一致。

### 5. Port 与 Huozi 集成使用同一证据链

#### `tiqian-rs` port 验收

```text
2. 规范化 LayoutInput
  + 2.5. Backend evidence
  -> tiqian-rs + replay backend
  -> actual LayoutResult
  -> 对比 3. 规范化 LayoutResult
```

这条测试隔离 B，确认差异来自 port 的数据模型或算法，而不是平台字体变化。

#### Huozi 集成验收

```text
1. 原始输入
  -> Huozi A
  -> tiqian-rs B
  -> Huozi C
```

集成测试需要同时捕获并比较中间结果：

- Huozi 生成的 `LayoutInput` 对比数据 2；
- Huozi backend 与 `tiqian-rs` 生成的 `LayoutResult` 对比数据 3；
- Huozi 输出对比自己的 SDF/WGPU screenshot golden；
- 原版数据 4 作为跨 renderer 视觉参照。

不能只比较输入与截图，否则失败时无法判断问题在 A、B 还是 C。

### 6. 原版截图和 Huozi 截图分别维护

长期保留两套视觉证据：

| 产物 | 用途 |
| --- | --- |
| Kotlin Tiqian 原版截图 | port 的视觉参考与跨 renderer 人工/感知对照 |
| Huozi 集成后的截图 golden | Huozi raster、SDF、atlas、vertices、shader 的自动回归 |

布局一致性以规范化 `LayoutResult` 为主要证据；截图用于发现几何和 renderer 问题。

### 7. 比较规则按字段类型制定

必须严格相等的内容包括：

- source/display text；
- UTF-8 ranges；
- grapheme/cluster/glyph range 关系；
- glyph count；
- line break 和 end reason；
- repair、profile、policy 与 capability issue；
- annotation 所属行；
- 结构化 decision 类型和选择结果。

在固定 backend 下可以严格比较 glyph id、stable font fixture key、OpenType features 和 shaping cluster mapping。

浮点字段使用按类型定义的绝对与相对容差：

$$
|a-b| \leq \max\left(\varepsilon_{abs}, \varepsilon_{rel}\cdot\max(|a|,|b|)\right)
$$

advance、offset、ink bounds、baseline、line box、adjustment 和 annotation geometry 分别定义容差，不使用一个全局 epsilon。

以下内容不直接比较：

- process-local `FontId`；
- opaque platform font key；
- 对象地址；
- cache 命中和 backend 调用顺序；
- 平台私有句柄。

这些身份在 fixture 中映射为稳定名称。

### 8. Fixture manifest 保存完整来源

每组 fixture 需要保存 manifest，至少包括：

- fixture schema version；
- fixture 名称与能力标签；
- Tiqian reference commit；
- exporter version；
- platform、OS、JDK/Kotlin 和 shaping backend；
- 字体文件、hash 和 face index；
- locale、profile、constraints 与 density；
- generation timestamp。

其中 schema version、reference commit、字体 hash、shaper、profile 和 constraints 是复现结果的关键字段。

### 9. 覆盖按能力矩阵组织

fixture 数量不是主要目标。覆盖矩阵至少包含：

- 文本与 shaping：CJK、Latin、Cyrillic、中西混排、combining mark、ligature、variation selector、fallback、跨 run 边界；
- 断行与标点：mandatory break、CRLF、U+200B、标点分类、PushIn、Hang、Carry、极窄 measure、西文断词；
- 行调整与结构：compression、autospace、word space、CJK justification、缩进、line-length grid、max lines、writing mode 及 unsupported 状态；
- annotations 与 inline：decoration、ruby、Bopomofo、inline box/object、富文本样式和 source boundaries。

每项能力尽量包含正常路径、边界情况和具名降级或失败路径。

### 10. Golden 更新必须受控

确定以下更新纪律：

1. fixture 由固定 exporter 生成，不手工修改大块输出；
2. 每次生成记录 Tiqian reference commit；
3. 变化先区分原版行为变化、fixture schema 变化和 port bug；
4. 更新结果 golden 时逐项检查 structured decision diff；
5. 更新截图时同步检查对应结构化几何；
6. 字体或 shaper 版本变化不能静默批量重生成；
7. port 完成前，原版 fixture 原则上只随明确的 reference commit 更新。

## 后续草案目标

下一份草案将围绕完整 port 的测试和相关工具展开，至少需要定义：

1. fixture 目录结构、schema 和版本管理；
2. Kotlin exporter、recorded backend 与 replay 自校验工具；
3. Rust fixture loader、replay backend 和比较器；
4. 算法、固定字体和平台数据集的生成与运行方式；
5. 字段比较、浮点容差、截图检查和差异报告；
6. 能力覆盖矩阵、参考 commit 与 golden 更新流程。

本纪要只记录已确认的测试证据链和验收原则，不定义工具的最终实现。
