//! Finding the 3D point under the mouse.
//!
//! The scene's faces are drawn once more, into a one-pixel depth image
//! whose camera is narrowed to exactly the pixel under the mouse. Reading
//! that single depth value back and undoing the camera's projection gives
//! the world position of whatever surface is there.
//!
//! Reading from the GPU takes a moment, so it never holds up a frame: ask
//! with [`Picker::request`], and collect the answer with [`Picker::poll`]
//! on a later frame (usually the next). Only one question is in flight at
//! a time.

use crate::frame::{FrameParams, FrameUniforms};
use crate::renderer::DEPTH_FORMAT;
use glam::{Mat4, Vec2, Vec3, Vec4};
use std::sync::{Arc, Mutex};

/// What to look for: the pixel under the mouse in a viewport image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickRequest {
    /// The camera's world-to-screen transform, as used for drawing.
    pub view_projection: Mat4,
    /// The pixel to look under, in image pixels from the top-left corner.
    pub pixel: [f32; 2],
    /// The image's size in pixels.
    pub size: [u32; 2],
}

/// The answer to a [`PickRequest`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickResult {
    /// The pixel that was asked about.
    pub pixel: [f32; 2],
    /// Where the surface under that pixel is, or `None` when only the
    /// background is there.
    pub point: Option<Vec3>,
}

/// The pixel's position in normalised device coordinates (-1 to 1, y up),
/// or `None` for a broken request.
pub(crate) fn pixel_ndc(request: &PickRequest) -> Option<Vec2> {
    let [w, h] = request.size;
    let [x, y] = request.pixel;
    if w == 0 || h == 0 || !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some(Vec2::new(
        2.0 * x / w as f32 - 1.0,
        1.0 - 2.0 * y / h as f32,
    ))
}

/// A world-to-screen transform that shows only the requested pixel,
/// stretched over the whole (one-pixel) image.
pub(crate) fn pick_matrix(request: &PickRequest) -> Option<Mat4> {
    let ndc = pixel_ndc(request)?;
    let [w, h] = request.size.map(|v| v as f32);
    let narrow = Mat4::from_cols(
        Vec4::new(w, 0.0, 0.0, 0.0),
        Vec4::new(0.0, h, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 0.0),
        Vec4::new(-ndc.x * w, -ndc.y * h, 0.0, 1.0),
    );
    let matrix = narrow * request.view_projection;
    matrix.is_finite().then_some(matrix)
}

/// The world point at `ndc` and `depth` (0 near to 1 far), or `None` for
/// the background (depth 1, nothing drawn) or a broken camera.
pub(crate) fn unproject(view_projection: Mat4, ndc: Vec2, depth: f32) -> Option<Vec3> {
    if !(depth.is_finite() && (0.0..1.0).contains(&depth)) {
        return None;
    }
    let inverse = view_projection.inverse();
    let world = inverse * Vec4::new(ndc.x, ndc.y, depth, 1.0);
    if world.w.abs() < 1e-12 {
        return None;
    }
    let point = world.truncate() / world.w;
    point.is_finite().then_some(point)
}

/// Bytes per row for copying the one-pixel image (the GPU's minimum).
const ROW_BYTES: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

/// A question in flight.
#[derive(Debug)]
struct Pending {
    request: PickRequest,
    /// Set by the GPU's callback once the depth can be read.
    ready: Arc<Mutex<Option<bool>>>,
}

/// The GPU resources for picking, and the question in flight.
#[derive(Debug)]
pub(crate) struct Picker {
    depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    buffer: wgpu::Buffer,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pending: Option<Pending>,
}

impl Picker {
    /// Creates the one-pixel depth image, the readback buffer and the
    /// camera data for picking.
    pub(crate) fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pick depth"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pick readback"),
            size: u64::from(ROW_BYTES),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pick uniforms"),
            size: std::mem::size_of::<FrameUniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pick bind group"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        Self {
            depth,
            depth_view,
            buffer,
            uniforms,
            bind_group,
            pending: None,
        }
    }

    /// Whether a question is still waiting for its answer.
    pub(crate) fn is_busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Asks what is under `request`'s pixel, drawing `faces` with
    /// `pipeline` (the depth-only face pipeline), and then whatever
    /// `meshes` draws (the objects' meshes). Returns `false`, doing
    /// nothing, while an earlier question is still in flight or when the
    /// request makes no sense (an empty image, a broken camera).
    pub(crate) fn request(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pipeline: &wgpu::RenderPipeline,
        faces: Option<(&wgpu::Buffer, u32)>,
        meshes: &dyn Fn(&mut wgpu::RenderPass<'_>),
        request: PickRequest,
    ) -> bool {
        if self.pending.is_some() {
            return false;
        }
        let Some(matrix) = pick_matrix(&request) else {
            return false;
        };
        let params = FrameParams {
            view_projection: matrix,
            camera_position: Vec3::ZERO,
            focus: Vec3::ZERO,
            sky_top: [0.0; 3],
            sky_horizon: [0.0; 3],
            ground: [0.0; 3],
            fine_spacing: None,
            flat_height: None,
            grid_plane: crate::frame::GROUND_PLANE,
        };
        queue.write_buffer(
            &self.uniforms,
            0,
            bytemuck::bytes_of(&FrameUniforms::new(&params)),
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("pick"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("pick pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
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
            if let Some((buffer, count)) = faces {
                pass.set_pipeline(pipeline);
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..count, 0..1);
            }
            meshes(&mut pass);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.depth,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::DepthOnly,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(ROW_BYTES),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        let ready = Arc::new(Mutex::new(None));
        let flag = Arc::clone(&ready);
        self.buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                if let Ok(mut slot) = flag.lock() {
                    *slot = Some(result.is_ok());
                }
            });
        self.pending = Some(Pending { request, ready });
        true
    }

    /// Collects the answer to the question in flight, if the GPU has
    /// finished; `wait` blocks until it has (for tests and screenshots).
    pub(crate) fn poll(&mut self, device: &wgpu::Device, wait: bool) -> Option<PickResult> {
        let pending = self.pending.as_ref()?;
        let poll = if wait {
            wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            }
        } else {
            wgpu::PollType::Poll
        };
        let _ = device.poll(poll);
        let state = pending.ready.lock().ok().and_then(|slot| *slot)?;
        let pending = self.pending.take()?;
        let request = pending.request;
        let point = if state {
            let depth = self
                .buffer
                .slice(..)
                .get_mapped_range()
                .ok()
                .and_then(|bytes| bytes.get(..4).map(|b| [b[0], b[1], b[2], b[3]]))
                .map(f32::from_le_bytes);
            self.buffer.unmap();
            depth.and_then(|depth| {
                pixel_ndc(&request).and_then(|ndc| unproject(request.view_projection, ndc, depth))
            })
        } else {
            None
        };
        Some(PickResult {
            pixel: request.pixel,
            point,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera() -> Mat4 {
        tools::Camera::looking_at(Vec3::new(0.0, -500.0, 200.0), Vec3::ZERO)
            .view_projection(Vec2::new(640.0, 400.0))
    }

    fn request(pixel: [f32; 2]) -> PickRequest {
        PickRequest {
            view_projection: camera(),
            pixel,
            size: [640, 400],
        }
    }

    #[test]
    fn the_middle_pixel_is_the_middle_of_the_screen() {
        let ndc = pixel_ndc(&request([320.0, 200.0])).unwrap();
        assert!(ndc.length() < 1e-6);
        let corner = pixel_ndc(&request([0.0, 0.0])).unwrap();
        assert_eq!(corner, Vec2::new(-1.0, 1.0));
    }

    #[test]
    fn the_narrowed_camera_centres_the_chosen_pixel() {
        // A world point drawn at some pixel lands in the middle of the
        // narrowed camera's view.
        let world = Vec3::new(30.0, 20.0, 10.0);
        let clip = camera() * world.extend(1.0);
        let ndc = clip.truncate() / clip.w;
        let pixel = [(ndc.x + 1.0) * 320.0, (1.0 - ndc.y) * 200.0];
        let narrowed = pick_matrix(&request(pixel)).unwrap() * world.extend(1.0);
        let centred = narrowed.truncate() / narrowed.w;
        assert!(
            centred.x.abs() < 1e-3 && centred.y.abs() < 1e-3,
            "{centred}"
        );
        assert!((centred.z - ndc.z).abs() < 1e-6, "depth is unchanged");
    }

    #[test]
    fn unprojecting_undoes_projecting() {
        let world = Vec3::new(-40.0, 75.0, 33.0);
        let clip = camera() * world.extend(1.0);
        let ndc = clip.truncate() / clip.w;
        let back = unproject(camera(), Vec2::new(ndc.x, ndc.y), ndc.z).unwrap();
        assert!((back - world).length() < 0.05, "{back}");
    }

    #[test]
    fn the_background_and_broken_input_give_no_point() {
        assert_eq!(unproject(camera(), Vec2::ZERO, 1.0), None, "nothing drawn");
        assert_eq!(unproject(camera(), Vec2::ZERO, f32::NAN), None);
        assert_eq!(unproject(Mat4::ZERO, Vec2::ZERO, 0.5), None);
        let mut broken = request([f32::NAN, 1.0]);
        assert_eq!(pick_matrix(&broken), None);
        broken = request([1.0, 1.0]);
        broken.size = [0, 400];
        assert_eq!(pick_matrix(&broken), None);
    }
}
