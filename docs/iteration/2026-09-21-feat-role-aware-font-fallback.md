# 按文本角色优先选择已声明用途的字体

> 状态：已完成
>
> 日期：2026-09-21
>
> 分类：feat

## 文档用途

本文定义 Huozi 根据 Tiqian 输出的 `FontRole` 和调用方在 `FontSource` 上声明的字体用途，调整普通文字字体候选顺序的迭代。

本文是本迭代后续讨论、实现、测试和验收的唯一依据。已确认的行为边界、数据模型、候选排序、显式 `[font]` 列表语义、性能约束、验证资产和文档更新要求均记录在本文中；实施不得依赖聊天记录或临时记忆补全规则。

本迭代只改变 Huozi 构造候选 face 的顺序。Tiqian 已完成上下文相关的文字角色判定，包括 CJK 正文、CJK 标点、拉丁正文以及引号等共享标点的上下文归属；Huozi 不重复执行 Unicode script itemization、配对引号分析或 CJK 标点分类。

## 背景与问题

Tiqian 的每个 `FontBackendRequest` 均携带 `FontRole`。当前 Huozi 已将该字段传入 `HuoziFontManager`，但 `HuoziFontManager::candidates()` 没有消费它。

当前 `FontSource` 只有：

```text
FontSource
├─ bytes: Vec<u8>
└─ alias: Option<String>
```

字体目录会缓存 family、静态 style、weight 和 variable font 轴。候选构造行为为：

1. `TextStyle.font_families` 非空且至少命中一个已注册 family 时，按该列表中的声明顺序选择 family。
2. 没有请求 family，或请求列表中的 family 均未注册时，按 `Vec<FontSource>` 的注册顺序选择 family。
3. 每个 family 内按请求的 normal、italic、oblique style 档，再按 CSS Fonts weight 规则排列 face。
4. 按候选顺序完整执行 HarfRust shaping；第一个没有 glyph id `0` 的候选获选。
5. 所有候选均缺字时，保留全序列第一个候选的 shaping 结果。

较早注册的拉丁字体若覆盖某个 CJK 标点，会在 Tiqian 已将该标点判为 `CjkPunctuation` 后先被选中。例如，已注册顺序为 `Inter`、`Source Han Sans SC` 时，CJK 引号会使用 Inter 的字形。

字体字节只能回答“该 face 能否提供某个字形”，不能可靠声明“该字体用于 CJK 正文”或“该字体用于 Latin 正文”。字体用途必须由调用方显式配置。

## 目标

1. 调用方能够为一个 `FontSource` 手动声明 CJK 或 Latin 用途。
2. Tiqian 返回 `CjkText` 或 `CjkPunctuation` 时，Huozi 在当前候选域中优先尝试声明为 CJK 的 face。
3. Tiqian 返回 `LatinText` 时，Huozi 在当前候选域中优先尝试声明为 Latin 的 face。
4. `Symbol`、`Emoji`、`Unknown` 保持当前候选顺序。
5. `[font]` 可传入逗号分隔的 family 列表；角色优先级必须在该显式列表内部生效，不能绕过该列表选择列表外的 family。
6. 未声明用途、没有目标用途字体或优先字体缺字时，保持现有的完整 shaping fallback 与局部退化原则。
7. 候选构造不引入 cmap 预扫描、第二套字体选择规则或重复的文本角色分析。

## 术语与数据模型

### `FontRole`

`FontRole` 由 Tiqian 根据完整文本上下文产生，描述当前 shaping 请求中的文本角色。Huozi 本迭代只消费下列值：

| Tiqian `FontRole` | Huozi 的目标用途 |
| --- | --- |
| `CjkText` | `FontSourceKind::Cjk` |
| `CjkPunctuation` | `FontSourceKind::Cjk` |
| `LatinText` | `FontSourceKind::Latin` |
| `Symbol` | 无目标用途 |
| `Emoji` | 无目标用途 |
| `Unknown` | 无目标用途 |

`FontRole` 不属于字体，不在 Huozi 中重新计算，也不写回 `FontSource`。

### `FontSourceKind`

本迭代新增公开枚举：

```text
FontSourceKind
├─ Cjk
└─ Latin
```

`FontSourceKind` 描述调用方希望字体来源优先服务的文字类别。调用方在 `FontSource` 上明确提供该值，候选排序直接消费这个值。

`FontSource` 新增可选字段：

```text
FontSource
├─ bytes: Vec<u8>
├─ alias: Option<String>
└─ kind: Option<FontSourceKind>
```

语义如下：

| `kind` 值 | 语义 |
| --- | --- |
| `Some(FontSourceKind::Cjk)` | 在 CJK 目标用途的请求中优先。 |
| `Some(FontSourceKind::Latin)` | 在 Latin 目标用途的请求中优先。 |
| `None` | 不进入角色优先组；仍属于通用 fallback 候选。 |

`FontSource::new(bytes)` 和 `FontSource::with_alias(bytes, alias)` 保持其现有行为并创建 `kind = None` 的来源。公开 API 应提供链式的 `with_kind(kind)`，以使别名与用途可独立组合：

```text
FontSource::with_alias(bytes, "Source Han Sans SC".to_owned())
    .with_kind(FontSourceKind::Cjk)
```

本迭代不为 `new`、`with_alias` 和 `kind` 的每种组合新增专用构造器。

### `FontSourceKind` 的作用范围

`FontSourceKind` 由一个 `FontSource` 声明。单个字体集合来源中的所有 collection member 共享该声明；字体目录在展开来源时将其复制到每个内部 `FontFaceRecord`。

同一 family 可能由多个来源、静态 face 或 collection member 组成。它们的 `kind` 可以不同。本迭代不把 family 强制绑定为单一用途，也不做运行时配置校验；候选排序以物理 face 的来源用途为准。因此，同 family 中不同用途的 face 会在不同候选部分中出现，且仍保留各部分内既有的 style 与 weight 选择规则。

`ReplayableFontFaceDescriptor.roles` 当前表示 Huozi 对 Tiqian backend contract 的可处理角色集合。本迭代的 `FontSourceKind` 只影响候选优先级，不改变该 descriptor 字段的含义或值。

## `[font]` 列表语义

parser 已支持逗号分隔的字体 family 列表：

```text
[font="Inter, Source Han Sans SC, Noto Serif CJK SC"]混排文字[/font]
```

它会写入：

```text
TextStyle.font_families = [
    "Inter",
    "Source Han Sans SC",
    "Noto Serif CJK SC",
]
```

该列表定义本次请求的候选域。候选域沿用当前规则：

1. 按声明顺序解析已注册 family，忽略未注册名称。
2. 只要至少解析出一个 family，候选域仅包含这些已注册 family。
3. 全部名称未注册时，候选域退化为全部已注册 family 的默认注册顺序。
4. 列表为空时，候选域为全部已注册 family 的默认注册顺序。

角色优先级只重排候选域内部的 face，不能把请求扩展到显式列表外的 family。例如：

```text
[font="Inter"]中文标点[/font]
```

即使 Huozi 还注册了 CJK 字体，`CjkPunctuation` 请求也只能尝试 Inter 的 face；若缺字，保持当前所有候选都缺字时返回首选 shaping 结果的规则。

## 已确认的候选排序

### 基础候选域

首先构造当前实现已有的基础 family group 序列：

```text
基础 family group 序列
├─ 显式 `[font]` 中已命中的 family，按声明顺序
└─ 或全部已注册 family，按注册顺序
```

每个 family group 中的 face 保持当前登记顺序。后续对一个 face 子集仍使用既有逻辑：

```text
style 档优先级
  → CSS Fonts weight 顺序
  → 完整 shaping 覆盖检查
```

本迭代不改变 variable font 的 `wght`、`ital`、`slnt` 选择，也不改变最终 `FontFaceId`、metrics、bounds、outline 或 SDF replay 的一致性要求。

### 有目标用途的请求

当 role 映射到目标 `FontSourceKind` 时，将基础 family group 中的 face 稳定分成两部分：

1. **优先部分**：`face.kind == Some(target_kind)` 的 face。
2. **剩余部分**：基础候选域中所有其他 face，包括另一种 `FontSourceKind` 和 `None`。

候选序列先追加所有 group 的优先部分，再追加所有 group 的剩余部分。每一部分都按基础 family group 的顺序遍历；同一 group 内继续按现有 style 和 weight 规则排序。

这个定义同时满足以下要求：

- 同一类字体保持调用方的显式列表或注册顺序；
- 无需为优先用途重新扫描、克隆或排序整个字体目录；
- 同一个 family 混有不同用途来源时，目标用途的 face 先参与完整 shaping；
- 优先部分全部缺字后，剩余部分提供当前规则中的通用 fallback；
- 已经尝试且缺字的优先 face 不会在剩余部分重复 shaping。

#### CJK 请求示例

注册 family 的默认顺序为：

| family | `FontSourceKind` |
| --- | --- |
| Inter | Latin |
| Source Han Sans SC | Cjk |
| Noto Color Emoji | `None` |
| Source Han Serif CN | Cjk |

对于 `CjkText` 或 `CjkPunctuation`，候选 family 顺序为：

```text
Source Han Sans SC
Source Han Serif CN
Inter
Noto Color Emoji
```

对于：

```text
[font="Inter, Source Han Serif CN"]中文文字[/font]
```

候选 family 顺序为：

```text
Source Han Serif CN
Inter
```

不能尝试 `[font]` 列表外的 `Source Han Sans SC` 或 `Noto Color Emoji`。

#### Latin 请求示例

使用同一注册顺序时，`LatinText` 的候选 family 顺序为：

```text
Inter
Source Han Sans SC
Noto Color Emoji
Source Han Serif CN
```

对于：

```text
[font="Source Han Serif CN, Inter"]Latin text[/font]
```

候选 family 顺序为：

```text
Inter
Source Han Serif CN
```

### 无目标用途的请求

`Symbol`、`Emoji` 和 `Unknown` 不根据 `FontSourceKind` 重排。候选序列必须与当前基础 family group 序列完全一致。

例如，上述默认注册顺序下，`Emoji` 仍按：

```text
Inter
Source Han Sans SC
Noto Color Emoji
Source Han Serif CN
```

尝试。后续若需要 emoji 或 symbol 专用优先级，应建立独立迭代并新增明确的用途值；本迭代不预留未使用的枚举成员或配置字段。

### 没有匹配用途字体时

如果基础候选域中没有任何 `Some(target_kind)` face，优先部分为空，候选序列就是当前基础候选域序列。结果、候选尝试顺序和缺字退化行为必须与本迭代前一致。

如果优先部分存在但全部 shaping 后包含 glyph id `0`，继续尝试剩余部分。第一个完整 shaping 的 face 获选；全部缺字时仍保留整个候选序列中的第一个结果，即优先部分的第一个候选结果。

## 完整 shaping 与选择边界

候选用途只决定尝试顺序。实际可用性仍由完整 HarfRust shaping 的 glyph id `0` 结果决定：

```text
FontRole
  → 目标 FontSourceKind
  → 稳定构造候选 face 序列
  → 对每个候选完整 shaping
  → 第一个无缺字 face 获选
```

禁止以下实现：

- 根据 cmap、单个代表汉字、Unicode block、字体名或文件名自动推断 `FontSourceKind`；
- 在构造候选前执行额外 cmap 覆盖预检查；
- 为角色优先级另建不经过完整 shaping 的快速路径；
- 在 Huozi 中重做 Tiqian 的引号、破折号、省略号、脚本或 CJK 标点角色分析；
- 因 role priority 让显式 `[font]` 列表泄漏到列表外的 family；
- 对优先部分失败的 face 从头重复完整 shaping。

Huozi 继续遵守 Tiqian ADR R0003：一次 request 只选择一个最终 face，最终 `FontFaceId` 必须贯穿 shaping、metrics、glyph bounds、轮廓回放和 SDF atlas key。

## 公开行为与兼容性

项目处于开发阶段，本迭代可以改变未显式指定 family 时的字体选择结果。

新增或改变的公开行为：

| 情况 | 本迭代后的行为 |
| --- | --- |
| `FontSource::new` / `with_alias` | 默认 `kind = None`，原有调用无需修改。 |
| `FontSource::with_kind` | 调用方可声明 `Cjk` 或 `Latin` 优先用途。 |
| 未指定 `[font]` 的 CJK role | 已声明 CJK 的 face 优先。 |
| 未指定 `[font]` 的 Latin role | 已声明 Latin 的 face 优先。 |
| 指定多个 `[font]` family 的 CJK 或 Latin role | 仅在命中的显式 family 内按用途稳定重排。 |
| 指定单个 `[font]` family | 不会选择该 family 之外的字体。 |
| 没有匹配用途、`kind = None`、`Symbol`、`Emoji`、`Unknown` | 保持现有候选顺序。 |

调用方若需要稳定的 CJK/Latin 标点字形，应在字体注册时为相应来源设置 `FontSourceKind`，并在 `[font]` 列表中包含希望允许的 family。未声明 `kind` 的来源仍可绘制文字，但不会获得 role priority。

## 范围

### 包含

- `FontSourceKind` 的公开定义、重导出与 `FontSource` 可选用途字段；
- 不破坏现有构造器的用途设置 API；
- 字体登记时将来源用途保存到每个物理 face；
- `CjkText`、`CjkPunctuation`、`LatinText` 的候选重排；
- 显式 `[font]` family 列表内的用途重排；
- 没有目标用途、用途未声明和优先候选缺字时的回归验证；
- `examples/render` 的字体注册用途声明、用途切换控件与可视化人工验证；
- `docs/architecture.md`、`docs/parser.md` 和本迭代文档的同步更新。

### 不包含

- 修改 Tiqian 的 `FontRole` 分类、标点归属、quote pair 分析、script itemization、布局或 shaping contract；
- 自动识别字体用途、系统字体发现、平台字体数据库或操作系统 fallback；
- emoji、symbol、数学、注音或其他新 `FontSourceKind`；
- 新增富文本标签来设置字体用途；
- 更改 `[font]` 的解析格式、列表解析、嵌套覆盖或未注册 family 的现有规则；
- 修改 `FontBackend` 接口、`FontFaceId`、variable font instance、完整 shaping 或全部缺字时的 `.notdef` 退化；
- synthetic bold、synthetic oblique 或任意字体轴公开配置。

## 实施计划

### Phase 1：公开来源用途与字体目录

1. 在 `src/font_backend.rs` 定义 `FontSourceKind`，并通过 `src/lib.rs` 公开重导出。
2. 为 `FontSource` 增加 `kind: Option<FontSourceKind>`，使 `new` 与 `with_alias` 默认保留 `None`。
3. 增加 `with_kind`，可与 `with_alias` 组合。
4. 为 `FontFaceRecord` 保存来源用途；展开 TTC/OTC 时复制同一来源用途到全部 collection member。
5. 不在字体登记阶段扫描 cmap、验证名称、验证 family 用途一致性或改变 capability descriptor roles。

阶段产出：Huozi 可持久保存调用方提供的来源用途，旧调用方的候选序列尚未改变。

### Phase 2：按用途构造候选 face 子集

1. 保持当前 family group 解析，先建立基础候选域。
2. 根据 request role 得到可选的目标 `FontSourceKind`。
3. 有目标用途时，先遍历每个 family group 中命中用途的 face 子集，再遍历其余 face 子集。
4. 对每个非空子集复用既有 style 档和 CSS weight 排序，构造同一个 `FontCandidate` 序列。
5. 无目标用途或无命中用途时复用当前顺序，不增加重排或额外分配。

阶段产出：`candidates()` 对 CJK 与 Latin role 产生稳定的优先序列；显式 family 候选域与现有行为一致。

### Phase 3：回归测试与示例

1. 在 `src/font_backend.rs` 单元测试中直接构造各 `FontRole` request，验证最终 face、尝试顺序与缺字退化。
2. 在 parser/布局入口测试中验证 `[font="family-a, family-b"]` 的列表顺序进入 role-aware 候选选择。
3. 更新 `examples/render` 的受控字体注册，声明其 CJK 或 Latin 用途。
4. 在示例中人工检查中文正文、CJK 标点、Latin 正文在混排文本中选择预期字体；同时检查显式 `[font]` 列表不使用列表外 family。字体行的 `On/Off` 后提供用途按钮，显示 `*`、`C` 或 `L`，每次点击按 `* → C → L → *` 循环，且切换后重建字体来源。
5. 复查 glyph bounds、SDF 和 atlas identity 没有因候选重排而出现不同 face 的 measure/draw 不一致。

阶段产出：最终代码、测试、示例和文档一致描述并证明用途优先级行为。

### Phase 4：持续维护文档与最终验证

1. 更新 `docs/architecture.md` 的 `FontSource`、候选排序和用户接入约定。
2. 更新 `docs/parser.md` 中 `[font]` 列表的实际选择语义，说明 CJK/Latin role 会在显式列表内部按声明用途调整优先顺序。
3. 更新本迭代文档的状态、实际 API 名称和验证结果；若实现中改变本文已确认的语义，先讨论并更新本文。
4. 运行相关自动验证、示例编译、WOFF 配置和文档检查。

## 最小必要测试

| 验证目标 | 最小证据 |
| --- | --- |
| CJK 正文优先 CJK | Latin 来源先注册且也覆盖测试字符；CJK 来源声明 `Cjk` 后，`CjkText` 最终选择 CJK face。 |
| CJK 标点优先 CJK | Latin 与 CJK 字体均覆盖引号或逗号；直接传入 `CjkPunctuation` request 后，最终选择 CJK face。 |
| Latin 正文优先 Latin | CJK 来源先注册且覆盖 ASCII；Latin 来源声明 `Latin` 后，`LatinText` 最终选择 Latin face。 |
| 显式列表内部重排 | `[font="latin, cjk"]` 的 CJK request 选择 cjk；`[font="cjk, latin"]` 的 Latin request 选择 latin。 |
| 显式列表不泄漏 | `[font="latin"]` 的 CJK request 不尝试列表外 CJK family。 |
| 未声明用途回归 | 所有来源 `kind = None` 时，每种 role 的最终 face 和候选顺序保持当前注册顺序。 |
| 无匹配目标用途回归 | 有 role 但基础候选域没有对应 kind 时，候选顺序保持当前规则。 |
| `Symbol`、`Emoji`、`Unknown` 回归 | 即使来源声明 CJK 或 Latin，上述 role 仍按当前基础顺序。 |
| 优先来源缺字 | 目标用途 face 缺字、剩余来源完整覆盖时，候选继续到剩余部分并选中完整 face。 |
| 不重复 shaping | 优先来源缺字后，`FontCandidateAttempt` 中同一物理 face 与 variation instance 只出现一次。 |
| family 内 style/weight | 同一用途来源的静态或 variable face 仍按现有 normal/italic/oblique 和 CSS weight 规则选择。 |
| collection 来源用途 | TTC/OTC 中多个 collection member 都继承同一 `FontSource.kind`。 |
| 回放一致性 | 选中的 `FontFaceId` 继续被 glyph bounds、metrics、outline 和 SDF atlas 使用。 |

测试必须断言最终 `FontFaceId`、候选尝试顺序或完整 shaping 结果，不得仅断言 `FontRole` 或 `FontSourceKind` 字段被传递。

Tiqian 的文本角色分类已经由 Tiqian 自身 fixture 与覆盖测试验证。Huozi 测试只验证它消费已给定 role 后的候选顺序，不复制 Tiqian 的引号、破折号、省略号或标点分类测试。

## 性能与内存验收

1. `FontSourceKind` 在 `Huozi::new` 时从 `FontSource` 写入字体目录；shaping 热路径不解析字体文件、name table、cmap 或额外 Unicode 数据。
2. 基础 family group 的构造和用途划分保持相对于当前候选 face 数量的 $O(n)$。
3. 只为现有 `FontCandidate` 结果分配必要的候选存储；不 clone 字体 bytes、family 名称或完整字体目录。
4. 优先部分与剩余部分各遍历一次，任何 face 在一次 request 中最多完整 shaping 一次。
5. 第一个无 glyph id `0` 的候选仍立即停止尝试。
6. `kind = None`、无目标用途或没有命中用途时不引入额外字体解析、cmap 扫描或不同的 fallback 结果。

## 风险与检查点

| 风险 | 处理方式 |
| --- | --- |
| 将 `FontRole` 与 `FontSourceKind` 混为同一个概念 | 保持前者由 Tiqian 描述文本，后者由调用方描述字体来源用途；名称和字段不复用 `role`。 |
| `[font]` 被 role priority 绕过 | 先固定基础候选域，再仅在域内分组；通过显式列表不泄漏测试防回归。 |
| 同 family 混合用途导致不明确 | 按 physical face 的来源用途分组；同一用途子集内保留既有 style/weight 排序。 |
| 优先字体缺字被重复 shaping | 剩余部分排除已在优先部分尝试的 face；通过 attempts 测试验证。 |
| 用 cmap 推断用途 | 只接受 `FontSource` 显式声明；覆盖仍由完整 shaping 结果判定。 |
| descriptor roles 语义被误改 | `FontSourceKind` 仅参与排序，本迭代不改变 backend capability report。 |

实施中如发现 Tiqian 的 `FontBackendRequest` 不能在 Huozi 所需的 request 粒度稳定提供 `CjkPunctuation`、`CjkText` 或 `LatinText`，或发现现有显式 `[font]` 调用方依赖“列表外全局 fallback”的行为，应停止实施，记录实际证据并讨论后再修改本文；后续方案必须保持单一候选路径。

## 验证结果

已完成以下自动验证：

```text
cargo test font_backend::tests
cargo test
cargo check --all-targets
cargo check --all-targets --features woff
cargo check --example render
cargo check --example render --features woff
cargo run --example render --release
cargo run --example render --release --features woff
git diff --check
```

字体后端相关单元测试通过 18 项；完整测试共通过 55 项。render 示例在默认和 `woff` 配置下均通过全目标编译和 release 启动。默认配置加载 3 个字体并完成首次布局；`woff` 配置加载 4 个字体并完成首次布局。中文文档措辞检查通过，未命中规则。

人工运行验证仍需要在具备 `resources/fonts` 的图形环境中执行：确认 CJK 正文、CJK 标点、Latin 正文和显式 `[font]` 列表得到预期 face，并检查 SDF 图集输出。

## 文档与验证

完成后至少运行：

```text
cargo test
cargo check --all-targets
cargo check --all-targets --features woff
cargo run --example render --release
cargo run --example render --release --features woff
git diff --check
```

对本迭代更新的中文文档运行项目采用的文档措辞检查，并人工通读新增与修改段落。人工检查 `examples/render` 时至少验证：

1. CJK 正文和 CJK 标点优先使用声明为 `Cjk` 的字体；
2. Latin 正文优先使用声明为 `Latin` 的字体；
3. 显式 `[font]` 列表内的 CJK/Latin family 会按 role 重排；
4. 单 family `[font]` 不会落到列表外字体；
5. 每个字体行的用途按钮按 `* → C → L → *` 循环，切换后立即按新用途完成布局。

## 相关文件

| 文件 | 本迭代职责 |
| --- | --- |
| `src/font_backend.rs` | `FontSourceKind`、`FontSource` 用途、字体目录元数据和 role-aware 候选排序的主要实现与单元测试。 |
| `src/lib.rs` | 公开重导出 `FontSourceKind`。 |
| `src/huozi.rs` | 确认 `Huozi::new(Vec<FontSource>)` 无需改变调用形态。 |
| `src/layout/tiqian_input.rs` | 保持将完整 `TextStyle` 写入 Tiqian，不在此处重复角色选择。 |
| `src/parser/elements_to_document.rs` | `[font]` 逗号列表 lowering；实现后补充相关入口测试。 |
| `examples/render/fonts.rs` | 受控示例字体及其用途声明来源。 |
| `examples/render/main.rs` | 将示例注册字体携带其 `FontSourceKind`，作为人工验证入口。 |
| `docs/architecture.md` | 持续维护的公开架构、字体候选和接入语义。 |
| `docs/parser.md` | `[font]` family 列表的实际选择语义。 |
| `docs/iteration/2026-09-20-feat-font-style-selection.md` | 已有 family/style/weight/variation 候选规则；本迭代在其基础上增加 role-aware 优先分组。 |
