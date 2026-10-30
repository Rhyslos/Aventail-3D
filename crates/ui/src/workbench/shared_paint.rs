//! Paint shared by accident ([decision 0214](../../../../docs/decisions/0214-shared-paint-warning.md)).
//!
//! A texture belongs to a material, as in Substance Painter and Blender:
//! every shape with that material paints on the same texture. When their
//! UVs lie on top of each other (new shapes all fill the same square),
//! paint on one shows on the others. While painting one shape, the
//! workbench looks for that and the Paint layers panel warns, with two
//! fixes: give the shape its own texture, or pack them all onto the
//! shared one.

use super::Workbench;
use crate::paint_layers::own_texture::{self, materials_of, paints};
use crate::paint_layers::shared_note::{SharedFix, SharedPaint};
use crate::text::tr;
use scene::{FaceId, ObjectId};
use std::sync::Arc;

/// How finely UVs are compared, in cells across the texture: fine enough
/// to tell pieces apart, quick enough to run whenever the scene changes.
const CHECK_SIZE: u32 = 256;

/// What was found, and for which scene and shape.
#[derive(Debug, Clone, Default)]
pub(crate) struct SharedLook {
    /// What the last look depended on (see [`look_key`]) and the shape
    /// looked at.
    key: Option<(u64, ObjectId)>,
    /// Whether "Packing…" was shown: packing is done the frame after, so
    /// the note shows while the window waits.
    packing_announced: bool,
}

/// One shape's polygons with one material, in pieces.
type Pieces = Vec<Vec<FaceId>>;

impl Workbench {
    /// Looks for paint shared by accident on the shape being painted, when
    /// the scene or the shape changed, and carries out a fix asked for.
    pub(super) fn update_shared_paint(&mut self, ctx: &egui::Context) {
        if let Some(fix) = self.paint_layers.shared_fix.take()
            && let Some(shared) = self.paint_layers.shared.clone()
        {
            match fix {
                SharedFix::OwnTexture => self.give_own_texture(&shared),
                // Packing moves the paint too, which takes a while: say so
                // first.
                SharedFix::PackTogether if !self.shared_look.packing_announced => {
                    self.shared_look.packing_announced = true;
                    self.note = Some(tr("Packing the pieces onto one texture…").to_string());
                    self.paint_layers.shared_fix = Some(fix);
                    ctx.request_repaint();
                }
                SharedFix::PackTogether => {
                    self.shared_look.packing_announced = false;
                    self.pack_shared(&shared);
                }
            }
        }
        let owner = match self.document.selection().as_slice() {
            [one] if self.attributes.painting => Some(*one),
            _ => None,
        };
        let Some(owner) = owner else {
            self.paint_layers.shared = None;
            self.shared_look.key = None;
            return;
        };
        let key = (look_key(self.document.scene()), owner);
        if self.shared_look.key == Some(key) {
            return;
        }
        self.shared_look.key = Some(key);
        self.paint_layers.shared = shared_paint(self.document.scene(), owner);
    }

    /// Gives `shared.owner` its own copy of the shared material, with a
    /// copy of its painting, in one undo step.
    fn give_own_texture(&mut self, shared: &SharedPaint) {
        let key = self.new_step_key();
        match own_texture::give_own(&mut self.document, shared.owner, &shared.material, key) {
            Ok((name, said)) => {
                self.paint_layers.show_material(&name);
                self.note = Some(said);
            }
            Err(why) => self.note = Some(why),
        }
    }

    /// The material a change for the Paint layers panel's `material`
    /// goes to: the one selected shape's own copy first, when it shares
    /// the standard material with another shape (decision 0232).
    pub(super) fn own_for_change(&mut self, material: &str) -> String {
        let owner = match self.document.selection().as_slice() {
            [one] => *one,
            _ => return material.to_string(),
        };
        self.own_for(owner, material)
    }

    /// The material painting `owner`'s `material` goes to: its own copy
    /// first, when it shares the standard material with another shape
    /// (decision 0232).
    pub(super) fn own_for(&mut self, owner: ObjectId, material: &str) -> String {
        if !own_texture::needs_own(self.document.scene(), owner, material) {
            return material.to_string();
        }
        let key = self.new_step_key();
        let name =
            own_texture::own_if_shared(&mut self.document, owner, material, key, &mut self.note);
        if name != material {
            self.paint_layers.show_material(&name);
        }
        name
    }

    /// Packs the UVs of every shape in `shared` side by side on their
    /// shared texture, as one undo step; their paint moves with them.
    fn pack_shared(&mut self, shared: &SharedPaint) {
        let scene = self.document.scene();
        let shapes: Vec<(ObjectId, Arc<scene::Mesh>)> = std::iter::once(shared.owner)
            .chain(shared.others.iter().map(|(id, _)| *id))
            .filter_map(|id| Some((id, scene.evaluated(id)?.mesh)))
            .collect();
        // All on one texture: by material, never one texture per object.
        let kept = self.uv_settings;
        self.uv_settings.per_object = false;
        self.uv_settings.per_material = true;
        let done = self.pack_shapes(shapes, true, None);
        self.uv_settings = kept;
        if done {
            self.note = Some(
                tr(
                    "Packed onto one texture: each shape has its own part of it now, its paint \
                    moved with it. Ctrl + Z takes it back.",
                )
                .into(),
            );
        }
    }
}

/// A key that changes when any shape's polygons, UVs or materials do, but
/// not with each stroke of paint (the document's revision does): looking
/// again on every frame of a stroke would be slow in a big scene. Shapes
/// are kept evaluated and shared, so a new shape is a new pointer.
fn look_key(scene: &scene::Scene) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for (id, _) in scene.walk() {
        id.hash(&mut hasher);
        if let Some(object) = scene.object(id) {
            for m in &object.materials {
                m.name.hash(&mut hasher);
            }
        }
        if let Some(shape) = scene.evaluated(id) {
            (Arc::as_ptr(&shape.mesh) as usize).hash(&mut hasher);
        }
    }
    scene.trim().rows.len().hash(&mut hasher);
    hasher.finish()
}

/// Paint shared by accident on `owner`: the first of its materials whose
/// texture other shapes share with UVs over its own.
pub(crate) fn shared_paint(scene: &scene::Scene, owner: ObjectId) -> Option<SharedPaint> {
    let object = scene.object(owner)?;
    if !paints(&object.kind) {
        return None;
    }
    let mesh = scene.evaluated(owner)?.mesh;
    if !mesh::uvs::has_uvs(&mesh) {
        return None;
    }
    let rows = super::baking::trim_rows(scene);
    for (slot, material) in materials_of(object).iter().enumerate() {
        // The standard material is never shared by painting: the shape
        // painted gets its own copy first (decision 0232).
        if material.name == scene::Material::standard().name {
            continue;
        }
        let mine = pieces_with(&mesh, slot as u32, &rows);
        if mine.is_empty() {
            continue;
        }
        let mut others = Vec::new();
        for (id, _) in scene.walk() {
            let Some(other) = scene.object(id) else {
                continue;
            };
            if id == owner || !paints(&other.kind) {
                continue;
            }
            let Some(slot) = materials_of(other)
                .iter()
                .position(|m| m.name == material.name)
            else {
                continue;
            };
            let Some(theirs) = scene.evaluated(id).map(|e| e.mesh) else {
                continue;
            };
            let theirs_pieces = pieces_with(&theirs, slot as u32, &rows);
            if lie_over(&mesh, &mine, &theirs, &theirs_pieces) {
                others.push((id, other.name.clone()));
            }
        }
        if !others.is_empty() {
            return Some(SharedPaint {
                owner,
                material: material.name.clone(),
                others,
            });
        }
    }
    None
}

/// `mesh`'s pieces with material `slot`, leaving out those on the trim
/// sheet (which repeat on purpose).
fn pieces_with(mesh: &scene::Mesh, slot: u32, rows: &[(f32, f32)]) -> Pieces {
    if !mesh::uvs::has_uvs(mesh) {
        return Vec::new();
    }
    mesh::uvs::islands(mesh)
        .into_iter()
        .filter_map(|island| {
            let faces: Vec<FaceId> = island
                .into_iter()
                .filter(|f| mesh::material::slot_of(mesh, *f) == slot)
                .collect();
            (!faces.is_empty() && !super::baking::on_trim(mesh, &faces, rows)).then_some(faces)
        })
        .collect()
}

/// Whether any of `b`'s pieces lie over (or exactly on) one of `a`'s.
/// Overlaps inside one shape (its own stacked pieces) don't count.
fn lie_over(a: &scene::Mesh, a_pieces: &Pieces, b: &scene::Mesh, b_pieces: &Pieces) -> bool {
    if a_pieces.is_empty() || b_pieces.is_empty() {
        return false;
    }
    let sheet = |pieces: Vec<uv::check::SheetPiece>| {
        let report = uv::check::check_sheet(&pieces, CHECK_SIZE, 0);
        report
            .overlapping
            .union(&report.stacked)
            .copied()
            .collect::<std::collections::BTreeSet<usize>>()
    };
    let as_pieces = |mesh, pieces: &Pieces| -> Vec<uv::check::SheetPiece> {
        pieces
            .iter()
            .map(|faces| uv::check::SheetPiece {
                mesh,
                faces: faces.clone(),
            })
            .collect()
    };
    let alone_a = sheet(as_pieces(a, a_pieces));
    let alone_b = sheet(as_pieces(b, b_pieces));
    let mut both = as_pieces(a, a_pieces);
    both.extend(as_pieces(b, b_pieces));
    let n = a_pieces.len();
    let together = sheet(both);
    // A piece of each that only lies over something once they're together.
    let a_hit = together.iter().any(|i| *i < n && !alone_a.contains(i));
    let b_hit = together
        .iter()
        .any(|i| *i >= n && !alone_b.contains(&(i - n)));
    a_hit && b_hit
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene::{Command, Document, ObjectKind, PrimitiveKind};

    fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
        doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
            .unwrap();
        doc.selection()[0]
    }

    /// Two cubes sharing the material Wood on purpose.
    fn two_cubes() -> (Document, ObjectId, ObjectId) {
        let mut doc = Document::new();
        let a = create(&mut doc, PrimitiveKind::Cube);
        let b = create(&mut doc, PrimitiveKind::Cube);
        wood(&mut doc, &[a, b]);
        (doc, a, b)
    }

    fn wood(doc: &mut Document, ids: &[ObjectId]) {
        let mut material = scene::Material::standard();
        material.name = "Wood".into();
        for id in ids {
            doc.execute(Command::SetMaterials {
                id: *id,
                materials: vec![material.clone()],
            })
            .unwrap();
        }
    }

    #[test]
    fn the_standard_material_is_never_warned_about() {
        let mut doc = Document::new();
        let a = create(&mut doc, PrimitiveKind::Cube);
        let _b = create(&mut doc, PrimitiveKind::Cube);
        assert!(shared_paint(doc.scene(), a).is_none());
    }

    #[test]
    fn shapes_on_the_same_square_share_their_paint() {
        let (doc, a, b) = two_cubes();
        let shared = shared_paint(doc.scene(), a).expect("found");
        assert_eq!(shared.owner, a);
        assert_eq!(shared.others.len(), 1);
        assert_eq!(shared.others[0].0, b);
    }

    #[test]
    fn a_shape_alone_shares_nothing() {
        let mut doc = Document::new();
        let a = create(&mut doc, PrimitiveKind::Sphere);
        assert!(shared_paint(doc.scene(), a).is_none());
    }

    #[test]
    fn another_material_shares_nothing() {
        let (mut doc, a, b) = two_cubes();
        let mut materials = materials_of(doc.scene().object(b).unwrap());
        materials[0].name = "Stone".into();
        doc.execute(Command::SetMaterials { id: b, materials })
            .unwrap();
        assert!(shared_paint(doc.scene(), a).is_none());
    }

    #[test]
    fn a_new_name_is_never_one_in_use() {
        let (doc, a, _) = two_cubes();
        let name = doc.scene().object(a).unwrap().name.clone();
        let fresh = own_texture::unused_name(doc.scene(), &name, &scene::Material::standard().name);
        // The standard material's copy is named after the shape.
        assert!(fresh.starts_with(&name), "{fresh}");
        let other = own_texture::unused_name(doc.scene(), &name, "Wood");
        assert!(other.starts_with("Wood"), "{other}");
        let used = |id: ObjectId| {
            let o = doc.scene().object(id).unwrap();
            o.materials.iter().any(|m| m.name == fresh)
        };
        assert!(!doc.scene().walk().iter().any(|(id, _)| used(*id)));
    }
}
