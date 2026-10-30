//! Baking the texture set's maps ([decision 0192](../../../../docs/decisions/0192-the-baker.md)):
//! when the Paint layers panel's Bake maps is clicked, the shown set's
//! maps are baked from the shapes using its material, on another thread,
//! and kept here (they are worked out from the shape, so they aren't
//! saved or undone). The panel shows them small, and says when the shape
//! or the texture size has changed since.

use super::Workbench;
use crate::drawing::SceneDrawing;
use crate::paint_layers::bakes::kind_name;
use crate::text::tr;
use bake::{BakeKind, BakeSettings};
use paint::{Image, PaintSurface};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

/// A texture set's baked maps.
pub(crate) struct Baked {
    /// The shapes' layout and the texture size they were baked for.
    key: u64,
    size: u32,
    /// Which bake this was (`Bakes::bakes_made`).
    made: u64,
    /// Each map, and its small picture for the panel.
    pub(crate) maps: Vec<(BakeKind, Image, egui::TextureHandle)>,
}

impl std::fmt::Debug for Baked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Baked")
            .field("key", &self.key)
            .field("size", &self.size)
            .field("maps", &self.maps.len())
            .finish()
    }
}

/// Baking under way.
#[derive(Debug)]
struct Job {
    material: String,
    key: u64,
    size: u32,
    done: Vec<(BakeKind, Image)>,
    from: Receiver<(BakeKind, Image)>,
    /// How far the map being baked is, and its cancel switch.
    progress: Arc<bake::Progress>,
}

/// The baked maps and any baking under way.
#[derive(Debug, Default)]
pub(super) struct Bakes {
    pub(crate) baked: HashMap<String, Baked>,
    job: Option<Job>,
    /// The map shown on the model as a guide (decision 0193): which
    /// material's, which map.
    guide: Option<(String, BakeKind)>,
    /// The guides as the views get them.
    pub(super) guides: crate::paint_view::Guides,
    /// What `guides` was made from: the guide, its strength, and the
    /// bake it came from.
    guides_made: Option<(String, BakeKind, u32, u64)>,
    /// Goes up with every bake, so guides know to follow.
    bakes_made: u64,
    /// The layout key of a material's shapes, and the drawing it was
    /// worked out for.
    layout: Option<(u64, String, u64)>,
}

impl Bakes {
    /// The baked map `kind` of `material`'s set, if baked.
    #[cfg(test)]
    pub(crate) fn map(&self, material: &str, kind: BakeKind) -> Option<&Image> {
        self.baked
            .get(material)?
            .maps
            .iter()
            .find(|(k, _, _)| *k == kind)
            .map(|(_, image, _)| image)
    }

    /// Whether baking is under way.
    #[cfg(test)]
    pub(crate) fn busy(&self) -> bool {
        self.job.is_some()
    }
}

impl Workbench {
    /// Each frame after the drawing is made: starts baking when asked,
    /// collects what's done, and fills in the panel's Bakes section.
    pub(super) fn update_bakes(&mut self, drawing: &SceneDrawing, ctx: &egui::Context) {
        let material = self.paint_layers.material(&self.document);
        if self.paint_layers.bakes.asked
            && self.bakes.job.is_none()
            && let Some(material) = &material
        {
            self.start_bake(material, drawing);
        }
        self.collect_bakes(ctx);
        self.update_guide(material.as_deref());
        // What the panel shows for its material.
        let section = &mut self.paint_layers.bakes;
        section.busy = self.bakes.job.is_some();
        section.shown.clear();
        section.status.clear();
        let Some(material) = material else {
            return;
        };
        if let Some(job) = &self.bakes.job {
            section.status = crate::trf!(
                "Baking {} of {} maps… {}%",
                job.done.len() + 1,
                BakeKind::ALL.len(),
                (job.progress.share() * 100.0).floor()
            );
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        let Some(baked) = self.bakes.baked.get(&material) else {
            if self.bakes.job.is_none() {
                section.status =
                    tr("Not baked yet: Bake maps works them out from the shape.").to_string();
            }
            return;
        };
        section.shown = baked
            .maps
            .iter()
            .map(|(kind, _, handle)| (*kind, handle.id()))
            .collect();
        if self.bakes.job.is_none() {
            let (baked_key, baked_size) = (baked.key, baked.size);
            let current = self.layout_key(&material, drawing);
            let size = self.document.scene().texture().size;
            let section = &mut self.paint_layers.bakes;
            section.status = if current != baked_key || size != baked_size {
                tr("Out of date: the shape or texture size changed since. Bake again.").to_string()
            } else {
                crate::trf!("Baked at {} px.", baked_size)
            };
        }
    }

    /// Where `material`'s shapes lie on the texture
    /// (`paint::follow::layout_key`), worked out once a drawing.
    fn layout_key(&mut self, material: &str, drawing: &SceneDrawing) -> u64 {
        if let Some((version, m, key)) = &self.bakes.layout
            && *version == drawing.version
            && m == material
        {
            return *key;
        }
        let shapes = self.material_shapes(material, drawing);
        let view: Vec<paint::follow::Shape<'_>> = shapes
            .iter()
            .map(|(id, surface, slots)| paint::follow::Shape {
                id: *id,
                surface,
                slots,
            })
            .collect();
        let key = paint::follow::layout_key(&view);
        self.bakes.layout = Some((drawing.version, material.to_string(), key));
        key
    }

    /// The shown shapes using `material`: their ids, surfaces and slots.
    pub(super) fn material_shapes(
        &mut self,
        material: &str,
        drawing: &SceneDrawing,
    ) -> Vec<(u64, Arc<PaintSurface>, Vec<u32>)> {
        let ids: Vec<scene::ObjectId> = self
            .document
            .scene()
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        let mut shapes = Vec::new();
        for id in ids {
            let slots = self.slots_of(id, material);
            if slots.is_empty() {
                continue;
            }
            let Some(mesh) = super::brushing::drawn_mesh(drawing, id) else {
                continue;
            };
            shapes.push((id.get(), self.brush_surface(id, &mesh), slots));
        }
        shapes
    }

    /// Starts baking `material`'s maps from the shown shapes using it.
    fn start_bake(&mut self, material: &str, drawing: &SceneDrawing) {
        let key = self.layout_key(material, drawing);
        let shapes: Vec<(Arc<PaintSurface>, Vec<u32>)> = self
            .material_shapes(material, drawing)
            .into_iter()
            .map(|(_, surface, slots)| (surface, slots))
            .collect();
        if shapes.is_empty() {
            self.note = Some(tr("Nothing shown uses this material: nothing to bake.").to_string());
            return;
        }
        let size = self.document.scene().texture().size;
        let settings = BakeSettings {
            rays: self.paint_layers.bakes.quality.rays(),
            ..BakeSettings::default()
        };
        let (to, from) = channel();
        let progress = Arc::new(bake::Progress::default());
        let watched = Arc::clone(&progress);
        std::thread::spawn(move || {
            let sources: Vec<bake::Source<'_>> = shapes
                .iter()
                .map(|(surface, slots)| bake::Source {
                    surface,
                    slots,
                    size,
                })
                .collect();
            for kind in BakeKind::ALL {
                let Some(image) = bake::bake_watched(&sources, kind, &settings, &watched) else {
                    return;
                };
                if to.send((kind, image)).is_err() {
                    return;
                }
            }
        });
        self.bakes.job = Some(Job {
            material: material.to_string(),
            key,
            size,
            done: Vec::new(),
            from,
            progress,
        });
    }

    /// Stops any baking under way (Cancel, or a new project).
    pub(super) fn cancel_bake(&mut self) {
        if let Some(job) = self.bakes.job.take() {
            job.progress
                .cancel
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Takes the maps baked so far; keeps them all once every one is.
    fn collect_bakes(&mut self, ctx: &egui::Context) {
        if std::mem::take(&mut self.paint_layers.bakes.cancel) && self.bakes.job.is_some() {
            self.cancel_bake();
            self.note = Some(tr("Baking cancelled.").to_string());
            return;
        }
        let Some(job) = &mut self.bakes.job else {
            return;
        };
        loop {
            match job.from.try_recv() {
                Ok(map) => job.done.push(map),
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        let Some(job) = self.bakes.job.take() else {
            return;
        };
        if job.done.len() < BakeKind::ALL.len() {
            self.note = Some(tr("Couldn't bake the maps.").to_string());
            return;
        }
        let maps = job
            .done
            .into_iter()
            .map(|(kind, image)| {
                let picture = small(&image, 64);
                let handle = ctx.load_texture(
                    format!("bake {} {}", job.material, kind_name(kind)),
                    picture,
                    egui::TextureOptions::LINEAR,
                );
                (kind, image, handle)
            })
            .collect();
        self.note = Some(
            tr("Baked the maps: they're in the Paint layers panel under Bakes. Click one to see it on the model.")
                .to_string(),
        );
        self.bakes.bakes_made += 1;
        self.bakes.baked.insert(
            job.material,
            Baked {
                key: job.key,
                size: job.size,
                made: self.bakes.bakes_made,
                maps,
            },
        );
    }

    /// Takes the Bakes section's clicks on the maps and its buttons, and
    /// lays the chosen map over the paint in the views (decision 0193).
    fn update_guide(&mut self, material: Option<&str>) {
        let section = &mut self.paint_layers.bakes;
        if let (Some(kind), Some(material)) = (section.toggled.take(), material) {
            let chosen = (material.to_string(), kind);
            self.bakes.guide = (self.bakes.guide.as_ref() != Some(&chosen)).then_some(chosen);
        }
        let (add_layer, as_mask) = (section.add_layer, section.as_mask);
        section.add_layer = false;
        section.as_mask = false;
        let opacity = section.guide_opacity;
        // The guide, while its map is baked.
        let shown = self.bakes.guide.clone().and_then(|(material, kind)| {
            let baked = self.bakes.baked.get(&material)?;
            let image = baked
                .maps
                .iter()
                .find(|(k, _, _)| *k == kind)
                .map(|(_, image, _)| image.clone())?;
            Some((material, kind, image, baked.made))
        });
        self.paint_layers.bakes.guide = shown
            .as_ref()
            .filter(|(m, ..)| Some(m.as_str()) == material)
            .map(|(_, kind, ..)| *kind);
        let made = shown
            .as_ref()
            .map(|(m, kind, _, bake)| (m.clone(), *kind, opacity.to_bits(), *bake));
        if made != self.bakes.guides_made {
            self.bakes.guides.revision += 1;
            self.bakes.guides.by_material.clear();
            if let Some((material, kind, image, _)) = &shown {
                self.bakes.guides.by_material.insert(
                    material.clone(),
                    crate::paint_view::Guide {
                        image: image.clone(),
                        opacity,
                        blend: crate::paint_layers::bakes::kind_blend(*kind),
                    },
                );
            }
            self.bakes.guides_made = made;
        }
        if let Some((material, kind, image, _)) = shown
            && (add_layer || as_mask)
        {
            self.bake_into_layers(&material, kind, image, as_mask);
        }
    }

    /// Puts map `kind` (`image`) of `material`'s bakes into its texture
    /// set: as a new paint layer above the chosen one, or as the chosen
    /// layer's mask (`as_mask`). One undo step.
    fn bake_into_layers(&mut self, material: &str, kind: BakeKind, image: Image, as_mask: bool) {
        let material = &self.own_for_change(material);
        let scene = self.document.scene();
        let mut set = match scene.paint().set(material) {
            Some(set) => (**set).clone(),
            None => match paint::TextureSet::new(scene.texture().size) {
                Some(set) => set,
                None => return,
            },
        };
        if image.size() != set.size {
            self.note = Some(
                tr("The texture size changed since the maps were baked: bake them again.")
                    .to_string(),
            );
            return;
        }
        let name = kind_name(kind);
        let chosen = self.paint_layers.active(material, &set);
        let (label, path) = if as_mask {
            let Some(path) = chosen else {
                self.note = Some(tr("Choose a layer to put the mask on first.").to_string());
                return;
            };
            let Some(layer) = set.layer_mut(&path) else {
                return;
            };
            layer.mask = Some(paint::Mask {
                image,
                white: true,
                inverted: false,
                strokes: Vec::new(),
                smart: None,
            });
            (
                crate::trf!("Use the {} bake as a mask", name.to_lowercase()),
                path,
            )
        } else {
            let mut layer = paint::Layer::paint(name, image);
            layer.blend = crate::paint_layers::bakes::kind_blend(kind);
            let Some(path) = set.insert_layer(layer, chosen.as_deref()) else {
                return;
            };
            (
                crate::trf!("Add the {} bake as a layer", name.to_lowercase()),
                path,
            )
        };
        let command = scene::Command::SetTextureSet {
            material: material.to_string(),
            set: Some(set),
        };
        match self.document.execute_labelled(command, label, None) {
            Ok(()) => self.paint_layers.choose(material, path, as_mask),
            Err(why) => self.note = Some(crate::trf!("Couldn't change the layers: {}.", why)),
        }
    }
}

/// `image` made `side` pixels across, each pixel the average of those it
/// covers (empty ones shown dark, so the pieces stand out).
fn small(image: &Image, side: u32) -> egui::ColorImage {
    let step = (image.size() / side).max(1);
    let side = (image.size() / step).max(1);
    let mut rgba = Vec::with_capacity((side * side * 4) as usize);
    for y in 0..side {
        for x in 0..side {
            let mut sum = [0u32; 4];
            for dy in 0..step {
                for dx in 0..step {
                    let p = image.pixel(x * step + dx, y * step + dy);
                    let a = u32::from(p[3]);
                    for c in 0..3 {
                        sum[c] += u32::from(p[c]) * a / 255;
                    }
                    sum[3] += a;
                }
            }
            let n = step * step;
            rgba.extend_from_slice(&[
                (sum[0] / n) as u8,
                (sum[1] / n) as u8,
                (sum[2] / n) as u8,
                255,
            ]);
        }
    }
    egui::ColorImage::from_rgba_unmultiplied([side as usize, side as usize], &rgba)
}
