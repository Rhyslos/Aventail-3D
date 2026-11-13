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

use super::Workbench;
use super::brushing::drawn_transform;
use crate::attributes::PaintTool;
use crate::drawing::SceneDrawing;
use crate::paint_layers::seam_rows::{SeamAsk, SeamView};
use crate::palette::Mode;
use crate::text::tr;
use crate::transforming::Tool;
use crate::viewport::{BrushEvent, SeamLine, SeamLines, SeamLook};
use glam::{Mat4, Vec2, Vec3};
use paint::seams::{self, Seam};
use paint::{Image, Layer, LayerPath, SeamBlend, SeamId, TextureSet};
use std::collections::{BTreeSet, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

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

/// The Blend seams tool's state.
#[derive(Debug, Default)]
pub(super) struct SeamPicking {
    /// The lines last shown: what they were made from (drawing, material
    /// and marked seams, hashed) and the lines.
    lines: Option<(u64, Arc<Vec<SeamLine>>)>,
    /// Counts the width's drags, for their undo merge keys.
    drags: u64,
    /// Whether the width was being dragged last frame.
    dragging: bool,
}

/// A seam line found under the mouse.
struct Picked {
    material: String,
    /// The edges of the line, by id.
    ids: Vec<SeamId>,
    /// The line in the world, to show.
    lines: Vec<SeamLine>,
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
            return;
        }
        if !self.seam_tool_on() {
            self.viewport.set_seam_lines(None);
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
        let hovered = self
            .viewport
            .pointer()
            .and_then(|p| self.pick_seam(drawing, &p.camera, p.size, p.at))
            .map(|p| p.lines)
            .unwrap_or_default();
        self.viewport
            .set_seam_lines(Some(SeamLines { all, hovered }));
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
        for (id, surface, _, found) in self.seam_shapes(material, drawing) {
            let model = drawn_transform(drawing, scene::ObjectId::from_raw(id));
            for seam in found.iter() {
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
                lines.push(line_in_world(seam, &surface, model, look));
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
        let found = self.seams_of(object, &material, &hit.surface, &slots);
        let per_point = camera.world_per_pixel(hit.world, size).max(1e-6);
        // Only seams near the point hit: not one on the far side that
        // happens to lie under the mouse.
        let reach = per_point * NEAR * 4.0;
        let mut best: Option<(f32, usize)> = None;
        for (i, seam) in found.iter().enumerate() {
            let ends = seam.ends.map(|p| hit.model.transform_point3(p));
            if segment_distance_3d(hit.world, ends) > reach {
                continue;
            }
            let (Some(a), Some(b)) = (camera.project(ends[0], size), camera.project(ends[1], size))
            else {
                continue;
            };
            let d = segment_distance_2d(at, a, b);
            if d <= NEAR && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, i));
            }
        }
        let (_, start) = best?;
        let line = seams::line_of(&found, start);
        Some(Picked {
            material,
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
                Some(picked) => self.mark_seams(&picked.material, picked.ids, more),
                None => self.note = Some(tr(NO_SEAM_HERE).to_string()),
            }
        }
        for event in self.uv_editor.take_brush_events() {
            if matches!(event, BrushEvent::Begin(_) | BrushEvent::Line(_)) {
                self.note = Some(
                    tr("Blend seams picks seams on the model: click one in the 3D view.")
                        .to_string(),
                );
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

    /// Marks `ids` on `material`'s Seam blend layer (made on top if it
    /// has none): in place of those marked, or with `more` added to them
    /// (or taken off, when all of them are marked already). One undo step.
    fn mark_seams(&mut self, material: &str, ids: Vec<SeamId>, more: bool) {
        let Some(set) = self.document.scene().paint().set(material).cloned() else {
            self.note = Some(crate::trf!(
                "Nothing is painted on {} yet, so there is nothing to blend. Paint it first, \
                 then blend its seams.",
                material
            ));
            return;
        };
        let mut set = (*set).clone();
        let Some(path) = self
            .seam_target(material, &set)
            .or_else(|| add_seam_layer(&mut set))
        else {
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
        self.put_seam_set(material, set, &path, label, None);
    }

    /// Gives `material` the texture set `set`, as one undo step
    /// (`label`, merged under `key` if any), with the layer at `path`
    /// chosen and shown.
    fn put_seam_set(
        &mut self,
        material: &str,
        set: TextureSet,
        path: &LayerPath,
        label: &'static str,
        key: Option<u64>,
    ) {
        let command = scene::Command::SetTextureSet {
            material: material.to_string(),
            set: Some(set),
        };
        match self.document.execute_labelled(command, tr(label), key) {
            Ok(()) => {
                self.paint_layers.show_material(material);
                self.paint_layers.choose(material, path.clone(), false);
            }
            Err(why) => self.note = Some(crate::trf!("Couldn't mark the seams: {}.", why)),
        }
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
        let (label, key) = match asked {
            SeamAsk::Width(width) => {
                blend.width = SeamBlend::usable_width(width);
                (
                    crate::text!("Seam blend width"),
                    Some(WIDTH_KEYS | self.seam_picking.drags),
                )
            }
            SeamAsk::Unmark | SeamAsk::Showing => {
                blend.seams.clear();
                (crate::text!("Unmark the seams to blend"), None)
            }
        };
        self.put_seam_set(&material, set, &path, label, key);
    }

    /// Marks every seam of `material` that shows (painted differently on
    /// its two sides, under the Seam blend layer at `path`, or the
    /// tool's) on that layer, with those already marked.
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
        for (id, _, _, found) in self.seam_shapes(material, drawing) {
            for i in seams::showing(&found, &image) {
                ids.insert(SeamId {
                    object: id,
                    points: found[i].points,
                });
            }
        }
        if ids.is_empty() {
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
        let count = ids.len();
        blend.mark(ids, true);
        self.note = Some(crate::trf!(
            "Marked {} seam edges that show: the Seam blend layer blends them.",
            count
        ));
        self.put_seam_set(
            material,
            set,
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
