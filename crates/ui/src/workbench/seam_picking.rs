//! The Blend seams tool ([decision 0293](../../../../docs/decisions/0293-blend-seams.md)):
//! key 9 in Paint mode, or its button in the tool row. A click on a seam
//! in the 3D view marks it (the whole cut between its two UV pieces) on
//! the material's Seam blend layer, in place of those marked; Shift +
//! click marks one more, or unmarks one marked. **Every seam that shows**
//! marks each seam whose sides don't match. The layer is made on top the
//! first time, and every change is one undo step, its blend joining it
//! once made (`seam_blending`).
//!
//! While the tool is in use the views show the material's seams faintly,
//! the marked ones in the accent colour and the one under the mouse in
//! white.
//!
//! In the UV Editor ([decision 0295](../../../../docs/decisions/0295-picking-seams-in-the-uv-editor.md))
//! the same clicks pick seams from the texture: each seam shows there as
//! two lines, one along each piece it joins, and a click near either
//! marks it. Pointing at a seam in either view lights it in both, both
//! its sides in the UV Editor.
//!
//! Where two materials meet ([decision 0296](../../../../docs/decisions/0296-blending-seams-between-materials.md))
//! the edge is a seam too, shown and picked with the material's own. A
//! click marks it on both materials' Seam blend layers in one undo step
//! (in place of the clicked material's marks, added to the other's), and
//! the status bar names the two materials.

use super::Workbench;
use super::brushing::drawn_transform;
use crate::attributes::PaintTool;
use crate::drawing::SceneDrawing;
use crate::paint_layers::seam_rows::{SeamAsk, SeamView};
use crate::palette::Mode;
use crate::text::tr;
use crate::transforming::Tool;
use crate::uv_editor::seam_lines::{UvSeamLine, UvSeamLines};
use crate::viewport::{BrushEvent, SeamLine, SeamLines, SeamLook};
use glam::{Mat4, Vec2, Vec3};
use paint::seams::{self, Seam};
use paint::{Image, Layer, LayerPath, SeamBlend, SeamId, TextureSet};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

/// A shape's seams on a material, then where it meets each other
/// material (named).
type SeamLists = Vec<(Option<String>, Arc<Vec<Seam>>)>;

/// How near a seam the mouse must be to pick it, in screen points.
const NEAR: f32 = 10.0;

/// The most seams drawn faintly (a huge shape's are left out past it).
const MAX_SHOWN: usize = 50_000;

/// Merge keys of the width's drags: apart from every other drag's.
const WIDTH_KEYS: u64 = 0x5345_414D_0000_0000;

/// Said when a click finds no seam.
pub(crate) const NO_SEAM_HERE: &str = crate::text!(
    "No seam under the mouse: click on one of the faint lines, where the model is cut open on \
     the texture."
);

/// Said when a click in the UV Editor finds no seam.
pub(crate) const NO_SEAM_ON_TEXTURE: &str = crate::text!(
    "No seam under the mouse: click on one of the faint lines along a piece's edge, where it \
     is cut from the piece it meets on the model."
);

/// The Blend seams tool's state.
#[derive(Debug, Default)]
pub(super) struct SeamPicking {
    /// The lines last shown: what they were made from (drawing, material
    /// and marked seams, hashed) and the lines.
    lines: Option<(u64, Arc<Vec<SeamLine>>)>,
    /// The same for the UV Editor: both sides of the seams of the
    /// objects it shows, and what they were made from (with which
    /// objects).
    flat_lines: Option<(u64, Arc<Vec<UvSeamLine>>)>,
    /// Counts the width's drags, for their undo merge keys.
    drags: u64,
    /// Whether the width was being dragged last frame.
    dragging: bool,
    /// The two materials of the seam under the mouse last frame, where
    /// two materials meet (to name them once in the status bar).
    hovered_across: Option<(String, String)>,
}

/// A seam line found under the mouse.
pub(super) struct Picked {
    material: String,
    /// The other material, where the line is where two materials meet.
    other: Option<String>,
    /// The edges of the line, by id.
    pub(super) ids: Vec<SeamId>,
    /// The line in the world, to show.
    lines: Vec<SeamLine>,
    /// Both sides of the line on the texture, to show in the UV Editor
    /// (when its object is shown there).
    flat: Vec<UvSeamLine>,
}

impl Workbench {
    /// Whether Blend seams is the tool in use (in Paint mode, painting
    /// the texture, no transform tool or select arrow chosen).
    pub(super) fn seam_tool_on(&self) -> bool {
        self.mode == Mode::Paint
            && self.tool == Tool::Select
            && !self.brushing.selecting
            && !self.attributes.picking
            && self.attributes.brush.tool == PaintTool::Seams
    }

    /// Whether the seam blend brush is in use (decision 0294).
    pub(super) fn seam_blend_brush_on(&self) -> bool {
        self.mode == Mode::Paint
            && self.tool == Tool::Select
            && !self.brushing.selecting
            && !self.attributes.picking
            && self.attributes.brush.tool == PaintTool::Brush
            && self.attributes.brush.brush.kind == paint::BrushKind::SeamBlend
    }

    /// Each frame in Paint mode, after the brush: the seams over the
    /// views and the Paint tab's view of the layer the tool marks on.
    /// With the seam blend brush the seams are shown faintly too, to
    /// stroke along (decision 0294).
    pub(super) fn update_seam_tool(&mut self, drawing: &SceneDrawing) {
        if self.seam_blend_brush_on() {
            let material = self
                .paint_layers
                .material(&self.document)
                .unwrap_or_default();
            let all = self.seam_lines(&material, &[], drawing);
            self.viewport.set_seam_lines(Some(SeamLines {
                all,
                hovered: Vec::new(),
            }));
            self.uv_editor.set_seam_lines(None);
            return;
        }
        if !self.seam_tool_on() {
            self.viewport.set_seam_lines(None);
            self.uv_editor.set_seam_lines(None);
            return;
        }
        let material = self
            .paint_layers
            .material(&self.document)
            .unwrap_or_default();
        let set = self.document.scene().paint().set(&material).cloned();
        // The layer the tool marks on, and its settings.
        let target: Option<(LayerPath, SeamBlend)> = set.as_ref().and_then(|s| {
            let path = self.seam_target(&material, s)?;
            let blend = s.layer(&path)?.seam_blend.clone()?;
            Some((path, blend))
        });
        let marked: Vec<SeamId> = target
            .as_ref()
            .map(|(_, b)| b.seams.clone())
            .unwrap_or_default();
        let view = match target {
            Some((path, blend)) => {
                let at = (material.clone(), path);
                SeamView {
                    layer: true,
                    marked: blend.seams.len(),
                    missing: self.seam_blends.missing.get(&at).copied().unwrap_or(0),
                    width: blend.width,
                    making: self.paint_layers.seam_making.as_ref() == Some(&at),
                }
            }
            None => SeamView::default(),
        };
        self.attributes.seam_tool = (material.clone(), view);
        let all = self.seam_lines(&material, &marked, drawing);
        let flat = self.flat_seam_lines(&material, &marked, drawing);
        // The line under the mouse, in whichever view it is over, lit in
        // both.
        let picked = match (self.viewport.pointer(), self.uv_editor.brush_hover()) {
            (Some(p), _) => self.pick_seam(drawing, &p.camera, p.size, p.at),
            (None, Some(p)) => self.pick_flat_seam(drawing, p.uv, p.zoom),
            (None, None) => None,
        };
        let across = picked
            .as_ref()
            .and_then(|p| Some((p.material.clone(), p.other.clone()?)));
        if across != self.seam_picking.hovered_across
            && let Some((material, other)) = &across
        {
            self.note = Some(crate::trf!(
                "Where {} meets {}: a click blends both sides, each on its own material's \
                 texture.",
                material,
                other
            ));
        }
        self.seam_picking.hovered_across = across;
        let (hovered, flat_hovered) = picked.map(|p| (p.lines, p.flat)).unwrap_or_default();
        self.viewport
            .set_seam_lines(Some(SeamLines { all, hovered }));
        self.uv_editor.set_seam_lines(Some(UvSeamLines {
            all: flat,
            hovered: flat_hovered,
        }));
    }

    /// Both sides of the seams of `material` on the objects the UV
    /// Editor shows, `marked` ones marked, made again only when they
    /// (or the objects shown) change.
    fn flat_seam_lines(
        &mut self,
        material: &str,
        marked: &[SeamId],
        drawing: &SceneDrawing,
    ) -> Arc<Vec<UvSeamLine>> {
        let shown = crate::uv_editor::shown_objects(&self.document);
        let key = {
            let mut h = DefaultHasher::new();
            (drawing.version, material, marked, &shown).hash(&mut h);
            h.finish()
        };
        if let Some((k, lines)) = &self.seam_picking.flat_lines
            && *k == key
        {
            return Arc::clone(lines);
        }
        let marked: HashSet<&SeamId> = marked.iter().collect();
        let mut lines = Vec::new();
        for (id, _, lists) in self.flat_seam_shapes(material, drawing) {
            for seam in lists.iter().flat_map(|(_, found)| found.iter()) {
                let look = if marked.contains(&SeamId {
                    object: id.get(),
                    points: seam.points,
                }) {
                    SeamLook::Marked
                } else if lines.len() < MAX_SHOWN {
                    SeamLook::Faint
                } else {
                    continue;
                };
                lines.extend(sides_on_texture(seam, look));
            }
        }
        let lines = Arc::new(lines);
        self.seam_picking.flat_lines = Some((key, Arc::clone(&lines)));
        lines
    }

    /// The objects the UV Editor shows that use `material`, as the brush
    /// sees them, with their seams on it and where it meets other
    /// materials (kept while their shapes stay the same).
    fn flat_seam_shapes(
        &mut self,
        material: &str,
        drawing: &SceneDrawing,
    ) -> Vec<(scene::ObjectId, Arc<paint::PaintSurface>, SeamLists)> {
        let mut out = Vec::new();
        for id in crate::uv_editor::shown_objects(&self.document) {
            let slots = self.slots_of(id, material);
            if slots.is_empty() {
                continue;
            }
            let Some(mesh) = super::brushing::drawn_mesh(drawing, id) else {
                continue;
            };
            let surface = self.brush_surface(id, &mesh);
            let lists = self.seam_lists(id.get(), material, &surface, &slots);
            out.push((id, surface, lists));
        }
        out
    }

    /// The seam line under the mouse at `uv` in the UV Editor (shown at
    /// `zoom` points per texture width): the seam whose side on the
    /// texture is nearest, within [`NEAR`] points. The material of the
    /// piece under the mouse is tried first, then the one the Paint
    /// layers panel shows (the mouse may be just outside a piece, on its
    /// edge).
    pub(super) fn pick_flat_seam(
        &mut self,
        drawing: &SceneDrawing,
        uv: Vec2,
        zoom: f32,
    ) -> Option<Picked> {
        if !uv.is_finite() {
            return None;
        }
        let reach = NEAR / zoom.max(1e-6);
        let under = self
            .flat_hit(drawing, uv)
            .map(|(id, triangle, _)| self.material_at(id, triangle.slot));
        let shown = self.paint_layers.material(&self.document);
        let mut materials: Vec<String> = under.into_iter().chain(shown).collect();
        materials.dedup();
        for material in materials {
            let mut best: Option<(f32, usize, usize, usize)> = None;
            let shapes = self.flat_seam_shapes(&material, drawing);
            for (s, (_, _, lists)) in shapes.iter().enumerate() {
                for (l, (_, found)) in lists.iter().enumerate() {
                    for (i, seam) in found.iter().enumerate() {
                        for side in seam.sides {
                            let d = segment_distance_2d(uv, side.uvs[0], side.uvs[1]);
                            if d <= reach && best.is_none_or(|(bd, ..)| d < bd) {
                                best = Some((d, s, l, i));
                            }
                        }
                    }
                }
            }
            let Some((_, s, l, start)) = best else {
                continue;
            };
            let (id, surface, lists) = &shapes[s];
            let (other, found) = &lists[l];
            let model = drawn_transform(drawing, *id);
            let line = seams::line_of(found, start);
            return Some(Picked {
                material,
                other: other.clone(),
                ids: line
                    .iter()
                    .map(|&i| SeamId {
                        object: id.get(),
                        points: found[i].points,
                    })
                    .collect(),
                lines: line
                    .iter()
                    .map(|&i| line_in_world(&found[i], surface, model, SeamLook::Hovered))
                    .collect(),
                flat: line
                    .iter()
                    .flat_map(|&i| sides_on_texture(&found[i], SeamLook::Hovered))
                    .collect(),
            });
        }
        None
    }

    /// The seams of `material` on the shapes shown, `marked` ones marked,
    /// made again only when they change.
    fn seam_lines(
        &mut self,
        material: &str,
        marked: &[SeamId],
        drawing: &SceneDrawing,
    ) -> Arc<Vec<SeamLine>> {
        let key = {
            let mut h = DefaultHasher::new();
            (drawing.version, material, marked).hash(&mut h);
            h.finish()
        };
        if let Some((k, lines)) = &self.seam_picking.lines
            && *k == key
        {
            return Arc::clone(lines);
        }
        let marked: HashSet<&SeamId> = marked.iter().collect();
        let mut lines = Vec::new();
        for shape in self.seam_shapes(material, drawing) {
            let (id, surface) = (shape.id, &shape.surface);
            let model = drawn_transform(drawing, scene::ObjectId::from_raw(id));
            let across = shape.across.iter().flat_map(|c| c.seams.iter());
            for seam in shape.seams.iter().chain(across) {
                let look = if marked.contains(&SeamId {
                    object: id,
                    points: seam.points,
                }) {
                    SeamLook::Marked
                } else if lines.len() < MAX_SHOWN {
                    SeamLook::Faint
                } else {
                    continue;
                };
                lines.push(line_in_world(seam, surface, model, look));
            }
        }
        let lines = Arc::new(lines);
        self.seam_picking.lines = Some((key, Arc::clone(&lines)));
        lines
    }

    /// The seam line under the mouse at `at` (points, in a view of `size`
    /// seen through `camera`): on the nearest shape there, the seam
    /// nearest the mouse within [`NEAR`] points.
    fn pick_seam(
        &mut self,
        drawing: &SceneDrawing,
        camera: &tools::Camera,
        size: Vec2,
        at: Vec2,
    ) -> Option<Picked> {
        let hit = self.brush_hit(drawing, camera, size, at)?;
        let slot = hit.surface.triangles().get(hit.triangle)?.slot;
        let material = self.material_at(hit.id, slot);
        let slots = self.slots_of(hit.id, &material);
        let object = hit.id.get();
        let lists = self.seam_lists(object, &material, &hit.surface, &slots);
        let per_point = camera.world_per_pixel(hit.world, size).max(1e-6);
        // Only seams near the point hit: not one on the far side that
        // happens to lie under the mouse.
        let reach = per_point * NEAR * 4.0;
        let mut best: Option<(f32, usize, usize)> = None;
        for (l, (_, found)) in lists.iter().enumerate() {
            for (i, seam) in found.iter().enumerate() {
                let ends = seam.ends.map(|p| hit.model.transform_point3(p));
                if segment_distance_3d(hit.world, ends) > reach {
                    continue;
                }
                let (Some(a), Some(b)) =
                    (camera.project(ends[0], size), camera.project(ends[1], size))
                else {
                    continue;
                };
                let d = segment_distance_2d(at, a, b);
                if d <= NEAR && best.is_none_or(|(bd, ..)| d < bd) {
                    best = Some((d, l, i));
                }
            }
        }
        let (_, l, start) = best?;
        let (other, found) = &lists[l];
        let line = seams::line_of(found, start);
        // Lit in the UV Editor too, when it shows this object.
        let in_editor = crate::uv_editor::shown_objects(&self.document).contains(&hit.id);
        Some(Picked {
            flat: if in_editor {
                line.iter()
                    .flat_map(|&i| sides_on_texture(&found[i], SeamLook::Hovered))
                    .collect()
            } else {
                Vec::new()
            },
            material,
            other: other.clone(),
            ids: line
                .iter()
                .map(|&i| SeamId {
                    object,
                    points: found[i].points,
                })
                .collect(),
            lines: line
                .iter()
                .map(|&i| line_in_world(&found[i], &hit.surface, hit.model, SeamLook::Hovered))
                .collect(),
        })
    }

    /// Takes the views' clicks while Blend seams is the tool: a click
    /// marks the seam under it, Shift + click one more. Returns whether
    /// it was the tool (and so took them).
    pub(super) fn carry_out_seam_clicks(&mut self, drawing: &SceneDrawing) -> bool {
        if !self.seam_tool_on() {
            return false;
        }
        for event in self.viewport.take_brush_events() {
            let (p, more) = match event {
                BrushEvent::Begin(p) => (p, false),
                BrushEvent::Line(p) => (p, true),
                _ => continue,
            };
            match self.pick_seam(drawing, &p.camera, p.size, p.at) {
                Some(picked) => self.mark_picked(picked, more),
                None => self.note = Some(tr(NO_SEAM_HERE).to_string()),
            }
        }
        // The same clicks in the UV Editor, near either side of a seam
        // (decision 0295).
        for event in self.uv_editor.take_brush_events() {
            let (p, more) = match event {
                BrushEvent::Begin(p) => (p, false),
                BrushEvent::Line(p) => (p, true),
                _ => continue,
            };
            match self.pick_flat_seam(drawing, p.uv, p.zoom) {
                Some(picked) => self.mark_picked(picked, more),
                None => self.note = Some(tr(NO_SEAM_ON_TEXTURE).to_string()),
            }
        }
        true
    }

    /// The Seam blend layer the tool marks seams on in `set` (of
    /// `material`): the chosen layer if it is one, else the topmost one.
    fn seam_target(&self, material: &str, set: &TextureSet) -> Option<LayerPath> {
        let is_blend = |path: &LayerPath| set.layer(path).is_some_and(|l| l.seam_blend.is_some());
        self.paint_layers
            .active(material, set)
            .filter(is_blend)
            .or_else(|| {
                set.listed(true)
                    .into_iter()
                    .map(|(path, _)| path)
                    .find(is_blend)
            })
    }

    /// Marks the line `picked`, Shift + click (`more`) adding it or
    /// taking it off.
    fn mark_picked(&mut self, picked: Picked, more: bool) {
        match &picked.other {
            Some(other) => self.mark_across(&picked.material, other, picked.ids, more),
            None => self.mark_seams(&picked.material, picked.ids, more),
        }
    }

    /// Marks `ids` on `material`'s Seam blend layer (made on top if it
    /// has none): in place of those marked, or with `more` added to them
    /// (or taken off, when all of them are marked already). One undo step.
    fn mark_seams(&mut self, material: &str, ids: Vec<SeamId>, more: bool) {
        let Some((mut set, path)) = self.seam_layer_to_mark(material) else {
            return;
        };
        let Some(blend) = set.layer_mut(&path).and_then(|l| l.seam_blend.as_mut()) else {
            return;
        };
        let count = ids.len();
        let label = if more && blend.marks_all(&ids) {
            blend.unmark(&ids);
            self.note = Some(tr("Unmarked that seam: it isn't blended any more.").to_string());
            crate::text!("Unmark a seam to blend")
        } else {
            blend.mark(ids, more);
            self.note = Some(crate::trf!(
                "Marked the seam between two UV pieces ({} edges): the Seam blend layer blends \
                 it. Shift + click marks more.",
                count
            ));
            crate::text!("Mark seams to blend")
        };
        self.put_seam_sets(vec![(material.to_string(), set)], &path, label, None);
    }

    /// Marks `ids`, where `material` meets `other`, on both materials'
    /// Seam blend layers (made on top where missing), as one undo step
    /// (decision 0296): in place of `material`'s marks and added to
    /// `other`'s; with `more`, added to both, or taken off both when
    /// `material`'s layer marks them all already.
    fn mark_across(&mut self, material: &str, other: &str, ids: Vec<SeamId>, more: bool) {
        if self.document.scene().paint().set(other).is_none() {
            self.note = Some(crate::trf!(
                "Nothing is painted on {} yet, so there is nothing to blend {} with. Paint it \
                 first, then blend where the two meet.",
                other,
                material
            ));
            return;
        }
        let Some((mut set, path)) = self.seam_layer_to_mark(material) else {
            return;
        };
        let Some((mut theirs, their_path)) = self.seam_layer_to_mark(other) else {
            return;
        };
        let Some(blend) = set.layer_mut(&path).and_then(|l| l.seam_blend.as_mut()) else {
            return;
        };
        let unmark = more && blend.marks_all(&ids);
        if unmark {
            blend.unmark(&ids);
        } else {
            blend.mark(ids.iter().copied(), more);
        }
        let Some(their_blend) = theirs
            .layer_mut(&their_path)
            .and_then(|l| l.seam_blend.as_mut())
        else {
            return;
        };
        let label = if unmark {
            their_blend.unmark(&ids);
            self.note = Some(crate::trf!(
                "Unmarked the seam where {} meets {}: neither side is blended any more.",
                material,
                other
            ));
            crate::text!("Unmark a seam to blend")
        } else {
            their_blend.mark(ids.iter().copied(), true);
            self.note = Some(crate::trf!(
                "Marked the seam where {} meets {} ({} edges): each material's Seam blend layer \
                 blends its own side. Shift + click marks more.",
                material,
                other,
                ids.len()
            ));
            crate::text!("Mark seams to blend")
        };
        let sets = vec![(material.to_string(), set), (other.to_string(), theirs)];
        self.put_seam_sets(sets, &path, label, None);
    }

    /// `material`'s texture set, to change, and the Seam blend layer the
    /// tool marks on in it (made on top if it has none); `None`, said in
    /// the status bar, when nothing is painted on it.
    fn seam_layer_to_mark(&mut self, material: &str) -> Option<(TextureSet, LayerPath)> {
        let Some(set) = self.document.scene().paint().set(material).cloned() else {
            self.note = Some(crate::trf!(
                "Nothing is painted on {} yet, so there is nothing to blend. Paint it first, \
                 then blend its seams.",
                material
            ));
            return None;
        };
        let mut set = (*set).clone();
        let path = self
            .seam_target(material, &set)
            .or_else(|| add_seam_layer(&mut set))?;
        Some((set, path))
    }

    /// Gives each material in `sets` its texture set, as one undo step
    /// (`label`, merged under `key` if any), with the first material's
    /// layer at `path` chosen and shown.
    fn put_seam_sets(
        &mut self,
        sets: Vec<(String, TextureSet)>,
        path: &LayerPath,
        label: &'static str,
        key: Option<u64>,
    ) {
        let Some(material) = sets.first().map(|(m, _)| m.clone()) else {
            return;
        };
        let mut commands: Vec<scene::Command> = sets
            .into_iter()
            .map(|(material, set)| scene::Command::SetTextureSet {
                material,
                set: Some(set),
            })
            .collect();
        let done = if commands.len() == 1 {
            let command = commands.remove(0);
            self.document.execute_labelled(command, tr(label), key)
        } else {
            let key = key.unwrap_or_else(|| self.new_step_key());
            self.document.execute_all_labelled(commands, tr(label), key)
        };
        match done {
            Ok(()) => {
                self.paint_layers.show_material(&material);
                self.paint_layers.choose(&material, path.clone(), false);
            }
            Err(why) => self.note = Some(crate::trf!("Couldn't mark the seams: {}.", why)),
        }
    }

    /// Object `id`'s seams on `material` (its `slots`, shown as
    /// `surface`), then those where it meets each other material, named.
    fn seam_lists(
        &mut self,
        id: u64,
        material: &str,
        surface: &Arc<paint::PaintSurface>,
        slots: &[u32],
    ) -> SeamLists {
        let own = self.seams_of(id, material, surface, slots);
        let across = self.cross_seams_of(id, material, surface, slots);
        std::iter::once((None, own))
            .chain(across.into_iter().map(|c| (Some(c.other), c.seams)))
            .collect()
    }

    /// Carries out what the Paint tab's Blend seams section and the Paint
    /// layers panel's Seam blend rows asked for.
    pub(super) fn carry_out_seam_asks(&mut self, drawing: &SceneDrawing) {
        let dragging = std::mem::take(&mut self.attributes.seam_dragging);
        if dragging && !self.seam_picking.dragging {
            self.seam_picking.drags += 1;
        }
        self.seam_picking.dragging = dragging;
        if let Some((material, path)) = self.paint_layers.seam_asked.take() {
            self.mark_showing(&material, Some(path), drawing);
        }
        let Some(asked) = self.attributes.seam_asked.take() else {
            return;
        };
        let material = self.attributes.seam_tool.0.clone();
        if material.is_empty() {
            return;
        }
        if asked == SeamAsk::Showing {
            self.mark_showing(&material, None, drawing);
            return;
        }
        let Some(set) = self.document.scene().paint().set(&material).cloned() else {
            return;
        };
        let mut set = (*set).clone();
        let Some(path) = self.seam_target(&material, &set) else {
            return;
        };
        let Some(blend) = set.layer_mut(&path).and_then(|l| l.seam_blend.as_mut()) else {
            return;
        };
        let mut sets = Vec::new();
        let (label, key) = match asked {
            SeamAsk::Width(width) => {
                blend.width = SeamBlend::usable_width(width);
                (
                    crate::text!("Seam blend width"),
                    Some(WIDTH_KEYS | self.seam_picking.drags),
                )
            }
            SeamAsk::Unmark | SeamAsk::Showing => {
                let cleared = std::mem::take(&mut blend.seams);
                // Where it meets other materials, their sides are
                // unmarked too, so neither is left half blended.
                sets = self.unmark_across(&material, &cleared, drawing);
                (crate::text!("Unmark the seams to blend"), None)
            }
        };
        sets.insert(0, (material.clone(), set));
        self.put_seam_sets(sets, &path, label, key);
    }

    /// The other materials' texture sets with `cleared` (seams where
    /// `material` meets them) unmarked from their Seam blend layers:
    /// those that changed.
    fn unmark_across(
        &mut self,
        material: &str,
        cleared: &[SeamId],
        drawing: &SceneDrawing,
    ) -> Vec<(String, TextureSet)> {
        let cleared: HashSet<&SeamId> = cleared.iter().collect();
        let mut by_other: BTreeMap<String, Vec<SeamId>> = BTreeMap::new();
        for shape in self.seam_shapes(material, drawing) {
            for c in &shape.across {
                let ids = c.seams.iter().map(|seam| SeamId {
                    object: shape.id,
                    points: seam.points,
                });
                by_other
                    .entry(c.other.clone())
                    .or_default()
                    .extend(ids.filter(|id| cleared.contains(id)));
            }
        }
        let mut out = Vec::new();
        for (other, ids) in by_other {
            let Some(set) = self.document.scene().paint().set(&other) else {
                continue;
            };
            let mut set = (**set).clone();
            let mut changed = false;
            for (path, _) in set.listed(true) {
                if let Some(blend) = set.layer_mut(&path).and_then(|l| l.seam_blend.as_mut()) {
                    let before = blend.seams.len();
                    blend.unmark(&ids);
                    changed |= blend.seams.len() != before;
                }
            }
            if changed {
                out.push((other, set));
            }
        }
        out
    }

    /// Marks every seam of `material` that shows (painted differently on
    /// its two sides, under the Seam blend layer at `path`, or the
    /// tool's) on that layer, with those already marked. Where it meets
    /// another material, its painting under its Seam blend layers is the
    /// other side, and the seams that show are marked on its layer too
    /// (decision 0296). One undo step.
    fn mark_showing(&mut self, material: &str, path: Option<LayerPath>, drawing: &SceneDrawing) {
        let Some(set) = self.document.scene().paint().set(material).cloned() else {
            self.note = Some(crate::trf!(
                "Nothing is painted on {} yet, so no seam shows.",
                material
            ));
            return;
        };
        let mut set = (*set).clone();
        let path = path
            .filter(|p| set.layer(p).is_some_and(|l| l.seam_blend.is_some()))
            .or_else(|| self.seam_target(material, &set));
        let below = match &path {
            Some(p) => set.below(p),
            None => set.layers.clone(),
        };
        let Some(image) = paint::composite::flatten(&below, set.size) else {
            return;
        };
        let mut ids = BTreeSet::new();
        // The other materials' paintings, each mixed once.
        let mut theirs: BTreeMap<String, Option<Image>> = BTreeMap::new();
        let mut across: BTreeMap<String, BTreeSet<SeamId>> = BTreeMap::new();
        for shape in self.seam_shapes(material, drawing) {
            let id_of = |seam: &Seam| SeamId {
                object: shape.id,
                points: seam.points,
            };
            for i in seams::showing(&shape.seams, &image) {
                ids.insert(id_of(&shape.seams[i]));
            }
            for c in &shape.across {
                let other = theirs.entry(c.other.clone()).or_insert_with(|| {
                    let set = self.document.scene().paint().set(&c.other)?;
                    paint::composite::flatten(&set.below_seam_blends(), set.size)
                });
                let Some(other) = other else {
                    continue;
                };
                for i in seams::showing_between(&c.seams, &image, other) {
                    across
                        .entry(c.other.clone())
                        .or_default()
                        .insert(id_of(&c.seams[i]));
                }
            }
        }
        let crossing: usize = across.values().map(BTreeSet::len).sum();
        if ids.is_empty() && crossing == 0 {
            self.note = Some(crate::trf!(
                "No seam shows on {}: the paint already matches across every seam.",
                material
            ));
            return;
        }
        let Some(path) = path.or_else(|| add_seam_layer(&mut set)) else {
            return;
        };
        let Some(blend) = set.layer_mut(&path).and_then(|l| l.seam_blend.as_mut()) else {
            return;
        };
        let count = ids.len() + crossing;
        blend.mark(
            ids.into_iter().chain(across.values().flatten().copied()),
            true,
        );
        let mut sets = vec![(material.to_string(), set)];
        for (other, ids) in across {
            if let Some((mut their_set, their_path)) = self.seam_layer_to_mark(&other)
                && let Some(blend) = their_set
                    .layer_mut(&their_path)
                    .and_then(|l| l.seam_blend.as_mut())
            {
                blend.mark(ids, true);
                sets.push((other, their_set));
            }
        }
        self.note = Some(if crossing > 0 {
            crate::trf!(
                "Marked {} seam edges that show, {} of them where {} meets another material: the \
                 Seam blend layers blend them.",
                count,
                crossing,
                material
            )
        } else {
            crate::trf!(
                "Marked {} seam edges that show: the Seam blend layer blends them.",
                count
            )
        });
        self.put_seam_sets(
            sets,
            &path,
            crate::text!("Mark every seam that shows"),
            None,
        );
    }
}

/// Puts a new, empty Seam blend layer on top of `set`; where it went.
fn add_seam_layer(set: &mut TextureSet) -> Option<LayerPath> {
    let mut layer = Layer::paint(tr("Seam blend"), Image::new(set.size)?);
    layer.seam_blend = Some(SeamBlend::default());
    set.insert_layer(layer, None)
}

/// `seam` of `surface`, placed by `model`, as a line in the world.
fn line_in_world(
    seam: &Seam,
    surface: &paint::PaintSurface,
    model: Mat4,
    look: SeamLook,
) -> SeamLine {
    let normals = model.inverse().transpose();
    SeamLine {
        ends: seam.ends.map(|p| model.transform_point3(p)),
        facing: seam
            .facing(surface)
            .map(|n| normals.transform_vector3(n).normalize_or_zero()),
        look,
    }
}

/// `seam`'s two sides on the texture, drawn as `look`.
fn sides_on_texture(seam: &Seam, look: SeamLook) -> [UvSeamLine; 2] {
    seam.sides.map(|side| UvSeamLine {
        uvs: side.uvs,
        look,
    })
}

/// How far `p` is from the segment between `ends`.
fn segment_distance_3d(p: Vec3, [a, b]: [Vec3; 2]) -> f32 {
    let d = b - a;
    let t = if d.length_squared() > 0.0 {
        ((p - a).dot(d) / d.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + d * t)
}

/// How far `p` is from the segment from `a` to `b`, on the screen.
fn segment_distance_2d(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let d = b - a;
    let t = if d.length_squared() > 0.0 {
        ((p - a).dot(d) / d.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + d * t)
}
