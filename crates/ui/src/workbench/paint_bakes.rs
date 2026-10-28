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
}

/// The baked maps and any baking under way.
#[derive(Debug, Default)]
pub(super) struct Bakes {
    pub(crate) baked: HashMap<String, Baked>,
    job: Option<Job>,
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
                "Baking {} of {} maps…",
                job.done.len() + 1,
                BakeKind::ALL.len()
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
            let current = self.following.key_of(&material);
            let size = self.document.scene().texture().size;
            section.status = if current.is_some_and(|k| k != baked.key) || size != baked.size {
                tr("Out of date: the shape or texture size changed since. Bake again.").to_string()
            } else {
                crate::trf!("Baked at {} px.", baked.size)
            };
        }
    }

    /// Starts baking `material`'s maps from the shown shapes using it.
    fn start_bake(&mut self, material: &str, drawing: &SceneDrawing) {
        let ids: Vec<scene::ObjectId> = self
            .document
            .scene()
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        let mut shapes: Vec<(Arc<PaintSurface>, Vec<u32>)> = Vec::new();
        for id in ids {
            let slots = self.slots_of(id, material);
            if slots.is_empty() {
                continue;
            }
            let Some(mesh) = super::brushing::drawn_mesh(drawing, id) else {
                continue;
            };
            shapes.push((self.brush_surface(id, &mesh), slots));
        }
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
                let Some(image) = bake::bake(&sources, kind, &settings) else {
                    return;
                };
                if to.send((kind, image)).is_err() {
                    return;
                }
            }
        });
        self.bakes.job = Some(Job {
            material: material.to_string(),
            key: self.following.key_of(material).unwrap_or_default(),
            size,
            done: Vec::new(),
            from,
        });
    }

    /// Takes the maps baked so far; keeps them all once every one is.
    fn collect_bakes(&mut self, ctx: &egui::Context) {
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
        self.note =
            Some(tr("Baked the maps: they're in the Paint layers panel under Bakes.").to_string());
        self.bakes.baked.insert(
            job.material,
            Baked {
                key: job.key,
                size: job.size,
                maps,
            },
        );
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
