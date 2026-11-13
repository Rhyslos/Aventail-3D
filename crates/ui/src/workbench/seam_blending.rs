//! Seam blend layers in the painter ([decision 0293](../../../../docs/decisions/0293-blend-seams.md)):
//! making each Seam blend layer's pixels from the layers under it, on
//! another thread, whenever what it is made from changes.
//!
//! A Seam blend layer keeps what its pixels were made from
//! (`SeamBlend::made`): a hash of its seams and width, the texture's size
//! and realism switch, the model's UVs, where its seams are on the
//! texture, the settings of the layers under it and their pixels on the
//! tiles near its seams (`seam_blend::tiles_used`). Whenever the scene or
//! the drawing changes and nothing is held down, a layer whose hash
//! differs is made again: after its seams or width change, after paint
//! lands near its seams, after the UVs change. Panning, orbiting and
//! painting far from its seams make nothing. The result joins the undo
//! step of the change that asked for it, as graph fills do (decision
//! 0261), so undo puts back the layer's settings and pixels together.
//!
//! The pixels' hashes are kept by tile (each tile is shared and never
//! changed in place, so one seen before needn't be read again).

use super::Workbench;
use crate::drawing::SceneDrawing;
use paint::SeamBrushShape;
use paint::seam_blend::{self, BlendShape};
use paint::seams::{self, Seam};
use paint::{
    Channel, ChannelPart, Content, Image, Layer, LayerPath, PaintSurface, SeamBlend, TextureSet,
    Tile,
};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

/// The most tile hashes kept before those of tiles nothing else holds are
/// let go.
const MAX_TILE_HASHES: usize = 1 << 16;

/// A layer's pixels as made: its colour, and each realism channel's.
struct Made {
    colour: Image,
    channels: Vec<(Channel, Image)>,
}

/// A Seam blend layer being made.
struct Job {
    material: String,
    path: LayerPath,
    /// The layer's settings when it started (its `made` aside).
    blend: SeamBlend,
    key: u64,
    revision: u64,
    from: Receiver<Option<Made>>,
}

/// A shown shape using a material: the object's id, its surface, the
/// material's slots on it and its seams there.
pub(super) type SeamShape = (u64, Arc<PaintSurface>, Vec<u32>, Arc<Vec<Seam>>);

/// A shape's seams on one material, kept while its surface is the same.
type KeptSeams = (Arc<PaintSurface>, Vec<u32>, Arc<Vec<Seam>>);

/// The Seam blend layers' state.
#[derive(Default)]
pub(crate) struct SeamBlends {
    job: Option<Job>,
    /// The scene and drawing last looked through.
    checked: Option<(u64, u64)>,
    /// Each object's seams on each material.
    seams: HashMap<(u64, String), KeptSeams>,
    /// Each tile's pixels hashed, by the tile (kept alive here, so the
    /// same pointer is always the same tile).
    tile_hashes: HashMap<usize, (Tile, u64)>,
    /// How many of each layer's seams are no longer on the model.
    pub(super) missing: HashMap<(String, LayerPath), usize>,
    /// The seam blend brush's last shape (decision 0294).
    brush_shape: Option<Arc<SeamBrushShape>>,
}

impl std::fmt::Debug for SeamBlends {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SeamBlends")
            .field("busy", &self.job.is_some())
            .field("shapes", &self.seams.len())
            .finish()
    }
}

impl SeamBlends {
    /// Whether a Seam blend layer is being made.
    #[cfg(test)]
    pub(crate) fn busy(&self) -> bool {
        self.job.is_some()
    }

    /// Stops any making under way and lets go of what is kept (a new
    /// project, or the tab going to sleep).
    pub(super) fn cancel(&mut self) {
        self.job = None;
        self.checked = None;
        self.seams.clear();
        self.tile_hashes.clear();
        self.missing.clear();
        self.brush_shape = None;
    }

    /// The hash of `tile`'s pixels (0 for an empty one).
    fn tile_hash(&mut self, tile: Option<&Tile>) -> u64 {
        let Some(tile) = tile else {
            return 0;
        };
        let at = Arc::as_ptr(tile) as usize;
        if let Some((_, hash)) = self.tile_hashes.get(&at) {
            return *hash;
        }
        if self.tile_hashes.len() >= MAX_TILE_HASHES {
            self.tile_hashes
                .retain(|_, (kept, _)| Arc::strong_count(kept) > 1);
        }
        let mut h = DefaultHasher::new();
        tile.hash(&mut h);
        let hash = h.finish() | 1;
        self.tile_hashes.insert(at, (Arc::clone(tile), hash));
        hash
    }
}

/// A Seam blend layer waiting to be made.
struct Wanted {
    material: String,
    path: LayerPath,
    blend: SeamBlend,
    key: u64,
}

impl Workbench {
    /// Each frame after the drawing is made: takes a finished Seam blend
    /// layer, and starts the next one waiting, when nothing is held down.
    pub(super) fn update_seam_blends(&mut self, drawing: &SceneDrawing, ctx: &egui::Context) {
        self.collect_seam_blend();
        self.paint_layers.seam_making = self
            .seam_blends
            .job
            .as_ref()
            .map(|j| (j.material.clone(), j.path.clone()));
        if self.seam_blends.job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
            return;
        }
        let held = ctx.input(|i| i.pointer.any_down()) || self.paint_layers.is_dragging();
        let now = (self.document.revision(), drawing.version);
        if held || self.seam_blends.checked == Some(now) {
            return;
        }
        self.seam_blends.checked = Some(now);
        if let Some(wanted) = self.next_seam_blend(drawing) {
            self.start_seam_blend(wanted, drawing);
            ctx.request_repaint();
        }
        self.paint_layers.seam_missing = self.seam_blends.missing.clone();
    }

    /// The seams of object `id` (shown as `surface`) on the material in
    /// `slots`, worked out once while its shape stays the same.
    pub(super) fn seams_of(
        &mut self,
        id: u64,
        material: &str,
        surface: &Arc<PaintSurface>,
        slots: &[u32],
    ) -> Arc<Vec<Seam>> {
        let key = (id, material.to_string());
        if let Some((kept, kept_slots, seams)) = self.seam_blends.seams.get(&key)
            && Arc::ptr_eq(kept, surface)
            && kept_slots == slots
        {
            return Arc::clone(seams);
        }
        let seams = Arc::new(seams::seams_of(surface, slots));
        self.seam_blends.seams.insert(
            key,
            (Arc::clone(surface), slots.to_vec(), Arc::clone(&seams)),
        );
        seams
    }

    /// What a seam blend stroke on object `id` (shown as `surface`) on
    /// `material` in `slots` needs, for a `size`-pixel texture (decision
    /// 0294): made once and kept while the shape, its seams and the size
    /// stay the same.
    pub(super) fn seam_brush_shape(
        &mut self,
        id: u64,
        material: &str,
        surface: &Arc<PaintSurface>,
        slots: &[u32],
        size: u32,
    ) -> Arc<SeamBrushShape> {
        let seams = self.seams_of(id, material, surface, slots);
        if let Some(kept) = &self.seam_blends.brush_shape
            && kept.is_for(surface, size)
            && std::ptr::eq(kept.seams(), seams.as_slice())
        {
            return Arc::clone(kept);
        }
        let made = Arc::new(SeamBrushShape::new(Arc::clone(surface), slots, seams, size));
        self.seam_blends.brush_shape = Some(Arc::clone(&made));
        made
    }

    /// The shown shapes using `material`, with their seams on it.
    pub(super) fn seam_shapes(&mut self, material: &str, drawing: &SceneDrawing) -> Vec<SeamShape> {
        self.material_shapes(material, drawing)
            .into_iter()
            .map(|(id, surface, slots)| {
                let seams = self.seams_of(id, material, &surface, &slots);
                (id, surface, slots, seams)
            })
            .collect()
    }

    /// The first Seam blend layer whose pixels aren't made from what it
    /// would be made from now.
    fn next_seam_blend(&mut self, drawing: &SceneDrawing) -> Option<Wanted> {
        let sets: Vec<(String, Arc<TextureSet>)> = self
            .document
            .scene()
            .paint()
            .sets()
            .map(|(m, s)| (m.to_string(), Arc::clone(s)))
            .collect();
        self.seam_blends.missing.clear();
        let mut wanted = None;
        for (material, set) in sets {
            let layers: Vec<(LayerPath, SeamBlend)> = set
                .listed(true)
                .into_iter()
                .filter_map(|(path, _)| Some((path.clone(), set.layer(&path)?.seam_blend.clone()?)))
                .collect();
            if layers.is_empty() {
                continue;
            }
            let shapes = self.seam_shapes(&material, drawing);
            let layout = self.layout_key(&material, drawing);
            for (path, blend) in layers {
                let key = self.seam_blend_key(&material, &set, &path, &blend, &shapes, layout);
                if key != blend.made && wanted.is_none() {
                    wanted = Some(Wanted {
                        material: material.clone(),
                        path,
                        blend,
                        key,
                    });
                }
            }
        }
        wanted
    }

    /// What the layer at `path` of `set` would be made from now, hashed
    /// (never 0); and how many of its seams aren't on the model, noted
    /// for the panels.
    fn seam_blend_key(
        &mut self,
        material: &str,
        set: &TextureSet,
        path: &LayerPath,
        blend: &SeamBlend,
        shapes: &[SeamShape],
        layout: u64,
    ) -> u64 {
        let view = blend_shapes(shapes);
        let (found, missing) = seam_blend::resolve(&view, &blend.seams);
        let tiles = seam_blend::tiles_used(&view, &blend.seams, blend.width, set.size);
        let mut h = DefaultHasher::new();
        (
            &blend.seams,
            blend.width.to_bits(),
            set.size,
            set.realism,
            layout,
        )
            .hash(&mut h);
        for (n, seam) in found {
            n.hash(&mut h);
            for side in seam.sides {
                for uv in side.uvs.into_iter().chain([side.inward]) {
                    (uv.x.to_bits(), uv.y.to_bits()).hash(&mut h);
                }
            }
        }
        let below = set.below(path);
        settings_text(&below).hash(&mut h);
        let mut images = Vec::new();
        images_of(&below, &mut images);
        for image in images {
            for &tile in &tiles {
                self.seam_blends.tile_hash(image.tile(tile)).hash(&mut h);
            }
        }
        if missing > 0 {
            self.seam_blends
                .missing
                .insert((material.to_string(), path.clone()), missing);
        }
        h.finish().max(1)
    }

    /// Starts making `wanted` on another thread.
    fn start_seam_blend(&mut self, wanted: Wanted, drawing: &SceneDrawing) {
        let Wanted {
            material,
            path,
            blend,
            key,
        } = wanted;
        let Some(set) = self.document.scene().paint().set(&material).cloned() else {
            return;
        };
        let shapes = self.seam_shapes(&material, drawing);
        let (to, from) = channel();
        let chosen = blend.seams.clone();
        let width = blend.width;
        let below = set.below(&path);
        std::thread::spawn(move || {
            let made = make(&set, below, &shapes, &chosen, width);
            let _ = to.send(made);
        });
        self.seam_blends.job = Some(Job {
            material,
            path,
            blend,
            key,
            revision: self.document.revision(),
            from,
        });
    }

    /// Takes a finished Seam blend layer into its place.
    fn collect_seam_blend(&mut self) {
        let Some(job) = &self.seam_blends.job else {
            return;
        };
        let made = match job.from.try_recv() {
            Ok(made) => made,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => None,
        };
        let Some(job) = self.seam_blends.job.take() else {
            return;
        };
        // Something changed while it was made: look again, for the scene
        // as it is now.
        self.seam_blends.checked = None;
        let Some(made) = made else {
            return;
        };
        if self.document.revision() != job.revision {
            return;
        }
        let Some(set) = self.document.scene().paint().set(&job.material) else {
            return;
        };
        let mut set = (**set).clone();
        let size = set.size;
        let realism = set.realism;
        let Some(layer) = set.layer_mut(&job.path) else {
            return;
        };
        let same = layer
            .seam_blend
            .as_ref()
            .is_some_and(|b| b.seams == job.blend.seams && b.width == job.blend.width);
        if !same || made.colour.size() != size {
            return;
        }
        put(layer, made, job.key, realism);
        let command = scene::Command::SetTextureSet {
            material: job.material,
            set: Some(set),
        };
        if let Err(why) = self.document.execute_with_last(command) {
            self.note = Some(crate::trf!("Couldn't blend the seams: {}.", why));
        }
    }
}

/// `shapes` as the blend sees them.
fn blend_shapes(shapes: &[SeamShape]) -> Vec<BlendShape<'_>> {
    shapes
        .iter()
        .map(|(id, surface, slots, seams)| BlendShape {
            id: *id,
            surface,
            slots,
            seams,
        })
        .collect()
}

/// The layer's pixels made from `below` (the layers under it in `set`):
/// its colour, and with the realism channels each channel's.
fn make(
    set: &TextureSet,
    below: Vec<Layer>,
    shapes: &[SeamShape],
    chosen: &[paint::SeamId],
    width: f32,
) -> Option<Made> {
    let view = blend_shapes(shapes);
    let tiles = seam_blend::tiles_used(&view, chosen, width, set.size);
    let mixed = paint::composite::flatten_tiles(&below, set.size, &tiles)?;
    let colour = seam_blend::blend(&mixed, &view, chosen, width)?;
    let mut channels = Vec::new();
    if set.realism {
        let under = TextureSet {
            size: set.size,
            layers: below,
            realism: true,
            glow_strength: set.glow_strength,
        };
        for channel in Channel::ALL {
            let view_set = paint::channel::channel_view(&under, channel);
            let mixed = paint::composite::flatten_tiles(&view_set.layers, set.size, &tiles)?;
            channels.push((channel, seam_blend::blend(&mixed, &view, chosen, width)?));
        }
    }
    Some(Made { colour, channels })
}

/// Puts `made` into `layer`, marked made from `key`.
fn put(layer: &mut Layer, made: Made, key: u64, realism: bool) {
    layer.content = Content::Paint(made.colour);
    for channel in Channel::ALL {
        *layer.channels.get_mut(channel) = None;
    }
    if realism {
        for (channel, image) in made.channels {
            *layer.channels.get_mut(channel) = Some(ChannelPart::Paint(image));
        }
    }
    if let Some(blend) = &mut layer.seam_blend {
        blend.made = key;
    }
}

/// The layers' settings as text, their strokes and pixels left out (the
/// pixels are hashed by tile; strokes don't change what shows).
fn settings_text(layers: &[Layer]) -> String {
    fn stripped(layer: &Layer) -> Layer {
        let mut out = layer.clone();
        out.strokes = Vec::new();
        if let Some(mask) = &mut out.mask {
            mask.strokes = Vec::new();
        }
        if let Content::Group(children) = &mut out.content {
            *children = children.iter().map(stripped).collect();
        }
        out
    }
    let stripped: Vec<Layer> = layers.iter().map(stripped).collect();
    format!("{stripped:?}")
}

/// Every image the layers mix from (paint, masks, channels), groups
/// included, in order.
fn images_of<'a>(layers: &'a [Layer], out: &mut Vec<&'a Image>) {
    for layer in layers {
        match &layer.content {
            Content::Paint(image) => out.push(image),
            Content::Group(children) => images_of(children, out),
            Content::Fill(_) => {}
        }
        if let Some(mask) = &layer.mask {
            out.push(&mask.image);
        }
        out.extend(layer.channels.images());
    }
}
