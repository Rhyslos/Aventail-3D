//! The render pipelines: how each kind of geometry is drawn.

use crate::geometry::{FaceVertex, LineVertex};
use crate::renderer::{COLOR_FORMAT, DEPTH_FORMAT};

/// The kinds of geometry the viewport draws, in drawing order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PipelineKind {
    /// The background gradient, behind everything.
    Sky,
    /// Solid, shaded faces.
    Faces,
    /// The ground grid: see-through, hidden behind solids.
    Grid,
    /// See-through lines hidden behind solids: axes and edges.
    Lines,
    /// Faces drawn for depth only, into the one-pixel image used to find
    /// the point under the mouse (see `pick`).
    PickDepth,
}

/// Builds the pipeline for one kind of geometry.
pub(crate) fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    sample_count: u32,
    kind: PipelineKind,
) -> wgpu::RenderPipeline {
    let line_buffers: &[Option<wgpu::VertexBufferLayout<'static>>] = &[Some(LineVertex::LAYOUT)];
    let face_buffers: &[Option<wgpu::VertexBufferLayout<'static>>] = &[Some(FaceVertex::LAYOUT)];
    let (label, vs, fs, topology, buffers) = match kind {
        PipelineKind::Sky => (
            "sky pipeline",
            "sky_vs",
            "sky_fs",
            wgpu::PrimitiveTopology::TriangleList,
            &[][..],
        ),
        PipelineKind::Faces | PipelineKind::PickDepth => (
            if kind == PipelineKind::Faces {
                "face pipeline"
            } else {
                "pick depth pipeline"
            },
            "face_vs",
            "face_fs",
            wgpu::PrimitiveTopology::TriangleList,
            face_buffers,
        ),
        PipelineKind::Grid => (
            "grid pipeline",
            "grid_vs",
            "grid_fs",
            wgpu::PrimitiveTopology::TriangleList,
            &[][..],
        ),
        PipelineKind::Lines => (
            "line pipeline",
            "line_vs",
            "line_fs",
            wgpu::PrimitiveTopology::LineList,
            line_buffers,
        ),
    };
    let blend = matches!(kind, PipelineKind::Grid | PipelineKind::Lines)
        .then_some(wgpu::BlendState::ALPHA_BLENDING);
    let faces = matches!(kind, PipelineKind::Faces | PipelineKind::PickDepth);
    let depth_write = faces;
    let depth_compare = if kind == PipelineKind::Sky {
        wgpu::CompareFunction::Always
    } else {
        wgpu::CompareFunction::LessEqual
    };
    // Faces are pushed very slightly back so their own edges, at exactly
    // the same depth, are drawn cleanly on top instead of flickering.
    let bias = if kind == PipelineKind::Faces {
        wgpu::DepthBiasState {
            constant: 4,
            slope_scale: 2.0,
            clamp: 0.0,
        }
    } else {
        wgpu::DepthBiasState::default()
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vs),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers,
        },
        primitive: wgpu::PrimitiveState {
            topology,
            // Closed shapes are wound outwards, so their back faces never
            // show; skipping them halves the work.
            cull_mode: faces.then_some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(depth_write),
            depth_compare: Some(depth_compare),
            stencil: wgpu::StencilState::default(),
            bias,
        }),
        multisample: wgpu::MultisampleState {
            count: if kind == PipelineKind::PickDepth {
                1
            } else {
                sample_count
            },
            ..Default::default()
        },
        // Picking only needs depth: no colour, no fragment shader.
        fragment: (kind != PipelineKind::PickDepth).then_some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fs),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}
