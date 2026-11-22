//! Live 3D ritual objects (wgpu `shader` widget).
//!
//! The model is drawn offscreen (4× MSAA + depth) in `prepare` and composited
//! into iced's frame in `draw` as a premultiplied quad with a soft contact
//! shadow. Drag rotates it; when idle it turns slowly. Under the tiny-skia
//! renderer (no GPU) custom shaders are not drawn at all, so the caller keeps
//! a static WebP render underneath until [`gpu_ready`] turns true.

mod glb;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use iced::mouse;
use iced::time::{Duration, Instant};
use iced::wgpu;
use iced::widget::image::Handle;
use iced::widget::shader::Viewport;
use iced::widget::shader::{self, Action};
use iced::{Element, Length, Point, Rectangle};

pub use glb::Mesh;

pub(crate) const KIDDUSH_CUP_GLB: &[u8] = include_bytes!("../../../assets/3d/kiddush-cup.glb");
const KIDDUSH_CUP_STILL: &[u8] = include_bytes!("../../../assets/3d/kiddush-cup.webp");

/// Settings `extra` key «Объёмные модели» (absent = on).
pub const SETTING_KEY: &str = "models3d";

pub fn enabled(ctx: &crate::ui::app::Ctx) -> bool {
    ctx.state
        .settings
        .extra
        .get(SETTING_KEY)
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

pub fn set_enabled(ctx: &mut crate::ui::app::Ctx, on: bool) {
    ctx.state
        .settings
        .extra
        .insert(SETTING_KEY.to_string(), serde_json::Value::Bool(on));
}

/// The embedded models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Model {
    KiddushCup,
}

impl Model {
    fn glb(self) -> &'static [u8] {
        match self {
            Model::KiddushCup => KIDDUSH_CUP_GLB,
        }
    }
    fn still(self) -> &'static [u8] {
        match self {
            Model::KiddushCup => KIDDUSH_CUP_STILL,
        }
    }
    fn slot(self) -> &'static OnceLock<Option<Arc<Mesh>>> {
        static CUP: OnceLock<Option<Arc<Mesh>>> = OnceLock::new();
        match self {
            Model::KiddushCup => &CUP,
        }
    }
    /// Decoded mesh, or None while it is still loading / failed.
    fn mesh(self) -> Option<Arc<Mesh>> {
        self.slot().get().cloned().flatten()
    }
    /// Decode the GLB on a worker thread (textures + mips take ~0.2–0.5 s).
    pub fn preload(self) {
        if self.slot().get().is_some() {
            return;
        }
        // Forced software renderer: custom shaders never draw, so do not
        // spend the decode time and memory.
        if std::env::var("ICED_BACKEND")
            .map(|b| b.contains("tiny"))
            .unwrap_or(false)
        {
            let _ = self.slot().set(None);
            return;
        }
        std::thread::spawn(move || {
            let t = Instant::now();
            let _ = self.slot().get_or_init(|| match glb::parse(self.glb()) {
                Ok(m) => {
                    if stats_enabled() {
                        eprintln!(
                            "model3d: {:?} decoded in {:?} ({} verts, {} tris)",
                            self,
                            t.elapsed(),
                            m.vertices.len(),
                            m.indices.len() / 3
                        );
                    }
                    Some(Arc::new(m))
                }
                Err(e) => {
                    eprintln!("model3d: {self:?}: {e}");
                    None
                }
            });
        });
    }
    /// Static render (fallback for the tiny-skia renderer or when 3D is off).
    pub fn still_image(self) -> Handle {
        static CACHE: OnceLock<Handle> = OnceLock::new();
        CACHE
            .get_or_init(|| match image::load_from_memory(self.still()) {
                Ok(i) => {
                    let i = i.to_rgba8();
                    Handle::from_rgba(i.width(), i.height(), i.into_raw())
                }
                Err(_) => Handle::from_rgba(1, 1, vec![0; 4]),
            })
            .clone()
    }
}

static GPU_READY: AtomicBool = AtomicBool::new(false);
/// Set by any `prepare` call: the wgpu renderer is active (tiny-skia never
/// calls it), even if the mesh is still decoding.
static WGPU_SEEN: AtomicBool = AtomicBool::new(false);
// Stats (SHORASHIM_3D_STATS=1): summed 3D-pass time incl. a GPU wait.
static PREP_NS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static PREP_N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// True once a model has actually been drawn by wgpu in this process.
pub fn gpu_ready() -> bool {
    GPU_READY.load(Ordering::Relaxed)
}

fn stats_enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| crate::util::env_hook("3D_STATS").is_some())
}

/// Screenshot hook `SHORASHIM_3D_YAW=<radians>`: fixed start angle, no spin.
fn fixed_yaw() -> Option<f32> {
    static Y: OnceLock<Option<f32>> = OnceLock::new();
    *Y.get_or_init(|| crate::util::env_hook("3D_YAW").and_then(|v| v.parse().ok()))
}

/// A rotatable model viewport. `on_ready` is published once when the GPU has
/// drawn it, so the caller can drop its static placeholder.
pub fn view<'a, M: Clone + 'a>(model: Model, size: f32, warm: bool, on_ready: M) -> Element<'a, M> {
    model.preload();
    let still = model.still_image();
    let live = shader::Shader::new(Viewer {
        model,
        warm,
        on_ready,
    })
    .width(Length::Fixed(size))
    .height(Length::Fixed(size));
    // Same tree shape before and after the GPU is ready (only the bottom
    // layer changes), so the viewer keeps its angle and an ongoing drag
    // when the placeholder is dropped.
    let under: Element<'a, M> = if gpu_ready() {
        iced::widget::space()
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .into()
    } else {
        iced::widget::image(still)
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .into()
    };
    iced::widget::stack![under, live].into()
}

/// Static render only (3D switched off).
pub fn still<'a, M: 'a>(model: Model, size: f32) -> Element<'a, M> {
    iced::widget::image(model.still_image())
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}

struct Viewer<M> {
    model: Model,
    warm: bool,
    on_ready: M,
}

pub struct State {
    yaw: f32,
    pitch: f32,
    drag: Option<(Point, f32, f32)>,
    last_move: Option<(Point, Instant)>,
    vel: f32,
    last: Option<Instant>,
    announced: bool,
    redraws: u32,
    // fps stats
    frames: u32,
    since: Option<Instant>,
}

impl Default for State {
    fn default() -> Self {
        State {
            yaw: fixed_yaw().unwrap_or(0.7),
            pitch: 0.28,
            drag: None,
            last_move: None,
            vel: 0.0,
            last: None,
            announced: false,
            redraws: 0,
            frames: 0,
            since: None,
        }
    }
}

const IDLE_SPIN: f32 = 0.35; // rad/s

impl<M: Clone> shader::Program<M> for Viewer<M> {
    type State = State;
    type Primitive = Primitive;

    fn update(
        &self,
        st: &mut State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<M>> {
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = cursor.position_over(bounds)?;
                st.drag = Some((p, st.yaw, st.pitch));
                st.last_move = Some((p, Instant::now()));
                st.vel = 0.0;
                Some(Action::request_redraw().and_capture())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let (start, yaw0, pitch0) = st.drag?;
                let k = 3.2 / bounds.width.max(1.0);
                st.yaw = yaw0 + (position.x - start.x) * k;
                st.pitch = (pitch0 + (position.y - start.y) * k).clamp(-0.5, 1.1);
                let now = Instant::now();
                if let Some((lp, lt)) = st.last_move {
                    let dt = (now - lt).as_secs_f32();
                    if dt > 0.0 {
                        st.vel = ((position.x - lp.x) * k / dt).clamp(-8.0, 8.0);
                    }
                }
                st.last_move = Some((*position, now));
                Some(Action::request_redraw().and_capture())
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                st.drag?;
                st.drag = None;
                // Released after holding still: no fling.
                if st
                    .last_move
                    .map(|(_, t)| t.elapsed() > Duration::from_millis(80))
                    .unwrap_or(true)
                {
                    st.vel = 0.0;
                }
                Some(Action::request_redraw().and_capture())
            }
            iced::Event::Window(iced::window::Event::RedrawRequested(now)) => {
                let dt = st
                    .last
                    .map(|l| (*now - l).as_secs_f32().min(0.1))
                    .unwrap_or(0.0);
                st.last = Some(*now);
                if st.drag.is_none() {
                    if fixed_yaw().is_none() {
                        st.yaw += (IDLE_SPIN + st.vel) * dt;
                    }
                    st.vel *= (-3.0 * dt).exp();
                }
                if stats_enabled() {
                    st.frames += 1;
                    let since = *st.since.get_or_insert(*now);
                    let el = (*now - since).as_secs_f32();
                    if el >= 5.0 {
                        let (ns, n) = (
                            PREP_NS.swap(0, Ordering::Relaxed),
                            PREP_N.swap(0, Ordering::Relaxed).max(1),
                        );
                        eprintln!(
                            "model3d: {:.1} fps over {:.1}s ({:.2} ms/frame); 3D pass avg {:.2} ms over {} renders",
                            st.frames as f32 / el,
                            el,
                            el * 1000.0 / st.frames as f32,
                            ns as f64 / n as f64 / 1e6,
                            n
                        );
                        st.frames = 0;
                        st.since = Some(*now);
                    }
                }
                if gpu_ready() && !st.announced {
                    st.announced = true;
                    return Some(Action::publish(self.on_ready.clone()));
                }
                // Keep animating only where it can be seen (wgpu); under
                // tiny-skia the static picture stays and nothing redraws.
                st.redraws = st.redraws.saturating_add(1);
                let loading = self.model.slot().get().is_none();
                let waiting_upload =
                    WGPU_SEEN.load(Ordering::Relaxed) && self.model.mesh().is_some();
                if gpu_ready() || loading || waiting_upload || st.redraws < 10 {
                    // ~60 fps while dragged or flung, ~30 fps for the idle spin.
                    let busy = st.drag.is_some() || st.vel.abs() > 0.05;
                    let gap = if busy { 16 } else { 33 };
                    Some(Action::request_redraw_at(*now + Duration::from_millis(gap)))
                } else {
                    // Decoded (or skipped) and no wgpu draw after the first
                    // frames: tiny-skia, so stop asking for frames.
                    None
                }
            }
            _ => None,
        }
    }

    fn draw(&self, st: &State, _cursor: mouse::Cursor, _bounds: Rectangle) -> Primitive {
        Primitive {
            model: self.model,
            yaw: st.yaw,
            pitch: st.pitch,
            warm: self.warm,
        }
    }

    fn mouse_interaction(
        &self,
        st: &State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if st.drag.is_some() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}

#[derive(Debug)]
pub struct Primitive {
    model: Model,
    yaw: f32,
    pitch: f32,
    warm: bool,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view: [f32; 4],
    fit: [f32; 4],
    base: [f32; 4],
    mat: [f32; 4],
}

struct GpuModel {
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    count: u32,
    ubuf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    mesh: Arc<Mesh>,
}

struct Target {
    size: (u32, u32),
    msaa: wgpu::TextureView,
    depth: wgpu::TextureView,
    resolve: wgpu::TextureView,
    blit: wgpu::BindGroup,
    shadow: wgpu::Buffer,
    drawn: Option<(f32, f32, bool)>,
}

pub struct Pipeline {
    format: wgpu::TextureFormat,
    model_pipe: wgpu::RenderPipeline,
    blit_pipe: wgpu::RenderPipeline,
    model_bgl: wgpu::BindGroupLayout,
    blit_bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    models: HashMap<Model, GpuModel>,
    targets: HashMap<Model, Target>,
    /// Which targets were used this frame (trim the rest).
    live: Vec<Model>,
}

/// 4× MSAA (guaranteed for these formats by WebGPU); `SHORASHIM_3D_MSAA=1`
/// turns it off for measurements.
fn samples() -> u32 {
    static S: OnceLock<u32> = OnceLock::new();
    *S.get_or_init(|| match crate::util::env_hook("3D_MSAA").as_deref() {
        Some("1") => 1,
        _ => 4,
    })
}
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

fn shaders() -> (&'static str, &'static str) {
    const SRC: &str = include_str!("model.wgsl");
    SRC.split_once("// ---- blit").unwrap_or((SRC, ""))
}

impl shader::Pipeline for Pipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let (model_src, blit_src) = shaders();
        let model_mod = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("model3d"),
            source: wgpu::ShaderSource::Wgsl(model_src.into()),
        });
        let blit_mod = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("model3d blit"),
            source: wgpu::ShaderSource::Wgsl(format!("// ---- blit{blit_src}").into()),
        });
        let tex_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let uniform_entry = |binding, visibility| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let sampler_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let model_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model3d"),
            entries: &[
                uniform_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT),
                sampler_entry(1),
                tex_entry(2),
                tex_entry(3),
                tex_entry(4),
            ],
        });
        let blit_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model3d blit"),
            entries: &[
                sampler_entry(0),
                tex_entry(1),
                uniform_entry(2, wgpu::ShaderStages::FRAGMENT),
            ],
        });
        let model_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("model3d"),
            bind_group_layouts: &[&model_bgl],
            push_constant_ranges: &[],
        });
        let blit_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("model3d blit"),
            bind_group_layouts: &[&blit_bgl],
            push_constant_ranges: &[],
        });
        let model_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("model3d"),
            layout: Some(&model_layout),
            vertex: wgpu::VertexState {
                module: &model_mod,
                entry_point: Some("vs_model"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<glb::Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &model_mod,
                entry_point: Some("fs_model"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: samples(),
                ..Default::default()
            },
            multiview: None,
            cache: None,
        });
        let blit_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("model3d blit"),
            layout: Some(&blit_layout),
            vertex: wgpu::VertexState {
                module: &blit_mod,
                entry_point: Some("vs_blit"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_mod,
                entry_point: Some("fs_blit"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("model3d"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            anisotropy_clamp: 4,
            ..Default::default()
        });
        Pipeline {
            format,
            model_pipe,
            blit_pipe,
            model_bgl,
            blit_bgl,
            sampler,
            models: HashMap::new(),
            targets: HashMap::new(),
            live: Vec::new(),
        }
    }

    fn trim(&mut self) {
        let live = std::mem::take(&mut self.live);
        self.targets.retain(|m, _| live.contains(m));
    }
}

fn upload_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    tex: Option<&glb::Texture>,
    fallback: [u8; 4],
    srgb: bool,
) -> wgpu::TextureView {
    let one = glb::Texture {
        width: 1,
        height: 1,
        mips: vec![fallback.to_vec()],
    };
    let tex = tex.unwrap_or(&one);
    let t = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("model3d texture"),
        size: wgpu::Extent3d {
            width: tex.width,
            height: tex.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: tex.mips.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: if srgb {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        },
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let (mut w, mut h) = (tex.width, tex.height);
    for (level, data) in tex.mips.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &t,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        w = (w / 2).max(1);
        h = (h / 2).max(1);
    }
    t.create_view(&Default::default())
}

impl Pipeline {
    fn gpu_model(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, model: Model) -> bool {
        if self.models.contains_key(&model) {
            return true;
        }
        let Some(mesh) = model.mesh() else {
            return false;
        };
        use wgpu::util::DeviceExt;
        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("model3d vertices"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("model3d indices"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model3d uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Textures are taken on the first upload; a second pipeline (new
        // device) decodes them again from the embedded GLB.
        let tex = mesh
            .textures
            .lock()
            .ok()
            .and_then(|mut t| t.take())
            .or_else(|| glb::parse(model.glb()).ok()?.textures.into_inner().ok()?);
        let tex = tex.unwrap_or(glb::Textures {
            base_color: None,
            metal_rough: None,
            normal: None,
        });
        let base = upload_texture(device, queue, tex.base_color.as_ref(), [255; 4], true);
        let mr = upload_texture(device, queue, tex.metal_rough.as_ref(), [255; 4], false);
        let nrm = upload_texture(
            device,
            queue,
            tex.normal.as_ref(),
            [128, 128, 255, 255],
            false,
        );
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model3d"),
            layout: &self.model_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubuf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&base),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&mr),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&nrm),
                },
            ],
        });
        self.models.insert(
            model,
            GpuModel {
                vbuf,
                ibuf,
                count: mesh.indices.len() as u32,
                ubuf,
                bind,
                mesh,
            },
        );
        true
    }

    fn target(&mut self, device: &wgpu::Device, model: Model, size: (u32, u32)) -> &mut Target {
        if self
            .targets
            .get(&model)
            .map(|t| t.size != size)
            .unwrap_or(true)
        {
            let tex = |samples, format, usage| {
                device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("model3d target"),
                        size: wgpu::Extent3d {
                            width: size.0,
                            height: size.1,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: samples,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage,
                        view_formats: &[],
                    })
                    .create_view(&Default::default())
            };
            let msaa = tex(
                samples(),
                self.format,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
            );
            let depth = tex(samples(), DEPTH, wgpu::TextureUsages::RENDER_ATTACHMENT);
            let resolve = tex(
                1,
                self.format,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            );
            let shadow = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("model3d shadow"),
                size: 16,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let blit = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("model3d blit"),
                layout: &self.blit_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&resolve),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: shadow.as_entire_binding(),
                    },
                ],
            });
            self.targets.insert(
                model,
                Target {
                    size,
                    msaa,
                    depth,
                    resolve,
                    blit,
                    shadow,
                    drawn: None,
                },
            );
        }
        self.targets.get_mut(&model).unwrap()
    }
}

impl shader::Primitive for Primitive {
    type Pipeline = Pipeline;

    fn prepare(
        &self,
        pipe: &mut Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        let t0 = Instant::now();
        WGPU_SEEN.store(true, Ordering::Relaxed);
        if !pipe.gpu_model(device, queue, self.model) {
            return;
        }
        let s = viewport.scale_factor();
        let size = (
            ((bounds.width * s).ceil() as u32).clamp(1, 4096),
            ((bounds.height * s).ceil() as u32).clamp(1, 4096),
        );
        let srgb = pipe.format.is_srgb();
        let (ubuf, mesh) = {
            let g = &pipe.models[&self.model];
            (g.ubuf.clone(), g.mesh.clone())
        };
        pipe.live.push(self.model);
        let target = pipe.target(device, self.model, size);
        let key = (self.yaw, self.pitch, self.warm);
        if target.drawn == Some(key) {
            return;
        }
        let inv_r = 1.0 / mesh.radius;
        let u = Uniforms {
            view: [
                self.yaw,
                self.pitch,
                size.0 as f32 / size.1 as f32,
                if srgb { 1.0 } else { 0.0 },
            ],
            fit: [mesh.center[0], mesh.center[1], mesh.center[2], inv_r],
            base: mesh.base_factor,
            mat: [
                mesh.metallic,
                mesh.roughness,
                if mesh.has_normal_map { 1.0 } else { 0.0 },
                if self.warm { 1.0 } else { 0.0 },
            ],
        };
        queue.write_buffer(&ubuf, 0, bytemuck::bytes_of(&u));
        // Contact shadow under the model's lowest point (projected roughly).
        let base_y = (mesh.min_y - mesh.center[1]) * inv_r * self.pitch.cos();
        let sy = 0.5 - base_y * 3.6 / 4.2 * 0.5 * (size.0 as f32 / size.1 as f32).min(1.0);
        queue.write_buffer(
            &target.shadow,
            0,
            bytemuck::cast_slice(&[sy.min(0.97), 0.30, 0.06, 0.38f32]),
        );
        let msaa = samples() > 1;
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("model3d"),
        });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("model3d"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: if msaa { &target.msaa } else { &target.resolve },
                    depth_slice: None,
                    resolve_target: msaa.then_some(&target.resolve),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: if msaa {
                            wgpu::StoreOp::Discard
                        } else {
                            wgpu::StoreOp::Store
                        },
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let g = &pipe.models[&self.model];
            pass.set_pipeline(&pipe.model_pipe);
            pass.set_bind_group(0, &g.bind, &[]);
            pass.set_vertex_buffer(0, g.vbuf.slice(..));
            pass.set_index_buffer(g.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..g.count, 0, 0..1);
        }
        queue.submit([enc.finish()]);
        if let Some(t) = pipe.targets.get_mut(&self.model) {
            t.drawn = Some(key);
        }
        GPU_READY.store(true, Ordering::Relaxed);
        if stats_enabled() {
            // Stats mode waits for the GPU after every render, so the number
            // is the whole 3D pass (CPU + GPU), not just command recording.
            let _ = device.poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            });
            PREP_NS.fetch_add(t0.elapsed().as_nanos() as u64, Ordering::Relaxed);
            PREP_N.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn draw(&self, pipe: &Pipeline, pass: &mut wgpu::RenderPass<'_>) -> bool {
        let Some(t) = pipe.targets.get(&self.model) else {
            return true;
        };
        if t.drawn.is_none() {
            return true;
        }
        pass.set_pipeline(&pipe.blit_pipe);
        pass.set_bind_group(0, &t.blit, &[]);
        pass.draw(0..3, 0..1);
        true
    }
}

#[cfg(test)]
mod tests {
    use iced::wgpu::naga;

    /// Both WGSL halves parse and validate (catches shader errors in CI,
    /// which has no GPU).
    #[test]
    fn shaders_validate() {
        let (model, blit) = super::shaders();
        for (name, src) in [
            ("model", model.to_string()),
            ("blit", format!("// ---- blit{blit}")),
        ] {
            let module = naga::front::wgsl::parse_str(&src)
                .unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(&src)));
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::empty(),
            )
            .validate(&module)
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
        }
    }
}
