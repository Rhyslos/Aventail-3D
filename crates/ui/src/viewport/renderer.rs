//! What the viewport asks of the renderer: the view to draw
//! ([`ViewportView`]), the question about what is under the mouse and its
//! answer, and the [`ViewportRenderer`] trait itself, with [`NoRenderer`]
//! for when nothing can be drawn.

use super::options::{GridDetail, Shading, Surface};
use crate::drawing::{ComponentOverlay, SceneDrawing};
use crate::text::tr;
use glam::{Mat4, Vec3};
use std::sync::Arc;

/// What a renderer needs to draw one viewport frame.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportView {
    /// Which of the four views this is (0 to 3), so a renderer can keep a
    /// separate image for each.
    pub slot: usize,
    /// Size of the image to draw, in physical pixels.
    pub size_px: [u32; 2],
    /// World-to-screen transform.
    pub view_projection: Mat4,
    /// Where the camera is.
    pub camera_position: Vec3,
    /// The point the camera looks at; the grid is drawn around it.
    pub focus: Vec3,
    /// For a flat (orthographic) view, the height it shows in centimetres;
    /// `None` for perspective.
    pub flat_height: Option<f32>,
    /// The plane the grid lies in, as two directions across it.
    pub grid_plane: [Vec3; 2],
    /// The sky straight up, sRGB 0 to 1.
    pub sky_top: [f32; 3],
    /// The sky at the horizon, sRGB 0 to 1.
    pub sky_horizon: [f32; 3],
    /// The ground just below the horizon, sRGB 0 to 1.
    pub ground: [f32; 3],
    /// How detailed the grid is.
    pub grid_detail: GridDetail,
    /// The scene's objects to draw.
    pub scene: Arc<SceneDrawing>,
    /// The colour selected objects are outlined in (the accent colour),
    /// sRGB 0 to 1.
    pub selection_color: [f32; 3],
    /// The numbered checker's two colours (decision 0107), from the
    /// accent colour: the top left cell's and the bottom right's, sRGB.
    pub grid_colors: [[u8; 3]; 2],
    /// What to draw over the edited objects in the Points, Edges and
    /// Polygons modes.
    pub overlay: Arc<ComponentOverlay>,
    /// How shapes are drawn.
    pub shading: Shading,
    /// Whether ridges are brightened and creases darkened (Cavity).
    pub cavity: bool,
    /// What the shapes' surfaces show.
    pub surface: Surface,
    /// Whether the selection is drawn larger with a dark rim (Settings →
    /// Interface → High contrast; decision 0126).
    pub high_contrast: bool,
}

/// A question for the renderer: what is under this pixel of the image?
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickAsk {
    /// The camera's world-to-screen transform for the image.
    pub view_projection: Mat4,
    /// The pixel, in physical pixels from the image's top-left corner.
    pub pixel: [f32; 2],
    /// The image's size in physical pixels.
    pub size_px: [u32; 2],
}

/// What the renderer found under a pixel it was asked about.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Picked {
    /// The pixel that was asked about, in physical pixels.
    pub pixel: [f32; 2],
    /// The surface point there, or `None` for the background.
    pub point: Option<Vec3>,
}

/// The renderer's reply about picking, each frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PickReply {
    /// An answer that has arrived since the last frame, if any.
    pub answer: Option<Picked>,
    /// Whether a question is still waiting for its answer (so the
    /// interface should draw another frame soon to collect it).
    pub waiting: bool,
}

/// Draws viewport images. The app implements this with the GPU; tests and
/// machines without a usable GPU use [`NoRenderer`].
pub trait ViewportRenderer {
    /// Draws a frame and returns the egui texture showing it, or a
    /// plain-language reason why it could not.
    fn render(&mut self, view: &ViewportView) -> Result<egui::TextureId, String>;

    /// Hands over any answer that has arrived about the point under the
    /// mouse, and asks about `ask`'s pixel if given. Never waits. A renderer
    /// that cannot pick answers nothing (the standard).
    fn pick(&mut self, ask: Option<PickAsk>) -> PickReply {
        let _ = ask;
        PickReply::default()
    }

    /// Whether a heavy shape is still being made ready to draw in the
    /// background (the viewport says so, and keeps drawing until it is).
    fn preparing(&self) -> bool {
        false
    }
}

/// A renderer that never draws, with a reason to show instead.
#[derive(Debug, Clone)]
pub struct NoRenderer {
    /// Why there is no 3D view.
    pub reason: String,
}

impl Default for NoRenderer {
    fn default() -> Self {
        Self {
            reason: tr("no graphics device").to_string(),
        }
    }
}

impl ViewportRenderer for NoRenderer {
    fn render(&mut self, _view: &ViewportView) -> Result<egui::TextureId, String> {
        Err(self.reason.clone())
    }
}
