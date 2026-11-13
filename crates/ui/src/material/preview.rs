//! The preview beside the canvas (decisions 0258 and 0260): the graph's
//! material (what Material out is given), or, after a click on a box's
//! picture, what that box makes ("looking through" it), shown on
//!
//! - the **selected model**, by its own UVs at the material's real-world
//!   size;
//! - a **ball** and a **cube** (the Sphere and Cube shapes, 1 m across),
//!   the same way;
//! - a **flat** square, repeated, with the **seam check**: three by three
//!   with the joins marked, and whether they show, in plain words.
//!
//! The 3D shapes are drawn by the views' own renderer (the painter's
//! painted surface, decision 0178) in one of the app's looks (unlit, lit,
//! toon, realistic), so the preview looks exactly as the material will
//! once given to a shape. Without a graphics card the ball falls back to
//! a picture drawn here on the processor.
//!
//! Above the picture is the options bar (decision 0291,
//! `preview_options`): the light and backdrop, comparing, and resetting
//! the view. The picture turns as the 3D view does (`turning`) and can
//! show a second material beside it (`comparing`); `preview_picture`
//! draws it all.
//!
//! Under the picture is the **texture budget** for the size chosen
//! (`budget`).

use super::MaterialEditor;
use super::budget::{Budget, SIZES};
use super::comparing::{CompareWith, Comparing, Source};
use super::preview_options::Lighting;
use super::turning::Turning;
use crate::drawing::{DrawItem, SceneDrawing, SceneSummary};
use crate::text::tr;
use crate::theme;
use crate::viewport::{PaintPreview, ViewportRenderer};
use crate::workbench::ExportSettings;
use egui::{TextureHandle, Ui};
use glam::Mat4;
use matgraph::{BoxId, Graph, SeamCheck, Value};
use scene::{Document, ObjectId};
use std::sync::Arc;

/// How many pixels across the processor-drawn ball is.
pub(super) const BALL: u32 = 256;
/// How many times the flat preview repeats the texture each way with the
/// seam check on.
pub(super) const SEAM_TILES: f32 = 3.0;
/// The preview's own objects, apart from any in the scene.
pub(super) const BALL_ID: ObjectId = ObjectId::from_raw(u64::MAX - 1);
pub(super) const CUBE_ID: ObjectId = ObjectId::from_raw(u64::MAX - 2);

/// The Preview panel's accessible names.
pub(crate) const PREVIEW_PICTURE: &str = crate::text!("Material preview");
pub(crate) const SEAM_CHECK: &str = crate::text!("Seam check");
pub(crate) const BUDGET_SIZE: &str = crate::text!("Texture size");

/// What the preview shows the picture on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PreviewShape {
    /// The selected model.
    Model,
    /// A ball.
    #[default]
    Ball,
    /// A cube.
    Cube,
    /// A flat square, repeated.
    Flat,
}

/// How the preview lights the 3D shapes: four of the app's looks
/// (decision 0198), as the plan names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PreviewLook {
    Unlit,
    #[default]
    Lit,
    Toon,
    Realistic,
}

impl PreviewLook {
    /// The views' preview it draws with.
    pub(crate) fn paint_preview(self) -> PaintPreview {
        match self {
            Self::Unlit => PaintPreview::Unlit,
            Self::Lit => PaintPreview::Lit,
            Self::Toon => PaintPreview::Toon,
            Self::Realistic => PaintPreview::Realistic,
        }
    }
}

/// The preview's own state.
#[derive(Default)]
pub(crate) struct Preview {
    /// What it shows the picture on.
    pub(crate) shape: PreviewShape,
    /// How it lights the 3D shapes.
    pub(crate) look: PreviewLook,
    /// Whether the flat preview shows the seam check.
    pub(crate) seams: bool,
    /// The size the texture budget is worked out for; the project's
    /// texture size (at most 4096) until one is chosen.
    pub(crate) budget_size: Option<u32>,
    /// The light and the backdrop (decision 0291).
    pub(crate) lighting: Lighting,
    /// The camera turned and the flat picture moved (decision 0291).
    pub(crate) turning: Turning,
    /// What the material is compared with (decision 0291).
    pub(crate) comparing: Comparing,
    /// The pictures drawn last on the processor (the material shown, and
    /// the one compared with it), with what each was drawn from.
    pub(super) drawn: [Option<(usize, DrawnFor, TextureHandle)>; 2],
    /// The 3D scenes laid last (the material shown, and the one compared
    /// with it), with what each was made from, so an unchanged one is
    /// the same drawing (and the renderer shows its last picture again).
    pub(super) laid: [Option<(LaidFrom, Arc<SceneDrawing>)>; 2],
    /// Counts the scenes made, for their versions.
    pub(crate) versions: u64,
    /// The seam check of the picture shown, by its key.
    pub(super) seam_check: Option<(usize, SeamCheck)>,
}

/// What a processor-drawn picture was drawn from: the shape (Flat or
/// the ball), crisp, and for Flat how far it was averaged down.
pub(super) type DrawnFor = (PreviewShape, bool, u32);

impl std::fmt::Debug for Preview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Preview")
            .field("shape", &self.shape)
            .field("look", &self.look)
            .field("seams", &self.seams)
            .field("lighting", &self.lighting)
            .finish_non_exhaustive()
    }
}

impl Preview {
    /// The tab is going to sleep (decision 0267): the pictures drawn are
    /// let go of, and drawn again when shown. Returns about how many
    /// bytes they took.
    pub(crate) fn sleep(&mut self) -> u64 {
        let mut bytes = self.comparing.sleep();
        for drawn in &mut self.drawn {
            if let Some((_, _, t)) = drawn.take() {
                let [w, h] = t.size();
                bytes += (w * h * 4) as u64;
            }
        }
        self.laid = [None, None];
        self.seam_check = None;
        bytes
    }
}

/// What a 3D preview scene was made from.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct LaidFrom {
    pub(super) picture: usize,
    pub(super) crisp: bool,
    pub(super) object: ObjectId,
    pub(super) mesh: usize,
    pub(super) transform: Mat4,
    pub(super) repeats: f32,
    pub(super) size: u32,
}

/// What the preview needs from the rest of the workbench.
pub(crate) struct PreviewContext<'a> {
    /// Draws the 3D shapes.
    pub(crate) renderer: &'a mut dyn ViewportRenderer,
    /// The scene's objects as drawn: the selected model.
    pub(crate) summary: &'a SceneSummary,
    /// How the export is set up, for the budget's maps.
    pub(crate) export: ExportSettings,
    /// The status bar's note (a health check fix says what it did).
    pub(crate) note: &'a mut Option<String>,
    /// How the 3D view is moved: the preview turns the same way
    /// (decision 0291).
    pub(crate) navigation: tools::NavigationSettings,
}

/// The box the preview shows: the one looked through, else Material
/// out, else the one selected.
pub(crate) fn shown_box(
    graph: &Graph,
    looking_at: Option<BoxId>,
    selected: &[BoxId],
) -> Option<BoxId> {
    looking_at
        .filter(|id| graph.get(*id).is_some())
        .or_else(|| graph.first_of(matgraph::eval::MATERIAL_OUT))
        .or(match selected {
            [one] => Some(*one),
            _ => None,
        })
}

/// The selected model to show the material on: the first selected object
/// with a surface (not a curve's points, a cage or a cutter).
pub(crate) fn selected_model(summary: &SceneSummary) -> Option<&DrawItem> {
    summary.drawing.items.iter().find(|item| {
        item.selected
            && !item.outline
            && item.cage.is_none()
            && item.drawn.face_count() > 0
            && item.transform.is_finite()
    })
}

/// The texture size a shape's material is laid into: enough for its
/// repeats at the picture's size, from 256 to 2048 (64 in tests, and 512
/// while something is held down).
pub(super) fn laid_size(repeats: f32, picture: u32, dragging: bool) -> u32 {
    if cfg!(test) {
        64
    } else {
        laid_size_for(repeats, picture, dragging)
    }
}

/// Whether `value` is one of the quick pictures worked out while the
/// graph changes under the pointer (smaller than the full size).
pub(super) fn quick(value: &Value) -> bool {
    value.size() < super::pictures::full().get()
}

/// [`laid_size`]'s rule.
fn laid_size_for(repeats: f32, picture: u32, dragging: bool) -> u32 {
    let most = if dragging { 512 } else { 2048 };
    let wanted = repeats * picture as f32;
    let wanted = if wanted.is_finite() {
        wanted.clamp(256.0, most as f32) as u32
    } else {
        256
    };
    wanted.next_power_of_two().min(most)
}

impl MaterialEditor {
    /// The Preview panel.
    pub(crate) fn preview_tab(
        &mut self,
        ui: &mut Ui,
        document: &mut Document,
        mut context: PreviewContext<'_>,
    ) {
        crate::dialog::segmented(
            ui,
            &mut self.preview.shape,
            &[
                (
                    PreviewShape::Model,
                    tr("Model"),
                    tr(
                        "The material on the selected model, by its UVs at the material's real-world size",
                    ),
                ),
                (
                    PreviewShape::Ball,
                    tr("Ball"),
                    tr("The material on a ball 1 m across, at its real-world size"),
                ),
                (
                    PreviewShape::Cube,
                    tr("Cube"),
                    tr(
                        "The material on a 1 m cube, at its real-world size: corners and flat sides",
                    ),
                ),
                (
                    PreviewShape::Flat,
                    tr("Flat"),
                    tr("The texture flat and repeated, so you can see that it tiles"),
                ),
            ],
        );
        ui.add_space(4.0);
        if self.preview.shape == PreviewShape::Flat {
            ui.checkbox(&mut self.preview.seams, tr(SEAM_CHECK))
                .on_hover_text(tr(
                    "Repeats the texture three times each way and marks where the copies meet, \
                     so you can see whether the joins show",
                ));
        } else {
            crate::dialog::segmented(
                ui,
                &mut self.preview.look,
                &[
                    (
                        PreviewLook::Unlit,
                        PaintPreview::Unlit.label(),
                        tr(
                            "The material's own colours, with no light: as most hand-painted games show it",
                        ),
                    ),
                    (
                        PreviewLook::Lit,
                        PaintPreview::Lit.label(),
                        tr("The viewport's soft studio light, with the height's bumps"),
                    ),
                    (
                        PreviewLook::Toon,
                        PaintPreview::Toon.label(),
                        tr(
                            "Cartoon (cel) shading: a lit side and a shaded side with a hard line between",
                        ),
                    ),
                    (
                        PreviewLook::Realistic,
                        PaintPreview::Realistic.label(),
                        tr(
                            "Lit as a game engine lights realistic materials, with the material's roughness and metal",
                        ),
                    ),
                ],
            );
        }
        ui.add_space(4.0);
        self.preview_options_bar(ui, document);
        ui.add_space(6.0);
        let Some(graph) = self.current(document).cloned() else {
            crate::dialog::help(ui, tr("Make a material graph to see it here."));
            return;
        };
        let selected: Vec<BoxId> = self.canvas.selected.iter().copied().collect();
        let Some(id) = shown_box(&graph, self.canvas.looking_at, &selected) else {
            crate::dialog::help(ui, tr("Add boxes to the graph to see what they make here."));
            return;
        };
        let name = graph
            .get(id)
            .and_then(|b| self.registry().get(&b.kind))
            .map_or_else(String::new, |t| t.info().name.show());
        let real_size = matgraph::real_size_cm(self.registry(), &graph);
        let crisp = matgraph::pixel_grid(self.registry(), &graph).is_some();
        let trim = matgraph::is_trim_sheet(&graph);
        let side = ui
            .available_width()
            .min(ui.available_height() - 60.0)
            .clamp(64.0, 640.0);
        // On the selected model, as often as the model's material repeats
        // it (its Tiling, decision 0270).
        let tiling = match (self.preview.shape, self.inside.is_empty(), self.graph) {
            (PreviewShape::Model, true, Some(graph)) => selected_model(context.summary)
                .map_or(1.0, |item| {
                    super::giving::tiling_on(document.scene(), item.id, graph)
                }),
            _ => 1.0,
        };
        match self.pictures.value(id).cloned() {
            Some(value) => {
                let shown_name = if self.preview.comparing.with == CompareWith::Before {
                    tr("Now").to_string()
                } else {
                    self.project_graph(document)
                        .map_or_else(|| graph.name().to_string(), |g| g.name().to_string())
                };
                let now = Source {
                    value,
                    real_size,
                    tiling,
                    crisp,
                    trim,
                    name: shown_name,
                };
                self.keep_before(&now);
                self.show_picture(ui, side, &now, document, &mut context);
            }
            None => {
                let rect = square(ui, side);
                ui.painter().rect_filled(rect, 4.0, theme::FIELD);
                let why = self
                    .problems(&graph)
                    .remove(&id)
                    .map_or_else(|| tr("Working it out…").to_string(), |r| r.reason);
                let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(16.0)));
                crate::dialog::help(&mut inner, why);
            }
        }
        // The player, for a graph that moves (decision 0277).
        self.player_rows(ui, document);
        ui.add_space(4.0);
        if self.canvas.looking_at.is_some_and(|l| l == id) {
            crate::dialog::help(
                ui,
                crate::trf!(
                    "Looking through {}: what it makes, on its own. Click its picture again to go \
                     back to the material.",
                    name
                ),
            );
            if ui.button(tr("Back to the material")).clicked() {
                self.canvas.looking_at = None;
            }
        } else {
            crate::dialog::help(
                ui,
                crate::trf!(
                    "Showing {}. Click a box's picture to look through that box.",
                    name
                ),
            );
        }
        ui.add_space(8.0);
        let card = self.card_maps(&graph);
        let size = self.budget_rows(ui, document, (real_size, card), context.export);
        let shapes = selected_model(context.summary).is_some();
        self.use_rows(ui, &graph, size, context.export, shapes);
        // Last, so the buttons above keep their place (decision 0290).
        self.health_rows(ui, document, &graph, context.note);
    }

    /// The size the texture budget is worked out for, and the material's
    /// textures exported at: the one chosen, else the project's texture
    /// size (at most 4096).
    pub(crate) fn budget_size(&self, document: &Document) -> u32 {
        let project = document.scene().texture();
        self.preview.budget_size.unwrap_or_else(|| {
            SIZES
                .into_iter()
                .rev()
                .find(|s| *s <= project.size)
                .unwrap_or(SIZES[0])
        })
    }

    /// The texture budget: the size, the maps and what they take, how
    /// sharp that is, and warnings. The size shown.
    fn budget_rows(
        &mut self,
        ui: &mut Ui,
        document: &Document,
        (real_size, card): (f32, fileio::export::graph_textures::CardMaps),
        export: ExportSettings,
    ) -> u32 {
        let project = document.scene().texture();
        let size = self.budget_size(document);
        crate::dialog::heading(ui, tr("Texture budget"));
        let mut chosen = size;
        crate::dialog::drop_down(
            ui,
            tr(BUDGET_SIZE),
            &crate::trf!("{} × {}", size, size),
            tr("pixels"),
            160.0,
            |ui| {
                for s in SIZES {
                    let hint = if s == project.size {
                        tr("the project's")
                    } else {
                        ""
                    };
                    if crate::dialog::drop_down_entry(
                        ui,
                        s == size,
                        &crate::trf!("{} × {}", s, s),
                        hint,
                    )
                    .clicked()
                    {
                        chosen = s;
                    }
                }
            },
        )
        .on_hover_text(tr(
            "The size the material's textures will be made at: larger is sharper but takes \
             more memory in the game",
        ));
        if chosen != size {
            self.preview.budget_size = Some(chosen);
        }
        let pixels = self
            .current(document)
            .and_then(|g| matgraph::pixel_grid(self.registry(), g));
        let budget = match pixels {
            Some(n) => Budget::of(n, real_size, card, export, project.density).into_pixel_art(),
            None => Budget::of(size, real_size, card, export, project.density),
        };
        if let Some(n) = pixels {
            crate::dialog::help(
                ui,
                crate::trf!(
                    "Pixel art: the Pixelate box makes this material {} × {} big pixels, so \
                     its textures go out at exactly {} × {}, one texel a pixel (worked out at \
                     the size above first).",
                    n,
                    n,
                    n,
                    n
                ),
            );
        }
        crate::dialog::help(
            ui,
            crate::trf!(
                "{} maps ({}), {} each: {} on the graphics card, with the smaller copies games \
                 make.",
                budget.maps.len(),
                budget.maps_line(),
                crate::format_bytes(budget.each),
                crate::format_bytes(budget.total)
            ),
        );
        if budget.pixel_art {
            crate::dialog::help(
                ui,
                crate::trf!(
                    "One repeat covers {} cm: {} pixels a metre, each pixel {} cm across. Give \
                     every pixel-art material the same pixels a metre (Pixels and Real-world \
                     size together) so their pixels match in the game.",
                    real_size.round(),
                    (budget.pixels_per_metre * 10.0).round() / 10.0,
                    (budget.cm_per_pixel() * 100.0).round() / 100.0
                ),
            );
        } else {
            crate::dialog::help(
                ui,
                crate::trf!(
                    "One repeat covers {} cm: {} pixels a metre (the project aims for {}).",
                    real_size.round(),
                    budget.pixels_per_metre.round(),
                    budget.aimed.round()
                ),
            );
        }
        for warning in budget.warnings() {
            ui.label(
                egui::RichText::new(warning)
                    .color(theme::WARNING)
                    .size(12.0),
            );
        }
        chosen
    }
}

/// A square `side` across, in the middle of the panel's width.
pub(super) fn square(ui: &mut Ui, side: f32) -> egui::Rect {
    let (row, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), side), egui::Sense::hover());
    egui::Rect::from_center_size(row.center(), egui::vec2(side, side))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_preview_shows_the_box_looked_through_first() {
        let registry = matgraph::Registry::standard();
        let mut g = Graph::new("Stone");
        for kind in ["noise", "material_out"] {
            g.apply(
                registry,
                &matgraph::GraphEdit::AddBox {
                    kind: kind.into(),
                    at: glam::Vec2::ZERO,
                },
            )
            .unwrap();
        }
        assert_eq!(shown_box(&g, None, &[]), Some(BoxId(2)), "the material");
        assert_eq!(shown_box(&g, Some(BoxId(1)), &[]), Some(BoxId(1)));
        assert_eq!(shown_box(&g, Some(BoxId(9)), &[]), Some(BoxId(2)), "gone");
        let empty = Graph::new("Empty");
        assert_eq!(shown_box(&empty, None, &[BoxId(3)]), Some(BoxId(3)));
        assert_eq!(shown_box(&empty, None, &[]), None);
    }

    #[test]
    fn the_laid_texture_size_follows_the_repeats() {
        assert_eq!(laid_size(1.0, 512, false), 64, "tests lay small");
        // One repeat of a 512 picture: 512; small ones never below 256.
        assert_eq!(laid_size_for(1.0, 512, false), 512);
        assert_eq!(laid_size_for(0.1, 512, false), 256);
        // Many repeats: as sharp as allowed, less while dragging.
        assert_eq!(laid_size_for(3.0, 512, false), 2048);
        assert_eq!(laid_size_for(3.0, 512, true), 512);
        assert_eq!(laid_size_for(f32::NAN, 512, false), 256);
    }
}
