use csscolorparser::Color;
use huozi::{
    charsets::{ASCII, CHS, CJK_SYMBOL},
    constant::TEXTURE_SIZE,
    layout::{
        ColorSpace, LayoutDirection, LayoutStyle, ShadowStyle, StrokeStyle, TextStyle, Vertex,
    },
    Huozi,
};
use log::{error, info};
use std::{
    iter,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
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

use crate::mvp::MVPUniform;

mod mvp;
mod texture;

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

    huozi: Huozi,
    text_rendered: bool,
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

        // initialize huozi instance
        let t = SystemTime::now();
        // let font_data = std::fs::read("examples/assets/SourceHanSansSC-Regular.otf").unwrap();
        let font_data = include_bytes!("../assets/SourceHanSansSC-Regular.otf").to_vec();
        // let font_data = std::fs::read("examples/assets/Zhudou Sans Regular.ttf").unwrap();
        // let font_data = std::fs::read("examples/assets/SourceHanSerifSC-Regular.otf").unwrap();
        // let font_data = std::fs::read("examples/assets/LXGWWenKaiLite-Regular.ttf").unwrap();
        // let font_data = std::fs::read("examples/assets/SweiGothicCJKsc-Regular.ttf").unwrap();

        info!(
            "font file loaded, {}ms",
            SystemTime::now().duration_since(t).unwrap().as_millis()
        );

        let mut huozi = huozi::Huozi::new(font_data);

        let t = SystemTime::now();

        huozi.preload(ASCII);
        huozi.preload(CJK_SYMBOL);
        huozi.preload(CHS);

        info!(
            "SDF texture preloaded, {}ms",
            SystemTime::now().duration_since(t).unwrap().as_millis()
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
            huozi,
            text_rendered: false,
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

    fn update(&mut self, _: u128) {
        // self.uniforms.color[0] = (time % 2000) as f32 / 2000. * 1.0;
        // self.queue.write_buffer(
        //     &self.uniform_buffer,
        //     0,
        //     bytemuck::cast_slice(&[self.uniforms]),
        // );

        if !self.text_rendered {
            self.text_rendered = true;

            // render text
            //                         let sample_text = "中国智造，惠及全球。
            // ——全球
            // ⸺全球。
            // ――全球。
            // ––––全球。
            // ⸻全球。
            // 　　全球。
            // 全  球 。";
            let sample_text = "Innovation in China 中国智造，惠及全球。\n\
            Innovation in China ——⸺全球。\n\
            This is a sample text. gM 123.!\\\"\\\"?;:<>\n\
            人人生而自由，在尊严和权利上一律平等。他们赋有理性和良心，并应以兄弟关系的精神相对待。\n\
            人人有资格享有本宣言所载的一切权利和自由，不分种族、肤色、性别、语言、宗教、政治或其他见解、国籍或社会出身、财产、出生或其他身分等任\
            何区别。并且不得因一人所属的国家或领土的政治的、行政的或者国际的地位之不同而有所区别，无论该领土是独立领土、托管领土、非自治领土或者\
            处于其他任何主权受限制的情况之下。";

            // let sample_text = "在游戏或其他多媒体内容中，文字被放置于通常称作“文本框”的版心内。与报刊杂志等紧凑的设计不同，游戏画面中的文本框四周时常留有足够多的空白以保证用户界面的美观，这使得后者在文字的布局调整方面具有更多先天的余地。以《W3C 中文排版需求》草案为参考，面向游戏或其他多媒体内容中文字使用的一般场景，开发了本工具。";

            let t = SystemTime::now();

            let layout_style = LayoutStyle {
                direction: LayoutDirection::Horizontal,
                box_width: 1200.,
                box_height: 600.,
                glyph_grid_size: 32.,
            };

            let style = TextStyle {
                fill_color: Color::new(0.0, 0.0, 0.0, 1.0),
                stroke: Some(StrokeStyle::default()),
                shadow: Some(ShadowStyle::default()),
                ..TextStyle::default()
            };

            match self.huozi.layout_parse(
                sample_text,
                &layout_style,
                &style,
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

                    if style.shadow.is_some() {
                        for glyph in glyphs.iter() {
                            vertices.extend(&glyph.shadow);
                            indices.extend(glyph.indices.iter().map(|i| i + index_offset));

                            index_offset += glyph.shadow.len() as u16;
                        }
                    }

                    if style.stroke.is_some() {
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
                        .write_bitmap(&self.queue, self.huozi.texture_image());
                }
                Err(err_msg) => {
                    error!("{}", err_msg);
                }
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
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.737,
                            g: 0.737,
                            b: 0.737,
                            a: 1.0,
                        }),
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
        match event {
            WindowEvent::RedrawRequested => {
                if let Some(state) = self.state.as_mut() {
                    let time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis();
                    state.update(time);
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
