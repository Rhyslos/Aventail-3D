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
//! Under the picture is the **texture budget** for the size chosen
//! (`budget`).

use super::MaterialEditor;
use super::budget::{Budget, SIZES};
use super::laying;
use super::pictures::{byte, colour_at, colour_image, picture_key};
use crate::drawing::{DrawItem, SceneDrawing, SceneSummary};
use crate::text::tr;
use crate::theme;
use crate::viewport::{
    Environment, GridDetail, PaintPreview, SceneLook, Shading, Surface, ViewportRenderer,
    ViewportView,
};
use crate::workbench::ExportSettings;
use egui::{Color32, ColorImage, TextureHandle, TextureOptions, Ui};
use glam::{Mat4, Vec2, Vec3};
use matgraph::{BoxId, Graph, SeamCheck, Value};
use scene::{Document, ObjectId};
use std::sync::Arc;

/// How many pixels across the processor-drawn ball is.
const BALL: u32 = 256;
/// How many times the flat preview repeats the texture each way, and with
/// the seam check on.
const TILES: f32 = 2.0;
const SEAM_TILES: f32 = 3.0;
/// The preview's own objects, apart from any in the scene.
const BALL_ID: ObjectId = ObjectId::from_raw(u64::MAX - 1);
const CUBE_ID: ObjectId = ObjectId::from_raw(u64::MAX - 2);
/// The way the camera looks at the shape: from the front right, a little
/// above, as the viewport opens.
const LOOK_FROM: Vec3 = Vec3::new(0.55, -1.0, 0.5);

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
    /// Whether the graph shown is pixel art (decision 0287): its big
    /// pixels are laid and drawn crisp, never blended.
    pub(crate) crisp: bool,
    /// The picture drawn last on the processor, with what it was drawn
    /// from (the picture, the shape, crisp).
    drawn: Option<(usize, (PreviewShape, bool), TextureHandle)>,
    /// The 3D scene drawn last, with what it was made from, so an
    /// unchanged one is the same drawing (and the renderer shows its last
    /// picture again).
    laid: Option<(LaidFrom, Arc<SceneDrawing>)>,
    /// Counts the scenes made, for their versions.
    versions: u64,
    /// The seam check of the picture shown, by its key.
    seam_check: Option<(usize, SeamCheck)>,
}

impl std::fmt::Debug for Preview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Preview")
            .field("shape", &self.shape)
            .field("look", &self.look)
            .field("seams", &self.seams)
            .finish_non_exhaustive()
    }
}

impl Preview {
    /// The tab is going to sleep (decision 0267): the pictures drawn are
    /// let go of, and drawn again when shown. Returns about how many
    /// bytes the flat picture took.
    pub(crate) fn sleep(&mut self) -> u64 {
        let bytes = self.drawn.as_ref().map_or(0, |(_, _, t)| {
            let [w, h] = t.size();
            (w * h * 4) as u64
        });
        self.drawn = None;
        self.laid = None;
        self.seam_check = None;
        bytes
    }
}

/// What a 3D preview scene was made from.
#[derive(Debug, Clone, PartialEq)]
struct LaidFrom {
    picture: usize,
    crisp: bool,
    object: ObjectId,
    mesh: usize,
    transform: Mat4,
    repeats: f32,
    size: u32,
}

/// What the preview needs from the rest of the workbench.
pub(crate) struct PreviewContext<'a> {
    /// Draws the 3D shapes.
    pub(crate) renderer: &'a mut dyn ViewportRenderer,
    /// The scene's objects as drawn: the selected model.
    pub(crate) summary: &'a SceneSummary,
    /// How the export is set up, for the budget's maps.
    pub(crate) export: ExportSettings,
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
fn laid_size(repeats: f32, picture: u32, dragging: bool) -> u32 {
    if cfg!(test) {
        64
    } else {
        laid_size_for(repeats, picture, dragging)
    }
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
        context: PreviewContext<'_>,
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
        self.preview.crisp = matgraph::pixel_grid(self.registry(), &graph).is_some();
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
            Some(value) => self.show_picture(
                ui,
                side,
                &value,
                (real_size, tiling),
                context.renderer,
                context.summary,
            ),
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

    /// `value` on the chosen shape, `side` across, at its real-world size
    /// and laid `tiling` times more often.
    fn show_picture(
        &mut self,
        ui: &mut Ui,
        side: f32,
        value: &Value,
        (real_size, tiling): (f32, f32),
        renderer: &mut dyn ViewportRenderer,
        summary: &SceneSummary,
    ) {
        if self.preview.shape == PreviewShape::Flat {
            self.show_flat(ui, side, value);
            return;
        }
        // The shape to lay it on: the model selected, or the ball or cube.
        let (object, mesh, transform, hardness) = match self.preview.shape {
            PreviewShape::Model => match selected_model(summary) {
                Some(item) => (
                    item.id,
                    Arc::clone(&item.drawn),
                    item.transform,
                    item.hardness,
                ),
                None => {
                    let rect = square(ui, side);
                    ui.painter().rect_filled(rect, 4.0, theme::FIELD);
                    let mut inner =
                        ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(16.0)));
                    crate::dialog::help(
                        &mut inner,
                        tr(
                            "Select a model (in Objects, or in the viewport) to see the material \
                            on it. Or choose Ball or Cube to see it on a shape of its own.",
                        ),
                    );
                    return;
                }
            },
            PreviewShape::Cube => (
                CUBE_ID,
                laying::cube(),
                Mat4::IDENTITY,
                mesh::Hardness::default(),
            ),
            _ => (
                BALL_ID,
                laying::ball(),
                Mat4::IDENTITY,
                mesh::Hardness::default(),
            ),
        };
        let Some(repeats) =
            laying::repeats(&mesh, transform, real_size).map(|r| laying::tiled(r, tiling))
        else {
            let rect = square(ui, side);
            ui.painter().rect_filled(rect, 4.0, theme::FIELD);
            let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(16.0)));
            crate::dialog::help(
                &mut inner,
                tr(
                    "This model has no UVs yet, so there is nothing to lay the material by. \
                    Unwrap it in the UV mode, or choose Ball or Cube.",
                ),
            );
            return;
        };
        let dragging = ui.ctx().input(|i| i.pointer.any_down()) || self.player.playing;
        let from = LaidFrom {
            picture: picture_key(value),
            crisp: self.preview.crisp,
            object,
            mesh: Arc::as_ptr(&mesh) as usize,
            transform,
            repeats,
            size: laid_size(repeats, value.size(), dragging),
        };
        let scene = match &self.preview.laid {
            Some((made, scene)) if *made == from => Arc::clone(scene),
            _ => {
                let Some(texture) = laying::laid(value, repeats, from.size, from.crisp) else {
                    return;
                };
                self.preview.versions += 1;
                let item = DrawItem {
                    id: object,
                    mesh: Arc::clone(&mesh),
                    shown: Arc::clone(&mesh),
                    drawn: Arc::clone(&mesh),
                    transform,
                    selected: false,
                    colors: Vec::new(),
                    // Every material slot shows the preview's texture.
                    textures: vec![laying::preview_key(); mesh::material::MAX_SLOTS as usize],
                    glow: 0.0,
                    hardness,
                    outline: false,
                    toon_outline: None,
                    // An alpha card shows cut out (decision 0276).
                    cut_out: value.material().is_some_and(|m| m.is_cut_out()),
                    curve: None,
                    cage: None,
                };
                let scene = Arc::new(SceneDrawing {
                    version: self.preview.versions,
                    items: vec![item],
                    textures: vec![texture],
                    ..SceneDrawing::default()
                });
                self.preview.laid = Some((from, Arc::clone(&scene)));
                scene
            }
        };
        let rect = square(ui, side);
        let view = preview_view(
            scene,
            &mesh,
            transform,
            rect,
            ui.ctx().pixels_per_point(),
            self.preview.look,
        );
        let accessible = |response: egui::Response| {
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Image, true, tr(PREVIEW_PICTURE))
            });
        };
        let response = ui.interact(rect, ui.id().with("material preview"), egui::Sense::hover());
        accessible(response);
        match view.and_then(|v| context_render(renderer, &v)) {
            Ok(texture) => {
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                ui.painter().image(texture, rect, uv, Color32::WHITE);
            }
            Err(why) if self.preview.shape == PreviewShape::Ball => {
                // No graphics card: the ball drawn here instead.
                let texture = self.preview_texture(ui.ctx(), value);
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                ui.painter().image(texture, rect, uv, Color32::WHITE);
                crate::dialog::help(
                    ui,
                    crate::trf!("A simpler ball, drawn without the graphics card ({}).", why),
                );
            }
            Err(why) => {
                ui.painter().rect_filled(rect, 4.0, theme::FIELD);
                let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(16.0)));
                crate::dialog::help(
                    &mut inner,
                    crate::trf!(
                        "The 3D preview can't be shown: {}. Ball and Flat still show the material.",
                        why
                    ),
                );
            }
        }
    }

    /// `value` flat and repeated; with the seam check, three by three with
    /// the joins marked, and what the check found.
    fn show_flat(&mut self, ui: &mut Ui, side: f32, value: &Value) {
        let texture = self.preview_texture(ui.ctx(), value);
        let rect = square(ui, side);
        let tiles = if self.preview.seams {
            SEAM_TILES
        } else {
            TILES
        };
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(tiles, tiles));
        if value.material().is_some_and(|m| m.is_cut_out()) {
            checker(ui.painter(), rect);
        }
        ui.painter().image(texture, rect, uv, Color32::WHITE);
        let response = ui.interact(rect, ui.id().with("material preview"), egui::Sense::hover());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Image, true, tr(PREVIEW_PICTURE))
        });
        if !self.preview.seams {
            return;
        }
        let key = picture_key(value);
        let check = match self.preview.seam_check {
            Some((k, check)) if k == key => check,
            _ => {
                let check = SeamCheck::of(value);
                self.preview.seam_check = Some((key, check));
                check
            }
        };
        // The joins, marked: warning-coloured where they show.
        let line = |shows: bool| {
            egui::Stroke::new(
                1.5,
                if shows {
                    theme::WARNING
                } else {
                    theme::TEXT_STRONG.gamma_multiply(0.55)
                },
            )
        };
        let painter = ui.painter();
        for i in 1..SEAM_TILES as usize {
            let t = i as f32 / SEAM_TILES;
            let x = rect.left() + rect.width() * t;
            let y = rect.top() + rect.height() * t;
            painter.vline(x, rect.y_range(), line(check.across.shows()));
            painter.hline(rect.x_range(), y, line(check.down.shows()));
        }
        ui.add_space(4.0);
        crate::dialog::help(ui, check.describe());
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
            Some(n) => Budget::of(n, real_size, card, export, project.density).as_pixel_art(),
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

    /// The flat picture or the processor-drawn ball of `value`, drawn
    /// again only when it or the shape changed.
    fn preview_texture(&mut self, ctx: &egui::Context, value: &Value) -> egui::TextureId {
        let key = picture_key(value);
        let shape = (self.preview.shape, self.preview.crisp);
        if let Some((k, s, handle)) = &self.preview.drawn
            && *k == key
            && *s == shape
        {
            return handle.id();
        }
        let (image, options) = match shape.0 {
            // Pixel art's big pixels stay sharp squares however large
            // they are drawn (decision 0287).
            PreviewShape::Flat if shape.1 => (
                colour_image(value, 512),
                TextureOptions {
                    magnification: egui::TextureFilter::Nearest,
                    ..TextureOptions::LINEAR_REPEAT
                },
            ),
            PreviewShape::Flat => (colour_image(value, 512), TextureOptions::LINEAR_REPEAT),
            _ => (ball(value, BALL), TextureOptions::LINEAR),
        };
        let handle = ctx.load_texture("material preview", image, options);
        let id = handle.id();
        self.preview.drawn = Some((key, shape, handle));
        id
    }
}

/// Asks `renderer` for the preview's picture.
fn context_render(
    renderer: &mut dyn ViewportRenderer,
    view: &ViewportView,
) -> Result<egui::TextureId, String> {
    renderer.render_preview(view)
}

/// The view of `scene` for the preview: the camera looking at `mesh`
/// (placed by `transform`) from the front right, filling `rect`, lit by
/// `look` on the Studio backdrop with no grid. `Err` for a shape with
/// nothing to frame.
pub(crate) fn preview_view(
    scene: Arc<SceneDrawing>,
    mesh: &mesh::Mesh,
    transform: Mat4,
    rect: egui::Rect,
    pixels_per_point: f32,
    look: PreviewLook,
) -> Result<ViewportView, String> {
    let (middle, radius) =
        laying::around(mesh, transform).ok_or_else(|| tr("the shape is empty").to_string())?;
    let ppp = if pixels_per_point.is_finite() && pixels_per_point > 0.0 {
        pixels_per_point
    } else {
        1.0
    };
    let px = |v: f32| (v * ppp).round().clamp(1.0, 4096.0) as u32;
    let size_px = [px(rect.width()), px(rect.height())];
    let mut camera = tools::Camera::looking_at(middle + LOOK_FROM, middle);
    // Far enough that the whole shape fits, with a little room.
    let distance = radius / (camera.fov_y * 0.5).sin() * 1.1;
    camera = tools::Camera::looking_at(middle + LOOK_FROM.normalize() * distance, middle);
    let studio = SceneLook::STUDIO;
    let [sky_top, sky_horizon, ground] = studio.backdrop();
    Ok(ViewportView {
        slot: 0,
        size_px,
        view_projection: camera.view_projection(Vec2::new(size_px[0] as f32, size_px[1] as f32)),
        camera_position: camera.position,
        focus: middle,
        flat_height: None,
        grid_plane: [Vec3::X, Vec3::Y],
        sky_top,
        sky_horizon,
        ground,
        grid_detail: GridDetail::default(),
        scene,
        selection_color: [1.0, 1.0, 1.0],
        grid_colors: theme::grid_colors(),
        checker_per_object: false,
        picture: None,
        mip_level: 0,
        overlay: Arc::default(),
        shading: Shading::default(),
        show_edges: false,
        show_grid: false,
        cavity: false,
        surface: Surface::Painted,
        paint_preview: look.paint_preview(),
        light_turn: 0.0,
        environment: Environment::default(),
        exposure: 0.0,
        pixels_per_point: ppp,
        high_contrast: false,
        look: studio,
        outline_selection: false,
    })
}

/// A grey checker filling `rect`, under an alpha card (decision 0276) so
/// its cut-away parts show as see-through.
fn checker(painter: &egui::Painter, rect: egui::Rect) {
    const CELLS: usize = 16;
    painter.rect_filled(rect, 0.0, theme::FIELD);
    let cell = rect.width() / CELLS as f32;
    for y in 0..CELLS {
        for x in (y % 2..CELLS).step_by(2) {
            let min = rect.min + egui::vec2(x as f32, y as f32) * cell;
            painter.rect_filled(
                egui::Rect::from_min_size(min, egui::vec2(cell, cell)),
                0.0,
                theme::TEXT_STRONG.gamma_multiply(0.12),
            );
        }
    }
}

/// A square `side` across, in the middle of the panel's width.
fn square(ui: &mut Ui, side: f32) -> egui::Rect {
    let (row, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), side), egui::Sense::hover());
    egui::Rect::from_center_size(row.center(), egui::vec2(side, side))
}

/// `value` on a lit ball `side` pixels across, drawn on the processor for
/// when there is no graphics card: wrapped round it twice, lit from the
/// top left, its bumps from the material's normal, shinier where it is
/// smooth.
pub(crate) fn ball(value: &Value, side: u32) -> ColorImage {
    let size = value.size().max(1) as f32;
    let light = Vec3::new(-0.5, 0.6, 0.7).normalize();
    let half = (light + Vec3::Z).normalize();
    let mut pixels = Vec::with_capacity((side * side) as usize);
    for y in 0..side {
        for x in 0..side {
            let nx = (x as f32 + 0.5) / side as f32 * 2.0 - 1.0;
            let ny = 1.0 - (y as f32 + 0.5) / side as f32 * 2.0;
            let r2 = nx * nx + ny * ny;
            if r2 > 1.0 {
                pixels.push(Color32::TRANSPARENT);
                continue;
            }
            let n = Vec3::new(nx, ny, (1.0 - r2).sqrt());
            let u = 0.5 + n.x.atan2(n.z) / std::f32::consts::TAU;
            let v = n.y.clamp(-1.0, 1.0).acos() / std::f32::consts::PI;
            let (px, py) = ((u * TILES * size) as i64, (v * size) as i64);
            let (colour, normal, rough, metal, open) = match value {
                Value::Material(m) => (
                    Vec3::from(m.colour.at(px, py)),
                    Vec3::from(m.normal.at(px, py)),
                    m.roughness.at(px, py),
                    m.metal.at(px, py),
                    m.occlusion.at(px, py),
                ),
                other => (Vec3::from(colour_at(other, px, py)), Vec3::Z, 0.6, 0.0, 1.0),
            };
            // An alpha card is cut out where it is see-through.
            let solid = value.material().map_or(1.0, |m| m.opacity_at(px, py));
            if solid < 0.5 {
                pixels.push(Color32::TRANSPARENT);
                continue;
            }
            let tangent = Vec3::new(n.z, 0.0, -n.x).normalize_or(Vec3::X);
            let up = n.cross(tangent);
            let bent = (tangent * normal.x + up * normal.y + n * normal.z).normalize_or(n);
            let diffuse = bent.dot(light).max(0.0);
            let power = 4.0 + (1.0 - rough) * 60.0;
            let shine = bent.dot(half).max(0.0).powf(power) * (1.0 - rough) * 0.6;
            let base = colour * (0.22 * open + 0.85 * diffuse) * (1.0 - 0.5 * metal);
            let tint = Vec3::ONE.lerp(colour, metal);
            let out = base + tint * shine;
            // Smooth the ball's rim over a pixel.
            let edge = ((1.0 - r2.sqrt()) * side as f32 / 2.0).clamp(0.0, 1.0);
            let alpha = byte(edge);
            pixels.push(Color32::from_rgba_unmultiplied(
                byte(out.x),
                byte(out.y),
                byte(out.z),
                alpha,
            ));
        }
    }
    ColorImage::new([side as usize, side as usize], pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use matgraph::{Size, ValueKind};

    #[test]
    fn the_ball_is_round_and_lit_from_the_top_left() {
        let value = Value::flat_grey(ValueKind::Height, Size::nearest(64), 1.0).unwrap();
        let image = ball(&value, 64);
        let at = |x: usize, y: usize| image.pixels[y * 64 + x];
        assert_eq!(at(0, 0), Color32::TRANSPARENT, "corners are outside");
        let top_left = at(20, 20).r();
        let bottom_right = at(44, 44).r();
        assert!(top_left > bottom_right, "{top_left} > {bottom_right}");
    }

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
    fn the_preview_view_frames_the_shape_in_the_chosen_look() {
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 200.0));
        let cube = laying::cube();
        let view = preview_view(
            Arc::default(),
            &cube,
            Mat4::IDENTITY,
            rect,
            2.0,
            PreviewLook::Toon,
        )
        .unwrap();
        assert_eq!(view.size_px, [400, 400]);
        assert_eq!(view.surface, Surface::Painted);
        assert_eq!(view.paint_preview, PaintPreview::Toon);
        assert!(!view.show_grid && !view.show_edges && !view.outline_selection);
        // The whole cube is in front of the camera and on the picture.
        let bounds = cube.bounds().unwrap();
        for i in 0..8 {
            let corner = Vec3::new(
                if i & 1 == 0 {
                    bounds.min.x
                } else {
                    bounds.max.x
                },
                if i & 2 == 0 {
                    bounds.min.y
                } else {
                    bounds.max.y
                },
                if i & 4 == 0 {
                    bounds.min.z
                } else {
                    bounds.max.z
                },
            );
            let clip = view.view_projection * corner.extend(1.0);
            assert!(clip.w > 0.0);
            let (x, y) = (clip.x / clip.w, clip.y / clip.w);
            assert!(x.abs() < 1.0 && y.abs() < 1.0, "corner {i} at {x}, {y}");
        }
        // Every look draws with its own paint preview.
        for (look, preview) in [
            (PreviewLook::Unlit, PaintPreview::Unlit),
            (PreviewLook::Lit, PaintPreview::Lit),
            (PreviewLook::Realistic, PaintPreview::Realistic),
        ] {
            assert_eq!(look.paint_preview(), preview);
        }
        // Nothing to frame: why.
        let empty = mesh::Mesh::default();
        assert!(
            preview_view(
                Arc::default(),
                &empty,
                Mat4::IDENTITY,
                rect,
                1.0,
                PreviewLook::Lit
            )
            .is_err()
        );
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
