//! The render pipelines: how each kind of geometry is drawn.

use crate::geometry::{FaceVertex, LineVertex, Sprite};
use crate::mesh_draw::MeshVertex;
use crate::renderer::{COLOR_FORMAT, DEPTH_FORMAT, FACING_FORMAT};

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
    /// The overlay's see-through tinted triangles, laid on faces.
    Tints,
    /// The overlay's points and thick lines, a fixed size on screen.
    Sprites,
    /// The same, drawn over everything, never hidden: helpers that run
    /// through a shape (a deformer's axis, decision 0158).
    SpritesOnTop,
    /// An object's mesh, placed in the world: its faces.
    MeshFaces,
    /// The same, for depth only (picking).
    MeshPickDepth,
    /// An object's edges.
    MeshLines,
    /// Every point of an object as a small dot.
    MeshPoints,
    /// Cavity: one triangle over the view, brightening ridges and
    /// darkening creases (reads the facing image, bind group 1).
    Cavity,
    /// An object's outline shell (decision 0201).
    MeshOutline,
}

impl PipelineKind {
    /// Whether it draws an object's mesh, and so reads the object's
    /// placement (bind group 1).
    pub(crate) fn per_object(self) -> bool {
        matches!(
            self,
            Self::MeshFaces
                | Self::MeshPickDepth
                | Self::MeshLines
                | Self::MeshPoints
                | Self::MeshOutline
        )
    }

    /// Whether it is drawn in the first stage, with the surfaces: it also
    /// writes the facing image cavity reads.
    pub(crate) fn writes_facing(self) -> bool {
        matches!(
            self,
            Self::Sky | Self::Faces | Self::MeshFaces | Self::MeshOutline
        )
    }
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
    let sprite_buffers: &[Option<wgpu::VertexBufferLayout<'static>>] = &[Some(Sprite::LAYOUT)];
    let mesh_buffers: &[Option<wgpu::VertexBufferLayout<'static>>] = &[Some(MeshVertex::LAYOUT)];
    let point_buffers: &[Option<wgpu::VertexBufferLayout<'static>>] =
        &[Some(MeshVertex::POINT_LAYOUT)];
    let outline_buffers: &[Option<wgpu::VertexBufferLayout<'static>>] =
        &[Some(crate::outline::OutlineVertex::LAYOUT)];
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
        PipelineKind::Tints => (
            "overlay tint pipeline",
            "line_vs",
            "line_fs",
            wgpu::PrimitiveTopology::TriangleList,
            line_buffers,
        ),
        PipelineKind::Sprites | PipelineKind::SpritesOnTop => (
            "overlay sprite pipeline",
            "sprite_vs",
            "sprite_fs",
            wgpu::PrimitiveTopology::TriangleList,
            sprite_buffers,
        ),
        PipelineKind::MeshFaces | PipelineKind::MeshPickDepth => (
            if kind == PipelineKind::MeshFaces {
                "object face pipeline"
            } else {
                "object pick depth pipeline"
            },
            "mesh_face_vs",
            "mesh_face_fs",
            wgpu::PrimitiveTopology::TriangleList,
            mesh_buffers,
        ),
        PipelineKind::MeshLines => (
            "object edge pipeline",
            "mesh_line_vs",
            "line_fs",
            wgpu::PrimitiveTopology::LineList,
            mesh_buffers,
        ),
        PipelineKind::MeshPoints => (
            "object point pipeline",
            "mesh_point_vs",
            "sprite_fs",
            wgpu::PrimitiveTopology::TriangleList,
            point_buffers,
        ),
        PipelineKind::MeshOutline => (
            "object outline pipeline",
            "outline_vs",
            "outline_fs",
            wgpu::PrimitiveTopology::TriangleList,
            outline_buffers,
        ),
        PipelineKind::Cavity => (
            "cavity pipeline",
            "cavity_vs",
            "cavity_fs",
            wgpu::PrimitiveTopology::TriangleList,
            &[][..],
        ),
    };
    let blend = matches!(
        kind,
        PipelineKind::Grid
            | PipelineKind::Lines
            | PipelineKind::Tints
            | PipelineKind::Sprites
            | PipelineKind::SpritesOnTop
            | PipelineKind::MeshLines
            | PipelineKind::MeshPoints
            | PipelineKind::Cavity
    )
    .then_some(wgpu::BlendState::ALPHA_BLENDING);
    let picking = matches!(kind, PipelineKind::PickDepth | PipelineKind::MeshPickDepth);
    let faces = matches!(
        kind,
        PipelineKind::Faces
            | PipelineKind::PickDepth
            | PipelineKind::MeshFaces
            | PipelineKind::MeshPickDepth
            | PipelineKind::MeshOutline
    );
    let depth_write = faces;
    let depth_compare = if matches!(kind, PipelineKind::Sky | PipelineKind::SpritesOnTop) {
        wgpu::CompareFunction::Always
    } else {
        wgpu::CompareFunction::LessEqual
    };
    // Faces are pushed very slightly back so their own edges, at exactly
    // the same depth, are drawn cleanly on top instead of flickering. The
    // grid is pushed back further still, so a shape lying on the ground
    // (a plane, a disc) hides the grid under it.
    let bias = match kind {
        PipelineKind::Faces | PipelineKind::MeshFaces => wgpu::DepthBiasState {
            constant: 4,
            slope_scale: 2.0,
            clamp: 0.0,
        },
        PipelineKind::Grid => wgpu::DepthBiasState {
            constant: 16,
            slope_scale: 6.0,
            clamp: 0.0,
        },
        _ => wgpu::DepthBiasState::default(),
    };
    let color = Some(wgpu::ColorTargetState {
        format: COLOR_FORMAT,
        blend,
        write_mask: wgpu::ColorWrites::ALL,
    });
    let color_only = [color.clone()];
    let with_facing = [
        color,
        Some(wgpu::ColorTargetState {
            format: FACING_FORMAT,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        }),
    ];
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
            // Both sides of every face are drawn: flat and open shapes (a
            // plane, a cylinder without ends) must show from behind too.
            cull_mode: None,
            ..Default::default()
        },
        // Cavity is drawn in a stage of its own, with no depth.
        depth_stencil: (kind != PipelineKind::Cavity).then_some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(depth_write),
            depth_compare: Some(depth_compare),
            stencil: wgpu::StencilState::default(),
            bias,
        }),
        multisample: wgpu::MultisampleState {
            count: if picking { 1 } else { sample_count },
            ..Default::default()
        },
        // Picking only needs depth: no colour, no fragment shader.
        fragment: (!picking).then_some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fs),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: if kind.writes_facing() {
                &with_facing
            } else {
                &color_only
            },
        }),
        multiview_mask: None,
        cache: None,
    })
}
