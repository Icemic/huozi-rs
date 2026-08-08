# Tiqian-rs 移植测试与工具方案（草案）

- 状态：Draft
- 日期：2026-07-26
- 适用范围：完整 `tiqian-rs` port 的 Kotlin 对照测试与后续 Huozi 集成证据
- 依据：[移植测试与证据链纪要](../notes/2026-07-26-port-testing-evidence.md) · [移植边界与数据契约纪要](../notes/2026-07-26-port-boundary-and-data-contract.md) · [范围与 Shaping 边界纪要](../notes/2026-07-26-scope-and-shaping-boundaries.md)
- 现状参考：[Tiqian 当前内部结构](../references/tiqian-current-structure.md) · [Huozi 当前内部结构](../references/huozi-current-structure.md)

## 1. 目标

本方案定义两组可以共同维护一条证据链的工具：

1. Kotlin/Compose Desktop fixture generator：从语言中立的 case JSON 运行 Kotlin Tiqian 参考实现，记录 backend evidence，完成 replay 自校验，并生成参考截图。
2. Rust fixture support 与 runner：读取同一组 fixture，以 replay backend 执行 `tiqian-rs`，比较规范化结果，并为本地调试和 CI 输出差异报告。

工具服务于完整 port 的内部实施和最终验收，不把 port 拆成若干对外交付版本，也不提前接入 Huozi 的 production 字体管理、SDF 或 renderer。

布局一致性以规范化 JSON 为主证据。Kotlin 截图只用于人工视觉参照，不把不同 rasterizer、hinting、gamma 或 shader 的差异误判为布局错误。

## 2. 已确认约束

- fixture corpus 归属 `tiqian-rs`，路径为 `fixtures/port`。
- 除 `reference.png` 外，case 输入、证据、结果、清单与报告都使用 JSON。
- Kotlin 工具是独立 JVM CLI 模块 `:tools:port-fixture-generator`，权威生成路径为 Compose Desktop + Skia。
- generator 只接受 case JSON，不提供 `--text` 一类旁路输入。
- Rust 侧使用 workspace 内非发布的测试支持 crate；薄 example、批量 integration test 和 CLI runner 复用同一实现。
- 每份 case JSON 独立版本化：拒绝未知主版本，允许未知字段，缺少必需字段时失败。
- backend replay 按规范化请求的语义 key 匹配，不以调用顺序为契约。
- source range 使用 UTF-8 byte offset；grapheme、glyph 等其他索引空间不得混用。
- 所有 fixture 使用固定的 `SourceHanSansSC-Regular.otf`，不生成依赖本机系统字体的 case。
- Kotlin 与 Rust 仓库各保存一份字体；Huozi 沿用 `examples/assets/SourceHanSansSC-Regular.otf`。三份文件必须具有相同 SHA-256 与 face index。
- generator 提供 `generate` 和 `verify`；默认拒绝覆盖，显式覆盖也要先在临时目录完成自校验。
- fixture JSON 使用统一的确定性 pretty 格式。
- 结构字段严格比较，浮点字段按字段类别配置容差。
- 缺失 evidence 和未消费 evidence 都使 replay 失败；少数例外必须由 manifest 显式声明。
- 不建设 platform-local 数据集，也不把系统字体结果纳入 CI。

### 2.1 对早期数据集划分的修订

本草案以讨论后确认的固定字体方案修订
[移植测试与证据链纪要](../notes/2026-07-26-port-testing-evidence.md) 第 4 节的“三类数据集”划分：

- 权威跨语言成功 corpus 统一使用 Compose Desktop + Skia、固定字体和 recorded backend evidence；
- 不建设 Android、Web、本机系统字体或其他 platform-local fixture；这些平台只在完整 port 之后的独立集成阶段验收；
- `tiqian-rs` 仍需提供 deterministic test backend，但它用于 Rust 单元测试和 backend contract tests，不形成另一套跨语言成功 corpus；
- 单字体无法自然触发的多 face fallback、variable axes、missing bounds 等受控路径，由 backend contract tests 验证，不能借用系统字体补齐。

后续定稿时应在原纪要增加指向本草案或定稿文档的修订说明，避免两份文档长期给出不同的现行数据集定义。

## 3. 仓库落点

### 3.1 Kotlin Tiqian

```text
tiqian/
  settings.gradle.kts
  tools/
    port-fixture-generator/
      build.gradle.kts
      src/main/kotlin/...
      src/main/resources/
        fonts/
          SourceHanSansSC-Regular.otf
          LICENSE.txt
```

`settings.gradle.kts` 增加 `:tools:port-fixture-generator`。该模块可依赖 `core`、`font`、`linebreak`、`clreq`、`layout`、`shaping:api`、`shaping:skia`、`frontend:compose` 和必要的测试支持代码，但现有核心模块不得反向依赖工具。

字体许可证文件必须与实际字体版本匹配。复制资产前先确认 Huozi 现有文件允许再分发；许可证或来源无法确认时，字体复制与权威 fixture 生成应视为阻塞，而不是省略许可证继续提交。

### 3.2 Rust Tiqian

```text
tiqian-rs/
  Cargo.toml
  src/
  crates/
    fixture-support/
      Cargo.toml
      src/
  tools/
    fixture-runner/
      Cargo.toml
      src/main.rs
  examples/
    verify_port_fixture.rs
  tests/
    port_fixtures.rs
  fixtures/
    fonts/
      SourceHanSansSC-Regular.otf
      LICENSE.txt
    port/
      corpus.json
      schemas/
      cases/
```

`fixture-support` 是 workspace 内的非发布 crate，集中实现 schema 类型、loader、规范化、replay backend、比较器和报告。它不能成为 `tiqian-rs` 公共 API 的依赖，也不能把 JSON、PNG 或 CLI 依赖带入 production library。

根 `Cargo.toml` 保留现有 `[package]`，同时增加 `[workspace]`，members 至少包括 `.`、`crates/fixture-support` 和 `tools/fixture-runner`。两个工具 crate 都设置 `publish = false`：

- 根 package 通过路径 `[dev-dependencies]` 使用 `fixture-support`，供 example 和 integration test 复用；
- runner 正常依赖根 package 与 `fixture-support`；
- 默认 fixture root 从根 package 的 `CARGO_MANIFEST_DIR/fixtures/port` 推导，不依赖进程当前目录；
- runner 允许通过显式参数覆盖 fixture root。

入口职责：

| 入口 | 用途 |
| --- | --- |
| `tests/port_fixtures.rs` | 按 `corpus.json` 批量执行全部启用 case，作为常规 CI 门禁 |
| `tools/fixture-runner` | 验证单例或 corpus，提供筛选和 JSON report，便于本地诊断与 CI 归档 |
| `examples/verify_port_fixture.rs` | 展示如何加载并验证单个 fixture；保持薄封装，不复制比较逻辑 |

### 3.3 Huozi

本方案不修改 Huozi 的运行时代码。Huozi 后续集成测试复用 `tiqian-rs/fixtures/port` 中的 source、normalized input、backend evidence 和 result golden，并继续使用：

```text
huozi-rs/examples/assets/SourceHanSansSC-Regular.otf
```

Huozi 的 SDF/WGPU screenshot golden 单独维护，不覆盖 Kotlin 的 `reference.png`。

## 4. Corpus 结构

```text
fixtures/port/
  corpus.json
  schemas/
    corpus.schema.json
    manifest.schema.json
    source-input.schema.json
    layout-input.schema.json
    backend-evidence.schema.json
    layout-result.schema.json
  cases/
    <case-id>/
      manifest.json
      source-input.json
      layout-input.json
      backend-evidence.json
      layout-result.json
      reference.png
```

每个 case 保存五份 JSON 和一张 PNG。`corpus.json` 是 corpus 级控制文件，不计入单个 case 的五类证据。

### 4.1 `corpus.json`

它是 runner 发现 case 的唯一入口，至少包含：

- `schemaVersion`；
- 按稳定顺序排列的 case ID；
- case 类别与能力标签；
- 是否启用；
- 可选的禁用原因；
- 对 engine/backend 能力的要求。

验证时必须双向检查：清单引用的 case 目录都存在，`cases/` 下也不存在未登记目录。临时目录必须放在 `cases/` 之外，避免被误识别为 fixture。

### 4.2 `manifest.json`

manifest 描述来源和复现条件，至少包含：

- `schemaVersion`、case ID、标题、说明与能力标签；
- Tiqian reference commit 与 exporter version；
- OS、JDK、Kotlin、Skia 和 shaper 版本；
- 字体相对路径、SHA-256、face index 与稳定 fixture font key；
- locale、profile、constraints、density 与 viewport 摘要；
- generation timestamp；
- 除 manifest 自身外，另外四份 case JSON 与 `reference.png` 的相对路径、byte length 和 SHA-256；
- 允许未消费 evidence 的显式白名单，默认为空；
- 已知 capability issue 或显式 unsupported 状态。

process-local `FontId`、平台字体对象或其他不透明句柄不得进入 manifest。fixture 使用稳定 font key，例如 `source-han-sans-sc-regular`。

manifest 不记录自身 hash，避免自引用。artifact hash 对最终落盘 bytes 计算；路径相对 case 目录、统一使用 `/`，禁止绝对路径与 `..`。如果需要校验 manifest 自身完整性，由 `corpus.json` 的 case entry 保存 manifest SHA-256；`corpus.json` 不记录自身 hash。

重复信息遵循以下事实来源：目录名是物理 case ID，corpus 和 manifest 中的 ID 必须与它完全相等；locale、profile、constraints、density 和 viewport 以 `source-input.json` 为准，manifest 只能保存 generator 重新计算并校验的摘要；capability tags 以 `corpus.json` 为准，manifest 若保存副本则必须完全一致；artifact 路径必须符合固定 case 布局，不能指向目录外。

### 4.3 `source-input.json`

该文件既是 generator 的唯一输入 schema，也是生成后保存的 A 前原始输入证据。generator 读取作者提供的文件，校验并规范化，再原样承担 fixture 中 `source-input.json` 的角色，不维护第二套 `case.json`。

它至少表达：

- UTF-8 source text；
- 富文本和样式范围；
- paragraph style、writing mode 与 constraints；
- profile、locale 与字体引用；
- decorations、ruby、inline boxes 和 inline objects；
- Compose Desktop 参考截图的 viewport、density 和 background；
- source boundaries 与其他影响 lowering 的宿主信息。

所有 source range 都是 UTF-8 byte offset，端点必须位于 UTF-8 boundary。Kotlin lowering 负责显式转换成内部 UTF-16 offset，导出 normalized input 时再转换回 UTF-8 byte offset，并验证往返一致。

所有 range 均为半开区间 `[start, end)`。`SourceByteRange` 只索引原始 source 的 UTF-8 bytes；如需保存 display buffer 范围，必须使用独立的 `DisplayByteRange`。grapheme range 与 glyph range 索引对应数组项，不是 byte offset。Kotlin 输入先拒绝 unpaired surrogate，再执行 UTF-16/UTF-8 转换；空范围、上界、单调性和每个端点都要验证。manifest 记录 Unicode/grapheme segmentation 版本，normalized grapheme table 是 replay 的事实来源，Rust B 不得重新分段后覆盖它。

### 4.4 `layout-input.json`

这是 A 输出、B 输入的语言中立表示，不是 Kotlin 对象序列化。它应完整覆盖 port 所需语义：

- source text、UTF-8 ranges 与 grapheme table；
- style ranges、source boundaries 与 shaping 属性；
- paragraph style、writing mode、constraints 与 profile；
- decoration、ruby、inline box/object；
- source、grapheme 与其他索引空间的具名类型；
- 原版保留但尚未实现能力的显式状态。

Rust loader 要验证 range 单调性、边界合法性、索引空间和跨引用完整性。不得为了让当前 Huozi 更容易接入而删除 Kotlin Tiqian 已有字段或扩展点。

### 4.5 `backend-evidence.json`

文件保存 Kotlin B 运行时对 font resolution、fallback、shaping 和 raw metrics 的请求与响应。顶层至少包含：

- `schemaVersion`；
- backend 身份与版本；
- evidence entries；
- 必须发生的具名 interaction assertions；
- 记录期间产生的 capability issues。

每个 entry 包含：

- operation；
- 完整规范化 request；
- response；
- 根据 canonical request 生成的稳定 `requestId`；
- 可选诊断信息。

`requestId` 用于索引和报告，完整 request 才是冲突检查的依据。不同 request 不得因 hash 或人工命名碰撞而共享 response。

Canonical request 使用与 fixture 相同的 JSON 规范化规则，并为 enum、缺省字段、有限浮点和语义集合定义唯一表示。`requestId` 定义为 `SHA-256(operation + NUL + canonical-request-json-bytes)`，只用于索引和诊断；匹配与冲突检查始终比较完整 operation 和 request。

Recording 期间，同一 canonical request 可以发生多次，但规范化 response 必须完全相同；同一 request 出现不同 response 时生成立即失败。持久化 evidence 按 semantic key 去重，每个 key 只保存一个 response。

Replay 规则：

1. 请求按语义内容匹配，不要求 Kotlin 与 Rust 调用顺序一致。
2. 同一语义请求可以重复，cache 策略不是 golden。
3. 找不到 entry 时立即失败，并输出实际 canonical request。
4. 运行结束后，每个非白名单 entry 至少被消费一次；未消费 entry 失败。这里比较的是 semantic key 集合，不比较调用次数。
5. 必须发生的 rollback、hyphen reshaping 等行为由 interaction assertions 单独验证，不能从最终结果反推。

白名单只能引用当前文件中已存在的 requestId，并填写原因。Interaction assertion 可以表达请求 A 后出现请求 B 的局部因果关系，但不能把全局调用顺序变成 golden。

### 4.6 `layout-result.json`

该文件同时保存规范化 `LayoutResult` 和 `LayoutDebugInfo`，至少覆盖：

- source/display text 与 UTF-8 ranges；
- grapheme、shaping cluster、layout cluster 与 glyph range 映射；
- stable font fixture key、glyph ID、advance、offset 与 ink bounds；
- glyph runs、features、variation axes 与 shaping decisions；
- line range、baseline、line box、natural/adjusted/visual width；
- break、end reason、repair、hyphen 与 hanging；
- punctuation body/glue、compression、autospace 与 justification；
- decoration、ruby、Bopomofo、inline box/object 几何；
- structured decisions 与 capability issues。

该格式不保存 process-local `FontId`、对象地址、cache 命中或平台私有句柄。

### 4.7 `reference.png`

截图由 Compose Desktop + Skia 使用同一次自校验通过的布局结果绘制。画布参数来自 `source-input.json`，包括 width、height、density 和 background。

固定规则：

- 输出 PNG，颜色空间为 sRGB；
- 禁止按内容自动裁切或扩容；
- 布局或 decoration 超出画布时生成失败；
- PNG 编码参数由工具固定；
- manifest 记录图片尺寸和 SHA-256。

`tiqian-rs` runner 不比较此图片。它是 Kotlin 原版视觉参考，用于人工检查断行、baseline、标点、行距、glyph placement 与 annotation geometry。

## 5. JSON 契约

### 5.1 版本与 schema

每份 case JSON 顶层都有 `schemaVersion`。读取规则为：

- 不支持的主版本：失败；
- 支持的主版本和较新的次版本：允许读取；
- 未知字段：忽略；
- 缺少当前版本的必需字段：失败。

`schemaVersion` 使用 `{ "major": 1, "minor": 0 }` 形式。minor 版本只能增加可选字段、可选 metadata 或不改变既有语义的能力；新增必需字段、改变字段含义或索引空间必须提升 major。reader 按已知主版本的最高 schema 校验已知字段。manifest 声明五份 case JSON 的版本组合，`corpus.json` 声明允许的 major 组合；verify 负责检查组合兼容性。

仓库提交五份 case 文档 schema，再为 `corpus.json` 提交一份 corpus schema。schema 用于编辑器提示、生成前预检和跨语言契约审阅；Kotlin/Rust 的领域校验仍负责 JSON Schema 难以表达的 UTF-8 boundary、range 映射和跨引用约束。

schema 应与“允许未知字段”的兼容策略一致，不能通过 `additionalProperties: false` 把次版本扩展变成硬错误。

### 5.2 确定性文本格式

所有持久化 JSON 使用同一 formatter：

- UTF-8，无 BOM；
- LF 换行；
- 两个空格缩进；
- 文件末尾保留一个换行；
- object key 使用稳定顺序；
- array 保留领域顺序；语义为集合的 array 在写入前按各自稳定 key 排序；
- number 使用有限 JSON number，拒绝 `NaN` 和正负 `Infinity`。

生成器、Rust report writer 和任何 migration 工具都复用这套格式。比较器解析 JSON 后按领域字段比较，不依赖文本字段顺序判定布局相等。

## 6. Kotlin generator

### 6.1 `generate`

输入是一份符合 `source-input.schema.json` 的 JSON、目标 case ID 和显式 fixture root。CLI 契约至少包括：

```text
generate --source <path> --case-id <id> --fixtures-root <path> [--overwrite]
verify --fixtures-root <path> [--case <id>]
```

工具不得从当前目录猜测 sibling repository。case ID 只允许匹配 `[a-z0-9][a-z0-9._-]*`，并拒绝路径分隔符、`..` 和 Windows 保留名称。目录名、manifest ID 与 corpus ID 必须一致。

`generate` 执行顺序：

```text
source-input.json
  -> schema 与领域校验
  -> Compose/Tiqian lowering
  -> normalized LayoutInput
  -> Kotlin B + recording Skia backend
  -> normalized LayoutResult + LayoutDebugInfo
  -> backend evidence
  -> Kotlin B + replay backend
  -> 与首次结果精确比较
  -> Compose Desktop reference.png
  -> manifest 与 hashes
  -> 临时 case 目录完整验证
  -> 原子安装到 fixtures/port/cases/<case-id>
```

生成器不得先写目标目录再逐项补文件。默认目标已存在时失败；只有显式 `--overwrite` 才允许替换。即使指定覆盖，也必须等临时目录的 schema、hash、evidence replay、result compare 和 screenshot 检查全部通过后再替换旧目录。失败时保留旧 fixture。

安装在支持时使用同一文件系统内的原子目录替换。Windows 上无法直接替换非空目录时，工具执行可恢复的事务式安装，而不宣称整个过程原子：temp、backup、transaction marker 和 corpus 级排他锁都位于 `fixtures/port/.transactions/`；流程为 `old -> backup`、`temp -> target`、成功后删除 backup。generator 启动时按 marker 恢复可判定事务；verify/runner 发现活动事务时失败，不读取半完成状态；无法判定归属的 backup 不得自动删除。

### 6.2 `verify`

`verify` 不修改 fixture，负责：

- 校验 corpus、case schema 和领域约束；
- 校验文件 hash、字体 hash 和 face index；
- 用 recorded backend 在 Kotlin B 重放；
- 比较 replay result 与 `layout-result.json`；
- 检查 evidence 完整消费和 interaction assertions；
- 检查 PNG 元数据与 manifest；
- 检查 case 目录和 `corpus.json` 双向一致。

`verify` 可验证单个 case 或整个 corpus。它不隐式更新 hash、timestamp、JSON 或 PNG。

### 6.3 固定字体

权威生成只加载 Tiqian 工具资源中的固定字体文件，不通过系统 family name 查找字体。运行前同时校验：

- 本地文件 SHA-256 与 manifest 预期一致；
- face index 一致；
- Skia 实际使用的 resolved face 可追溯到该文件；
- 没有发生未记录的平台 fallback。

缺字或 backend capability 不足必须进入结构化 evidence/capability issue；不得静默切换系统字体。

为满足这项约束，generator 必须实现专用 file-backed Skia font session：

- 从指定 OTF 和 face index 创建唯一受控 `Typeface`；
- shaping、raw metrics 和截图绘制共享同一个 session/resolver；
- session 禁止调用 `FontMgr.default`、`SkiaSystemTypefaces` 或 family-name fallback；
- 所有逻辑 font key 显式映射到已登记 face；未登记 family、weight、italic 或 variation 请求产生结构化 capability issue；
- 截图优先直接按 `LayoutResult` 的 glyph ID 与 position 构造绘制对象；若平台限制要求重新 shaping，必须验证 glyph ID、advance 与 offset 和布局阶段一致。

generator 的确定性测试应在可控环境中改变系统字体可见性，证明输出只取决于固定 OTF。单个 Regular face 只验证它实际支持的能力，不把 synthetic bold/italic、第二字体 fallback 或 variable font 行为冒充为固定字体证据。

## 7. Rust fixture support 与 runner

### 7.1 Loader 与领域校验

`fixture-support` 负责：

- 读取 `corpus.json` 和 case 文件；
- 按主版本选择 schema/types；
- 校验 UTF-8 ranges、grapheme ranges、glyph ranges 与交叉引用；
- 将 stable fixture font key 映射为 replay backend identity；
- 校验 font/file hashes；
- 将 normalized input 转换成 `tiqian-rs` 的实际 Rust 类型；
- 将实际结果规范化为共同 result schema。

跨语言 fixture 类型属于测试契约，不要求与 `tiqian-rs` 的公共 API 同构。转换层必须显式，避免测试格式反向绑死 production 模型。

### 7.2 Replay backend

Replay backend 实现 `tiqian-rs` 定义的 font/fallback/shaping/raw metrics trait。它只返回 evidence 中记录的响应，不读取字体文件、不调用 production shaper，也不自行补默认 metrics。

找不到请求、request 冲突、响应类型不符、interaction assertion 未满足或 evidence 未消费完毕，都返回结构化测试失败。

### 7.3 Comparator

比较器按领域字段比较，不比较序列化文本：

严格相等：

- source/display text；
- UTF-8 ranges 与各索引空间映射；
- enum、ID、glyph count 与 stable font fixture key；
- line break、end reason、repair、policy 和 capability issue；
- structured decision 的类型、输入与选择结果；
- annotation 所属 line 和结构关系。

按类别使用绝对与相对容差：

- shaping advance；
- glyph offset 与 ink bounds；
- baseline 与 line box；
- cluster/layout advance；
- compression、justification 与 spacing；
- decoration、ruby 和 inline geometry。

判定公式为：

$$
|a-b| \leq \max\left(\varepsilon_{abs}, \varepsilon_{rel}\cdot\max(|a|,|b|)\right)
$$

初始默认是严格比较，即非零容差不能凭经验随手加入。实现期间选择一组覆盖不同数值尺度的 seed fixtures，分别执行 Kotlin 重复生成和 Rust 对照；只有证据表明序列化或跨语言计算存在稳定、合理的数值误差时，才为对应字段类别加入最小阈值。阈值保存在版本化 comparator 配置中，修改时必须附带差异样本和原因。

Kotlin recording → replay 自校验不使用这套跨语言容差：同一 runtime 和 normalizer 产生的结构与所有有限浮点必须精确相等，推荐直接比较 canonical 数据模型或 canonical bytes。字段分类容差只适用于 Kotlin golden 与 Rust actual 的跨语言比较。

每个 diff 至少输出：

- case ID；
- JSON Pointer 风格字段路径；
- expected 与 actual；
- 绝对误差、相对误差和适用阈值；
- 所属 decision/backend operation（可以定位时）。

### 7.4 报告与退出状态

runner 默认输出适合终端阅读的摘要：case 总数、通过数、失败数、首个失败路径和报告位置。可选输出完整 JSON report，供 CI artifact 归档。

- 全部启用 case 通过：退出码 0；
- schema、hash、replay、compare 或 corpus 完整性失败：非 0；
- 被显式禁用的 case 不计为通过，报告中单独列出原因。

example 默认只验证调用者指定的单个 case，不建立另一套默认 tolerance 或宽松模式。

## 8. 生成与验收流程

### 8.1 新增 case

1. 作者编写并校验 `source-input.json`。
2. Kotlin `generate` 在临时目录完成 recording、replay、自比较和截图。
3. 作者检查 structured decisions、backend evidence 与 `reference.png`。
4. 将 case 加入 `corpus.json`，运行 Kotlin `verify`。
5. 运行 Rust 单例 runner 和完整 integration test，再提交 JSON/PNG/font hash 相关 diff。

### 8.2 更新 case

允许更新的原因必须属于以下类别之一：

- 明确切换 Tiqian reference commit；
- fixture schema migration；
- 字体或 shaper 的受控升级；
- 修复 generator/normalizer 本身的错误；
- 经确认的 Kotlin 参考行为变化。

更新使用 `generate --overwrite`，不能手工修改大块 `layout-result.json` 或直接替换 PNG。评审时先看 structured decision 与 backend evidence diff，再看几何与截图。字体、shaper 或 schema 变化不得与 port bug 修复混在一次无说明的批量重生成中。

### 8.3 Port 验收

```text
layout-input.json
  + backend-evidence.json
  -> tiqian-rs + replay backend
  -> actual normalized result
  -> compare layout-result.json
```

完整 port 的验收要求 corpus 覆盖当前 Tiqian 被移植的类型、策略、算法、诊断、扩展点和显式 unsupported 状态。内部可以按领域切片逐步增加通过数，但只有整个能力矩阵满足并且全部启用 case 通过，port 才算完成。

### 8.4 Huozi 后续集成

```text
source-input.json
  -> Huozi lowering / font / shaping
  -> compare layout-input.json
  -> tiqian-rs
  -> compare layout-result.json
  -> Huozi SDF / atlas / vertices / renderer
  -> Huozi screenshot golden
```

Huozi 集成不得只比较最终截图。A 边界和 B 结果都要保留独立 diff，才能判断失败来自 lowering、font/shaping、layout 还是 renderer。

## 9. 覆盖组织

`corpus.json` 以 capability tags 管理覆盖，不另建系统字体数据集。所有 case 都使用固定思源黑体和 recorded Skia evidence；偏算法的 case 也使用同一证据格式，避免再维护一套不可互换的输入输出协议。

固定字体权威 corpus 与 Rust backend contract tests 共同承担完整验收：前者验证可由受控 Skia session 确定性记录的真实路径；后者使用 deterministic/mock backend 验证多 face fallback、missing bounds、request 冲突、非法 evidence、weight/italic/variation 转发等单个静态 Regular face 无法自然覆盖的路径。能力矩阵必须为每项能力标注验证位置，不能宣称单字体 corpus 验证了 production 多字体行为。

覆盖至少分为以下组：

1. 文本与 shaping：CJK、Latin、Cyrillic、中西混排、combining mark、ligature、variation selector、fallback、必要 run 边界。
2. 断行与标点：mandatory break、CRLF、U+200B、标点分类、PushIn、Hang、Carry、极窄 measure、西文断词。
3. 行调整与段落：compression、autospace、word space、CJK justification、缩进、line-length grid、max lines。
4. Annotation 与 inline：decoration、ruby、Bopomofo、inline box/object、富文本样式和 source boundaries。
5. 扩展点与失败：writing mode 等当前未实现能力、missing glyph/bounds、capability issues、非法 range 和 evidence 缺失。

每项能力应有正常、边界和具名降级/失败案例。错误输入的 schema/loader 测试可以使用小型内联或专用 invalid fixtures，不进入权威成功 corpus，也不生成 `reference.png`。

`corpus.json` 还要定义 `port-complete` completion profile，列出必需 case 或必需 capability。普通开发运行可以报告被禁用 case，但完整 port 验收时，任一必需 case 被禁用都直接失败；disabled case 不满足任何必需覆盖。当前尚未实现但模型要求保留的能力，应以启用且期望输出 structured unsupported/capability issue 的正向 case 验收，不能靠禁用隐藏。

## 10. 实施顺序

这些步骤是测试基础设施的内部实施顺序，不改变完整 port 一次交付的原则：

1. 定义六份 JSON Schema、canonical formatter、fixture 目录和固定字体校验。
2. 实现 Kotlin normalizer、recording backend、replay backend 和 `generate`/`verify`。
3. 建立少量 seed fixtures，验证 Kotlin record → replay 自闭环并校准 comparator。
4. 实现 Rust `fixture-support`、CLI runner、薄 example 和批量 integration test。
5. 按能力矩阵扩展 corpus，并在完整 port 过程中持续运行 Kotlin verify 与 Rust tests。

不得为了提前产生绿色测试而省略 debug info、放宽 unknown request、容忍未消费 evidence，或给所有浮点字段设置统一大 epsilon。

## 11. 验收条件

工具方案落地后，至少满足：

- 任意 case 可由 source JSON 在无系统字体依赖的环境中确定性生成；
- Kotlin generator 证明 recorded evidence 能独立 replay 出同一 normalized result；
- Rust runner 不读取真实字体也能以同一 evidence 执行 `tiqian-rs`；
- corpus 清单、schema、range、hash、evidence 消费和 interaction assertions 都有自动校验；
- 单例 example、批量 integration test 和 CLI 使用同一 loader/comparator；
- 失败报告能定位到 case、字段路径、backend request 或 structured decision；
- `reference.png` 可人工复核，但不会被误用为跨 renderer 像素 golden；
- 更新 fixture 必须经过显式覆盖、自校验和可审阅 diff；
- 完整能力矩阵通过后，才把 port 标记为完成并进入 Huozi production 集成。

## 12. 本草案不处理的内容

- Huozi production `FontManager`、`FontId` 生命周期和 cache 设计；
- Huozi production shaping backend 的库选择与实现；
- SDF、atlas、vertices、WGPU renderer 或 Huozi screenshot comparator；
- Android、Web 或本机系统字体 fixture；
- Kotlin 与 Rust 公共 API 同构；
- 竖排、RTL、婆罗米系等当前范围外能力的新增实现；
- port 完成后的字段精简、debug feature 裁剪或 API 重构。
