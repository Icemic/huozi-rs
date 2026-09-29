# 富文本布局输出与 SDF 基础图形呈现

> 状态：已完成（2026-09-29 归属修订后重做）
>
> 日期：2026-09-21
>
> 最后校准：2026-09-24
>
> 首次实施：2026-09-26（图形呈现部分的归属错误，见“归属修订”）
>
> 重做完成：2026-09-29
>
> 分类：feat

## 文档用途

本文定义 Huozi 将现有富文本 parser 与 Tiqian 布局结果转成完整公开布局输出，并为这些结果生成可直接提交的 SDF 绘制数据的迭代。官方 WGPU demo 作为首个消费方验证该路径可用。

本文是本迭代后续讨论、实施、测试和验收的依据。已经确认的目标、范围、数据语义、逐字显示规则、连续图形处理方式、SDF 基础图形方案、风险和待讨论问题均记录在本文中；后续工作不得依赖聊天记录或临时记忆补全规则。

文末 33 项待讨论问题已于 2026-09-24 逐项确认，结论直接写在对应条目上；本文由此成为可以据此编码的实施规格。除这些已确定条目外，不应在实施时自行扩大或缩小范围。

“讨论中”时期的约束同样适用：结论未回写本文前不得用临时实现替代；实施中如发现需要改变任一结论，应先讨论并回写本文。

本文成文后，仓库又完成了若干独立迭代，Tiqian 依赖也从 0.3 升到 0.5，因此部分现状描述和目标已经过期。“校准记录”一节只把过期描述改成当前事实，并把新版本 Tiqian 已经提供、或平台参考实现已经固定的做法写成候选答案；未确认的条目仍保留在文末待讨论问题中。

## 归属修订（2026-09-29）

### 错误决定

本文原稿把“非文字图形的模板生成、九宫格与重复图元构造、顶点输出”划归 `examples/render`，只要求 Huozi 公开后端无关的几何描述。“不包含”一节据此写明不为非文字变体公开模板 UV 或 GPU 顶点，SDF 方案总体原则写“模板位图由 demo 生成；图集分配使用 Huozi 的公开低层入口，GPU 顶点只属于 demo 呈现层”。

这个归属是错的。活字的定位是 CJK 文字排印与 SDF 字形渲染核心，图形模板生成与顶点输出是它的本职能力。把这一步推给调用方，等于要求每个用户重写一遍实现。

### 怎么错的

2026-09-19 的方案讨论中，最初的问题是“类似 demo 这样的场景怎么绘制背景和下划线”，这里的 demo 指的是用户侧可能的用法。助手据此推荐“在 demo 内新增解析式图形 shader”，该方案下图形几何由 shader 代码表达，确实与特定后端绑定。

随后用户提出改用“预先把图形放进 SDF 纹理，再按九宫格拉伸或重复”的方案。这一步把图形几何从 shader 代码换成 CPU 生成的模板位图与四边形，后端绑定随之消失，原推荐方案的前提不再成立。

但助手在回答“能行”的同一条回复末尾自行补了一节“数据层与 demo 的边界”，照抄了原方案下的结论，理由是“其他调用方仍可选择 Canvas、Skia、SVG”。该论证来自 tiqian：tiqian 是布局核心，几何消费方式确实因后端而异；huozi 是 SDF 渲染核心，这条论证不适用。

该边界随后写进本文原稿的目标、包含项、不包含项和总体原则，并在 2026-09-24 校准中作为既成前提保留：33 项待讨论问题逐条复查过，但边界躺在散文段落里，没有被重新审视。直接后果是第 20 项决策：为了让 demo 能上传模板，要求 `Huozi` 新增“调用方提供位图”的公开入口。为了让调用方能画而修改库的 API，方向已经反了。

### 修订后的归属

| 能力 | 归属 |
| --- | --- |
| 形状模板生成（SDF 位图）、图集分配与缓存 | Huozi 内部 |
| 可见前缀合并、九宫格与重复图元构造、顶点与索引生成 | Huozi |
| 绘制层顺序 | Huozi |
| 纹理上传、渲染管线、draw call、窗口、调试面板 | 调用方 |
| 布局几何（断行、标点、注音位置、范围切分） | tiqian（不变） |

`UnitVertices` 的非文字变体继续公开几何与 `row`、`col`；图形顶点由 Huozi 在布局阶段生成，调用方不需要写图形代码。

### 已完成与本次修订

| 范围 | 状态 |
| --- | --- |
| 公开布局输出、多段与空段、`row`/`col`、逐字显示顺序、注音、来源 span、交互区域、高度限制 | 已完成，本次修订不改动语义 |
| 形状模板集合与九宫格、三宫格、重复单元、端帽的几何规则 | 规则未变，实现位置已从 `examples/render` 移入 `src/shape/` |
| 图形顶点输出 | 已改为每个图形片段自带三层顶点 |
| `Huozi::allocate_resident_template`、公开的 `AtlasTemplate`、公开的 `sdf` 模块 | 已收回 |

### 实施方式

模板与图元构造移入核心后不再是对外 API，实施按以下方式组织：

1. 不引入 `ShapeTemplates`、`ShapeShader`、`ShapeBatch` 这类包装类型。模板集合、阈值换算与顶点累积都是 `Huozi` 的内部状态与函数。
2. 模板按需生成并缓存，不在建实例时烤出全部半径。按整数半径 0..=32 各一份会占约 21% 的图集，而一份文档通常只用其中几个。
3. 网格预留、块内居中与写入抽成一个内部函数，字形路径与模板路径共用，不再各写一份。
4. 三级 paint（阴影、描边、填充）的构造合并为一个函数，不在四处重复，也不为单个元素分配临时 `Vec`。
5. 绘制数据只对外给出顶点与索引两个数组，顺序已经可直接提交。分层是库内的排序手段，调用方不需要理解，也不需要自己按层拆 draw call。
6. `TextVertices.indices` 固定为 `[0, 1, 2, 0, 2, 3]`，不再逐元素保存。

以上都是移入核心时顺带完成的简化，不改变任何绘制结果。

### 本次不做

以下三件事与归属纠正无关，记录在此，需要时另开讨论：

1. **跨行背景的行内侧圆角。** 活字的书写顺序是带折行的一维线性关系：一个跨行背景的第一行左边是起点（圆角）、右边是折行处（直边），中间行两侧都直，末行左边直、右边圆。tiqian 已按 `corner_radii` 的左列 / 右列给出这两个值，但背景绘制用 `max()` 把两列压成一个值，因此 `radius > 0` 的跨行背景每行都画成完整四角矩形，行间出现凹口。修它需要按左右不同的半径生成模板。当前 `radius` 默认为 0，不触发该问题。
2. **文字顶点是否也由绘制数据入口生成。** 目前 `TextVertices` 在布局阶段解图集并保存 `[Vertex; 4]`。改为绘制侧生成后，公开结果不再含图集 UV 与 page，并消除图集 LRU 淘汰后既有顶点 UV 失效的隐患；代价是绘制侧每次重建都要重算文字顶点。
3. **`UnitVertices` 是否改名。** 若文字顶点移走，该类型的任何变体都不再携带顶点，名字会失真。

## 校准记录（2026-09-24）

本文成文后完成的独立迭代如下。它们不在本文范围内，但改变了本文的事实前提，或已经完成本文的一部分目标：

| 迭代 | 已完成内容 | 对本文的影响 |
| --- | --- | --- |
| `2026-09-21-feat-interaction-hit-testing.md`（已完成） | 公开 `RichTextLayoutOutput`、`Interaction`、`InteractionArea`；`[link id=...]` 与 `[object id=...]`；`interactions` 输出；`col` 改为按 Tiqian positioned cluster 递增；demo 的交互点击示例。 | 本文目标 1、目标 3 的“独立公开”部分、目标 4 的 `col` 部分、目标 7 的命中区域部分已经完成；本文旧的 `links: Vec<LinkRegion>` 设计由该文档改写为 `Interaction`。 |
| `2026-09-21-feat-paragraph-alignment.md`（已完成） | `LayoutStyle.align` 与 `[br align=... /]`。 | 本文多段组合必须逐段保留 `ParagraphStyleOverride`，包括 `align`。 |
| `2026-09-20-feat-font-style-selection.md`、`2026-09-21-feat-role-aware-font-fallback.md`、`2026-09-21-feat-synthetic-font-styles.md`（均已完成） | family、weight、italic 选择，`FontSourceKind` 角色优先，仿粗与仿斜实例。 | 本文“不包含”中的字体选择与合成字重、斜体已由这些迭代交付；`Text` 变体必须沿用 `FontFaceId` 已带的 variation 与 synthesis，不引入第二套字体身份。 |
| `2026-09-21-perf-layout-hotspots.md`（已完成） | shaping 数据复用与 1024 项未缩放 glyph ink bounds LRU。 | 输出适配不得为每个元素重复 shaping 或绕过已缓存的 bounds；新增结果保持线性扫描。 |
| `2026-09-23-bugfix-sdf-edge-smoothing.md` | `FILL_THRESHOLD_BIAS`、`EDGE_SMOOTHING_HALF_WIDTH`，以及 `fill_buffer` 与 `gamma` 的内缘覆盖率语义。 | 非文字 SDF 模板必须沿用同一 `Vertex` 字段和同一阈值、内缘语义，不能另起一套。 |

依赖版本：本文写作时对照的是 `tiqian` 0.3，当前 `Cargo.toml` 使用 `tiqian = "0.5"`。0.4 增加字体合成策略与最终 face 身份，0.5 增加链接与行内对象的 `id`。工作区中的 `tiqian-rs` 就是该 crate 的源码（`Cargo.toml` 版本 0.5.0）。

## 背景与问题

富文本标签语法和 Tiqian 输入适配已经能够表达：

- 文字字号、颜色、描边、阴影、字体、字重、斜体、locale 和基线偏移；
- 背景与行内代码背景；
- 下划线和删除线；
- Ruby 和 bopomofo；
- 着重号、示亡号、专名号和书名号；
- 链接；
- 行内对象；
- `[br /]` 产生的多段文档。

Tiqian 已经负责 shaping、字体选择、字体度量、标点几何、断行、行调整、注音避让、跨行范围切分和行内对象布局。Huozi 不应重新实现这些布局规则。

当前 Huozi 的最终输出已经是 `RichTextLayoutOutput`：

```text
RichTextLayoutOutput { glyphs, segment_glyph_spans, interactions, width, height }
```

`UnitVertices` 仍是只表示普通文字 SDF 四边形的 struct。`HuoziTiqianOutputAdapter` 只输出正文和行尾自动连字符 glyph；链接和行内对象的命中区域已经进入 `interactions`，但 Ruby、bopomofo、背景、线条、CLREQ 装饰以及对象本身的绘制结果都没有进入公开结果。`layout_parse` 和 `layout_parse_with` 只布局 `ParsedText` 的第一段。

当前 `examples/render` 只消费普通文字的 shadow、stroke 和 fill 四边形。它使用一条 SDF pipeline、一个文字图集、一个 `u16` index buffer，并按文字阴影、描边、填充三遍收集顶点后一次提交；它另外保存 `interactions`，并在点击命中后由 egui 显示交互提示。非文字富文本没有可见结果。

先前“只返回完整状态下的最终图元，实际绘制以后再处理”的方案不能满足游戏逐字显示：

1. 若每个 cluster 的背景分别绘制，会产生内部圆角、重复描边、重复阴影和半透明重叠。
2. 若每个 cluster 的线条分别绘制，会产生重复端帽、重复阴影和断裂。
3. 虚线、点线和波浪线若按每个可见前缀重新拟合，已经显示的图案会随新文字出现而移动。
4. 跨行示亡号既有 Tiqian 根据完整源范围决定的开口，又有逐字显示当前可见前缀形成的临时末端；两者不能混为一谈。
5. SDF 模板解决的是图形栅格化和复用，不会自动告诉绘制方哪些增量片段属于同一个连续视觉范围。

因此，本迭代必须同时完成后端无关的富文本布局输出、连续范围信息和这些结果的可提交 SDF 绘制数据，形成可验证的完整可用路径。绘制数据由 Huozi 生成；官方 demo 只负责上传纹理与提交，见“归属修订”。

## 目标

1. 为所有公开布局入口提供统一的 `RichTextLayoutOutput`。（已完成）
2. 将现有 `UnitVertices` 扩展为 enum，使一条平铺序列能够表示文字、背景、线条、CLREQ 装饰和行内对象。（已完成）
3. 继续独立公开生产环境正在使用的 `SegmentGlyphSpan`，并让它覆盖一个输入 segment 产生的全部绘制元素。（已完成）
4. 以 Tiqian positioned cluster 为单位定义 `row`、`col`，直接支持游戏中的逐字显示。（已完成）
5. 为需要合并绘制的背景、线条和装饰按 authored range 归组，使同一个范围的片段能拼成完整图形。（已完成）
6. 布局全部 `[br /]` 产生的段落，并正确处理连续分隔形成的空段。（已完成）
7. 输出链接和行内对象所需的文档坐标矩形，使调用方能够进行 hit test。（已完成）
8. 复用现有文字 SDF 思路，用少量基础图形 SDF 模板绘制背景、实线、虚线、点线、波浪线、着重号和示亡号，并输出可直接提交的顶点。（已完成）
9. 逐字显示时同一个 authored range 的片段各自自带顶点，拼起来是完整图形，避免内部边界和重复 paint；合并与图元构造都由 Huozi 完成。（已完成）
10. 保持 Tiqian 为布局真值来源；Huozi 不重新计算断行、标点空间、注音位置或对象布局。（已完成）

## 范围

### 包含

- `RichTextLayoutOutput` 及所有公开布局入口的统一返回值；
- `UnitVertices` 从文字 struct 改为包含多种绘制结果的 enum；
- 正文、Ruby 和 bopomofo 的文字角色；
- 背景和行内代码背景的布局与 paint 数据；
- 下划线、删除线、着重号、示亡号、专名号和书名号的布局与 paint 数据；
- 行内对象的稳定资源键、替代文本和布局矩形；
- 链接的目标和逐行文档坐标矩形；
- 独立的 `SegmentGlyphSpan` 及其新范围语义；
- positioned cluster 级 `row`、`col` 和逐字可见规则；
- 图形片段的三层顶点；
- 全部解析段落的顺序布局和空段高度；
- 与现有高度限制一致的可见行筛选；
- Huozi 内部的基础图形 SDF 模板生成、图集分配与缓存；
- Huozi 内部的九宫格、三宫格、重复单元和末端裁剪图元构造；
- Huozi 侧在布局阶段把同一个 authored range 的片段定形成完整图形并按排版单元切段，每段自带顶点；
- 官方 WGPU demo 对全部新增绘制结果的消费；
- 公开输出、逐字显示、连续范围、顶点生成和 demo 视觉结果的必要测试。

### 不包含

- 新增本文未列出的富文本标签；
- 文字光标、选择、选词、复制文本查询或为这些查询保存 replay index；
- 通用对象资源管理、资源加载器、对象生命周期或对象回调系统；
- 链接 hover、按下状态、导航和打开外部目标；
- 为 WGPU 之外的其他渲染后端实现绘制器；非文字变体继续公开后端无关几何，愿意自行实现绘制的调用方可以据此接入，但本迭代不提供这类绘制器；
- 对调用方公开形状模板的图集条目、模板 UV、模板原始位图或 sampler 配置；
- 改写 Tiqian 的 shaping、字体 fallback、断行、标点处理、行调整、注音避让或对象布局；
- 竖排、完整 bidi/RTL、分页、多栏和字体原生彩色 glyph；
- 合成粗体、合成斜体或其他字体选择能力（已由字体选择与合成迭代交付，本迭代不重复处理）；
- 对外承诺稳定的富文本 ABI 或兼容旧 tuple 返回类型。

## 当前事实来源

本节描述 2026-09-26 首次实施前的代码状态，用于记录本迭代的起点。实际交付与偏差见“实施结果”。

### Huozi 当前实现

| 文件 | 当前事实 |
| --- | --- |
| `src/glyph_vertices.rs` | `UnitVertices` 仍是普通文字 struct，保存 shadow、stroke、fill、indices、row、col 和文字占位矩形；enum 变体尚未定义。 |
| `src/layout.rs` | 四个布局入口已经返回 `RichTextLayoutOutput`；`layout_parsed_text` 仍只读取第一段，并对后续段落记录固定 warn。 |
| `src/layout/layout_output.rs` | `RichTextLayoutOutput`、`Interaction`、`InteractionArea` 已经公开；`InteractionArea.rect` 使用 `tiqian::core::geometry::Rect`，坐标是 Tiqian occupied 几何。 |
| `src/layout/glyph_span.rs` | `SegmentGlyphSpan` 独立公开，保存 `segment_id` 和 `glyph_range`；当前只覆盖文字 glyph。 |
| `src/layout/tiqian_output.rs` | 输出正文与行尾自动连字符 glyph、按 positioned cluster 递增的 `row`/`col`、以及 `interactions`；遇到注音、着重号或装饰 segment 仍记录 warning 并跳过。 |
| `src/layout/vertex.rs` | `Vertex` 保存 `position`、`tex_coords`、`page`、`buffer`、`fill_buffer`、`gamma`、`color`；新增图形需要复用该格式。 |
| `examples/render/main.rs` | 只为普通文字构建 SDF 顶点与 `u16` 索引，按 shadow、stroke、fill 三遍收集后一次提交；已经保存 `interactions` 并提供点击提示。 |
| `examples/render/shader.wgsl` | 从 RGBA 图集指定通道采样 SDF，并用 `buffer`、`fill_buffer`、`gamma` 表达 fill、stroke 和 shadow；`gamma` 同时承担内缘覆盖率与额外边缘平滑。 |

### Tiqian 已提供的数据

Huozi 应读取 Tiqian 的最终结果，不从源文字重复推导：

| 能力 | Tiqian 数据 |
| --- | --- |
| 正文位置与逐字单位 | `positioned_clusters`（occupied `Rect`、`draw_x`、`baseline`、`source_stops`）、`glyph_runs`、`lines`。 |
| 富文本逐行几何 | `positioned_rich_text_segments()`：按 positioned cluster 切分、同行同来源连续时合并的 occupied 矩形。 |
| 背景 | `rich_text_background_segments()`（逐视觉行一个连续绘制盒，已去掉外侧允许去掉的 autospace、justify 和标点 glue）、`rich_text_background_corner_radii(segment, inset)`、`RichTextBackgroundPaint` 的 padding、圆角、延续圆角、`metric_policy` 和相邻同样式间隙。 |
| 下划线和删除线 | `rich_text_decoration_segments()`、`rich_text_decoration_line_y(segment, stroke_width)`、`RichTextLinePaint` 的 `thickness`、`pattern`（`Solid`、`Dashed { dash_length, gap_length }`、`Dotted { gap_length }`）和相邻同样式间隙。 |
| 装饰与注音 layer 关联 | `rich_text_decoration_layers(range, kind)`、`rich_text_annotation_layers(base_range, kind)`；当前的 `positioned_rich_text_segments` 会把每个 layer 拆成只含该 layer 的 span，因此也能直接从 `segment.span.layers` 取 paint。 |
| Ruby | `debug.ruby_decisions`：`base_range`、`line_index`、`center_x`、`baseline_y`、`font_size`、`width`、`overhang`、`font_families`、`font_weight`、`locale` 和已 shaping 的 `glyphs`。 |
| Bopomofo | `debug.bopomofo_decisions`：`base_range`、`line_index`、`placements`（每项含 glyph、`draw_x`、`baseline_y`、`font_size` 和 `Symbol`、`Tone`、`Neutral` 角色）。 |
| 着重号 | `debug.decoration_decisions`：`cluster_range`、`applied`、`anchor_x`、`anchor_y`、`dot_diameter`。 |
| 示亡号、专名号、书名号 | `debug.decoration_segments`：`source_range`、`kind`、`line_index`、`left/top/right/bottom`、`open_start`、`open_end`。示亡号是上下边闭合框，专名号与书名号是 `top == bottom` 的中心线。 |
| 行内对象 | `debug.inline_object_decisions`（`range`、`advance`、`ascent`、`descent`、`cluster_index`、`line_index`）、`input.inline_objects` 的 `id`，以及对应 positioned cluster。 |
| 链接区域 | `input.rich_text` 的 `RichTextSemantic::Link { id, target }` 与 positioned cluster 几何。 |
| 命中与复制几何 | `get_bounding_boxes`、`get_cursor_rect`、`get_text_for_copy`；本迭代只用链接与对象矩形。 |

`RichTextLineSegment` 携带 `Arc<RichTextSpan>`、`line_index`、`range`、`left/top/right/bottom` 和 `baseline`。Tiqian 内部的 `segment_layer` 辅助函数不对外公开，消费方需要自己在 `segment.span.layers` 中按 layer 类型取 paint 和 pattern。

本迭代需要 tiqian-rs 新增的部分（见待讨论问题第 9、10、15 项的结论）：富文本范围与装饰范围的可选 `id`，按 positioned cluster 细分的最终几何输出，以及 ParagraphBuilder 的 Segment 边界声明入口。

Tiqian 现有 `fitted_dashed_line_segments` 和 `fitted_dotted_line_centers`（位于 `core::fitted_line_pattern_geometry`，0.5 仍然提供）为完整范围生成首尾拟合图案。它们可以继续服务静态完整绘制，但不能直接决定本迭代的逐字动态图案：当前可见前缀增长时重新拟合会移动已经显示的 dash 或 dot。本迭代使用固定相位和固定节距，在当前可见末端裁剪。

## 目标数据路径

```text
Vec<Segment>
  → parser
  → ParsedText { paragraphs }
  → 每个 ParsedParagraph 分别建立 Tiqian LayoutInput
  → Tiqian LayoutResult
  → HuoziTiqianOutputAdapter
  → RichTextLayoutOutput
       ├─ glyphs: Vec<UnitVertices>
       ├─ segment_glyph_spans: Vec<SegmentGlyphSpan>
       ├─ interactions: Vec<Interaction>
       ├─ width
       └─ height
  → 调用方按逐字进度取可见前缀
  → Huozi 合并连续范围并生成图形顶点，按 PaintLayer 分层
  → 调用方上传纹理、提交 draw call
```

该路径中 `RichTextLayoutOutput`、`segment_glyph_spans`、`interactions` 和非文字 `UnitVertices` 变体已经接通。

多段布局时，Huozi 依次布局每个段落，将后续段落的纵向坐标和视觉行号累加到文档坐标。公开结果保持一条平铺序列，不新增段落结果集合或行集合。

## 公开布局结果

### `RichTextLayoutOutput`

统一结果至少表达：

```text
RichTextLayoutOutput
├─ glyphs: Vec<UnitVertices>
├─ segment_glyph_spans: Vec<SegmentGlyphSpan>
├─ interactions: Vec<Interaction>
├─ width: u32
└─ height: u32
```

`layout`、`layout_plain`、`layout_parse` 和 `layout_parse_with` 均已返回该类型。项目处于开发阶段，本迭代不保留旧 tuple 返回值的兼容包装。

`width` 和 `height` 是整个文档**已输出可见结果**的布局尺寸（见待讨论问题第 8 项的结论）。多段文档的 `width` 是各段实际输出宽度的最大值；`height` 包含全部已输出段落和空段占用的高度，并继续服从现有布局高度限制。两者都不包含 stroke、shadow 或 glyph ink 的外溢。

### `UnitVertices`

`UnitVertices` 改为具名 enum。各变体的字段形状已确定（2026-09-24）：

```text
UnitVertices
├─ Text（文字 SDF 四边形，另加 role）
├─ Background { rect, corner_radii, vertices, row, col }
├─ Line { left, right, line_y, thickness, pattern, vertices, row, col }
├─ Decoration { kind, shape, thickness, wave, vertices, row, col }
└─ InlineObject { rect, id, alt, row, col }
```

所有矩形复用 `tiqian::core::geometry::Rect`，与 `InteractionArea` 一致。非文字变体的 paint 使用一个最小公开类型表达 fill、stroke 和 shadow，颜色在布局时按 `ColorSpace` 转成 `[f32; 4]`；不复用 Tiqian 的 ARGB `RichTextPaint`，避免调用方再解析颜色。`corner_radii`、`thickness`、`pattern` 与波浪参数都直接放在变体字段上，不依赖调用方持有 Tiqian 数据。

名称沿用现有 `UnitVertices`，不新增与它职责重复的 `RichTextLayoutItem`、`SdfQuad` 或其他外层包装。非文字变体保存的是**切段后**的几何：同一个 authored range 的片段共享一份整段几何，但各自只写入自己那一段的顶点，见“图形顶点与分段”。

所有变体都必须能够取得：

- 文档坐标中的最终布局位置；
- `row`；
- `col`；
- 自己的绘制顶点（行内对象除外，由调用方按资源绘制）。

公开字段的最终 Rust 拆分方式仍是待讨论项，但不得改变本文规定的语义。

### `Text`

`Text` 保留当前文字 SDF 输出所需的数据：

- 可选 shadow 四边形；
- 可选 stroke 四边形；
- fill 四边形；
- indices；
- 占位矩形；
- scale ratio；
- `row`、`col`；
- 文字角色 `role`。

文字角色只包含：

```text
TextRole
├─ Body
├─ Ruby
└─ Bopomofo
```

正文、Ruby 和 bopomofo 复用同一个字体 glyph SDF 图集和现有文字 shader。Ruby 与 bopomofo 必须重放 Tiqian 已 shaping 的 glyph 和最终位置，不从注音字符串再次 shaping。

### `Background`

背景结果表达一个排版单元对连续背景范围的贡献，至少包括：

- 该单元对应的文档坐标矩形；
- 四角半径：直接保存 Tiqian 解析出的最终值（见待讨论问题第 3 项的结论）；
- `shadow`、`stroke`、`fill` 三层顶点；
- `row`、`col`。

背景结果不保存模板 UV 或图集位置。`Huozi` 在布局时按 authored range 求整段几何，生成一次九宫格，再按各排版单元切段；每段的顶点拼起来是完整的圆角矩形。

背景与行内代码背景使用同一输出语义。跨行范围在每个视觉行上分别成形；同一个源范围跨行时每行各有自己的完整图形，不会把两行错误接成一个矩形。

### `Line`

`Line` 用于下划线和删除线，至少表达：

- 线条种类；
- 该单元对连续线条范围贡献的横向区间；
- Tiqian 决定的中心线纵坐标；
- thickness 与 pattern，直接转写 Tiqian `RichTextLinePaint` 的 `thickness` 与 `pattern`（见待讨论问题第 4 项的结论）；
- `shadow`、`stroke`、`fill` 三层顶点；
- `row`、`col`。

`Line` 不保存完整状态下拟合好的所有 dash 或 dot。`Huozi` 以连续范围的固定左端作为相位锚点，用原始 dash、gap、dot 参数产生固定节距图案，再按排版单元切段；已出现的 dash 位置不随后缀增长而改变。

### `Decoration`

`Decoration` 表达 CLREQ 装饰：

- 着重号：圆点锚点与直径（来自 `debug.decoration_decisions`）；
- 示亡号：矩形边界和 Tiqian 根据完整源范围决定的 `open_start` / `open_end`；
- 专名号：直线范围与线宽；
- 书名号：波浪线范围、线宽、波周期与振幅；
- `shadow`、`stroke`、`fill` 三层顶点；
- `row`、`col`。

着重号按字符独立出现，每个点自身就是一个完整图形，不参与范围分组；它仍按对应基文的 `row`、`col` 控制可见性。示亡号、专名号和书名号需要连续分组。

书名号公开结果保存线条范围和图案参数，不公开最终 GPU 曲线段。Huozi 内部使用可连续拼接的波浪 SDF 周期模板，固定左端相位，重复完整周期并裁剪末端。

线宽与波浪参数在布局时按该范围解析后的字号算成逻辑像素后公开（见待讨论问题第 5 项的结论）：线宽 `fontSize / 16`（最小 1px），半波长度 `0.2em`（即波周期 `0.4em`），振幅 `0.06em`。它们取自 CLREQ 参考实现（Compose Skia 与 Android renderer 的 `wavyLinePath` 和 `browserLikeSkipInkClearance`），公开后调用方不再自行选值。

### `InlineObject`

行内对象结果保存：

- `id`：调用方资源表使用的可选字符串键；
- `alt`：替代文本和语义文本；
- 文档坐标矩形；
- `row`、`col`。

`[object /]` 增加可选 `id` 属性。缺少或为空时，对象照常完成布局和输出，不产生对应的 `Interaction`。现有必填 `alt`、`width`、`ascent` 和 `descent` 语义保持不变。

Huozi 不保存对象资源、GPU handle、回调或资源生命周期。

### `Interaction`

链接和行内对象的交互结果统一保存：

- `id`；
- 按最终排版单元生成的 `Vec<InteractionArea>`。

每个 `InteractionArea` 保存 Tiqian occupied `Rect` 以及对应的 `row`、`col`。跨行链接仍输出一个 `Interaction`，其区域分布在多行。链接 `target` 继续作为 Tiqian 断行使用的元数据，不进入交互结果。

完整交互设计由 `docs/iteration/2026-09-21-feat-interaction-hit-testing.md` 定义。

## 不进入公开结果的数据

以下数据明确不进入新增的非文字公开结果：

- parser/Tiqian 显示坐标使用的 `display_range`；
- 每个绘制元素上的完整 `SourceRange`；
- Tiqian `LayoutResult` 或 query replay index；
- 文字光标、选择、选词和复制所需的查询索引；
- 非文字形状模板的图集条目、模板 UV、模板原始位图或 sampler 配置；
- 图集分配、模板缓存与淘汰策略。

`SourceRange` 继续供 parser、Tiqian 输入映射和 `SegmentGlyphSpan` 构造内部使用，但不复制到每个公开绘制元素。

该限制不删除现有文字路径已经公开的 `Vertex`、纹理坐标和 page。`UnitVertices::Text` 继续保存这些 SDF 绘制数据；非文字变体保存切段后的顶点，与它们共用同一个 `Vertex` 格式。模板内部的几何位图与尺寸仍然不对外暴露。

## `SegmentGlyphSpan`

`SegmentGlyphSpan` 是自 v0.16.0 起的公开生产 API，用于调用方按输入 `SegmentId` 管理 glyph。它继续作为 `RichTextLayoutOutput` 的独立集合，不并入 `UnitVertices`。

语义调整为：

- `glyph_range` 仍是指向主 `glyphs` 的半开连续下标范围；
- 范围覆盖该输入 segment 产生的全部绘制元素，包括文字、背景、线条、装饰和对象；
- 同一个 `SegmentId` 在主序列中出现多个不连续区间时，使用多个 `SegmentGlyphSpan`，不强行合并；
- 链接区域不属于 `glyphs`，因此不进入 `glyph_range`；
- 不新增 `SourceRange` 作为替代品。

输出适配器按输入来源组织每个 `glyph_range`。Huozi 输入适配会把每个输入 Segment 的显示范围端点声明为 Tiqian `source_boundaries`（见待讨论问题第 10、14 项的结论），因此 cluster 不跨 Segment，每个绘制元素只属于一个 `SegmentId`；同一 `SegmentId` 在主序列中出现多个不连续区间时仍使用多个 `SegmentGlyphSpan`。具体构造算法必须保持相对于输出元素数量的线性复杂度，不得为每个 segment 反复全表扫描。

## `row`、`col` 与逐字显示

### 基本单位

`col` 按 Tiqian positioned cluster 递增，不按 glyph 递增：

1. 同一个 positioned cluster 产生的多个字体 glyph 共用同一个 `row`、`col`。
2. 该 cluster 对应的背景、线条和装饰增量片段共用相同 `row`、`col`。
3. 一个行内对象占一个 `col`。
4. 下一 cluster 才令 `col += 1`。
5. 每个新的视觉行从 `col = 0` 开始。
6. `row` 是整个多段文档中的连续视觉行号，不在段落边界重置。

该行为已经由交互迭代修正：当前实现按 positioned cluster 递增 `col`；`row` 是多段文档内的连续视觉行号。

### 注音

Ruby 和 bopomofo 跟随基文显示：

- 能够逐 cluster 对应时，注音 glyph 使用相应基文 cluster 的 `row`、`col`；
- 一个基文 cluster 对应多个注音 glyph 时，它们共用该 cluster 的 `row`、`col`；
- Tiqian 只提供覆盖多个 cluster 的整体注音、无法可靠拆到单个 cluster 时，该注音使用基文范围最后一个 cluster 的 `row`、`col`；
- 不为注音额外插入独立 col。

以上规则已在 2026-09-24 确定，详见待讨论问题第 18 项。无绘制 glyph 的 cluster 与行尾连字符的处理见第 16、17 项结论。

### 行尾自动连字符

Tiqian 生成的行尾自动连字符跟随本行最后一个正文 cluster，使用相同 `row`、`col`。它不占新的 col，也不提前于对应正文显示。

### 当前可见前缀

调用方以逐字进度取可见前缀：`glyphs[..end]` 就是“打印到第 `end` 个元素”的完整可见状态，顺序由库保证。`(row, col)` 是同一进度的另一种表达，仍可用于调用方自己的行级判断。

每个元素自带顶点，因此取前缀就够。绘制层顺序与数组顺序不同，提交前需要按固定绘制层拼一遍；不得用主 `glyphs` 的单次线性顺序替代绘制层排序。

## 连续视觉范围

### 分组身份

背景、下划线、删除线、示亡号、专名号和书名号在布局时需要按 authored range 归组：只有同一个范围跨多个排版单元时才需要“先定形、再切段”。这个身份是内部实现，不出现在公开类型里。

语义如下：

1. 同一个 authored range 的片段属于同一个图形；跨视觉行时每行各自成形。
2. 嵌套范围、相邻但来源不同的范围，即使样式完全相同，也不属于同一个图形。
3. 身份只在单次布局内有效。
4. 不增加 `sequence` 字段；组内顺序由 `row`、`col` 和主 `glyphs` 中的稳定顺序确定。
5. 身份来源是 Tiqian 携带的范围 `id`：Huozi 输入适配按范围遇见的顺序分配序号（见待讨论问题第 9、13 项的结论）。

### 片段定形与切段

Huozi 对每一个分组执行：

1. 按该组在一个视觉行内的全部片段求整段几何；
2. 在整段几何上生成完整图形（九宫格、三宫格或固定相位重复）；
3. 每个片段只取自己那一段的顶点，切分边界落在相邻片段的接缝上。

同一组只生成一个图形，范围内的几何空洞不断开（见待讨论问题第 11 项的结论）。不能按片段分别生成完整图形再靠重叠拼起来：那会产生内部圆角、重复描边、重复阴影和半透明重叠。

### 相位稳定

虚线、点线和波浪线以连续范围的最终左端作为固定相位锚点：

- dash、dot 或 wave 的节距固定；
- 已显示部分的位置不随可见前缀增长而改变；
- 当前可见右端只裁剪最后一个模板实例；
- 不为了让当前前缀首尾都完整而重新分配间距；
- 跨行时每个视觉行根据该行片段的左端开始绘制，具体是否继承上一行相位属于待讨论问题。

## 图形顶点与分段

### 每个元素自带顶点

文字与图形都自带顶点，因此取 `glyphs[..end]` 就是逐字显示状态，不需要第二个入口，也不需要调用方自己合并。

文字变体保存三层 `[Vertex; 4]`；图形变体保存 `ShapeVertices`：

```text
ShapeVertices
├─ shadow: Vec<Vertex>
├─ stroke: Vec<Vertex>
└─ fill: Vec<Vertex>
```

每 4 个连续顶点构成一个四边形，顺序为左上、左下、右下、右上；层为空表示该元素没有这一层。索引由调用方按 `[0, 1, 2, 0, 2, 3]` 展开，不需要逐元素保存。

### 一个范围定形一次，再切成段

一个 authored range 跨多个排版单元时，每个单元是一个独立元素，但图形不能按单元分别画：那会产生内部圆角、重复描边、重复阴影和半透明重叠。

做法是先在整段上定形，再按各单元的范围切开：

1. 按 authored range（跨行时再按视觉行）求整段几何；
2. 在整段几何上生成完整图形；
3. 各片段只在相邻片段的接缝处切开，整段首尾不设边界。

分段窗口因此是：

```text
片段 0: (-∞, r0]     片段 1: (r0, r1]     片段 2: (r1, +∞)
```

首尾不设边界，圆角、描边、阴影与端帽都不会被切掉；接缝对齐在上一段的右边界，两侧正好衔接，既不重叠也不留缝。切片时 UV 按目标坐标的同一比例插值，因此相邻两段在接缝处的采样完全一致。

拼起来就是一个完整图形，逐字推进时图形跟着长。

### 着重号不参与合并

着重号按字符独立出现，每个点自身就是一个完整图形，不进入范围分组。

### 形状模板

Huozi 内部按需生成并缓存以下模板，与文字图集同源管理，调用方不可见：

- 圆角矩形（按整数半径，用到哪个生成哪个）；
- 圆点；
- 线带与端帽；
- 波浪周期；
- 示亡号边框及开口变体。

模板使用与文字相同的 SDF 编码和阈值语义（`FILL_THRESHOLD_BIAS`、`EDGE_SMOOTHING_HALF_WIDTH`、`fill_buffer`、`gamma`），因此图形与文字的边缘、描边和阴影过渡一致。模板的过采样倍率、缓冲边距和块内居中规则是 Huozi 的内部数值决策，不对外暴露；调用方只面对语义化结果（圆角半径、线宽、图案参数）。

不再需要“调用方提供位图”的公开入口：模板生成进入库内后不存在该路径。公开的 `sdf` 模块同样收回，距离变换是内部实现细节。

## 多段文档与空段

`layout_parse` 和 `layout_parse_with` 必须布局 `ParsedText.paragraphs` 中的全部段落，不再只取第一段。

段落按源顺序纵向排列：

1. 每段使用其已经解析的 `ParagraphStyleOverride`。
2. 后续段落的所有文档坐标加上前面段落累积高度。
3. 后续段落的 `row` 加上前面段落累积视觉行数。
4. 每段独立调用 Tiqian，不在 Huozi 中合并文本后重新模拟段落规则。
5. 每段输出继续使用同一个主 `glyphs`、`segment_glyph_spans` 和 `interactions` 集合。

连续 `[br /]` 形成的空段必须占一行高度：

- 优先使用该空段解析后的绝对 `line_height`；
- 没有绝对覆盖时使用 `initial_text_style.font_size * layout_style.line_height`；
- 空段不产生 `UnitVertices`；
- 空段令后续内容的 `row += 1`；
- 空段高度计入文档 `height` 和现有 `box_height` 限制。

以上规则已在 2026-09-24 确定，详见待讨论问题第 29、30、31 项。

该规则由 Huozi 的文档组合层处理，因为 Tiqian 对空输入返回零行。它不表示向空段伪造 glyph 或 source range。

## 高度限制

当前适配器只输出满足 `line.bottom <= max_height` 的连续正文行。本迭代延续这一行为，并统一应用到：

- 正文、Ruby 和 bopomofo；
- 背景和线条；
- CLREQ 装饰；
- 行内对象；
- 链接区域；
- `SegmentGlyphSpan`。

多段布局使用剩余文档高度作为后续段落可用高度。遇到第一个无法完整容纳的视觉行后停止输出，不从更后的段落恢复输出。跨行范围只保留已输出行对应的增量片段。以上行为见待讨论问题第 31 项的结论。

空段也消耗高度；无法完整容纳一个空段行高时停止，不增加对应 row 和文档高度。

## 绘制层顺序

Huozi 按以下固定顺序输出顶点：

| 顺序 | 层 | 内容 |
| --- | --- | --- |
| 1 | 背景阴影 | 背景和行内代码背景的阴影。 |
| 2 | 背景描边 | 背景和行内代码背景的描边。 |
| 3 | 背景填充 | 背景和行内代码背景的填充。 |
| 4 | 文字阴影 | 正文、Ruby 和 bopomofo 的阴影。 |
| 5 | 文字描边 | 正文、Ruby 和 bopomofo 的描边。 |
| 6 | 文字填充 | 正文、Ruby 和 bopomofo 的填充。 |
| 7 | 文字装饰 | 下划线、删除线、着重号、示亡号、专名号和书名号。 |
| 8 | 行内对象 | 调用方提供资源的行内对象。 |

层只用于确定顶点的先后次序，不出现在公开类型里。嵌套背景之间的稳定先后顺序由主 `glyphs` 顺序保持。

官方 demo 必须按该顺序产生实际 draw 结果，不能继续只绘制文字三层。（已完成：demo 取可见前缀后按固定层次拼一遍。）

## 基础图形 SDF 方案

### 总体原则

1. 保留现有文字 glyph SDF pipeline 和 fill/stroke/shadow 阈值语义。
2. 为非文字图形准备少量预生成的基础 SDF 模板，模板生成与图集分配都在 Huozi 内部完成。
3. `RichTextLayoutOutput` 给出最终布局位置、paint、pattern 和连续关系；Huozi 据此生成顶点，调用方负责上传纹理和提交 draw call。
4. 优先用重复 quad 和末端裁剪，不立即依赖 shader 内 `fract` 实现 atlas 子区域 repeat。
5. 图形模板需要能够使用现有 `Vertex` 和 SDF shader；是否需要小幅扩展 shader 由实施验证决定，不能以“完全零修改 shader”为验收条件。
6. 调用方不可见模板本身：既不公开模板图集条目或原始位图，也不要求调用方选模板、定尺寸或选阈值。

### 基础模板集合

当前方案需要的最小模板类别为：

- 圆角矩形或可组成圆角矩形的九宫格模板；
- 圆点；
- 方头或圆头短线；
- 可连续横向拼接的波浪周期；
- 示亡号所需的边和角，可复用矩形边角模板或使用专门模板。

模板的基准像素尺寸、过采样倍率和 SDF spread 是 Huozi 的内部数值决策，实施时按视觉样例校准；它们不进入公开 API。

### 背景：九宫格

背景使用九宫格：

- 四角保持比例，不拉伸圆角；
- 上下边只沿横向拉伸；
- 左右边只沿纵向拉伸；
- 中心同时沿两个方向拉伸；
- 四角半径使用 Tiqian 已解析的最终值；
- shadow、stroke 和 fill 对同一个合并后背景各生成一次；
- 小尺寸背景需要在角半径超过可用尺寸时沿用 Tiqian 已 clamp 的半径，Huozi 不另建一套圆角规则。

九个块必须共享完全相同的目标边界坐标，避免浮点舍入形成可见空隙。图集内每个模板还必须预留防线性采样串色的 padding 和边缘 texel 扩展。

### 实线：三宫格或矩形

实线和专名号可以使用横向三宫格胶囊或矩形：

- 左右端帽保持比例；
- 中间只沿横向拉伸；
- 当前可见范围只提交一对端帽；
- shadow、stroke 和 fill 各只覆盖合并后的整条线一次。

如果线条语义要求方头，可使用方头模板或裁掉圆头端帽，不应通过多个重叠短线模拟。

### 虚线

虚线使用短线模板重复：

1. 以连续范围左端为固定相位锚点；
2. 使用布局结果中的 `dash_length` 和 `gap_length` 作为目标逻辑长度；
3. 为每个完整 dash 生成一个模板 quad；
4. 最后一个不完整 dash 同时裁剪目标几何和模板 UV；
5. 不在当前可见范围内重新拟合 gap；
6. 阴影和描边跟随每个可见 dash，但同一个 dash 不因 cluster 切分重复提交。

### 点线与着重号

点线使用圆点模板按固定节距重复。最后一个超出当前可见末端的完整圆点不提交；是否允许末端出现被裁剪的半圆点属于待讨论问题。

着重号使用同一个圆点模板，但位置和直径完全来自 Tiqian `decoration_decisions`，不按下划线点线节距重新计算。

### 书名号波浪线

书名号使用可连续横向重复的波浪 SDF 周期模板：

- 连续范围左端固定相位；
- 重复完整周期；
- 当前可见末端裁剪目标几何和 UV；
- 已显示周期不会随新 cluster 出现而移动；
- 振幅、线宽和周期必须来自明确的数据，不在 shader 中按目标宽度临时拟合。

本迭代不把公开输出固定为贝塞尔曲线段，也不要求 CPU 逐段三角化波浪线。

### 示亡号

示亡号使用四角、水平边和垂直边拼接：

- Tiqian `open_start` / `open_end` 表示完整源范围跨行时应省略的边；
- 当前逐字可见前缀的临时末端需要闭合，以形成完整可见框；
- 当后续 cluster 出现时，临时末端移动到新的右端；
- 完整源范围标记 `open_end` 的行末不能被逐字临时闭合规则错误补回；
- 每个视觉行独立拼接，不跨行生成一个大矩形。

完整源范围的跨行开口与当前可见临时端点的精确优先级需在实施前用状态表确认。

### Shadow padding 与 SDF 距离范围

基础模板必须为以下内容预留足够空间：

- SDF spread；
- 最大需要支持的 stroke 外扩；
- shadow spread；
- shadow offset；
- blur 需要的距离范围；
- atlas tile 防串色 padding；
- 线性采样所需的边缘 texel 扩展。

九宫格和重复模板不能把有效 SDF 距离区拉伸到失真。边、角、中心需要使用适合各自拉伸方向的模板区域。具体模板尺寸和最大 paint 参数仍需基于当前常量和视觉测试确定。

### Atlas 采样

不能把整个 glyph atlas sampler 改成 `Repeat`，因为 UV 超出当前 tile 后会采样到相邻 glyph 或其他模板。

首选实现是：

- 每个重复单元生成独立 quad；
- UV 始终限制在当前模板 tile 内；
- 最后一个单元通过几何与 UV 同步裁剪；
- atlas 继续使用 clamp 型过滤采样。

只有在重复 quad 的数量经实际测量成为问题时，才讨论为 shader 增加 tile-local repeat；本迭代不预先引入该优化。

## 缓冲与提交

生成的索引统一为 `u32`，调用方不需要自行分批或换用非索引绘制。无论采用哪种方式，都必须：

- 保持固定 PaintLayer 顺序；
- 不为每个单独 glyph 或 dash 创建独立 GPU buffer；
- 不在每帧为未变化文本重复生成模板资源；
- 对当前可见前缀变化只重建必要的 CPU 顶点/索引数据；
- 在 WebGL2 downlevel limits 下仍可运行 demo。

## 行内对象在 demo 中的边界

公开输出必须完整提供对象 `id`、`alt` 和矩形。官方 demo 需要证明它实际消费 `InlineObject` 变体并在正确层级显示占位内容，但本迭代不建立通用资源系统。

具体使用固定示例纹理、简单占位图形还是已有 demo 资源，属于待讨论问题。无论选择哪种验证方式，都不能把 demo 专用资源句柄写入 Huozi 公开布局结果。

## 宽容处理

Huozi 面向游戏运行时，单个坏数据不应中止整段布局或整帧绘制：

- 某个注音 glyph 缺少可回放字体身份：warning，跳过该 glyph，保留其他结果；
- 无效或无法关联的富文本几何：warning，跳过局部图形；
- demo 缺少对象资源：显示固定占位或跳过对象图形，但保留布局占位；
- 基础模板生成失败：局部退化，跳过对应图形，不 panic 或停止文字绘制。

warning 不应为格式化复杂上下文而重新扫描、排序、clone 或收集整份布局数据。

## 实施阶段

本迭代需要交付一条完整可用路径。以下阶段仅表示实施顺序，不可作为多个迭代分别交付。最终验收前，公开输出和 demo 绘制必须同时完成。

按 2026-09-24 的校准，Phase 1 第 1 项、Phase 2 第 2 项和 Phase 2 第 5 项的链接与对象部分已经由交互迭代完成，见“校准记录”。下面各阶段中已标注的部分不再重复实施，但仍需与新增结果保持同一套 `row`、`col` 和输出集合语义。

本迭代还依赖一次 tiqian-rs 迭代：为富文本范围与装饰范围增加可选 `id`，并提供逐 cluster 的最终富文本几何。该迭代已完成（`tiqian` 0.6）。全部阶段已于 2026-09-26 完成，实际交付与差异见“实施结果”。

### Phase 1：确定公开类型和对象键

1. 为 `[object /]` 增加可选 `id`，更新 parser lowering 和相关文档。（已完成）
2. 定义 `RichTextLayoutOutput`、`UnitVertices` enum、`TextRole`、`Interaction`、`InteractionArea` 及非文字变体需要的后端无关数据。（已完成）
3. 保留独立 `SegmentGlyphSpan`，明确 `glyph_range` 指向主 `glyphs`。（已完成）
4. 定义图形片段的分组身份与单次布局生命周期。（已完成）
5. 不在新增的非文字公开类型中加入模板 atlas、UV、GPU vertex 或文字查询索引；`Text` 保留既有文字 SDF 顶点数据。（已完成）

阶段完成条件：公开类型可表达本文列出的所有结果，字段语义经过讨论确认；尚不以 demo 可见结果作为该阶段完成条件。

### Phase 2：多段与 Tiqian 输出适配

1. 依次布局全部 `ParsedParagraph`，累加文档 Y 坐标和 row。（已完成）
2. 为 positioned cluster 建立唯一的 `row`、`col`，同 cluster 多 glyph 共用。（已完成）
3. 输出正文、自动连字符、Ruby 和 bopomofo 的 `Text`。（已完成）
4. 输出背景、下划线、删除线和 CLREQ 装饰的 cluster 级增量片段。（已完成，依赖 tiqian-rs 的逐 cluster 几何）
5. 输出行内对象和链接矩形。（已完成）
6. 按 authored range 归组图形片段，使嵌套、相邻和跨行范围可明确区分。（已完成，依赖 tiqian-rs 的范围 `id`）
7. 构造覆盖全部 `glyphs` 变体的 `SegmentGlyphSpan`。（已完成，依赖 tiqian-rs 的 Segment 边界声明）
8. 对所有结果应用同一可见行和高度限制。（已完成）
9. 实现空段一行高度规则。（已完成）
10. 新增：把全部结果排成逐字显示顺序，使前缀即为可见状态。（见“实施结果”第 1 条）

阶段完成条件：无需 WGPU 即可通过结构化测试证明布局结果、逐字顺序、连续分组、来源 span、多段坐标和 hit-test 矩形正确。

### Phase 3：形状模板与图形顶点（已完成）

几何规则与首次实施的视觉结论都保留，实现位置从 `examples/render` 移入 `src/shape/`：

1. 把模板生成、阈值换算与图元构造移入 `src/shape/`，不保留 demo 侧的包装类型。（已完成）
2. 模板按需生成并缓存进图集，不预先烤出全部半径。（已完成）
3. 字形路径与模板路径共用一个图集写入函数。（已完成）
4. 继续复用现有 `Vertex` 与 SDF 阈值语义；只在实际需要时小幅调整 shader。（已完成）
5. 为模板拼接边界、分段边界与阈值语义建立受控测试。（已完成）
6. 收回 `allocate_resident_template`、公开 `AtlasTemplate` 与公开 `sdf` 模块。（已完成）

阶段完成条件：不依赖 WGPU 即可通过结构化测试证明各基础图形的顶点几何、拼接边界与分段正确性。（已满足）

### Phase 4：图形定形与 demo 消费（已完成）

1. 在布局阶段把同一个 authored range 的片段定形成完整图形，再按排版单元切成段。（已完成）
2. 分段只在相邻片段的接缝处切开，整段首尾不设边界，保留圆角、描边、阴影与端帽。（已完成）
3. 为背景生成九宫格；为实线生成三宫格；为虚线、点线和波浪生成固定相位重复模板。（已完成）
4. 根据 Tiqian 开口拼接示亡号；着重号按字符独立成形。（已完成）
5. 每个图形片段自带 `shadow`、`stroke`、`fill` 三层顶点。（已完成）
6. 消费 `InlineObject` 并保留对象占位（不绘制对象内容，见待讨论问题第 27 项的结论）。（已完成）
7. demo 取可见前缀、按绘制层拼顶点、上传纹理；保留 egui 控制层在内容之后绘制的现有关系。（已完成）
8. 逐字滑条只重建缓冲、不重新排版。（已完成）

阶段完成条件：官方 demo 能实际显示本文范围内的全部富文本图形，逐字前缀下连续背景、线条和装饰不产生内部重叠或相位跳动。（已满足）

### Phase 5：持续文档与最终验证

1. 更新 `docs/architecture.md` 的布局输出、段落组合、逐字显示和绘制数据入口。（已完成；2026-09-29 已按归属修订同步）
2. 更新 `docs/api-design.md` 的公开类型和布局入口。（已完成）
3. 更新 `docs/parser.md` 的对象 `id` 语义。（已完成）
4. 更新 `docs/rendering-gap-analysis.md`，将本迭代完成的能力改为已实现，并继续保留文字查询等延期能力。（已完成）
5. 将本文状态改为“已完成”，记录实际类型、待讨论问题的最终结论和验证结果。（已完成）
6. 审阅目标 diff，确认没有改写无关字体、parser 或 Tiqian 规则。（已完成）

## 最小必要测试

### 公开输出与逐字顺序

| 验证目标 | 最小证据 |
| --- | --- |
| cluster 级 col | 一个 cluster 产生多个 glyph，断言它们及对应背景/线条共用同一 `row`、`col`，下一 cluster 才递增。 |
| 行尾自动连字符 | 断言 hyphen 与本行最后一个正文 cluster 共用 `row`、`col`。 |
| Ruby 跟随基文 | 可逐 cluster 对应的 Ruby 使用对应 col；整体注音使用基文最后一个 col。 |
| Bopomofo 跟随基文 | 同一基文的符号、调号和轻声点在预期 col 出现。 |
| 对象占一 col | 对象位于两个文字 cluster 之间，后一个文字 col 正确递增。 |
| 多段 row/y | 两个非空段落的 row 连续，第二段全部 y 坐标包含首段高度偏移。 |
| 空段 | 连续 `[br /]` 产生的空段不生成 glyph，但各增加一行高度和一个 row 偏移。 |
| 高度限制 | 超出 `box_height` 的第一行及其背景、注音、装饰、对象、链接和 span 都不输出。 |

`cluster 级 col` 与 `行尾自动连字符` 两行的行为已经在当前实现中生效：`col` 按 positioned cluster 递增，连字符使用本行最后一个 cluster 的 `col`。现有测试只断言了链接区域与 glyph 的 `row`、`col` 一致，多 glyph cluster 和注音的断言仍待补。

### 连续范围

| 验证目标 | 最小证据 |
| --- | --- |
| 同范围各段拼成完整图形 | 同一背景跨多个 cluster 时，各段的顶点首尾相接，拼起来是一个圆角矩形。 |
| 按行独立成形 | 同一范围跨行时，每行各有自己的完整图形。 |
| 相邻范围不合并 | 两个相邻但来源不同的背景不会被当成同一个图形。 |
| 嵌套范围保持独立 | 内层把外层切成前后两段时，外层两段仍属同一图形，内层是另一个。 |
| 分段不切掉外扩 | 整段首尾保留模板边距，描边、阴影、端帽与圆角都完整。 |
| 固定相位 | 增加可见 col 后，已经出现的 dash、dot 和 wave 顶点位置不变。 |

### `SegmentGlyphSpan`

| 验证目标 | 最小证据 |
| --- | --- |
| 独立集合 | `segment_glyph_spans` 不嵌入 `UnitVertices`。 |
| 全元素覆盖 | 同一 segment 产生文字、背景、线条和对象时，其 span 覆盖这些主序列元素。 |
| 不连续位置 | 同一 `SegmentId` 在主序列中不连续时产生多个 span。 |
| 无 SourceRange 替代 | 公开绘制元素不携带完整 `SourceRange`。 |

### 链接与对象

| 验证目标 | 最小证据 |
| --- | --- |
| 链接跨行 | 一个跨两行链接输出一个 `Interaction`，其区域分布在两行。 |
| 对象数据 | 对象输出包含可选 `id`、`alt`、最终矩形和 `row/col`；有 `id` 时另有对应 `Interaction`。 |
| 对象缺少 id | 对象照常完成布局和输出，不产生 `Interaction`。 |
| hit test | 用矩形内外各一点证明调用方无需 replay index 即可命中链接和对象。 |

链接和行内对象的命中区域已经由交互迭代实现，并有 `tests/layout_entrypoints.rs` 中的结构化测试。

### 图形绘制数据

这些断言都属于库侧结构化测试，不需要 WGPU：

| 验证目标 | 最小证据 |
| --- | --- |
| 九宫格背景 | 不同宽高和圆角下角块保持 1:1、块间共享同一目标边界、无空隙。 |
| 背景 paint | fill、stroke、shadow 各绘制一次，逐字增长时无内部重叠。 |
| 圆角模板选择 | 每个整数半径选中同半径模板，弧不落入被拉伸的中块。 |
| 实线 | 端帽只出现在合并范围两端。 |
| 虚线 | 固定节距、固定相位，末端裁剪正确。 |
| 点线 | 圆点间距稳定，不因可见前缀增长整体移动。 |
| 波浪线 | 周期拼接处连续，端帽轴线与切线对齐。 |
| 着重号 | 圆点使用 Tiqian 锚点和直径。 |
| 示亡号 | 单行、跨行开口和逐字临时末端均符合状态表。 |
| 注音 | Ruby 和 bopomofo 使用 Tiqian glyph 与最终位置。 |
| 分层与索引 | 顶点与索引顺序符合上述层序；超过旧 `u16` 顶点上限的输入不溢出。 |
| 可见前缀 | 取不同 `visible` 时，文字部分只有前缀差异，非文字图形只在合并范围端点延长。 |

测试应断言 Huozi 自己的数据转换和顶点生成结果，不复制 Tiqian 已有的 shaping、断行或标点规则测试。

## 人工视觉验收

官方 demo 至少提供一个覆盖以下内容的固定文本：

1. 跨多个 cluster、跨行、嵌套和相邻的圆角背景；
2. 带 fill、stroke 和 shadow 的背景；
3. 实线、虚线、点线下划线和删除线；
4. Ruby 与 bopomofo；
5. 着重号、示亡号、专名号和书名号；
6. 跨行链接；
7. 一个带稳定 `id` 的行内对象，demo 保留其空位但不绘制内容；
8. 多个段落和连续 `[br /]` 形成的空段；
9. 配置面板最上方的逐字进度滑条。

人工检查逐字推进时：

- 背景只延长外边界，不出现内部圆角、重叠透明度或重复阴影；
- 实线不出现内部圆头；
- 虚线、点线和波浪线已显示部分不跳动；
- 跨行范围按行独立绘制；
- 示亡号按完整源范围决定的开口与临时末端正确；
- 注音、对象和行尾连字符与基文同时出现；
- 空段高度和后续行号符合预期。

## 性能与内存要求

1. Tiqian 输出适配保持相对于 cluster、glyph 和富文本结果数量的线性复杂度；不得为每个输出元素全表扫描全部 source map、span 或 rich-text range。
2. 分组身份在适配时顺序分配，不使用字符串 key、哈希内容或跨布局全局表。
3. 当前可见合并按一次线性扫描或按组线性收集完成，不对同一行片段执行 $O(n^2)$ 邻接查找。
4. 基础 SDF 模板只生成一次；文本内容变化不重复栅格化固定模板。
5. 重复 dash、dot 和 wave 可以增加 quad 数量，但不得为每个单元创建 GPU buffer 或 draw call。
6. 取可见前缀后的顶点生成保持线性：一次扫描分组，再按组生成图元，不对同一行片段做重复查找。
7. 不 clone 字体 bytes、整份 `LayoutResult`、完整 parser 树或 atlas pixels 来构造公开结果。
8. `SegmentGlyphSpan` 构造使用单次输出遍历，不按每个 segment 反复过滤全部 `glyphs`。
9. warning 路径不额外收集、排序或格式化完整布局上下文。
10. 完成后使用实际长文本比较顶点数量、CPU 顶点生成时间和 draw call 数；本文不预设未经测量的性能数字。

## 兼容性与回滚

项目处于开发阶段，本迭代允许修改公开布局返回类型和 `UnitVertices` 形状，不提供旧 tuple API 的兼容保证。

必须保留的既有行为：

- 普通文字使用当前 SDF atlas、fill、stroke 和 shadow 结果，包括 `FILL_THRESHOLD_BIAS`、`EDGE_SMOOTHING_HALF_WIDTH` 与 `fill_buffer`、`gamma` 的内缘语义；
- 字体选择、shaping、metrics、glyph bounds 和 SDF replay 使用同一个 Tiqian `FontFaceId`；
- `SegmentGlyphSpan` 继续公开且独立存在；
- 当前高度限制不会输出被截断行的普通文字；
- parser 错误尽力局部退化并继续布局。

若 SDF 基础模板无法在现有图集和 shader 路径中稳定表达某一图形，应先保留已经确认的公开布局语义，再回退或替换该图形的栅格化方式；不得因此删除对应布局结果或让 parser/Tiqian 数据重新 no-op。

本次归属修订改变了公开 API 形状：“调用方提供位图”的常驻模板入口与公开 `AtlasTemplate` 将被收回，`sdf` 模块改为内部模块。项目处于开发阶段，不提供兼容包装。

若分组身份无法从 Tiqian 携带的范围 `id` 稳定传到几何片段，应先补充事实再继续，不得退回按样式值猜测相邻片段连续性的方案。

## 风险与检查点

| 风险 | 处理方式 |
| --- | --- |
| cluster 与 glyph 混淆 | 先建立 positioned cluster 到 `row/col` 的映射，所有 glyph 和附属结果复用它。 |
| 连续范围按样式误合并 | 使用 Tiqian 携带的范围 `id` 作为分组身份，不比较 paint 值推断 authored range。 |
| 跨行误合并 | 分组的行键让每行各自成形，不会把两行接成一条。 |
| 逐字图案跳动 | 固定左端相位和节距，只裁剪当前右端。 |
| 背景内部重叠 | 先合并可见片段，再生成一次九宫格和 paint。 |
| atlas repeat 串色 | 不使用全局 repeat sampler；重复 quad 的 UV 留在 tile 内。 |
| 九宫格拼接痕迹 | 所有块共享目标边界，模板预留 padding 并扩展边缘 texel。 |
| shadow 被 tile 裁掉 | 模板尺寸同时考虑 spread、offset、blur 和 stroke。 |
| 索引容量 | 索引统一为 `u32`，并添加超过旧 `u16` 上限的回归测试。 |
| 图形几何外泄成调用方负担 | 模板与图形几何全部在库内完成，调用方只拿到顶点。 |
| SDF 阈值语义漂移 | 新增图形沿用 `FILL_THRESHOLD_BIAS`、`fill_buffer` 与 `gamma` 的内缘语义，不另建一套阈值；人工对比文字与图形的边缘一致。 |
| 对象范围扩大成资源系统 | 只输出键、语义和矩形；demo 仅提供最小固定示例。 |
| 查询能力偷渡进本迭代 | 不保存 source range、LayoutResult 或 replay index；只做链接和对象矩形 hit test。 |
| Huozi 重算 Tiqian 布局 | 所有位置从 Tiqian 最终结果读取，只做多段坐标平移和输出组织。 |

## 待讨论问题

以下问题的结论记录在对应条目上，实施时如需改变结论应先回写本节。

2026-09-24 校准修正了原稿中“公开类型”与“连续范围与来源”重复编号的问题，本节编号现在连续；原第 30 项已确定。标注“候选事实”的条目只补充可核对的现状或参考实现做法，不代表结论。

已确定：1、2、3、4、5、6、7、8、9、10、11、12、13、14、15、16、17、18、19、20、21、22、23、24、25、26、27、28、29、30、31、32、33。

2026-09-29 归属修订重新打开了第 19、20、21、22、25 项：原结论把模板生成与顶点输出划给调用方，与活字的职责不符。实现重做时又修订了第 1、2、6、9、11 项：图形在布局时定形并切段，每个片段自带顶点，公开类型不再携带绘制参数与分组身份。修订结论写在对应条目上，汇总见“图形顶点与分段”。其余条目不受影响。

### 公开类型

1. 已确定（2026-09-24；2026-09-29 修订）：`Background`、`Line`、`Decoration` 的字段形状如下。矩形复用 `tiqian::core::geometry::Rect`；`corner_radii`、`thickness`、`pattern` 与波浪参数直接放在变体字段上；绘制参数（颜色、描边、阴影）与图形几何在布局时换算成顶点，不再公开。
   - `Background { rect, corner_radii, vertices, row, col }`；
   - `Line { left, right, line_y, thickness, pattern, vertices, row, col }`；
   - `Decoration { kind, shape, thickness, wave, vertices, row, col }`；
   - `InlineObject { rect, id, alt, row, col }`。
2. 已确定（2026-09-24；2026-09-29 修订）：图形片段的分组身份只在库内使用，不出现在公开字段上；语义为 Tiqian 范围 `id` 的转写。
3. 已确定（2026-09-24）：四角半径直接保存在每个背景增量片段上，值来自该片段所属行的最终 segment。
4. 已确定（2026-09-24）：公开结果直接转写 Tiqian `RichTextLinePaint` 的 `thickness` 与 `pattern`，不使用 Tiqian 的完整状态拟合 helper。
5. 已确定（2026-09-24）：线宽与波浪参数由 Huozi 按该范围解析后的字号算成逻辑像素后公开：线宽 `fontSize / 16`（最小 1px），波周期 `0.4em`，振幅 `0.06em`。
6. 已确定（2026-09-24；2026-09-29 修订）：装饰片段只携带 Tiqian 决定的 `open_start` 与 `open_end`；逐字临时末端在生成绘制数据时按第 33 项的状态表闭合，公开布局结果不预测临时端点。
7. 已确定（2026-09-24）：对象绘制结果复用其 positioned cluster 的 occupied `Rect`，与 `InteractionArea` 同类型同语义；不另算 glyph ink、paint 外溢或对象边界调整，行尾悬挂与边界处理沿用 Tiqian 给出的该矩形。
8. 已确定（2026-09-24）：`width` 和 `height` 描述已输出的可见结果，不含 stroke、shadow 或 glyph ink 外溢。`width` 取各段实际输出宽度的最大值（`box_width` 未指定时取该段内容宽度）；`height` 取最后一个已输出行或空段行的 bottom，没有输出时为 0；两者只在最终数值上四舍五入成整数，舍入不参与逐行筛选和高度判断。

### 连续范围与来源

9. 已确定（2026-09-24；2026-09-29 修订）：分组身份使用 Tiqian 携带的 authored range 身份。Tiqian 为富文本范围增加可选 `id`，Huozi 输入适配在遍历范围时按顺序分配序号并写入该 `id`；逐 cluster 几何片段随该 `id` 返回，Huozi 用它把同一个范围的片段归为一组。示亡号、专名号、书名号所属的装饰范围需要同样的可选 `id`。该身份不进入公开类型。
10. 已确定（2026-09-24）：Huozi 输入适配把每个输入 Segment 的显示范围端点声明为 Tiqian `source_boundaries`，使 cluster 不跨 Segment；需要使用 tiqian-rs 的 ParagraphBuilder 边界声明入口（可合入同一迭代）。
11. 已确定（2026-09-24；2026-09-29 修订）：同一 authored range 在同一视觉行内只生成一个图形，再按排版单元切段。范围内因 autospace、标点 glue、行内对象占位或被跳过的 glyph 形成的几何空洞仍被覆盖，不断开；不同 range 或不同视觉行的片段不合并。
12. 跨行虚线、点线和波浪是否每行重新从左端开始相位，还是按 authored range 延续相位；无论选择哪种方式，单行逐字增长时相位必须固定。
13. 已确定（2026-09-24）：沿用第 9 项的结论，由 Tiqian 输入携带可选 `id`，由 Huozi 输入适配分配序号。parser lowering 与 `HuoziSourceMap` 不新增 scope ordinal；不得退回按 range 与 paint 值猜测嵌套或相邻范围。
14. 已确定（2026-09-24）：每项范围都在单个 Segment 内（标签不能跨 Segment），声明 Segment 边界后 cluster 也不跨 Segment，因此每个输出元素只属于一个 `SegmentId`，不需要“无归属”回退。背景与线条片段按所属 cluster 的 Segment 归属。
15. 已确定（2026-09-24）：由 Tiqian 输出逐 cluster 的最终富文本几何。它在现有逐行修剪、padding、圆角与 clearance 结果上按 positioned cluster 细分，覆盖背景（含行内代码背景）、下划线、删除线以及示亡号、专名号、书名号；每个片段携带自己的范围 `id` 与最终几何。Tiqian 继续是修剪、padding、圆角、起点标点 glue、clearance 与纵向度量的唯一计算方，Huozi 只做转写。具体 API 形态由 tiqian-rs 自己的迭代文档确定。

### 逐字顺序与注音

16. 已确定（2026-09-24）：无绘制 glyph 的 cluster（空格、零宽字符、Tiqian 合成 cluster、缺 ink bounds 或缺可回放字体身份）仍占一个 `col`（现状行为）；调用方推进到这类 col 时只前进、不绘制。行内对象所在 cluster 同样占一个 `col`，其绘制结果由 `InlineObject` 变体表达。
17. 已确定（2026-09-24）：行尾连字符沿用本行最后一个 positioned cluster 的 `col`（现状行为），即使该 cluster 没有可绘制 glyph。
18. 已确定（2026-09-24）：注音 glyph 按 `base_range` 与 positioned cluster 的交集定位，取交集内的基文 cluster；无法唯一对应单个 cluster 时用基文范围最后一个 cluster；调号与轻声点共用所属基文 cluster 的 `col`；基文跨行时按行拆分。

### 形状模板与绘制数据

19. 已确定（2026-09-24；2026-09-29 修订归属）：模板在 Huozi 内部按需生成一次并分配进图集；文本变化不重新生成。生成时机由库决定，不由调用方触发。
20. 已确定（2026-09-29 修订）：模板与字体 glyph 共用同一个 RGBA 图集，不建立第二张纹理。原文要求的“调用方提供位图”的公开入口（`allocate_resident_template`）是错误归属的产物，予以收回；模板位图生成、图集分配与常驻管理全部在库内完成。
21. 已确定（2026-09-29 修订）：模板基准尺寸、过采样倍率、四边距离裕量和 SDF spread 都是 Huozi 的内部数值决策，按视觉样例校准并记录在持续维护文档中；不进入公开 API，也不要求调用方选值。
22. 已确定（2026-09-24；2026-09-29 修订）：示亡号使用专用边框九宫格模板，形状盒是可带左右竖边的矩形框，四角为完整 L 形角块；不复用直角矩形边角模板（原结论），因四条线段拼接会在四角重叠或多出可见小段。
23. 已确定（2026-09-24）：点线圆点直径取 line `thickness`；末端只提交完整圆点，不裁剪半个圆点。
24. 已确定（2026-09-24）：shader 只在模板呈现确有需要时做最小扩展；不引入 tile-local repeat。
25. 已确定（2026-09-24；2026-09-29 修订）：索引统一使用 `u32`；顶点与索引都由调用方在提交前拼一遍，库不提供额外的绘制数据入口。
26. 已确定（2026-09-24）：只用一个 vertex buffer 和一个 index buffer，按 PaintLayer 建立 draw range；不为单个元素或图形创建 buffer。
27. 已确定（2026-09-24）：demo 不为行内对象绘制任何内容，只保留布局占位；`InlineObject` 变体仍被遍历消费，命中区域照常参与点击提示。
28. 已确定（2026-09-24）：egui 配置面板最上方增加一个占满宽度的 `(row, col)` 推进滑条；本迭代不实现自动播放。

### 多段、空段与高度

29. 已确定（2026-09-24）：parser 保留的每个 `ParsedParagraph` 都占位，空段各占一行，不做首尾特判。
   - `A`、`A[br /]B`、`[br /]A`、`A[br /][br /]B` 分别输出 1、2、2、3 行；`A[br /]` 与单独的 `[br /]` 各占 2 行。
30. 已确定（2026-09-24）：空段行高优先取该段 `ParagraphStyleOverride.line_height`（绝对像素），没有覆盖时取 `initial_text_style.font_size * layout_style.line_height`。
31. 已确定（2026-09-24）：每段开始前用 `box_height` 减已输出高度作为该段可用高度；段内遇到第一行无法完整容纳即停止整篇输出，不从更后的段落恢复；空段无法完整容纳时停止且不增加 `row` 与文档高度；`width` 与 `height` 只在最终输出时四舍五入。

### Tiqian 依赖事实

32. 已确定（2026-09-24 校准）：`Cargo.toml` 使用 `tiqian = "0.5"`，工作区中的 `tiqian-rs` 就是该 crate 的 0.5.0 源码，本文列出的查询和 debug 数据都已在该版本中存在。实施时仍应在第一次编译通过后确认签名与本文一致。

### 示亡号状态

33. 在编码前补充单行完整范围、跨行首行、中间行、末行，以及逐字前缀位于完整源范围中间或末端时的边闭合状态表。状态表必须明确 Tiqian `open_start/open_end` 与临时可见末端的优先级。
   - 候选事实：参考实现只对 `ProperNoun`、`BookTitle` 画中心线，对示亡号画上下边，并在 `!open_start`、`!open_end` 时画竖直边；逐字临时末端不在 Tiqian 数据中，必须由本文规定。

## 实施结果

### 首次实施（2026-09-26）

当时的判断是“公开输出与官方 demo 呈现均已完成”。经 2026-09-29 归属复核，布局输出部分成立，图形呈现部分的实现位置错了，见“归属修订”。下面分列两部分。

#### 已完成且本次修订不改动

| 范围 | 交付内容 |
| --- | --- |
| 公开类型 | `UnitVertices` 改为具名 enum（`Text`、`Background`、`Line`、`Decoration`、`InlineObject`），新增 `TextRole`、`ShapeVertices`、`DecorationShape`、`DecorationWave`。 |
| 逐字显示 | 全部绘制元素按逐字显示顺序输出，取前缀即为打印状态（见“顺序保证”）。 |
| 来源 span | `SegmentGlyphSpan` 覆盖该 segment 产生的全部绘制元素；不连续使用多个 span。 |
| 连续范围 | 图形片段的分组身份来自 Tiqian 层身份（Huozi 输入适配按遇见的顺序分配）；身份不对外公开。 |
| 多段 | 四个布局入口布局全部段落，累加纵向坐标与视觉行号；空段占一行高度。 |
| 注音 | ruby 与 bopomofo 重放 Tiqian 已 shaping 的 glyph，`row`、`col` 跟随基文 cluster。 |

#### 重做（已于 2026-09-29 完成）

| 范围 | 处理方式 |
| --- | --- |
| 常驻模板 | `Huozi::allocate_resident_template` 与公开 `AtlasTemplate` 已收回；模板由 `src/shape/` 内部生成。 |
| 模板与图元 | `examples/render/shapes.rs` 的模板生成与几何构造移入 `src/shape/`，改为 `Huozi` 的内部能力。 |
| 图形定点 | 同一个 authored range 的片段在布局时定形成完整图形，再按排版单元切成段；每段自带三层顶点，`continuity_id` 不再公开。 |
| demo | 两个文件已删除；demo 取可见前缀、按绘制层拼顶点、上传纹理，一次 `draw_indexed`。 |

### 与本文原文不同的实现决定

以下决定是首次实施中形成的技术结论，重做后全部保留。

1. **顺序保证（新增）。** 本文原文只规定 `row`、`col` 与“主序列顺序”。实现改为由库在适配阶段把输出排成逐字显示顺序：`(row, col)` 为主序，同一排版单元内按绘制层排列（背景 → 正文文字 → 线条与装饰 → 注音 → 行内对象）。这样调用方取 `glyphs[..end]` 就是“打印到第 `end` 个元素”的完整可见状态，不需要自己重排；绘制提交仍按固定层序进行。原文第 13 项“不增加 sequence 字段”不变。
2. **逐字滑条（改变第 28 项）。** 原文是面板最上方的 `(row, col)` 推进滑条；实现改为同位置的一条**占满宽度**滑条，其值是一个 `glyphs` 的结束下标（0 到元素总数，最右端即显示全部）。逐位置推进无法表达“同一个字先出背景再出文字”，也做不到逐元素打字机效果。
3. **模板分辨率（改变第 21 项）。** 原文取 `128px` 基准栅格；实现改为按 `SDF_SCALE = 4` 过采样（一逻辑像素对应 4 个模板 texel）。若按 1 texel = 1 逻辑像素栅格化，1 像素宽的描边只落在约一个 texel 上，弧线处的描边会被采样成断续的直边。
4. **示亡号（改变第 22 项）。** 原文“复用直角矩形边角模板”；实现为专用**边框九宫格**模板：形状盒是一个可带左右竖边的矩形框，四角为完整 L 形角块。角块深度是「线宽 + 空腔侧深度」，使空腔边界上的描边不被几何边缘切掉；若用四条线段拼接，四角会重叠或多出可见小段。
5. **背景圆角。** 模板按**整数半径**生成，绘制时取同半径模板；取最近模板会把多个半径折叠到同一形状，使圆角跳变，并让弧落进被拉伸的中块而拉成斜边。模板改为按需生成，不再预先烤出全部半径。
6. **线段端点语义。** 实线用三宫格（两端 1:1 端帽、中段拉伸），描边与阴影才有外扩空间；虚线的每个 dash 用裁剪端（几何正好落在范围内），否则相邻 dash 的描边会在间隙里相接，虚线在视觉上并成实线。
7. **波浪线端点。** 周期模板没有端点，两端用与切线对齐的线带端帽闭合：形状中心落在端面上（只向内侧延伸的短端帽会被抗锯齿过渡带冲淡，端点会读成断开），轴线方向等于该处切线。
8. **图形抗锯齿。** 图形顶点不叠加额外平滑半宽，只保留阴影 `blur`；图形是解析距离场且已过采样，梯度项给出的半宽已足够，叠加固定半宽会把斜坡推过形状端点、留下可见短段。文字路径仍使用 `EDGE_SMOOTHING_HALF_WIDTH`。
9. **依赖版本（更新第 32 项）。** `Cargo.toml` 使用 `tiqian = "0.6"`（原文记录为 0.5），并使用工作区内的 `tiqian-rs` 作为 `[patch.crates-io]`。
10. **进度路径。** 滑条只改变可见前缀，因此只重建绘制数据并复用上一次布局结果（`progress_changed` 路径）；其余配置改动才重新排版。
11. **shader 未改动。** 图形复用现有 `Vertex` 字段与阈值语义，`shader.wgsl` 保持原样。

### 测试命令与验证结果

重做后（2026-09-29）：

```text
cargo test --all-targets              # 库 82、富文本输出 20、集成 9+1
cargo check --all-targets
cargo check --all-targets --features woff
cargo run --example render --release --features woff
```

首次实施时的记录：

```text
cargo test                  # 库 64、富文本输出 20、集成 9+1、demo 17
```

结构化验证（无需 WGPU）：

| 验证目标 | 证据 |
| --- | --- |
| 逐字显示顺序 | `glyphs_are_ordered_by_display_sequence`：位置非递减、同位置按绘制层、文字出现前背景必定已出现。 |
| cluster 级 `row`/`col` | `col_advances_per_positioned_cluster`、`background_fragments_share_the_cluster_position`。 |
| 连续范围 | 同一个 authored range 的各段拼成完整图形；相邻范围不合并，嵌套范围保持独立；分段不切掉整段首尾的外扩。 |
| 前缀合并与固定相位 | `visible_prefix_only_extends_the_merged_background`、`dashed_line_keeps_its_phase_while_the_prefix_grows`。 |
| 来源 span 覆盖与不连续 | `segment_glyph_spans_cover_all_draw_elements`、`one_segment_id_appearing_twice_gets_two_spans`、`a_segment_that_produces_no_elements_gets_no_span`。 |
| 多段与空段 | `multiple_paragraphs_accumulate_rows_and_offsets`、`consecutive_breaks_produce_a_blank_row`。 |
| 高度限制 | `output_stops_at_the_first_line_that_does_not_fit`、`tiqian_output_drops_a_line_that_exceeds_box_height`。 |
| 注音与对象 | `ruby_and_bopomofo_follow_their_base_cluster`、`inline_object_takes_one_col_and_carries_its_key`、`object_without_id_still_lays_out`。 |
| 模板与图元几何 | `each_radius_gets_its_own_template`、`background_corner_block_keeps_the_arc_unstretched`、`shape_templates_oversample_the_sdf`、`shape_margin_covers_the_paint`、`band_template_closes_its_ends`、`mourning_frame_uses_eight_nine_slice_blocks`、`wave_ends_use_tangent_aligned_caps`、`line_ends_follow_the_fragment_semantics`、`atlas_bitmap_is_centered_in_its_grid_block`。位于 `src/shape/tests.rs` 与 `src/huozi.rs`。 |
| 容量 | `u32_indices_cover_more_than_the_u16_limit`：超过 65535 顶点时索引不溢出。 |

人工视觉验收已在官方 demo 上完成：背景（跨 cluster、跨行、嵌套、相邻、描边与阴影）、四种线型、注音、四种 CLREQ 装饰、跨行链接、行内对象占位、多段与空段、逐字滑条均按预期呈现。图形部分的实现移入库后需重新执行一遍。

### 未包含（保持不变）

- 文字光标、选择、复制与无障碍查询；
- 竖排、完整 bidi/RTL、分页、多栏；
- 通用行内对象资源管理与加载器；
- WGPU 之外的渲染后端绘制器。

## 文档更新

| 文件 | 更新内容 |
| --- | --- |
| `docs/architecture.md` | 公开布局结果、Tiqian 输出适配、多段组合、输出顺序与逐字显示、图形顶点与分段、renderer 边界。 |
| `docs/api-design.md` | `RichTextLayoutOutput`、`UnitVertices` enum 与各变体、`ShapeVertices`、`SegmentGlyphSpan`、链接和对象 API。 |
| `docs/parser.md` | `[link id=...]` 与 `[object id=...]` 的可选键、多段布局与渲染边界。 |
| `docs/rendering-gap-analysis.md` | 已完成的富文本呈现能力与延期项。 |
| 本文 | 归属修订、待讨论问题结论与实施结果。 |

持续维护文档只描述已实现的现状。上表已于 2026-09-29 随重做同步。

## 验证要求

完成 Phase 3、4 的重做后必须运行：

```text
cargo test
cargo check --all-targets
cargo check --all-targets --features woff
cargo run --example render --release
cargo run --example render --release --features woff
git diff --check
```

并完成：

- 对本迭代修改的中文文档运行项目采用的文档措辞检查；
- 检查编辑器诊断；
- 不依赖 WGPU 的结构化测试证明顶点几何、拼接边界、相位稳定与分层索引；
- 人工运行官方 demo，完成本文“人工视觉验收”中的逐项检查；
- 检查默认和 `woff` 字体配置下的文字、注音与图形组合；
- 用超过旧 `u16` 顶点容量的受控输入验证索引方案；
- 审阅 `git status` 和目标 diff，确认没有修改无关文件或覆盖并行工作。

首次实施已经运行过的命令与结果见“实施结果”的“测试命令与验证结果”；重做后需重新执行。

## 相关文件

| 文件 | 本迭代职责 |
| --- | --- |
| `src/parser/parsed_text.rs` | 为 `InlineObject` 保存稳定 `id`。（已完成） |
| `src/parser/elements_to_document.rs` | 解析和验证 `[object id=...]`。（已完成） |
| `src/glyph_vertices.rs` | 定义扩展后的 `UnitVertices`、文字角色和非文字布局结果。（已完成；需同步文档注释中“由调用方按自己的渲染方式生成图元”的旧归属描述） |
| `src/layout.rs` | 统一公开返回类型，组合全部段落和空段。（已完成） |
| `src/layout/layout_output.rs` | 已公开的 `RichTextLayoutOutput`、`Interaction`、`InteractionArea`。（已完成） |
| `src/layout/layout_style.rs` | `LayoutStyle`（含 `align`）；多段组合按段使用其覆盖。 |
| `src/layout/glyph_span.rs` | 保留独立 `SegmentGlyphSpan` 及其扩展范围语义。（已完成） |
| `src/layout/tiqian_input.rs` | 保持 parser scope、来源映射和 Tiqian 输入之间的身份信息，供 continuity/source 关联使用。（已完成） |
| `src/layout/tiqian_output.rs` | 转换正文、注音、背景、线条、装饰、对象、链接、row/col、continuity、来源 span 与显示顺序。（已完成） |
| `src/layout/vertex.rs` | 文字与图形共用的顶点格式；`Vertex::desc()` 在 `wgpu` feature 下可用。（已完成） |
| `src/shape/`（新增） | 形状模板生成与缓存、图形定形与切段、图元构造。（已完成） |
| `src/huozi.rs` | 持有形状模板与图集写入的共用入口；模板分配改为内部。（已完成） |
| `examples/render/main.rs` | 消费统一结果、逐字筛选、上传纹理、一次提交绘制与对象占位。（已完成） |
| `examples/render/ui.rs` | 逐字进度滑条放在配置面板最上方，占满宽度，只触发绘制数据重建。（已完成） |
| `examples/render/shader.wgsl` | 保留现有 SDF 语义。（未改动） |
| `docs/architecture.md` | 持续维护的稳定架构和模块边界。（已完成） |
| `docs/api-design.md` | 公开类型与调用方式。（已完成） |
| `docs/parser.md` | 对象与链接标签的用户语法。（已完成） |
| `docs/rendering-gap-analysis.md` | 富文本最终呈现能力状态。（已完成） |
