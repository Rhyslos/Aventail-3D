//! Drawing a viewport into an offscreen image on the GPU.
//!
//! Each viewport renders into its own [`ViewportTarget`] (colour and depth
//! images, with multisampling when the GPU supports it). The caller then
//! shows the finished colour image however it likes; the app hands it to
//! egui as a texture.

use crate::frame::{FrameParams, FrameUniforms, GRID_HALF_EXTENT};
use crate::geometry::{Overlay, Shape, ground_axes};
use crate::pick::{PickRequest, PickResult, Picker};
use crate::pipelines::{PipelineKind, create_pipeline};
use wgpu::util::DeviceExt;

/// Colour format the viewport renders in: 8-bit RGBA, sRGB-encoded, so
/// shaders work in linear colour and the GPU encodes on write.
pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
/// Format of [`ViewportTarget::display_view`]: the same bytes, read as plain
/// RGBA without sRGB decoding. egui expects this for native textures; giving
/// it the sRGB view would decode the colours twice and darken the image.
pub const DISPLAY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// Depth buffer format.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// The viewport shaders, in WGSL.
pub const SHADER_SOURCE: &str = include_str!("viewport.wgsl");

/// Picks the multisample count for smooth edges: 4 when the GPU supports it
/// for both colour and depth, otherwise 1 (no multisampling).
pub fn best_sample_count(adapter: &wgpu::Adapter) -> u32 {
    let supports = |format| {
        adapter
            .get_texture_format_features(format)
            .flags
            .sample_count_supported(4)
    };
    if supports(COLOR_FORMAT) && supports(DEPTH_FORMAT) {
        4
    } else {
        1
    }
}

/// GPU images one viewport draws into. Create with
/// [`ViewportRenderer::create_target`].
#[derive(Debug)]
pub struct ViewportTarget {
    size: [u32; 2],
    /// Multisampled colour image (only when multisampling).
    msaa_view: Option<wgpu::TextureView>,
    /// The finished, single-sample colour image.
    resolved: wgpu::Texture,
    resolved_view: wgpu::TextureView,
    display_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
}

impl ViewportTarget {
    /// Size in pixels.
    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    /// The finished colour image for showing in egui: the same pixels,
    /// viewed as plain RGBA ([`DISPLAY_FORMAT`]).
    pub fn display_view(&self) -> &wgpu::TextureView {
        &self.display_view
    }

    /// The finished colour image as a texture (for copying).
    pub fn color_texture(&self) -> &wgpu::Texture {
        &self.resolved
    }
}

/// Vertices uploaded to the GPU, with how many there are.
#[derive(Debug)]
struct GpuBuffer {
    buffer: wgpu::Buffer,
    count: u32,
}

impl GpuBuffer {
    fn new<T: bytemuck::Pod>(device: &wgpu::Device, label: &str, data: &[T]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Some(Self {
            buffer,
            count: u32::try_from(data.len()).unwrap_or(u32::MAX),
        })
    }

    fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_vertex_buffer(0, self.buffer.slice(..));
        pass.draw(0..self.count, 0..1);
    }
}

/// Everything needed to draw viewports on one GPU device.
#[derive(Debug)]
pub struct ViewportRenderer {
    sample_count: u32,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    sky_pipeline: wgpu::RenderPipeline,
    face_pipeline: wgpu::RenderPipeline,
    grid_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    pick_pipeline: wgpu::RenderPipeline,
    tint_pipeline: wgpu::RenderPipeline,
    sprite_pipeline: wgpu::RenderPipeline,
    picker: Picker,
    axes: Option<GpuBuffer>,
    faces: Option<GpuBuffer>,
    edges: Option<GpuBuffer>,
    overlay_tints: Option<GpuBuffer>,
    overlay_sprites: Option<GpuBuffer>,
}

impl ViewportRenderer {
    /// Builds shaders, pipelines and fixed geometry. `sample_count` is 1 or
    /// the value of [`best_sample_count`].
    pub fn new(device: &wgpu::Device, sample_count: u32) -> Self {
        let sample_count = if sample_count == 4 { 4 } else { 1 };
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame uniforms"),
            size: std::mem::size_of::<FrameUniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frame layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("viewport layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = |kind| create_pipeline(device, &layout, &shader, sample_count, kind);
        let mut renderer = Self {
            sample_count,
            uniform_buffer,
            bind_group,
            sky_pipeline: pipeline(PipelineKind::Sky),
            face_pipeline: pipeline(PipelineKind::Faces),
            grid_pipeline: pipeline(PipelineKind::Grid),
            line_pipeline: pipeline(PipelineKind::Lines),
            pick_pipeline: pipeline(PipelineKind::PickDepth),
            tint_pipeline: pipeline(PipelineKind::Tints),
            sprite_pipeline: pipeline(PipelineKind::Sprites),
            picker: Picker::new(device, &bind_group_layout),
            axes: GpuBuffer::new(device, "ground axes", &ground_axes(GRID_HALF_EXTENT)),
            faces: None,
            edges: None,
            overlay_tints: None,
            overlay_sprites: None,
        };
        renderer.set_shape(device, &Shape::test_cube());
        renderer
    }

    /// Replaces the shape the viewport shows. Until objects exist (Phase 2)
    /// this is the test cube.
    pub fn set_shape(&mut self, device: &wgpu::Device, shape: &Shape) {
        self.faces = GpuBuffer::new(device, "shape faces", &shape.faces);
        self.edges = GpuBuffer::new(device, "shape edges", &shape.edges);
    }

    /// Replaces the overlay drawn over the shape (the Points, Edges and
    /// Polygons modes' points, lines and tints). An empty overlay draws
    /// nothing.
    pub fn set_overlay(&mut self, device: &wgpu::Device, overlay: &Overlay) {
        self.overlay_tints = GpuBuffer::new(device, "overlay tints", &overlay.tints);
        self.overlay_sprites = GpuBuffer::new(device, "overlay sprites", &overlay.sprites);
    }

    /// Asks what is under a pixel, without waiting: the answer arrives
    /// through [`Self::poll_pick`] on a later frame. Returns `false` (and
    /// does nothing) while an earlier question is still waiting, or for a
    /// request that makes no sense.
    pub fn request_pick(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        request: PickRequest,
    ) -> bool {
        let faces = self.faces.as_ref().map(|f| (&f.buffer, f.count));
        self.picker
            .request(device, queue, &self.pick_pipeline, faces, request)
    }

    /// The answer to the last [`Self::request_pick`], if the GPU has
    /// finished. Never waits.
    pub fn poll_pick(&mut self, device: &wgpu::Device) -> Option<PickResult> {
        self.picker.poll(device, false)
    }

    /// Whether a pick question is still waiting for its answer.
    pub fn is_picking(&self) -> bool {
        self.picker.is_busy()
    }

    /// Asks what is under a pixel and waits for the answer. For tests and
    /// tools; the interface uses [`Self::request_pick`] so it never waits.
    pub fn pick_now(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        request: PickRequest,
    ) -> Option<PickResult> {
        // Finish any question already in flight first.
        let _ = self.picker.poll(device, true);
        if !self.request_pick(device, queue, request) {
            return None;
        }
        self.picker.poll(device, true)
    }

    /// The multisample count in use.
    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// Creates images to draw a viewport of `size` pixels into. Sizes are
    /// clamped to at least 1×1 and at most the GPU's largest image.
    pub fn create_target(&self, device: &wgpu::Device, size: [u32; 2]) -> ViewportTarget {
        let max = device.limits().max_texture_dimension_2d.max(1);
        let size = [size[0].clamp(1, max), size[1].clamp(1, max)];
        let extent = wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        };
        let texture = |label, format, usage, samples, view_formats: &[wgpu::TextureFormat]| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: extent,
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats,
            })
        };
        let resolved = texture(
            "viewport colour",
            COLOR_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            1,
            &[DISPLAY_FORMAT],
        );
        let msaa_view = (self.sample_count > 1).then(|| {
            texture(
                "viewport colour (multisampled)",
                COLOR_FORMAT,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
                self.sample_count,
                &[],
            )
            .create_view(&wgpu::TextureViewDescriptor::default())
        });
        let depth_view = texture(
            "viewport depth",
            DEPTH_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
            self.sample_count,
            &[],
        )
        .create_view(&wgpu::TextureViewDescriptor::default());
        let resolved_view = resolved.create_view(&wgpu::TextureViewDescriptor::default());
        let display_view = resolved.create_view(&wgpu::TextureViewDescriptor {
            label: Some("viewport display view"),
            format: Some(DISPLAY_FORMAT),
            ..Default::default()
        });
        ViewportTarget {
            size,
            msaa_view,
            resolved,
            resolved_view,
            display_view,
            depth_view,
        }
    }

    /// Draws one frame into `target` and submits it to the GPU.
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &ViewportTarget,
        params: &FrameParams,
    ) {
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&FrameUniforms::new(params).with_image_size(target.size)),
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("viewport frame"),
        });
        {
            let (view, resolve_target, store) = match &target.msaa_view {
                Some(msaa) => (msaa, Some(&target.resolved_view), wgpu::StoreOp::Discard),
                None => (&target.resolved_view, None, wgpu::StoreOp::Store),
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_pipeline(&self.sky_pipeline);
            pass.draw(0..3, 0..1);
            // Solids before the see-through grid and lines, so those are
            // hidden behind them.
            pass.set_pipeline(&self.face_pipeline);
            if let Some(faces) = &self.faces {
                faces.draw(&mut pass);
            }
            pass.set_pipeline(&self.grid_pipeline);
            pass.draw(0..6, 0..1);
            pass.set_pipeline(&self.line_pipeline);
            if let Some(axes) = &self.axes {
                axes.draw(&mut pass);
            }
            if let Some(edges) = &self.edges {
                edges.draw(&mut pass);
            }
            // The overlay last, still hidden behind surfaces in front.
            if let Some(tints) = &self.overlay_tints {
                pass.set_pipeline(&self.tint_pipeline);
                tints.draw(&mut pass);
            }
            if let Some(sprites) = &self.overlay_sprites {
                pass.set_pipeline(&self.sprite_pipeline);
                sprites.draw(&mut pass);
            }
        }
        queue.submit([encoder.finish()]);
    }
}
