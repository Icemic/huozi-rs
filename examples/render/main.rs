use egui::epaint::text::{FontInsert, InsertFontFamily};
use huozi::{
    FontSourceKind, Huozi,
    constant::TEXTURE_SIZE,
    glyph_vertices::UnitVertices,
    layout::{ColorSpace, Interaction, LayoutStyle, RichTextLayoutOutput, Vertex},
    parser::{Segment, TextStyle},
};
use log::{error, info};
use std::{
    iter,
    sync::Arc,
    time::{Duration, Instant},
};
use wgpu::{BlendState, util::DeviceExt};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::{
    defaults::text_style_default,
    fonts::{FontFile, load_fonts},
    mvp::MVPUniform,
    ui::render_control_panel_ui,
};

mod defaults;
mod fonts;
mod mvp;
mod texture;
mod ui;

/// 逐字显示进度：`glyphs` 的结束下标，`None` 表示显示全部。
type Progress = Option<usize>;

/// 固定的绘制层次序。
///
/// 层的先后决定半透明重叠、描边与阴影是否正确：背景在文字之下，装饰在文字之上。文字与图形都按
/// 这套层次序提交，区别只在各自有哪些层。
#[derive(Clone, Copy, PartialEq, Eq)]
enum DrawLayer {
    BackgroundShadow,
    BackgroundStroke,
    BackgroundFill,
    TextShadow,
    TextStroke,
    TextFill,
    Decoration,
}

/// 按绘制层顺序把可见元素拼成一份顶点与索引缓冲。
///
/// 外层遍历层、内层遍历元素，因此同一层内保持逐字顺序，层与层之间保持固定的先后。文字与图形走
/// 同一条路径，区别只是 [`push_layer`] 里各自有哪些层。
fn assemble(visible: &[UnitVertices]) -> (Vec<Vertex>, Vec<u32>) {
    const DRAW_LAYERS: [DrawLayer; 7] = [
        DrawLayer::BackgroundShadow,
        DrawLayer::BackgroundStroke,
        DrawLayer::BackgroundFill,
        DrawLayer::TextShadow,
        DrawLayer::TextStroke,
        DrawLayer::TextFill,
        DrawLayer::Decoration,
    ];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for layer in DRAW_LAYERS {
        for element in visible {
            push_layer(&mut vertices, &mut indices, element, layer);
        }
    }
    (vertices, indices)
}

/// 追加一个元素在某一层的顶点；该元素没有这一层时什么都不做。
fn push_layer(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    element: &UnitVertices,
    layer: DrawLayer,
) {
    match (element, layer) {
        (UnitVertices::Background(background), DrawLayer::BackgroundShadow) => {
            push_quads(vertices, indices, &background.vertices.shadow);
        }
        (UnitVertices::Background(background), DrawLayer::BackgroundStroke) => {
            push_quads(vertices, indices, &background.vertices.stroke);
        }
        (UnitVertices::Background(background), DrawLayer::BackgroundFill) => {
            push_quads(vertices, indices, &background.vertices.fill);
        }
        (UnitVertices::Text(text), DrawLayer::TextShadow) => {
            push_quads(
                vertices,
                indices,
                text.shadow.as_ref().map_or(&[], |quad| &quad[..]),
            );
        }
        (UnitVertices::Text(text), DrawLayer::TextStroke) => {
            push_quads(
                vertices,
                indices,
                text.stroke.as_ref().map_or(&[], |quad| &quad[..]),
            );
        }
        (UnitVertices::Text(text), DrawLayer::TextFill) => {
            push_quads(vertices, indices, &text.fill);
        }
        // 线条与装饰共处一层，元素内部按阴影、描边、填充。
        (UnitVertices::Line(line), DrawLayer::Decoration) => {
            push_quads(vertices, indices, &line.vertices.shadow);
            push_quads(vertices, indices, &line.vertices.stroke);
            push_quads(vertices, indices, &line.vertices.fill);
        }
        (UnitVertices::Decoration(decoration), DrawLayer::Decoration) => {
            push_quads(vertices, indices, &decoration.vertices.shadow);
            push_quads(vertices, indices, &decoration.vertices.stroke);
            push_quads(vertices, indices, &decoration.vertices.fill);
        }
        // 行内对象由调用方按自己的资源绘制，Huozi 只给出位置。
        _ => {}
    }
}

/// 追加一串四边形；每 4 个连续顶点构成一个，索引按逆时针展开。
fn push_quads(vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>, quads: &[Vertex]) {
    let base = vertices.len() as u32;
    vertices.extend_from_slice(quads);
    for quad in 0..(quads.len() / 4) as u32 {
        let offset = base + quad * 4;
        indices.extend([
            offset,
            offset + 1,
            offset + 2,
            offset,
            offset + 2,
            offset + 3,
        ]);
    }
}

const DEFAULT_TEXT: &str = r##"一个功能完善的中日韩文字排印引擎，为[shadow offsetX=1.5 offsetY=1.5 blur=0 width=0.4 color="rgba(255, 64, 153, 1.0)"]游戏富文本[/shadow]特别设计。[link id="demo-interaction" target="https://example.com/interaction"]点击这里显示交互提示[/link]
A fully functional typography engine for CJK languages, especially designed for game rich-text.
huózì 活字 gM 123.!""?;:-_/+=<>==
CJK 标点——⸺，。：；“”？、《》「」【】
中文[color=yellow]“[/color]引号[color=yellow]”[/color]与western [color=yellow]“[/color]quote[color=yellow]”[/color]虽然是同一个字符，但需要渲染成不同的样子。
[locale=zh-hans]骨直肩示[/locale] [locale=zh-hant]骨直肩示[/locale] [locale=zh-hk]骨直肩示[/locale] [locale=ja-jp]骨直肩示[/locale] [locale=ko-kr]骨直肩示[/locale]
[font="思源黑体 VF"]思源黑体[/font] / [font="Source Han Serif VF"]思源宋体[/font] ｜ [font="思源黑体 VF"][weight=400]常规[/weight] / [bold]粗体[/bold][/font] ｜ [font="Inter Variable"]Inter Normal / [italic]Inter Italic[/italic][/font]
[font="獅尾圓體SC"][weight=400]常规ABCxyz[/weight] / [bold]仿粗体ABCxyz[/bold] / [italic]仿斜体ABCxyz[/italic] / [bold][italic]仿粗斜体ABCxyz[/italic][/bold][/font]
[background color="#FFF3BF" paddingX=6 paddingY=2 radius=8]跨多个字格与行内空隙的圆角背景[/background]与[background color="rgba(56, 189, 248, 0.6)" paddingX=4 paddingY=2 radius=8 strokeColor="#0EA5E9" strokeWidth=1]相邻的另一个背景[/background]。
[background color="#1E293B" paddingX=6 paddingY=2 radius=8 shadowColor="#000000" shadowOffsetX=2 shadowOffsetY=2 shadowBlur=2 shadowWidth=1]带描边与阴影的背景[/background]与[code paddingX=4 paddingY=2 radius=8]cargo test[/code]共用同一套图形。
[underline color="#1677FF" thickness=1]实线下划线[/underline] ⁄ [underline color="#1677FF" thickness=1 pattern=dashed dashLength=3 gapLength=2]虚线下划线[/underline] ⁄ [underline color="#1677FF" thickness=1 pattern=dotted gapLength=2]点线下划线[/underline] ⁄ [lineThrough color="#94A3B8" thickness=1]删除线[/lineThrough]
[ruby text="tí qiàn"]提椠[/ruby]与[bopomofo text="ㄊㄧˊ"]提[/bopomofo][bopomofo text="ㄑㄧㄢˋ"]椠[/bopomofo]把注音也画出来。
[emphasis]着重号[/emphasis]、[mourning]示亡号[/mourning]、[properNoun]专名号[/properNoun]与[bookTitle]书名号[/bookTitle]各自使用 Tiqian 的最终几何。
[link id="demo-rich-link" target="https://example.com/rich"]带背景的跨行链接会在这里换行显示，背景、线条与链接区域都按排版单元推进[/link]，[object id="demo-rich-object" alt="图标" width=24 ascent=18 descent=6 /]对象在这里占位。
第一段结束。
[br /][br /]空段之后是新的一段，逐字进度会跨过空段。
"##;

const DEFAULT_FONT_FALLBACKS: [(&str, FontSourceKind); 5] = [
    ("InterVariable.ttf", FontSourceKind::Latin),
    ("InterVariable-Italic.ttf", FontSourceKind::Latin),
    ("SourceHanSansSC-VF.otf", FontSourceKind::Cjk),
    ("SourceHanSerif-VF.otf.woff2", FontSourceKind::Cjk),
    ("SweiGothicCJKsc-Regular.ttf", FontSourceKind::Cjk),
];

#[cfg(target_os = "windows")]
const REDRAW_DELAY: Duration = Duration::from_millis(1);

#[cfg(not(target_os = "windows"))]
const REDRAW_DELAY: Duration = Duration::ZERO;

struct FontFallback {
    name: String,
    kind: Option<FontSourceKind>,
    enabled: bool,
}

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    mvp_buffer: wgpu::Buffer,
    mvp_bind_group: wgpu::BindGroup,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    num_indices: Option<u32>,
    // NEW!
    #[allow(dead_code)]
    texture: texture::Texture,
    texture_bind_group: wgpu::BindGroup,

    font_files: Vec<FontFile>,
    font_fallbacks: Vec<FontFallback>,
    font_to_add: Option<String>,
    huozi: Option<Huozi>,

    // egui integration
    egui_context: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,

    // Text input
    input_text: String,

    // Layout and style configuration
    background_color: wgpu::Color,
    layout_config: LayoutStyle,
    text_config: TextStyle,
    stroke_enabled: bool,
    shadow_enabled: bool,
    config_changed: bool,
    /// 逐字进度发生变化；只需要重建顶点缓冲，不需要重新排版。
    progress_changed: bool,
    interactions: Vec<Interaction>,
    cursor_position: Option<(f32, f32)>,
    interaction_notice: Option<String>,
    /// 逐字显示进度：`glyphs` 的结束下标；`None` 表示显示全部。
    progress: Progress,
    /// 上一次布局输出的绘制元素总数，供逐字滑条使用。
    element_count: usize,
    /// 上次完整布局的结果；逐字进度变化时复用它重建缓冲，避免重复排版。
    layout_output: Option<RichTextLayoutOutput>,

    // Store egui render data
    egui_paint_jobs: Vec<egui::ClippedPrimitive>,
    egui_textures_delta: egui::TexturesDelta,
}

enum RenderOutcome {
    Success,
    Suboptimal,
    Timeout,
    Occluded,
    Outdated,
    Lost,
    Validation,
}

impl State {
    async fn new(window: &Arc<Window>) -> Self {
        let size = window.inner_size();
        let font_files = load_fonts().expect("failed to read ./resources/fonts");
        assert!(
            !font_files.is_empty(),
            "no font files found in ./resources/fonts"
        );

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let mut instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_descriptor.backends = wgpu::Backends::all();
        instance_descriptor.backend_options.dx12.shader_compiler = wgpu::Dx12Compiler::Fxc;
        let instance = wgpu::Instance::new(instance_descriptor);
        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface.");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let caps = surface.get_capabilities(&adapter);
        let format = *caps
            .formats
            .iter()
            .find(|f| !f.is_srgb())
            .expect("Cannot find a proper surface format.");

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        let texture = texture::Texture::empty(
            &device,
            TEXTURE_SIZE,
            TEXTURE_SIZE,
            Some("sdf texture"),
            Some(wgpu::TextureFormat::Rgba8Unorm),
        );

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        let logical_size = size.to_logical(window.scale_factor());
        let mvp_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::bytes_of(&MVPUniform {
                width: logical_size.width,
                height: logical_size.height,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let mvp_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<MVPUniform>() as u64
                        ),
                    },
                    count: None,
                }],
            });

        let mvp_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &mvp_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: mvp_buffer.as_entire_binding(),
            }],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&mvp_bind_group_layout),
                    Some(&texture_bind_group_layout),
                ],
                immediate_size: 0,
            });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::desc())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        // Initialize egui
        let egui_context = egui::Context::default();
        let proportional_font = font_files
            .iter()
            .find(|font| {
                font.name
                    .eq_ignore_ascii_case("SweiGothicCJKsc-Regular.ttf")
            })
            .unwrap_or(&font_files[0]);
        egui_context.add_font(FontInsert::new(
            "custom_font",
            egui::FontData::from_owned(proportional_font.data.clone()),
            vec![InsertFontFamily {
                family: egui::FontFamily::Proportional,
                // use lowest priority to avoid overriding other fonts
                priority: egui::epaint::text::FontPriority::Highest,
            }],
        ));
        egui_context.global_style_mut(|style| {
            style.text_styles.insert(
                egui::TextStyle::Name("custom_font".into()),
                egui::FontId::new(16.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(14.0, egui::FontFamily::Proportional),
            );
        });
        let egui_state = egui_winit::State::new(
            egui_context.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            config.format,
            egui_wgpu::RendererOptions {
                msaa_samples: 1,
                ..Default::default()
            },
        );

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            mvp_buffer,
            mvp_bind_group,
            vertex_buffer: None,
            index_buffer: None,
            num_indices: None,
            texture,
            texture_bind_group,
            font_fallbacks: DEFAULT_FONT_FALLBACKS
                .iter()
                .filter_map(|(name, kind)| {
                    font_files
                        .iter()
                        .find(|font| font.name.eq_ignore_ascii_case(name))
                        .map(|font| (font, *kind))
                })
                .map(|(font, kind)| FontFallback {
                    name: font.name.clone(),
                    kind: Some(kind),
                    enabled: true,
                })
                .collect(),
            font_files,
            font_to_add: None,
            huozi: None,
            egui_context,
            egui_state,
            egui_renderer,
            input_text: DEFAULT_TEXT.to_string(),
            background_color: wgpu::Color {
                r: 0.160,
                g: 0.160,
                b: 0.160,
                a: 1.0,
            },
            layout_config: LayoutStyle {
                box_width: Some(1280.),
                box_height: Some(800.),
                line_height: 1.5,
                indent: 0.,
                ..LayoutStyle::default()
            },
            text_config: text_style_default(),
            stroke_enabled: true,
            shadow_enabled: false,
            config_changed: false,
            progress_changed: false,
            interactions: Vec::new(),
            cursor_position: None,
            interaction_notice: None,
            progress: None,
            element_count: 0,
            layout_output: None,
            egui_paint_jobs: Vec::new(),
            egui_textures_delta: Default::default(),
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn update(&mut self, window: &Window) {
        // Reset config changed flag
        self.config_changed = false;

        // Build egui UI
        let mut full_output = render_control_panel_ui(self, window);

        // Mark config as changed if there was any UI interaction in config panels
        if full_output.platform_output.events.iter().any(|_| true) {
            self.config_changed = true;
        }

        if let Some(ime) = full_output.platform_output.ime.as_mut() {
            // egui-winit uses `ime.rect` for the IME cursor area, but for Windows IMEs we want
            // the caret rectangle instead of the whole text box.
            ime.rect = ime.cursor_rect;
        }

        self.egui_state
            .handle_platform_output(window, full_output.platform_output);

        // Store textures delta and paint jobs for rendering
        self.egui_textures_delta = full_output.textures_delta;
        self.egui_paint_jobs = self
            .egui_context
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        // 逐字进度不改变布局，只改变可见前缀，因此走重建缓冲的路径；其余改动重新排版。
        if std::mem::take(&mut self.progress_changed) {
            self.rebuild_buffers();
        } else if self.config_changed {
            self.render_huozi_text();
        }
    }

    fn render_huozi_text(&mut self) {
        let started_at = Instant::now();

        if self.huozi.is_none() {
            let enabled_fonts = self
                .font_fallbacks
                .iter()
                .filter(|font| font.enabled)
                .collect::<Vec<_>>();
            if enabled_fonts.is_empty() {
                self.layout_output = None;
                self.vertex_buffer = None;
                self.index_buffer = None;
                self.num_indices = None;
                self.interactions.clear();
                return;
            }

            info!(
                "load font fallbacks: {:?}",
                enabled_fonts
                    .iter()
                    .map(|font| &font.name)
                    .collect::<Vec<_>>()
            );
            // initialize huozi instance
            let t = Instant::now();

            let font_sources = enabled_fonts
                .iter()
                .map(|font_fallback| {
                    let font = self
                        .font_files
                        .iter()
                        .find(|font| font.name == font_fallback.name)
                        .expect("font fallback sequence contains an unknown font file");
                    let source = huozi::FontSource::new(font.data.clone());
                    match font_fallback.kind {
                        Some(kind) => source.with_kind(kind),
                        None => source,
                    }
                })
                .collect();

            info!("font files loaded, {:?}", t.elapsed());

            let huozi =
                huozi::Huozi::new(font_sources).expect("Failed to initialize Huozi font manager");
            self.huozi = Some(huozi);
        }

        let Some(huozi) = self.huozi.as_mut() else {
            error!("Huozi instance is not initialized");
            return;
        };

        match huozi.layout_parse(
            &vec![Segment::dummy(&self.input_text)],
            &self.layout_config,
            &self.text_config,
            ColorSpace::SRGB,
            None,
        ) {
            Ok(mut output) => {
                info!("text layouting finished, {:?}", started_at.elapsed(),);

                info!(
                    "total_width: {}, total_height: {}",
                    output.width, output.height
                );

                let texture = huozi.texture_pixels();
                self.texture.write_pixels(
                    &self.queue,
                    texture.pixels(),
                    texture.width(),
                    texture.height(),
                );

                self.interactions = std::mem::take(&mut output.interactions);
                self.layout_output = Some(output);
                self.apply_layout_output();
            }
            Err(err_msg) => {
                self.layout_output = None;
                self.vertex_buffer = None;
                self.index_buffer = None;
                self.num_indices = None;
                self.interactions.clear();
                error!("{}", err_msg);
            }
        }
    }

    /// 用缓存的布局结果重建顶点与索引缓冲，不重新布局、不重传图集纹理。
    ///
    /// 逐字进度只改变哪些元素可见，不影响 shaping、断行与图集内容；若走完整布局路径，每移动一次
    /// 滑条都会重跑一次排版（约数百毫秒）。
    fn rebuild_buffers(&mut self) {
        if self.layout_output.is_none() {
            // 还没有布局结果时（例如字体刚被禁用）退回完整路径，由它处理建实例与报错。
            self.render_huozi_text();
            return;
        }
        self.apply_layout_output();
    }

    /// 根据当前进度把缓存的布局结果组装成缓冲。
    fn apply_layout_output(&mut self) {
        let Some(output) = self.layout_output.as_ref() else {
            error!("Huozi instance is not initialized");
            return;
        };

        // 逐字进度超出文档时回到完整显示，避免文本变短后卡在空进度。
        let total = output.glyphs.len();
        self.element_count = total;
        if self.progress.is_some_and(|count| count >= total) {
            self.progress = None;
        }
        let end = self.progress.unwrap_or(total);

        let (vertices, indices) = assemble(&output.glyphs[..end]);
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
        self.num_indices = Some(indices.len() as u32);
    }

    fn render(&mut self) -> RenderOutcome {
        let (output, needs_reconfigure) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(output) => (output, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(output) => (output, true),
            wgpu::CurrentSurfaceTexture::Timeout => return RenderOutcome::Timeout,
            wgpu::CurrentSurfaceTexture::Occluded => return RenderOutcome::Occluded,
            wgpu::CurrentSurfaceTexture::Outdated => return RenderOutcome::Outdated,
            wgpu::CurrentSurfaceTexture::Lost => return RenderOutcome::Lost,
            wgpu::CurrentSurfaceTexture::Validation => return RenderOutcome::Validation,
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        if self.vertex_buffer.is_some() {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.background_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            let vertex_buffer = self.vertex_buffer.as_ref().unwrap();
            let index_buffer = self.index_buffer.as_ref().unwrap();
            let num_indices = self.num_indices.unwrap();

            if self.vertex_buffer.as_ref().unwrap().size() > 0 {
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.mvp_bind_group, &[]);
                render_pass.set_bind_group(1, &self.texture_bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..num_indices, 0, 0..1);
            }
        }

        // Render egui (only if there are paint jobs to render)
        if !self.egui_paint_jobs.is_empty() {
            // Update textures
            for (id, image_deltas) in &self.egui_textures_delta.set {
                for image_delta in image_deltas {
                    self.egui_renderer
                        .update_texture(&self.device, &self.queue, *id, image_delta);
                }
            }

            let screen_descriptor = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [self.config.width, self.config.height],
                pixels_per_point: self.egui_context.pixels_per_point(),
            };

            // Update buffers - this is required before rendering!
            self.egui_renderer.update_buffers(
                &self.device,
                &self.queue,
                &mut encoder,
                &self.egui_paint_jobs,
                &screen_descriptor,
            );

            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            self.egui_renderer.render(
                &mut render_pass.forget_lifetime(),
                &self.egui_paint_jobs,
                &screen_descriptor,
            );
        }

        // Free egui textures
        for id in &self.egui_textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit(iter::once(encoder.finish()));
        self.queue.present(output);

        if needs_reconfigure {
            RenderOutcome::Suboptimal
        } else {
            RenderOutcome::Success
        }
    }
}

struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
    pending_redraw: bool,
    next_redraw_at: Option<Instant>,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            state: None,
            pending_redraw: false,
            next_redraw_at: None,
        }
    }

    fn queue_redraw(&mut self) {
        self.pending_redraw = true;

        let scheduled_at = Instant::now() + REDRAW_DELAY;
        match self.next_redraw_at {
            Some(current) if current <= scheduled_at => {}
            _ => self.next_redraw_at = Some(scheduled_at),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Huozi Render Example")
                .with_inner_size(LogicalSize::new(1440, 900))
                .with_max_inner_size(LogicalSize::new(1440, 900))
                .with_min_inner_size(LogicalSize::new(1440, 900));

            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

            #[cfg(target_arch = "wasm32")]
            {
                use winit::platform::web::WindowExtWebSys;
                web_sys::window()
                    .and_then(|win| win.document())
                    .and_then(|doc| {
                        let dst = doc.get_element_by_id("wasm-example")?;
                        let canvas = web_sys::Element::from(window.canvas()?);
                        dst.append_child(&canvas).ok()?;
                        Some(())
                    })
                    .expect("Couldn't append canvas to document body.");
            }

            let state = pollster::block_on(State::new(&window));

            self.window = Some(window);
            self.state = Some(state);

            // Initial render text
            self.state.as_mut().unwrap().render_huozi_text();
            self.queue_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        // Let egui handle the event first
        if let (Some(state), Some(window)) = (self.state.as_mut(), self.window.as_ref()) {
            let response = state.egui_state.on_window_event(window, &event);

            if response.repaint {
                window.request_redraw();
            } else if state.egui_context.has_requested_repaint() {
                self.queue_redraw();
            }

            if response.consumed {
                return; // Event was consumed by egui, don't process it further
            }
        }

        match event {
            WindowEvent::RedrawRequested => {
                if let (Some(state), Some(window)) = (self.state.as_mut(), self.window.as_ref()) {
                    self.pending_redraw = false;
                    self.next_redraw_at = None;

                    state.update(window);
                    match state.render() {
                        RenderOutcome::Success => {}
                        RenderOutcome::Suboptimal
                        | RenderOutcome::Outdated
                        | RenderOutcome::Lost => state.resize(state.size),
                        RenderOutcome::Timeout | RenderOutcome::Occluded => {}
                        RenderOutcome::Validation => log::warn!("Surface validation error"),
                    }
                }
            }
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::Resized(physical_size) => {
                if let Some(state) = self.state.as_mut() {
                    state.resize(physical_size);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let (Some(state), Some(window)) = (self.state.as_mut(), self.window.as_ref()) {
                    let scale_factor = window.scale_factor() as f32;
                    state.cursor_position = Some((
                        position.x as f32 / scale_factor,
                        position.y as f32 / scale_factor,
                    ));
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let (Some(state), Some(window)) = (self.state.as_mut(), self.window.as_ref())
                    && let Some((x, y)) = state.cursor_position
                    && let Some(interaction) = state.interactions.iter().find(|interaction| {
                        interaction.areas.iter().any(|area| {
                            area.rect.left <= x
                                && x <= area.rect.right
                                && area.rect.top <= y
                                && y <= area.rect.bottom
                        })
                    })
                {
                    state.interaction_notice = Some(format!("已点击交互元素：{}", interaction.id));
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            if self.pending_redraw {
                let now = Instant::now();
                let next_redraw_at = self.next_redraw_at.get_or_insert(now + REDRAW_DELAY);

                // On Windows IME composition, issuing the redraw one idle turn later is enough
                // to avoid the candidate window getting disrupted by immediate redraw requests.
                if now >= *next_redraw_at {
                    window.request_redraw();
                }
            }

            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run(event_loop: EventLoop<()>) {
    let mut app = App::new();
    let _ = event_loop.run_app(&mut app);
}

fn main() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Could't initialize logger");
        } else {
            let env = env_logger::Env::default().default_filter_or("huozi=debug,render=debug");
            env_logger::init_from_env(env);
        }
    }
    let event_loop = EventLoop::new().unwrap();
    run(event_loop);
}

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[allow(dead_code)]
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    let event_loop = winit::event_loop::EventLoopBuilder::new()
        .with_android_app(app)
        .build()
        .unwrap();
    run(event_loop);
}
