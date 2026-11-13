//! A Stamps box's rows in Attributes ([decision 0285](../../../../docs/decisions/0285-stamp-images.md)):
//! the small pictures it holds, each with a picture, its name, **How
//! often** it is picked and a button taking it off, and **Add a stamp…**.
//!
//! The pictures are read by the app (the existing open dialog, the image
//! reader in `fileio`, see-through parts kept): the editor only asks for
//! one ([`MaterialEditor::stamp_asked`]) and is handed it back
//! ([`MaterialEditor::put_stamp`]). Every change is one
//! [`GraphEdit::SetStamps`] with the whole list (the pictures shared
//! between the old list and the new), so it is one undo step; dragging
//! How often is one step however long the drag.

use super::MaterialEditor;
use super::own_boxes::Target;
use crate::dialog::help;
use crate::text::tr;
use matgraph::stamp::MAX_WEIGHT;
use matgraph::{BoxId, Graph, GraphBox, GraphEdit, MAX_STAMPS, Stamp, Stamps};
use scene::{Document, SceneError};
use std::collections::HashMap;
use std::sync::Arc;

/// The button bringing a stamp in.
pub(crate) const ADD_LABEL: &str = crate::text!("Add a stamp…");

/// The button taking a stamp off.
pub(crate) const TAKE_OFF_LABEL: &str = crate::text!("Take off");

/// How big a stamp's picture is drawn in Attributes, in points.
const THUMB: f32 = 28.0;

/// Stamps' pictures as textures, by their key.
#[derive(Default)]
pub(crate) struct StampTextures(HashMap<u64, egui::TextureHandle>);

impl std::fmt::Debug for StampTextures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("StampTextures").field(&self.0.len()).finish()
    }
}

impl MaterialEditor {
    /// A Stamps box's rows: its stamps (picture, name, How often, Take
    /// off) and Add a stamp….
    pub(super) fn stamp_rows(
        &mut self,
        ui: &mut egui::Ui,
        b: &GraphBox,
        id: BoxId,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        ui.add_space(4.0);
        let Some(stamps) = b.stamps.clone() else {
            help(ui, tr("No stamps brought in yet."));
            self.add_button(ui, id, 0);
            return;
        };
        help(
            ui,
            crate::trf!(
                "{} stamps. Each copy picks one at random; How often makes one more likely.",
                stamps.len()
            ),
        );
        let mut changed: Option<(Option<Stamps>, Option<u64>)> = None;
        for (i, entry) in stamps.entries().iter().enumerate() {
            ui.horizontal(|ui| {
                if let Some(texture) = self.stamp_texture(ui.ctx(), &entry.stamp) {
                    let (w, h) = (entry.stamp.width() as f32, entry.stamp.height() as f32);
                    let scale = THUMB / w.max(h);
                    ui.add(
                        egui::Image::new((texture, egui::vec2(w * scale, h * scale)))
                            .bg_fill(super::look::STAMP_BACK),
                    );
                }
                let name = ui.add(egui::Label::new(entry.stamp.name()).truncate());
                name.on_hover_text(crate::trf!(
                    "{}, kept at {} × {} pixels.",
                    entry.stamp.name(),
                    entry.stamp.width(),
                    entry.stamp.height()
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let off = ui
                        .small_button(tr(TAKE_OFF_LABEL))
                        .on_hover_text(tr("Take this stamp off the box (undo puts it back)"));
                    if off.clicked() {
                        changed = Some((stamps.without(i), None));
                    }
                    let mut weight = entry.weight;
                    let slider = ui
                        .add(egui::DragValue::new(&mut weight).range(1..=MAX_WEIGHT))
                        .on_hover_text(tr(
                            "How often: how likely each copy is to pick this stamp. 2 is twice \
                             as likely as 1.",
                        ));
                    ui.label(tr("How often"));
                    if slider.changed() && weight != entry.weight {
                        let key = self.field_key(id.0 << 8 | 0xE0 | i as u64);
                        changed = Some((Some(stamps.with_weight(i, weight)), Some(key)));
                    }
                });
            });
        }
        self.add_button(ui, id, stamps.len());
        if let Some((list, key)) = changed {
            let edit = GraphEdit::SetStamps {
                node: id,
                stamps: list.map(Arc::new),
            };
            let label = edit.describe();
            if let Err(why) = self.run(document, vec![edit], &label, key, false) {
                *note = Some(why.reason);
            }
        }
    }

    /// Add a stamp…, unless the box holds `held` = as many as it can.
    fn add_button(&mut self, ui: &mut egui::Ui, id: BoxId, held: usize) {
        let full = held >= MAX_STAMPS;
        let add = ui
            .add_enabled(!full, egui::Button::new(tr(ADD_LABEL)))
            .on_hover_text(tr(
                "Pick a small picture (a PNG with see-through parts keeps its outline; JPEG, TGA \
                 and BMP work too). Big pictures are made smaller (512 pixels at most) and kept \
                 in the project.",
            ))
            .on_disabled_hover_text(tr(
                "A Stamps box holds at most 16 stamps: take one off first, or use a second \
                 Stamps box.",
            ));
        if add.clicked() {
            self.ask_for_stamp(id);
        }
    }

    /// `stamp`'s picture as a texture, made once.
    fn stamp_texture(&mut self, ctx: &egui::Context, stamp: &Stamp) -> Option<egui::TextureId> {
        if let Some(handle) = self.stamp_textures.0.get(&stamp.key()) {
            return Some(handle.id());
        }
        let size = [stamp.width() as usize, stamp.height() as usize];
        let image = egui::ColorImage::from_rgba_unmultiplied(size, stamp.rgba());
        let handle = ctx.load_texture(
            format!("stamp {:x}", stamp.key()),
            image,
            egui::TextureOptions::LINEAR,
        );
        let id = handle.id();
        // Few stamps are ever shown: the oldest are let go of past 64.
        if self.stamp_textures.0.len() >= 64 {
            self.stamp_textures.0.clear();
        }
        self.stamp_textures.0.insert(stamp.key(), handle);
        Some(id)
    }

    /// Asks the app for a picture to add to Stamps box `id` of the graph
    /// shown.
    pub(crate) fn ask_for_stamp(&mut self, id: BoxId) {
        if let Some(target) = self.target() {
            self.stamp_for = Some((target, id));
            self.stamp_asked = true;
        }
    }

    /// Adds `stamp` to the Stamps box that asked for it, as one undo step;
    /// what to tell the user.
    pub(crate) fn put_stamp(&mut self, document: &mut Document, stamp: Stamp) -> String {
        let Some((target, id)) = self.stamp_for.take() else {
            return tr("Choose a Stamps box first: the picture goes on it.").to_string();
        };
        let held =
            graph_at(document, &target).and_then(|g| g.get(id).and_then(|b| b.stamps.clone()));
        let name = stamp.name().to_string();
        let see_through = stamp.has_see_through();
        let list = match held {
            Some(list) => list.with_added(stamp),
            None => Stamps::one(stamp),
        };
        let list = match list {
            Ok(list) => list,
            Err(why) => return crate::trf!("{} wasn't added: {}.", name, tr(why)),
        };
        let edit = GraphEdit::SetStamps {
            node: id,
            stamps: Some(Arc::new(list)),
        };
        let label = edit.describe();
        let key = self.next_key();
        match document.execute_all_labelled([target.command(edit)], &label, key) {
            Ok(()) if see_through => crate::trf!(
                "{} is on the Stamps box. Set its Size and Spacing in Attributes.",
                name
            ),
            Ok(()) => crate::trf!(
                "{} is on the Stamps box. It has no see-through parts, so each stamp is its \
                 whole picture: a PNG with a see-through background gives it an outline.",
                name
            ),
            Err(SceneError::Graph(why)) => why.reason,
            Err(other) => other.to_string(),
        }
    }
}

/// The graph `target` names: a graph of the project, or your own box's
/// inside.
fn graph_at(document: &Document, target: &Target) -> Option<Arc<Graph>> {
    let graphs = document.scene().graphs();
    match target {
        Target::Graph(id) => graphs.get(*id).cloned(),
        Target::Own(id) => graphs.own(id).map(|o| o.inner.clone()),
    }
}
