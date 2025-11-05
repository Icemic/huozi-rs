use csscolorparser::Color;
use egui::epaint::text::{FontInsert, InsertFontFamily};
use huozi::{
    constant::TEXTURE_SIZE,
    layout::{
        ColorSpace, LayoutDirection, LayoutStyle, ShadowStyle, StrokeStyle, TextStyle, Vertex,
    },
    Huozi,
};
use log::{error, info};
use std::{iter, sync::Arc, time::SystemTime};
use wgpu::{util::DeviceExt, BlendState};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::{fonts::get_builtin_fonts, mvp::MVPUniform};

mod fonts;
mod mvp;
mod texture;

const DEFAULT_TEXT: &str = "Innovation in China 中国智造，惠及全球。
Innovation in China ——⸺全球。
This is a sample text. gM 123.!\\\"\\\"?;:<>
人人生而自由，在尊严和权利上一律平等。他们赋有理性和良心，并应以兄弟关系的精神相对待。";

struct State {
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

    current_font: String,
    huozi: Option<Huozi>,

    // egui integration
    egui_context: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,

    // Text input
    input_text: String,
    last_rendered_text: String,

    // Layout and style configuration
    background_color: wgpu::Color,
    layout_config: LayoutStyle,
    text_config: TextStyle,
    stroke_enabled: bool,
    shadow_enabled: bool,
    config_changed: bool,

    // Store egui render data
    egui_paint_jobs: Vec<egui::ClippedPrimitive>,
    egui_textures_delta: egui::TexturesDelta,
}

impl State {
    async fn new(window: &Arc<Window>) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            backend_options: wgpu::BackendOptions {
                dx12: wgpu::Dx12BackendOptions {
                    shader_compiler: wgpu::Dx12Compiler::Fxc,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        });
        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface.");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
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

        println!("logical_size: {:?}", logical_size);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&mvp_bind_group_layout, &texture_bind_group_layout],
                push_constant_ranges: &[],
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
                buffers: &[Vertex::desc()],
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
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
            cache: None,
        });

        // Initialize egui
        let egui_context = egui::Context::default();
        egui_context.add_font(FontInsert::new(
            "custom_font",
            egui::FontData::from_owned(get_builtin_fonts()[0].1.to_vec()),
            vec![InsertFontFamily {
                family: egui::FontFamily::Proportional,
                // use lowest priority to avoid overriding other fonts
                priority: egui::epaint::text::FontPriority::Lowest,
            }],
        ));
        egui_context.add_font(FontInsert::new(
            "firacode",
            egui::FontData::from_owned(get_builtin_fonts().last().unwrap().1.to_vec()),
            vec![InsertFontFamily {
                family: egui::FontFamily::Monospace,
                // use highest priority to ensure monospace texts use Fira Code
                priority: egui::epaint::text::FontPriority::Highest,
            }],
        ));
        egui_context.style_mut(|style| {
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
            current_font: get_builtin_fonts().get(0).unwrap().0.to_string(),
            huozi: None,
            egui_context,
            egui_state,
            egui_renderer,
            input_text: DEFAULT_TEXT.to_string(),
            last_rendered_text: String::new(),
            background_color: wgpu::Color {
                r: 0.737,
                g: 0.737,
                b: 0.737,
                a: 1.0,
            },
            layout_config: LayoutStyle {
                direction: LayoutDirection::Horizontal,
                box_width: 1200.,
                box_height: 600.,
                glyph_grid_size: 32.,
            },
            text_config: TextStyle {
                fill_color: Color::new(0.0, 0.0, 0.0, 1.0),
                stroke: Some(StrokeStyle::default()),
                shadow: Some(ShadowStyle::default()),
                ..TextStyle::default()
            },
            stroke_enabled: true,
            shadow_enabled: true,
            config_changed: false,
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
        let raw_input = self.egui_state.take_egui_input(window);
        let full_output = self.egui_context.run(raw_input, |ctx| {
            // Bottom panel for text input and configuration
            egui::TopBottomPanel::bottom("text_input_panel")
                .resizable(false)
                .default_height(350.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            // Text input section
                            // ui.heading("Playground");

                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.heading("Text");
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.input_text)
                                            .desired_width(400.)
                                            .desired_rows(16)
                                            .font(egui::TextStyle::Name("custom_font".into())),
                                    );
                                });
                                // ui.add_space(10.0);
                                ui.separator();
                                // Layout configuration
                                ui.vertical(|ui| {
                                    ui.set_width(300.);
                                    ui.heading("🎨 Basic");
                                    ui.horizontal(|ui| {
                                        ui.label("Background Color:");
                                        let mut color = [
                                            self.background_color.r as f32,
                                            self.background_color.g as f32,
                                            self.background_color.b as f32,
                                        ];
                                        if ui.color_edit_button_rgb(&mut color).changed() {
                                            self.background_color.r = color[0] as f64;
                                            self.background_color.g = color[1] as f64;
                                            self.background_color.b = color[2] as f64;
                                            self.config_changed = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Font:");
                                        egui::ComboBox::from_label("")
                                            .selected_text(self.current_font.clone())
                                            .show_ui(ui, |ui| {
                                                for (font_name, _) in get_builtin_fonts().iter() {
                                                    if ui
                                                        .selectable_value(
                                                            &mut self.current_font,
                                                            font_name.to_string(),
                                                            font_name.to_string(),
                                                        )
                                                        .clicked()
                                                    {
                                                        let _ = self.huozi.take();
                                                        self.config_changed = true;
                                                    }
                                                }
                                            });
                                    });

                                    ui.add_space(10.);

                                    ui.heading("⚙ Layout Configuration");
                                    if ui
                                        .horizontal(|ui| {
                                            ui.disable();
                                            ui.label("Direction:")
                                                .on_disabled_hover_text("Not supported by now");
                                            ui.radio_value(
                                                &mut self.layout_config.direction,
                                                LayoutDirection::Horizontal,
                                                "Horizontal",
                                            ) | ui.radio_value(
                                                &mut self.layout_config.direction,
                                                LayoutDirection::Vertical,
                                                "Vertical",
                                            )
                                        })
                                        .inner
                                        .changed()
                                    {
                                        self.config_changed = true;
                                    }

                                    if ui
                                        .horizontal(|ui| {
                                            ui.label("Box Width:");
                                            ui.add(
                                                egui::DragValue::new(
                                                    &mut self.layout_config.box_width,
                                                )
                                                .speed(10.0)
                                                .range(100.0..=2000.0),
                                            )
                                        })
                                        .inner
                                        .changed()
                                    {
                                        self.config_changed = true;
                                    }

                                    if ui
                                        .horizontal(|ui| {
                                            ui.label("Box Height:");
                                            ui.add(
                                                egui::DragValue::new(
                                                    &mut self.layout_config.box_height,
                                                )
                                                .speed(10.0)
                                                .range(100.0..=1000.0),
                                            )
                                        })
                                        .inner
                                        .changed()
                                    {
                                        self.config_changed = true;
                                    }

                                    if ui
                                        .horizontal(|ui| {
                                            ui.label("Glyph Grid Size:");
                                            ui.add(
                                                egui::DragValue::new(
                                                    &mut self.layout_config.glyph_grid_size,
                                                )
                                                .speed(1.0)
                                                .range(8.0..=128.0),
                                            )
                                        })
                                        .inner
                                        .changed()
                                    {
                                        self.config_changed = true;
                                    }
                                });

                                ui.separator();

                                // Text style configuration
                                ui.vertical(|ui| {
                                    ui.heading("🎨 Text Style Configuration");
                                    ui.horizontal(|ui| {
                                        ui.label("Font Size:");
                                        ui.add(
                                            egui::DragValue::new(&mut self.text_config.font_size)
                                                .speed(1.0)
                                                .range(8.0..=128.0),
                                        );
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Line Height:");
                                        ui.add(
                                            egui::DragValue::new(&mut self.text_config.line_height)
                                                .speed(0.1)
                                                .range(0.5..=3.0),
                                        );
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Indent:");
                                        ui.add(
                                            egui::DragValue::new(&mut self.text_config.indent)
                                                .speed(1.0)
                                                .range(0.0..=200.0),
                                        );
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Fill Color:");
                                        let mut color = self.text_config.fill_color.to_rgba8();
                                        if ui
                                            .color_edit_button_srgba_premultiplied(&mut color)
                                            .changed()
                                        {
                                            self.text_config.fill_color = color.into();
                                            self.config_changed = true;
                                        }
                                    });

                                    ui.add_space(5.0);

                                    // Stroke configuration
                                    ui.checkbox(&mut self.stroke_enabled, "Enable Stroke");
                                    if self.stroke_enabled {
                                        ui.indent("stroke", |ui| {
                                            if self.text_config.stroke.is_none() {
                                                self.text_config.stroke =
                                                    Some(StrokeStyle::default());
                                            }

                                            if let Some(stroke) = &mut self.text_config.stroke {
                                                ui.horizontal(|ui| {
                                                    ui.label("Stroke Width:");
                                                    ui.add(
                                                        egui::DragValue::new(
                                                            &mut stroke.stroke_width,
                                                        )
                                                        .speed(0.1)
                                                        .range(0.0..=20.0),
                                                    );
                                                });

                                                ui.horizontal(|ui| {
                                                    ui.label("Stroke Color:");
                                                    let mut color = stroke.stroke_color.to_rgba8();
                                                    if ui
                                                        .color_edit_button_srgba_premultiplied(
                                                            &mut color,
                                                        )
                                                        .changed()
                                                    {
                                                        stroke.stroke_color = color.into();
                                                        self.config_changed = true;
                                                    }
                                                });
                                            }
                                        });
                                    } else {
                                        self.text_config.stroke = None;
                                    }

                                    ui.add_space(5.0);

                                    // Shadow configuration
                                    ui.checkbox(&mut self.shadow_enabled, "Enable Shadow");
                                    if self.shadow_enabled {
                                        ui.indent("shadow", |ui| {
                                            if self.text_config.shadow.is_none() {
                                                self.text_config.shadow =
                                                    Some(ShadowStyle::default());
                                            }

                                            if let Some(shadow) = &mut self.text_config.shadow {
                                                ui.horizontal(|ui| {
                                                    ui.label("Shadow Offset X:");
                                                    ui.add(
                                                        egui::DragValue::new(
                                                            &mut shadow.shadow_offset_x,
                                                        )
                                                        .speed(0.5)
                                                        .range(-50.0..=50.0),
                                                    );
                                                });

                                                ui.horizontal(|ui| {
                                                    ui.label("Shadow Offset Y:");
                                                    ui.add(
                                                        egui::DragValue::new(
                                                            &mut shadow.shadow_offset_y,
                                                        )
                                                        .speed(0.5)
                                                        .range(-50.0..=50.0),
                                                    );
                                                });

                                                ui.horizontal(|ui| {
                                                    ui.label("Shadow Blur:");
                                                    ui.add(
                                                        egui::DragValue::new(
                                                            &mut shadow.shadow_blur,
                                                        )
                                                        .speed(0.5)
                                                        .range(0.0..=50.0),
                                                    );
                                                });

                                                ui.horizontal(|ui| {
                                                    ui.label("Shadow Width:");
                                                    ui.add(
                                                        egui::DragValue::new(
                                                            &mut shadow.shadow_width,
                                                        )
                                                        .speed(0.1)
                                                        .range(0.0..=20.0),
                                                    );
                                                });

                                                ui.horizontal(|ui| {
                                                    ui.label("Shadow Color:");
                                                    let mut color = shadow.shadow_color.to_rgba8();
                                                    if ui
                                                        .color_edit_button_srgba_premultiplied(
                                                            &mut color,
                                                        )
                                                        .changed()
                                                    {
                                                        shadow.shadow_color = color.into();
                                                        self.config_changed = true;
                                                    }
                                                });
                                            }
                                        });
                                    } else {
                                        self.text_config.shadow = None;
                                    }
                                });
                            });
                        });
                });
        });

        // Mark config as changed if there was any UI interaction in config panels
        if full_output.platform_output.events.iter().any(|_| true) {
            self.config_changed = true;
        }

        self.egui_state
            .handle_platform_output(window, full_output.platform_output);

        // Store textures delta and paint jobs for rendering
        self.egui_textures_delta = full_output.textures_delta;
        self.egui_paint_jobs = self
            .egui_context
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        // Check if text or config changed and re-render
        if self.input_text != self.last_rendered_text || self.config_changed {
            self.render_huozi_text(&self.input_text.clone());
            self.last_rendered_text = self.input_text.clone();
        }
    }

    fn render_huozi_text(&mut self, text: &str) {
        let t = SystemTime::now();

        if self.huozi.is_none() {
            info!("load font: {}", self.current_font);
            // initialize huozi instance
            let t = SystemTime::now();

            let font_data = get_builtin_fonts()
                .iter()
                .find(|(name, _)| *name == &self.current_font)
                .map(|(_, data)| *data)
                .expect("Failed to find font data for the current font");

            info!(
                "font file loaded, {}ms",
                SystemTime::now().duration_since(t).unwrap().as_millis()
            );

            let huozi = huozi::Huozi::new(font_data.to_vec());

            // {
            //     use huozi::charsets::{ASCII, CHS, CJK_SYMBOL};

            //     let t = SystemTime::now();

            //     huozi.preload(ASCII);
            //     huozi.preload(CJK_SYMBOL);
            //     huozi.preload(CHS);

            //     info!(
            //         "SDF texture preloaded, {}ms",
            //         SystemTime::now().duration_since(t).unwrap().as_millis()
            //     );
            // }

            self.huozi = Some(huozi);
        }

        let Some(huozi) = self.huozi.as_mut() else {
            error!("Huozi instance is not initialized");
            return;
        };

        match huozi.layout_parse(
            text,
            &self.layout_config,
            &self.text_config,
            ColorSpace::SRGB,
            None,
        ) {
            Ok((glyphs, total_width, total_height)) => {
                info!(
                    "text layouting finished, {}ms",
                    SystemTime::now().duration_since(t).unwrap().as_millis()
                );

                info!(
                    "total_width: {}, total_height: {}",
                    total_width, total_height
                );

                let mut vertices: Vec<Vertex> = Vec::with_capacity(glyphs.len() * 4 * 3);
                let mut indices: Vec<u16> = Vec::with_capacity(glyphs.len() * 6);

                let mut index_offset = 0;

                if self.text_config.shadow.is_some() {
                    for glyph in glyphs.iter() {
                        vertices.extend(&glyph.shadow);
                        indices.extend(glyph.indices.iter().map(|i| i + index_offset));

                        index_offset += glyph.shadow.len() as u16;
                    }
                }

                if self.text_config.stroke.is_some() {
                    for glyph in glyphs.iter() {
                        vertices.extend(&glyph.stroke);
                        indices.extend(glyph.indices.iter().map(|i| i + index_offset));

                        index_offset += glyph.stroke.len() as u16;
                    }
                }

                for glyph in glyphs.iter() {
                    vertices.extend(&glyph.fill);
                    indices.extend(glyph.indices.iter().map(|i| i + index_offset));

                    index_offset += glyph.fill.len() as u16;
                }

                let vertex_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Vertex Buffer"),
                            contents: bytemuck::cast_slice(&vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });
                let index_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Index Buffer"),
                            contents: bytemuck::cast_slice(&indices),
                            usage: wgpu::BufferUsages::INDEX,
                        });
                let num_indices = indices.len() as u32;

                self.vertex_buffer = Some(vertex_buffer);
                self.index_buffer = Some(index_buffer);
                self.num_indices = Some(num_indices);

                self.texture
                    .write_bitmap(&self.queue, huozi.texture_image());
            }
            Err(err_msg) => {
                error!("{}", err_msg);
            }
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        if self.vertex_buffer.is_none() {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
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

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.mvp_bind_group, &[]);
            render_pass.set_bind_group(1, &self.texture_bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..num_indices, 0, 0..1);
        }

        // Render egui (only if there are paint jobs to render)
        if !self.egui_paint_jobs.is_empty() {
            // Update textures
            for (id, image_delta) in &self.egui_textures_delta.set {
                self.egui_renderer
                    .update_texture(&self.device, &self.queue, *id, image_delta);
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
        output.present();

        Ok(())
    }
}

struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            state: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Huozi Render Example")
                .with_inner_size(LogicalSize::new(1280, 720))
                .with_max_inner_size(LogicalSize::new(1280, 720))
                .with_min_inner_size(LogicalSize::new(1280, 720));

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
            if response.consumed {
                return; // Event was consumed by egui, don't process it further
            }
        }

        match event {
            WindowEvent::RedrawRequested => {
                if let (Some(state), Some(window)) = (self.state.as_mut(), self.window.as_ref()) {
                    state.update(window);
                    match state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            state.resize(state.size)
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            event_loop.exit();
                        }
                        Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                        Err(wgpu::SurfaceError::Other) => {
                            log::warn!("Surface other error")
                        }
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
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
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
