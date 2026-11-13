//! The preview's picture (decisions 0260 and 0291): the material on the
//! chosen shape, turned as the 3D view turns, lit as chosen, and, while
//! comparing, the second material beside it (side by side) or under a
//! line you drag (the slider).
//!
//! Each material is laid into its own scene and drawn by its own
//! renderer slot (0 the material shown, 1 the one compared with it), so
//! the two never replace each other's textures. Turning the picture or
//! its light changes only the view: the same scenes are drawn from
//! elsewhere, and nothing is worked out or laid again.

use super::MaterialEditor;
use super::comparing::{CompareHow, Source};
use super::laying;
use super::pictures::picture_key;
use super::preview::{
    BALL, BALL_ID, CUBE_ID, LaidFrom, PreviewContext, PreviewShape, SEAM_TILES, laid_size, quick,
    selected_model, square,
};
use super::preview_ball::{averaged, ball, checker};
use super::preview_options::has_light;
use super::turning::{self, FramedOn, LIGHT_SPEED, PreviewDrag, drag_kind, preview_view};
use crate::drawing::{DrawItem, SceneDrawing, SceneSummary};
use crate::text::tr;
use crate::theme;
use crate::viewport::ViewportRenderer;
use egui::{Color32, Rect, TextureOptions, Ui};
use glam::Mat4;
use matgraph::SeamCheck;
use scene::Document;
use std::sync::Arc;
use tools::{DragKind, Modifiers, MouseButton};

/// The room between two pictures side by side, in points.
const GAP: f32 = 6.0;
/// The size of the names on the picture, in points.
const NAME_SIZE: f32 = 11.5;
/// The size of Flat's texture before it is averaged down.
const FLAT_SIZE: u32 = 512;

/// What the picture is shown on this frame.
pub(super) enum Stage {
    /// Flat, repeated.
    Flat,
    /// A 3D shape.
    Shape {
        framed: FramedOn,
        mesh: Arc<mesh::Mesh>,
        transform: Mat4,
        hardness: mesh::Hardness,
    },
}

/// What one material's side of the picture shows.
enum Shown {
    /// A texture, the part of it to draw, whether a checker goes under it
    /// (an alpha card on Flat), and a note for under the picture.
    Picture {
        texture: egui::TextureId,
        uv: Rect,
        checker: bool,
        note: Option<String>,
    },
    /// Words in place of a picture.
    Words(String),
}

impl MaterialEditor {
    /// `now` on the chosen shape, `side` across, with the material
    /// compared with it, if any; turned by drags and the wheel on it.
    pub(super) fn show_picture(
        &mut self,
        ui: &mut Ui,
        side: f32,
        now: &Source,
        document: &Document,
        context: &mut PreviewContext<'_>,
    ) {
        let stage = match self.stage(context.summary) {
            Ok(stage) => stage,
            Err(words) => {
                let rect = square(ui, side);
                words_in(ui, rect, &words);
                return;
            }
        };
        if let Stage::Shape {
            framed,
            mesh,
            transform,
            ..
        } = &stage
            && let Err(why) = self.preview.turning.camera(*framed, mesh, *transform)
        {
            let words = crate::trf!("The 3D preview can't be shown: {}.", why);
            let rect = square(ui, side);
            words_in(ui, rect, &words);
            return;
        }
        let compared = self.compared(document, context.summary);
        let side_by_side =
            compared.is_some() && self.preview.comparing.how == CompareHow::SideBySide;
        let (whole, halves) = if side_by_side {
            let half = ((ui.available_width() - GAP) / 2.0).min(side).max(32.0);
            let (row, _) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), half), egui::Sense::hover());
            let size = egui::vec2(half, half);
            let left = Rect::from_min_size(
                egui::pos2(row.center().x - GAP / 2.0 - half, row.top()),
                size,
            );
            let right =
                Rect::from_min_size(egui::pos2(row.center().x + GAP / 2.0, row.top()), size);
            (left.union(right), Some((left, right)))
        } else {
            (square(ui, side), None)
        };
        let response = ui.interact(
            whole,
            ui.id().with("material preview"),
            egui::Sense::click_and_drag(),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Image,
                true,
                tr(super::preview::PREVIEW_PICTURE),
            )
        });
        let one = halves.map_or(whole, |(_, right)| right);
        self.turn_preview(ui, &response, &stage, (one, whole), context);
        let ctx = ui.ctx().clone();
        let mine = self.shown(&ctx, 0, now, &stage, one, &mut *context.renderer);
        let theirs = compared.map(|c| match c {
            Ok(source) => (
                source.name.clone(),
                self.shown(&ctx, 1, &source, &stage, one, &mut *context.renderer),
            ),
            Err(words) => (String::new(), Shown::Words(words)),
        });
        let mut notes = Vec::new();
        match (&theirs, halves) {
            (None, _) => paint(ui, whole, whole, &mine, &mut notes),
            (Some((name, shown)), Some((left, right))) => {
                paint(ui, left, left, shown, &mut notes);
                paint(ui, right, right, &mine, &mut notes);
                name_on(ui, left, name, false, left.width() - 12.0);
                name_on(ui, right, &now.name, false, right.width() - 12.0);
            }
            (Some((name, shown)), None) => {
                let x = whole.left() + whole.width() * self.preview.comparing.split;
                let (left, right) = (
                    Rect::from_x_y_ranges(whole.left()..=x, whole.y_range()),
                    Rect::from_x_y_ranges(x..=whole.right(), whole.y_range()),
                );
                paint(ui, whole, left, shown, &mut notes);
                paint(ui, whole, right, &mine, &mut notes);
                split_line(ui, whole, x);
                // Both names or neither, so one is never taken for the
                // other.
                let room = whole.width() * 0.5 - 8.0;
                if name_width(ui, name).max(name_width(ui, &now.name)) <= room {
                    name_on(ui, whole, name, false, room);
                    name_on(ui, whole, &now.name, true, room);
                }
            }
        }
        if let Stage::Flat = stage
            && self.preview.seams
        {
            self.seam_marks(ui, one, now);
        }
        if self.preview.turning.turning_light {
            let words = crate::trf!("Light turned {}°", self.preview.lighting.light_turn.round());
            name_on(
                ui,
                Rect::from_min_max(whole.left_bottom() - egui::vec2(0.0, 26.0), whole.max),
                &words,
                false,
                whole.width() - 12.0,
            );
        }
        notes.dedup();
        for note in notes {
            crate::dialog::help(ui, note);
        }
        self.where_from_line(ui, &stage, now);
    }

    /// What the picture is shown on: Flat, or the 3D shape chosen; `Err`
    /// says what to do when Model is chosen and nothing is selected.
    fn stage(&self, summary: &SceneSummary) -> Result<Stage, String> {
        let (object, mesh, transform, hardness) = match self.preview.shape {
            PreviewShape::Flat => return Ok(Stage::Flat),
            PreviewShape::Model => match selected_model(summary) {
                Some(item) => (
                    item.id,
                    Arc::clone(&item.drawn),
                    item.transform,
                    item.hardness,
                ),
                None => {
                    return Err(tr(
                        "Select a model (in Objects, or in the viewport) to see the material \
                        on it. Or choose Ball or Cube to see it on a shape of its own.",
                    )
                    .to_string());
                }
            },
            PreviewShape::Cube => (
                CUBE_ID,
                laying::cube(),
                Mat4::IDENTITY,
                mesh::Hardness::default(),
            ),
            PreviewShape::Ball => (
                BALL_ID,
                laying::ball(),
                Mat4::IDENTITY,
                mesh::Hardness::default(),
            ),
        };
        Ok(Stage::Shape {
            framed: FramedOn {
                object,
                mesh: Arc::as_ptr(&mesh) as usize,
                transform,
            },
            mesh,
            transform,
            hardness,
        })
    }

    /// Drags and the wheel on the picture: the camera (or Flat's place)
    /// as in the 3D view, the light with Shift + right drag, the compare
    /// line with a left drag. `rects` are one picture's and the whole's.
    fn turn_preview(
        &mut self,
        ui: &Ui,
        response: &egui::Response,
        stage: &Stage,
        (one, whole): (Rect, Rect),
        context: &mut PreviewContext<'_>,
    ) {
        let flat = matches!(stage, Stage::Flat);
        let turning = &mut self.preview.turning;
        if ui.rect_contains_pointer(whole) && ui.ctx().dragged_id().is_none_or(|d| d == response.id)
        {
            let scroll = turning::wheel(ui, one.height());
            if scroll != 0.0 {
                // The wheel zooms the picture, not the panel.
                ui.ctx()
                    .input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
                let amount = scroll * tools::navigation::WHEEL_DOLLY_SPEED;
                match turning.navigator_mut() {
                    _ if flat => turning.flat_zoom(amount),
                    Some(navigator) => navigator.dolly(amount),
                    None => {}
                }
            }
        }
        let button = [
            (egui::PointerButton::Primary, MouseButton::Primary),
            (egui::PointerButton::Secondary, MouseButton::Secondary),
            (egui::PointerButton::Middle, MouseButton::Middle),
        ]
        .into_iter()
        .find(|(b, _)| response.dragged_by(*b));
        let Some((pointer, button)) = button else {
            turning.turning_light = false;
            if response.clicked()
                && self.preview.comparing.slider()
                && let Some(at) = response.interact_pointer_pos()
            {
                self.preview
                    .comparing
                    .set_split((at.x - whole.left()) / whole.width());
            }
            return;
        };
        let keys = ui.input(|i| Modifiers {
            alt: i.modifiers.alt,
            shift: i.modifiers.shift,
        });
        let slider = self.preview.comparing.slider();
        let delta = response.drag_delta();
        let started = response.drag_started_by(pointer);
        match drag_kind(button, keys, &context.navigation, slider) {
            Some(PreviewDrag::Light) => {
                let light = !flat && has_light(self.preview.look);
                if !light {
                    if started {
                        let why = if flat {
                            tr(
                                "Flat shows the texture as it is, with no light to turn: choose \
                                Ball, Cube or Model.",
                            )
                        } else {
                            tr("Unlit has no light to turn: choose Lit, Toon or Realistic.")
                        };
                        *context.note = Some(why.to_string());
                    }
                    return;
                }
                self.preview.turning.turning_light = true;
                self.preview.lighting.turn_light(delta.x * LIGHT_SPEED);
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            Some(PreviewDrag::Camera(kind)) => {
                let turning = &mut self.preview.turning;
                turning.turning_light = false;
                let delta = context
                    .navigation
                    .movement(kind, glam::Vec2::new(delta.x, delta.y));
                if flat {
                    turning.flat_drag(kind, delta, one.width());
                } else if let Some(navigator) = turning.navigator_mut() {
                    navigator.drag(kind, delta, glam::Vec2::new(one.width(), one.height()));
                }
                ui.ctx().set_cursor_icon(match kind {
                    DragKind::Orbit => egui::CursorIcon::Grabbing,
                    DragKind::Pan => egui::CursorIcon::Move,
                    DragKind::Dolly => egui::CursorIcon::ZoomIn,
                });
            }
            Some(PreviewDrag::Split) => {
                if let Some(at) = response.interact_pointer_pos() {
                    self.preview
                        .comparing
                        .set_split((at.x - whole.left()) / whole.width());
                }
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            None => {}
        }
    }

    /// `source` on the stage, drawn into a picture `rect` across (slot 0
    /// the material shown, 1 the one compared with it).
    fn shown(
        &mut self,
        ctx: &egui::Context,
        slot: usize,
        source: &Source,
        stage: &Stage,
        rect: Rect,
        renderer: &mut dyn ViewportRenderer,
    ) -> Shown {
        let Stage::Shape {
            framed,
            mesh,
            transform,
            hardness,
        } = stage
        else {
            let texture = self.processor_picture(ctx, slot, source);
            let uv = if self.preview.seams {
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(SEAM_TILES, SEAM_TILES))
            } else {
                self.preview.turning.flat_uv()
            };
            return Shown::Picture {
                texture,
                uv,
                checker: source.value.material().is_some_and(|m| m.is_cut_out()),
                note: None,
            };
        };
        // A trim sheet is laid once, so its strips stay on the rows.
        let Some(repeats) = laying::repeats(mesh, *transform, source.real_size).map(|r| {
            if source.trim {
                1.0
            } else {
                laying::tiled(r, source.tiling)
            }
        }) else {
            return Shown::Words(
                tr(
                    "This model has no UVs yet, so there is nothing to lay the material by. \
                    Unwrap it in the UV mode, or choose Ball or Cube.",
                )
                .to_string(),
            );
        };
        let Some(scene) = self.laid_scene(slot, source, (*framed, mesh, *hardness), repeats) else {
            return Shown::Words(tr("The material can't be laid on this shape.").to_string());
        };
        let Some(navigator) = self.preview.turning.navigator() else {
            return Shown::Words(tr("Working it out…").to_string());
        };
        let lighting = self.preview.lighting;
        let view = crate::viewport::ViewportView {
            slot,
            ..preview_view(
                scene,
                navigator,
                rect,
                ctx.pixels_per_point(),
                (self.preview.look, &lighting),
            )
        };
        let whole = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        match renderer.render_preview(&view) {
            Ok(texture) => Shown::Picture {
                texture,
                uv: whole,
                checker: false,
                note: None,
            },
            Err(why) if self.preview.shape == PreviewShape::Ball => Shown::Picture {
                // No graphics card: the ball drawn here instead.
                texture: self.processor_picture(ctx, slot, source),
                uv: whole,
                checker: false,
                note: Some(crate::trf!(
                    "A simpler ball, drawn without the graphics card ({}).",
                    why
                )),
            },
            Err(why) => Shown::Words(crate::trf!(
                "The 3D preview can't be shown: {}. Ball and Flat still show the material.",
                why
            )),
        }
    }

    /// `source` laid `repeats` times across the shape's texture, as a
    /// scene of that one shape: the same scene while nothing it is made
    /// from changes. `None` if it can't be laid.
    fn laid_scene(
        &mut self,
        slot: usize,
        source: &Source,
        (framed, mesh, hardness): (FramedOn, &Arc<mesh::Mesh>, mesh::Hardness),
        repeats: f32,
    ) -> Option<Arc<SceneDrawing>> {
        // Laid smaller only while the graph changes (its pictures are then
        // the quick ones) or plays: a button held for anything else
        // (panning, turning the preview) keeps the texture laid.
        let value = &source.value;
        let dragging = quick(value) || self.player.playing;
        let from = LaidFrom {
            picture: picture_key(value),
            crisp: source.crisp,
            object: framed.object,
            mesh: framed.mesh,
            transform: framed.transform,
            repeats,
            size: laid_size(repeats, value.size(), dragging),
        };
        if let Some((made, scene)) = &self.preview.laid[slot]
            && *made == from
        {
            return Some(Arc::clone(scene));
        }
        let texture = laying::laid(value, repeats, from.size, from.crisp)?;
        self.preview.versions += 1;
        let item = DrawItem {
            id: framed.object,
            mesh: Arc::clone(mesh),
            shown: Arc::clone(mesh),
            drawn: Arc::clone(mesh),
            transform: framed.transform,
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
        self.preview.laid[slot] = Some((from, Arc::clone(&scene)));
        Some(scene)
    }

    /// Flat's picture or the processor-drawn ball of `source`, drawn
    /// again only when it, the shape or (on Flat) how far it is averaged
    /// down changed.
    fn processor_picture(
        &mut self,
        ctx: &egui::Context,
        slot: usize,
        source: &Source,
    ) -> egui::TextureId {
        let value = &source.value;
        let key = picture_key(value);
        let shape = self.preview.shape;
        // Zoomed out on Flat, a texture averaged down by whole halvings, so
        // its pixels stay about the screen's (decision 0291).
        let halvings = if shape == PreviewShape::Flat && !self.preview.seams {
            (self.preview.turning.flat_repeats / turning::FLAT_REPEATS)
                .log2()
                .floor()
                .clamp(0.0, 5.0) as u32
        } else {
            0
        };
        let wanted = (shape, source.crisp, halvings);
        if let Some((k, s, handle)) = &self.preview.drawn[slot]
            && *k == key
            && *s == wanted
        {
            return handle.id();
        }
        let (image, options) = match shape {
            // Pixel art's big pixels stay sharp squares however large
            // they are drawn (decision 0287).
            PreviewShape::Flat if source.crisp => (
                averaged(value, FLAT_SIZE >> halvings),
                TextureOptions {
                    magnification: egui::TextureFilter::Nearest,
                    ..TextureOptions::LINEAR_REPEAT
                },
            ),
            PreviewShape::Flat => (
                averaged(value, FLAT_SIZE >> halvings),
                TextureOptions::LINEAR_REPEAT,
            ),
            _ => (ball(value, BALL), TextureOptions::LINEAR),
        };
        let handle = ctx.load_texture(format!("material preview {slot}"), image, options);
        let id = handle.id();
        self.preview.drawn[slot] = Some((key, wanted, handle));
        id
    }

    /// The seam check's marks over Flat's picture `rect`, three by three,
    /// and what it found.
    fn seam_marks(&mut self, ui: &mut Ui, rect: Rect, now: &Source) {
        let key = picture_key(&now.value);
        let check = match self.preview.seam_check {
            Some((k, check)) if k == key => check,
            _ => {
                let check = SeamCheck::of(&now.value);
                self.preview.seam_check = Some((key, check));
                check
            }
        };
        // A trim sheet's last strip meets its first in a gap: expected
        // (decision 0289), so only its left and right edges are checked.
        let check = if now.trim { check.across_only() } else { check };
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

    /// Under the picture, once it is turned, moved or zoomed: how far
    /// away the camera is, or how much of the material Flat shows (so a
    /// "mush at 12 m" from the health check can be seen for yourself), and
    /// Reset the view.
    fn where_from_line(&mut self, ui: &mut Ui, stage: &Stage, now: &Source) {
        let turning = &self.preview.turning;
        let words = match stage {
            Stage::Flat if self.preview.seams => return,
            Stage::Flat if turning.moved(PreviewShape::Flat) => {
                let metres = turning.flat_repeats * now.real_size / 100.0;
                crate::trf!(
                    "Shows {} m of the material across. Zoom out with the wheel to see it as it \
                     looks from afar.",
                    (metres * 10.0).round() / 10.0
                )
            }
            Stage::Shape { .. } if turning.moved(self.preview.shape) => {
                let metres = turning.navigator().map_or(0.0, |n| n.distance() / 100.0);
                crate::trf!(
                    "Seen from {} m away. Zoom out with the wheel to see it as it looks from \
                     afar.",
                    (metres * 10.0).round() / 10.0
                )
            }
            _ => return,
        };
        crate::dialog::help(ui, words);
        if ui
            .button(tr(super::preview_options::RESET_VIEW))
            .on_hover_text(tr(
                "Back to the view the preview opens with, after turning, moving or zooming it",
            ))
            .clicked()
        {
            self.preview.turning.reset();
        }
    }
}

/// `shown` drawn over `rect`, only within `part` of it (the slider's
/// side); its note added to `notes`.
fn paint(ui: &mut Ui, rect: Rect, part: Rect, shown: &Shown, notes: &mut Vec<String>) {
    let painter = ui.painter().with_clip_rect(part.intersect(ui.clip_rect()));
    match shown {
        Shown::Picture {
            texture,
            uv,
            checker: under,
            note,
        } => {
            if *under {
                checker(&painter, rect);
            }
            painter.image(*texture, rect, *uv, Color32::WHITE);
            notes.extend(note.clone());
        }
        Shown::Words(words) => {
            painter.rect_filled(part, 4.0, theme::FIELD);
            let mut inner = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(part.shrink(12.0))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            inner.set_clip_rect(part);
            crate::dialog::help(&mut inner, words.clone());
        }
    }
}

/// Words in place of the picture, filling `rect`.
fn words_in(ui: &mut Ui, rect: Rect, words: &str) {
    ui.painter().rect_filled(rect, 4.0, theme::FIELD);
    let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(16.0)));
    crate::dialog::help(&mut inner, words.to_string());
}

/// How wide a name is on the picture, with its backing.
fn name_width(ui: &Ui, name: &str) -> f32 {
    let font = egui::FontId::proportional(NAME_SIZE);
    let galley = ui
        .painter()
        .layout_no_wrap(name.to_string(), font, theme::TEXT_STRONG);
    galley.size().x + 10.0
}

/// A material's name in a corner of `rect` (the top left, or the top
/// right when `right`), on a dark backing so it reads on any picture;
/// left out when it is wider than `room`.
fn name_on(ui: &Ui, rect: Rect, name: &str, right: bool, room: f32) {
    if name.is_empty() {
        return;
    }
    let painter = ui.painter();
    let font = egui::FontId::proportional(NAME_SIZE);
    let galley = painter.layout_no_wrap(name.to_string(), font, theme::TEXT_STRONG);
    let size = galley.size() + egui::vec2(10.0, 4.0);
    if size.x > room {
        return;
    }
    let min = if right {
        egui::pos2(rect.right() - 6.0 - size.x, rect.top() + 6.0)
    } else {
        rect.left_top() + egui::vec2(6.0, 6.0)
    };
    let backing = Rect::from_min_size(min, size);
    painter.rect_filled(backing, 3.0, Color32::from_black_alpha(160));
    painter.galley(
        backing.min + egui::vec2(5.0, 2.0),
        galley,
        theme::TEXT_STRONG,
    );
}

/// The slider's line at `x` across `rect`, with a round handle in the
/// middle.
fn split_line(ui: &Ui, rect: Rect, x: f32) {
    let painter = ui.painter();
    painter.vline(
        x,
        rect.y_range(),
        egui::Stroke::new(3.0, Color32::from_black_alpha(120)),
    );
    painter.vline(
        x,
        rect.y_range(),
        egui::Stroke::new(1.5, theme::TEXT_STRONG),
    );
    let middle = egui::pos2(x, rect.center().y);
    painter.circle(
        middle,
        7.0,
        theme::FIELD,
        egui::Stroke::new(1.5, theme::TEXT_STRONG),
    );
}
