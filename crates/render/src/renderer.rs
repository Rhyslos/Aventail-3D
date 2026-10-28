//! Drawing a viewport into an offscreen image on the GPU.
//!
//! Each viewport renders into its own [`ViewportTarget`] (colour and depth
//! images, with multisampling when the GPU supports it). The caller then
//! shows the finished colour image however it likes; the app hands it to
//! egui as a texture.

use crate::frame::{FrameParams, FrameUniforms, GRID_HALF_EXTENT};
use crate::geometry::{Overlay, Shape, ground_axes};
use crate::mesh_draw::{MeshDraw, MeshGeometry, ObjectUniforms};
use crate::outline::OutlineGeometry;
use crate::paint_textures::PaintMap;
use crate::pick::{PickRequest, PickResult, Picker};
use crate::pipelines::{PipelineKind, create_pipeline};
use std::collections::HashMap;
use std::ops::Range;
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
/// Format of the facing image: which way the surface at each pixel faces
/// across the screen, for cavity ([decision 0065](../../../docs/decisions/0065-cavity.md)).
pub const FACING_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
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
    /// Multisampled facing image (only when multisampling).
    facing_msaa_view: Option<wgpu::TextureView>,
    /// The finished facing image, which cavity reads.
    facing_view: wgpu::TextureView,
    /// Cavity's way to read it.
    cavity_bind_group: wgpu::BindGroup,
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

/// One object's mesh on the GPU.
#[derive(Debug)]
struct GpuMesh {
    vertices: wgpu::Buffer,
    vertex_count: u32,
    edge_length: f32,
    triangles: Option<GpuIndices>,
    /// Each material slot's part of `triangles` (decision 0177).
    slot_ranges: Vec<(u32, Range<u32>)>,
    edges: Option<GpuIndices>,
}

/// An outline shell on the GPU (decision 0201).
#[derive(Debug)]
struct GpuOutline {
    vertices: wgpu::Buffer,
    triangles: GpuIndices,
}

/// Vertex numbers on the GPU, with how many there are.
#[derive(Debug)]
struct GpuIndices {
    buffer: wgpu::Buffer,
    count: u32,
}

impl GpuIndices {
    fn new(device: &wgpu::Device, label: &str, data: &[u32]) -> Option<Self> {
        (!data.is_empty()).then(|| Self {
            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::INDEX,
            }),
            count: u32::try_from(data.len()).unwrap_or(u32::MAX),
        })
    }
}

/// The objects to draw this frame: each one's placement in the uniform
/// buffer, where it starts.
#[derive(Debug)]
struct Placements {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// How many objects fit.
    capacity: usize,
    draws: Vec<(MeshDraw, u32)>,
}

/// Which part of the object meshes to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MeshPart {
    Faces,
    Edges,
    Points,
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
    on_top_pipeline: wgpu::RenderPipeline,
    picker: Picker,
    axes: Option<GpuBuffer>,
    faces: Option<GpuBuffer>,
    edges: Option<GpuBuffer>,
    overlay_tints: Option<GpuBuffer>,
    overlay_sprites: Option<GpuBuffer>,
    overlay_on_top: Option<GpuBuffer>,
    mesh_face_pipeline: wgpu::RenderPipeline,
    mesh_pick_pipeline: wgpu::RenderPipeline,
    mesh_line_pipeline: wgpu::RenderPipeline,
    mesh_point_pipeline: wgpu::RenderPipeline,
    cavity_pipeline: wgpu::RenderPipeline,
    outline_pipeline: wgpu::RenderPipeline,
    /// Outline shells by mesh key (decision 0201).
    outlines: HashMap<u64, GpuOutline>,
    /// How cavity reads a target's facing image.
    cavity_layout: wgpu::BindGroupLayout,
    object_layout: wgpu::BindGroupLayout,
    /// The numbered checker image (decision 0107) and how it is read.
    grid_view: wgpu::TextureView,
    grid_sampler: wgpu::Sampler,
    /// Bytes between objects' placements in the uniform buffer (the GPU's
    /// alignment for them).
    object_stride: u64,
    meshes: HashMap<u64, GpuMesh>,
    placements: Option<Placements>,
    /// The painted textures (decision 0177).
    paint: crate::paint_textures::PaintTextures,
    /// Whether objects' faces are drawn (off for a wireframe look, where
    /// only their edges show, so every edge can be seen through the shape).
    show_faces: bool,
    /// Whether ridges are brightened and creases darkened (cavity).
    show_cavity: bool,
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
        let object_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("object layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<ObjectUniforms>() as u64,
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // A painted surface's roughness and metal, and its glow
                // (decision 0207).
                texture_entry(3),
                texture_entry(4),
            ],
        });
        let object_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("object layout"),
                bind_group_layouts: &[Some(&bind_group_layout), Some(&object_layout)],
                immediate_size: 0,
            });
        let cavity_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cavity layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let cavity_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("cavity pipeline layout"),
                bind_group_layouts: &[Some(&bind_group_layout), Some(&cavity_layout)],
                immediate_size: 0,
            });
        let pipeline = |kind: PipelineKind| {
            let layout = if kind.per_object() {
                &object_pipeline_layout
            } else if kind == PipelineKind::Cavity {
                &cavity_pipeline_layout
            } else {
                &layout
            };
            create_pipeline(device, layout, &shader, sample_count, kind)
        };
        let alignment = u64::from(device.limits().min_uniform_buffer_offset_alignment).max(1);
        let size = std::mem::size_of::<ObjectUniforms>() as u64;
        let object_stride = size.div_ceil(alignment) * alignment;
        let grid_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("numbered checker sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        // Until an image is given: a black stand-in.
        let grid_view = grid_texture(device, None, 1, &[0; 4]);
        let mut renderer = Self {
            sample_count,
            grid_view,
            grid_sampler,
            uniform_buffer,
            bind_group,
            sky_pipeline: pipeline(PipelineKind::Sky),
            face_pipeline: pipeline(PipelineKind::Faces),
            grid_pipeline: pipeline(PipelineKind::Grid),
            line_pipeline: pipeline(PipelineKind::Lines),
            pick_pipeline: pipeline(PipelineKind::PickDepth),
            tint_pipeline: pipeline(PipelineKind::Tints),
            sprite_pipeline: pipeline(PipelineKind::Sprites),
            on_top_pipeline: pipeline(PipelineKind::SpritesOnTop),
            picker: Picker::new(device, &bind_group_layout),
            axes: GpuBuffer::new(device, "ground axes", &ground_axes(GRID_HALF_EXTENT)),
            faces: None,
            edges: None,
            overlay_tints: None,
            overlay_sprites: None,
            overlay_on_top: None,
            mesh_face_pipeline: pipeline(PipelineKind::MeshFaces),
            mesh_pick_pipeline: pipeline(PipelineKind::MeshPickDepth),
            mesh_line_pipeline: pipeline(PipelineKind::MeshLines),
            mesh_point_pipeline: pipeline(PipelineKind::MeshPoints),
            cavity_pipeline: pipeline(PipelineKind::Cavity),
            outline_pipeline: pipeline(PipelineKind::MeshOutline),
            outlines: HashMap::new(),
            cavity_layout,
            object_layout,
            object_stride,
            meshes: HashMap::new(),
            placements: None,
            paint: crate::paint_textures::PaintTextures::new(device, None),
            show_faces: true,
            show_cavity: false,
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

    /// Uploads an object's mesh under `key`, replacing any mesh it had.
    /// It is drawn where [`Self::set_draws`] places it.
    pub fn set_mesh(&mut self, device: &wgpu::Device, key: u64, geometry: &MeshGeometry) {
        if geometry.vertices.is_empty() {
            self.meshes.remove(&key);
            return;
        }
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("object vertices"),
            contents: bytemuck::cast_slice(&geometry.vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        self.meshes.insert(
            key,
            GpuMesh {
                vertices,
                vertex_count: u32::try_from(geometry.vertices.len()).unwrap_or(u32::MAX),
                edge_length: geometry.edge_length,
                triangles: GpuIndices::new(device, "object triangles", &geometry.triangles),
                slot_ranges: geometry.slot_ranges.clone(),
                edges: GpuIndices::new(device, "object edges", &geometry.edges),
            },
        );
    }

    /// Uploads only `ranges` of an object's vertices, after
    /// [`MeshGeometry::moved`]. Does nothing if the mesh under `key` has a
    /// different number of vertices (upload it whole instead).
    pub fn update_mesh(
        &mut self,
        queue: &wgpu::Queue,
        key: u64,
        geometry: &MeshGeometry,
        ranges: &[Range<usize>],
    ) {
        let Some(mesh) = self.meshes.get(&key) else {
            return;
        };
        if mesh.vertex_count as usize != geometry.vertices.len() {
            return;
        }
        let stride = std::mem::size_of::<crate::MeshVertex>() as u64;
        for range in ranges {
            let Some(part) = geometry.vertices.get(range.clone()) else {
                continue;
            };
            queue.write_buffer(
                &mesh.vertices,
                range.start as u64 * stride,
                bytemuck::cast_slice(part),
            );
        }
    }

    /// Uploads the outline shell of the mesh under `key` (decision 0201),
    /// replacing any it had: draws of that mesh with an outline use it.
    /// An empty shell removes it.
    pub fn set_outline(&mut self, device: &wgpu::Device, key: u64, shell: &OutlineGeometry) {
        let Some(triangles) = GpuIndices::new(device, "outline triangles", &shell.triangles) else {
            self.outlines.remove(&key);
            return;
        };
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("outline vertices"),
            contents: bytemuck::cast_slice(&shell.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.outlines.insert(
            key,
            GpuOutline {
                vertices,
                triangles,
            },
        );
    }

    /// Forgets the outline shell under `key`.
    pub fn remove_outline(&mut self, key: u64) {
        self.outlines.remove(&key);
    }

    /// The keys of the outline shells held, in order.
    pub fn outline_keys(&self) -> Vec<u64> {
        let mut keys: Vec<u64> = self.outlines.keys().copied().collect();
        keys.sort_unstable();
        keys
    }

    /// Forgets every object mesh except those under `keys`.
    pub fn keep_meshes(&mut self, keys: &[u64]) {
        self.outlines.retain(|key, _| keys.contains(key));
        self.meshes.retain(|key, _| keys.contains(key));
    }

    /// How many object meshes are on the GPU.
    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    /// The bind group of the objects' placements in `buffer`, with the
    /// numbered checker image.
    fn placements_bind_group(
        &self,
        device: &wgpu::Device,
        buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        object_bind_group(
            device,
            &self.object_layout,
            buffer,
            [&self.grid_view; 3],
            &self.grid_sampler,
        )
    }

    /// The numbered checker image (decision 0107): `size` pixels square,
    /// four bytes a pixel (sRGB), its first row the top of the texture
    /// square. Smaller copies for far away are made from it.
    pub fn set_grid_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: u32,
        rgba: &[u8],
    ) {
        if size == 0 || rgba.len() != (size as usize).pow(2) * 4 {
            return;
        }
        self.grid_view = grid_texture(device, Some(queue), size, rgba);
        if let Some(placements) = &self.placements {
            let bind_group = self.placements_bind_group(device, &placements.buffer);
            if let Some(placements) = &mut self.placements {
                placements.bind_group = bind_group;
            }
        }
    }

    /// Which object meshes to draw from now on, where, and how.
    pub fn set_draws(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, draws: &[MeshDraw]) {
        let needed = draws.len().max(1);
        if self.placements.as_ref().is_none_or(|p| p.capacity < needed) {
            let capacity = needed.next_power_of_two();
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("object placements"),
                size: self.object_stride * capacity as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind_group = self.placements_bind_group(device, &buffer);
            self.placements = Some(Placements {
                buffer,
                bind_group,
                capacity,
                draws: Vec::new(),
            });
            self.bind_paint(device, true);
        }
        let Some(placements) = self.placements.as_mut() else {
            return;
        };
        let stride = self.object_stride as usize;
        let mut bytes = vec![0u8; stride * draws.len()];
        placements.draws.clear();
        for (i, draw) in draws.iter().enumerate() {
            let edge_length = self.meshes.get(&draw.key).map_or(0.0, |m| m.edge_length);
            let uniforms = ObjectUniforms::new(draw, edge_length);
            let raw = bytemuck::bytes_of(&uniforms);
            bytes[i * stride..i * stride + raw.len()].copy_from_slice(raw);
            placements.draws.push((*draw, (i * stride) as u32));
        }
        if !bytes.is_empty() {
            queue.write_buffer(&placements.buffer, 0, &bytes);
        }
    }

    /// Whether the next frames draw objects' faces (on as standard). With
    /// them off, only edges and points show, and every edge is seen
    /// through the shape: a wireframe. Picking still finds the faces.
    pub fn set_show_faces(&mut self, show: bool) {
        self.show_faces = show;
    }

    /// Whether objects' faces are drawn.
    pub fn shows_faces(&self) -> bool {
        self.show_faces
    }

    /// Whether the next frames brighten ridges and darken creases on the
    /// surfaces (cavity, off as standard here; [decision 0065](../../../docs/decisions/0065-cavity.md)).
    pub fn set_show_cavity(&mut self, show: bool) {
        self.show_cavity = show;
    }

    /// Draws every placed object mesh with `part`.
    fn draw_meshes(&self, pass: &mut wgpu::RenderPass<'_>, part: MeshPart) {
        draw_meshes(
            &self.meshes,
            self.placements.as_ref(),
            Some(&self.paint),
            pass,
            part,
        );
    }

    /// Sets painted texture `key` (decision 0177) to `rgba`: `size`
    /// pixels square, four bytes a pixel (sRGB), its first row the top of
    /// the texture square. Draws whose surface is
    /// [`crate::SurfaceDraw::Painted`] name it by `key` (not 0).
    pub fn set_paint_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: u64,
        size: u32,
        rgba: &[u8],
    ) {
        self.set_paint_map(device, queue, key, PaintMap::Colour, size, rgba);
    }

    /// Sets map `map` of painted texture `key` (decision 0207) to `rgba`,
    /// as [`Self::set_paint_texture`] does the colour: the surface map
    /// holds roughness in red and metal in green (linear, alpha 255 where
    /// painted), the glow map the light given off (sRGB).
    pub fn set_paint_map(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: u64,
        map: PaintMap,
        size: u32,
        rgba: &[u8],
    ) {
        self.paint.set(device, queue, key, map, size, rgba);
        self.bind_paint(device, false);
    }

    /// Replaces an area of map `map` of painted texture `key`, as
    /// [`Self::update_paint_texture`] does the colour.
    #[allow(clippy::too_many_arguments)]
    pub fn update_paint_map(
        &mut self,
        queue: &wgpu::Queue,
        key: u64,
        map: PaintMap,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) {
        self.paint
            .update(queue, key, map, x, y, width, height, rgba);
    }

    /// Forgets map `map` of painted texture `key` (the surface or glow;
    /// the colour forgets them all).
    pub fn remove_paint_map(&mut self, device: &wgpu::Device, key: u64, map: PaintMap) {
        self.paint.remove(key, map);
        self.bind_paint(device, false);
    }

    /// The painted textures uploaded holding map `map`, by key.
    pub fn paint_map_keys(&self, map: PaintMap) -> Vec<u64> {
        self.paint.keys(map)
    }

    /// Replaces an area of painted texture `key`: from column `x`, row
    /// `y`, `width` × `height` pixels of `rgba`. Ignored if it doesn't
    /// fit.
    #[allow(clippy::too_many_arguments)]
    pub fn update_paint_texture(
        &mut self,
        queue: &wgpu::Queue,
        key: u64,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) {
        self.paint
            .update(queue, key, PaintMap::Colour, x, y, width, height, rgba);
    }

    /// Forgets painted texture `key`, with its other maps.
    pub fn remove_paint_texture(&mut self, key: u64) {
        self.paint.remove(key, PaintMap::Colour);
    }

    /// The painted textures uploaded, by key.
    pub fn paint_texture_keys(&self) -> Vec<u64> {
        self.paint.keys(PaintMap::Colour)
    }

    /// Makes the painted textures' bind groups with the placements
    /// buffer (all of them again with `again`).
    fn bind_paint(&mut self, device: &wgpu::Device, again: bool) {
        let Some(placements) = &self.placements else {
            return;
        };
        let (layout, sampler, buffer) =
            (&self.object_layout, &self.grid_sampler, &placements.buffer);
        self.paint.bind(again, |views| {
            object_bind_group(device, layout, buffer, views, sampler)
        });
    }

    /// Replaces the overlay drawn over the shape (the Points, Edges and
    /// Polygons modes' points, lines and tints). An empty overlay draws
    /// nothing.
    pub fn set_overlay(&mut self, device: &wgpu::Device, overlay: &Overlay) {
        self.overlay_tints = GpuBuffer::new(device, "overlay tints", &overlay.tints);
        self.overlay_sprites = GpuBuffer::new(device, "overlay sprites", &overlay.sprites);
        self.overlay_on_top = GpuBuffer::new(device, "overlay on top", &overlay.on_top);
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
        let (all, placements, pipeline) = (
            &self.meshes,
            self.placements.as_ref(),
            &self.mesh_pick_pipeline,
        );
        let meshes = |pass: &mut wgpu::RenderPass<'_>| {
            pass.set_pipeline(pipeline);
            draw_meshes(all, placements, None, pass, MeshPart::Faces);
        };
        self.picker
            .request(device, queue, &self.pick_pipeline, faces, &meshes, request)
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
        let facing_view = texture(
            "viewport facing",
            FACING_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            1,
            &[],
        )
        .create_view(&wgpu::TextureViewDescriptor::default());
        let facing_msaa_view = (self.sample_count > 1).then(|| {
            texture(
                "viewport facing (multisampled)",
                FACING_FORMAT,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
                self.sample_count,
                &[],
            )
            .create_view(&wgpu::TextureViewDescriptor::default())
        });
        let cavity_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cavity bind group"),
            layout: &self.cavity_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&facing_view),
            }],
        });
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
            facing_msaa_view,
            facing_view,
            cavity_bind_group,
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
        // With multisampling, drawing goes into the multisampled images,
        // which are smoothed into the finished ones at the end.
        let color = target.msaa_view.as_ref().unwrap_or(&target.resolved_view);
        let smoothed = target.msaa_view.is_some().then_some(&target.resolved_view);
        // First the backdrop and the surfaces, which also record which way
        // they face at each pixel, for cavity.
        {
            let (facing, facing_resolve) = match &target.facing_msaa_view {
                Some(msaa) => (msaa, Some(&target.facing_view)),
                None => (&target.facing_view, None),
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport surfaces"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: color,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: facing,
                        depth_slice: None,
                        resolve_target: facing_resolve,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: if facing_resolve.is_some() {
                                wgpu::StoreOp::Discard
                            } else {
                                wgpu::StoreOp::Store
                            },
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
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
            pass.set_pipeline(&self.face_pipeline);
            if let Some(faces) = &self.faces {
                faces.draw(&mut pass);
            }
            if self.show_faces {
                pass.set_pipeline(&self.mesh_face_pipeline);
                self.draw_meshes(&mut pass, MeshPart::Faces);
                // Outlines round them (decision 0201).
                if let Some(placements) = &self.placements {
                    pass.set_pipeline(&self.outline_pipeline);
                    for (draw, offset) in &placements.draws {
                        let Some(shell) = self
                            .outlines
                            .get(&draw.key)
                            .filter(|_| draw.faces && draw.outline.is_some_and(|(_, w)| w > 0.0))
                        else {
                            continue;
                        };
                        pass.set_bind_group(1, &placements.bind_group, &[*offset]);
                        pass.set_vertex_buffer(0, shell.vertices.slice(..));
                        pass.set_index_buffer(
                            shell.triangles.buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        pass.draw_indexed(0..shell.triangles.count, 0, 0..1);
                    }
                }
            }
        }
        // Then cavity, on the surfaces only, before anything is drawn over
        // them.
        if self.show_cavity && self.show_faces {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport cavity"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_bind_group(1, &target.cavity_bind_group, &[]);
            pass.set_pipeline(&self.cavity_pipeline);
            pass.draw(0..3, 0..1);
        }
        // Last the see-through grid, lines and overlay, hidden behind the
        // surfaces in front of them.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport lines"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color,
                    depth_slice: None,
                    resolve_target: smoothed,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: if smoothed.is_some() {
                            wgpu::StoreOp::Discard
                        } else {
                            wgpu::StoreOp::Store
                        },
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_pipeline(&self.grid_pipeline);
            pass.draw(0..6, 0..1);
            pass.set_pipeline(&self.line_pipeline);
            if let Some(axes) = &self.axes {
                axes.draw(&mut pass);
            }
            if let Some(edges) = &self.edges {
                edges.draw(&mut pass);
            }
            pass.set_pipeline(&self.mesh_line_pipeline);
            self.draw_meshes(&mut pass, MeshPart::Edges);
            // The overlay last, still hidden behind surfaces in front.
            if let Some(tints) = &self.overlay_tints {
                pass.set_pipeline(&self.tint_pipeline);
                tints.draw(&mut pass);
            }
            pass.set_pipeline(&self.mesh_point_pipeline);
            self.draw_meshes(&mut pass, MeshPart::Points);
            if let Some(sprites) = &self.overlay_sprites {
                pass.set_pipeline(&self.sprite_pipeline);
                pass.set_vertex_buffer(0, sprites.buffer.slice(..));
                pass.draw(0..6, 0..sprites.count);
            }
            // Helpers that run through shapes, over everything.
            if let Some(on_top) = &self.overlay_on_top {
                pass.set_pipeline(&self.on_top_pipeline);
                pass.set_vertex_buffer(0, on_top.buffer.slice(..));
                pass.draw(0..6, 0..on_top.count);
            }
        }
        queue.submit([encoder.finish()]);
    }
}

/// The object layout's entry for a texture at `binding`.
fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

/// A bind group of the objects' placements in `buffer` with the textures
/// `views` (the colour or numbered checker, the surface and the glow),
/// read through `sampler`.
fn object_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
    views: [&wgpu::TextureView; 3],
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    let [view, surface, glow] = views;
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("object placements"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(std::mem::size_of::<ObjectUniforms>() as u64),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(surface),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(glow),
            },
        ],
    })
}

/// Draws every placed object mesh with `part`.
fn draw_meshes(
    meshes: &HashMap<u64, GpuMesh>,
    placements: Option<&Placements>,
    paint: Option<&crate::paint_textures::PaintTextures>,
    pass: &mut wgpu::RenderPass<'_>,
    part: MeshPart,
) {
    let Some(placements) = placements else {
        return;
    };
    for (draw, offset) in &placements.draws {
        let Some(mesh) = meshes.get(&draw.key) else {
            continue;
        };
        pass.set_bind_group(1, &placements.bind_group, &[*offset]);
        match part {
            MeshPart::Faces => {
                if let Some(triangles) = mesh.triangles.as_ref().filter(|_| draw.faces) {
                    pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    pass.set_index_buffer(triangles.buffer.slice(..), wgpu::IndexFormat::Uint32);
                    match (draw.surface, paint) {
                        // Each slot's part with its own texture (decision
                        // 0177).
                        (crate::SurfaceDraw::Painted { textures, .. }, Some(paint))
                            if !mesh.slot_ranges.is_empty() =>
                        {
                            for (slot, range) in &mesh.slot_ranges {
                                let key = textures.get(*slot as usize).copied().unwrap_or(0);
                                if let Some(group) = paint.group(key) {
                                    pass.set_bind_group(1, group, &[*offset]);
                                }
                                pass.draw_indexed(range.clone(), 0, 0..1);
                            }
                        }
                        _ => pass.draw_indexed(0..triangles.count, 0, 0..1),
                    }
                }
            }
            MeshPart::Edges => {
                if let Some(edges) = mesh.edges.as_ref().filter(|_| draw.edges) {
                    pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    pass.set_index_buffer(edges.buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..edges.count, 0, 0..1);
                }
            }
            MeshPart::Points => {
                if draw.points.is_some() {
                    pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    pass.draw(0..6, 0..mesh.vertex_count);
                }
            }
        }
    }
}

/// A texture of `size` square holding `rgba` (sRGB), with every smaller
/// copy down to one pixel (each the average of four pixels of the one
/// before); written through `queue`, or left black without one (the
/// stand-in until an image is given).
fn grid_texture(
    device: &wgpu::Device,
    queue: Option<&wgpu::Queue>,
    size: u32,
    rgba: &[u8],
) -> wgpu::TextureView {
    let levels = 32 - size.leading_zeros();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("numbered checker"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: levels,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    if let Some(queue) = queue {
        let mut level = rgba.to_vec();
        let mut side = size;
        for mip in 0..levels {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: mip,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &level,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(side * 4),
                    rows_per_image: Some(side),
                },
                wgpu::Extent3d {
                    width: side,
                    height: side,
                    depth_or_array_layers: 1,
                },
            );
            if side == 1 {
                break;
            }
            level = halved(&level, side as usize);
            side /= 2;
        }
    }
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// An image of `side` square at half the size: each pixel the average of
/// four.
fn halved(rgba: &[u8], side: usize) -> Vec<u8> {
    let half = (side / 2).max(1);
    let mut out = vec![0u8; half * half * 4];
    for y in 0..half {
        for x in 0..half {
            for c in 0..4 {
                let at = |xx: usize, yy: usize| {
                    u32::from(rgba[((yy.min(side - 1)) * side + xx.min(side - 1)) * 4 + c])
                };
                let sum = at(2 * x, 2 * y)
                    + at(2 * x + 1, 2 * y)
                    + at(2 * x, 2 * y + 1)
                    + at(2 * x + 1, 2 * y + 1);
                out[(y * half + x) * 4 + c] = ((sum + 2) / 4) as u8;
            }
        }
    }
    out
}
