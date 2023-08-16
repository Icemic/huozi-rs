use csscolorparser::Color;
use huozi::{
    charsets::{ASCII, CHS, CJK_SYMBOL},
    constant::TEXTURE_SIZE,
    layout::{LayoutDirection, LayoutStyle, ShadowStyle, StrokeStyle, TextStyle, Vertex},
    Huozi,
};
use log::{error, info};
use std::{
    iter,
    time::{SystemTime, UNIX_EPOCH},
};
use wgpu::{util::DeviceExt, BlendState};
use winit::{
    dpi::LogicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod texture;

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
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
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
        });
        let surface = unsafe {
            instance
                .create_surface(window)
                .expect("Failed to create surface.")
        };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let caps = surface.get_capabilities(&adapter);
        let format = *caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .expect("Cannot find a proper surface format.");

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
        });

        // initialize huozi instance
        let t = SystemTime::now();
        let font_data = std::fs::read("examples/assets/SourceHanSansSC-Regular.otf").unwrap();
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

    #[allow(unused_variables)]
    fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self, time: u128) {
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
            let sample_text = "Innovation in China 中国智造，惠及全球。
Innovation in China ——⸺全球。
This is a sample text. gM 123.!\\\"\\\"?;:<>
人人生而自由，在尊严和权利上一律平等。他们赋有理性和良心，并应以兄弟关系的精神相对待。
人人有资格享有本宣言所载的一切权利和自由，不分种族、肤色、性别、语言、宗教、政治或其他见解、国籍或社会出身、财产、出生或其他身分等任何区别。并且不得因一人所属的国家或领土的政治的、行政的或者国际的地位之不同而有所区别，无论该领土是独立领土、托管领土、非自治领土或者处于其他任何主权受限制的情况之下。";

            // let sample_text = "在游戏或其他多媒体内容中，文字被放置于通常称作“文本框”的版心内。与报刊杂志等紧凑的设计不同，游戏画面中的文本框四周时常留有足够多的空白以保证用户界面的美观，这使得后者在文字的布局调整方面具有更多先天的余地。以《W3C 中文排版需求》草案为参考，面向游戏或其他多媒体内容中文字使用的一般场景，开发了本工具。";

            let t = SystemTime::now();

            let layout_style = LayoutStyle {
                direction: LayoutDirection::Horizontal,
                box_width: 1200.,
                box_height: 600.,
                glyph_grid_size: 24.,
                viewport_width: 1280.,
                viewport_height: 720.,
            };

            let style = TextStyle {
                fill_color: Color::new(0.0, 0.0, 0.0, 1.0),
                stroke: Some(StrokeStyle::default()),
                shadow: Some(ShadowStyle::default()),
                ..TextStyle::default()
            };

            match self
                .huozi
                .layout_parse(sample_text, &layout_style, &style, None)
            {
                Ok((vertices, indices)) => {
                    info!(
                        "text layouting finished, {}ms",
                        SystemTime::now().duration_since(t).unwrap().as_millis()
                    );

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
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.5,
                            g: 0.5,
                            b: 0.5,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            let vertex_buffer = self.vertex_buffer.as_ref().unwrap();
            let index_buffer = self.index_buffer.as_ref().unwrap();
            let num_indices = self.num_indices.unwrap();

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..num_indices, 0, 0..1);
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Could't initialize logger");
        } else {
            let env = env_logger::Env::default().default_filter_or("huozi=debug,render=debug");
            env_logger::init_from_env(env);
        }
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    window.set_inner_size(LogicalSize::new(1280, 720));

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(450, 400));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

    // State::new uses async code, so we're going to wait for it to finish
    let mut state = State::new(&window).await;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &mut so w have to dereference it twice
                            state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                state.update(time);
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        state.resize(state.size)
                    }
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // We're ignoring timeouts
                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }
            _ => {}
        }
    });
}

fn main() {
    pollster::block_on(run());
}
