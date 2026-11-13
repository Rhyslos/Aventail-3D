//! From a photo to a material ([decision 0284](../../../../docs/decisions/0284-image-to-material.md)):
//! an Image box's rows in Attributes (bringing a picture in, the
//! real-world size, the seam check) and **Make into material**, which
//! adds the boxes turning the picture into a material, wired and inside a
//! comment frame saying what each step does.
//!
//! The picture itself is read by the app (the existing open dialog, the
//! image reader in `fileio`): the editor only asks for one
//! ([`MaterialEditor::photo_asked`]) and is handed it back
//! ([`MaterialEditor::put_photo`]).

use super::MaterialEditor;
use super::box_rows::setting_row;
use super::look::{COLUMN, FRAME_MARGIN, FRAME_TITLE, box_rect};
use crate::dialog::{help, main_button, row};
use crate::text::tr;
use glam::Vec2;
use matgraph::boxes::even_light::{EDGES, EVEN_LIGHT, PHOTO_EDGES};
use matgraph::boxes::guess_height::GUESS_HEIGHT;
use matgraph::boxes::guess_roughness::GUESS_ROUGHNESS;
use matgraph::boxes::seamless::SEAMLESS;
use matgraph::{BoxId, Frame, Graph, GraphBox, GraphEdit, Photo, PlugRef, SeamCheck, SettingValue};
use scene::{Document, SceneError};
use std::sync::Arc;

/// The button bringing a picture in.
pub(crate) const BRING_IN_LABEL: &str = crate::text!("Bring in a picture…");

/// The button choosing another picture.
pub(crate) const ANOTHER_LABEL: &str = crate::text!("Choose another picture…");

/// The button adding the chain of boxes.
pub(crate) const MAKE_LABEL: &str = crate::text!("Make into material");

/// The title of the comment frame round the chain.
pub(crate) const FRAME_TITLE_TEXT: &str = crate::text!("From a photo");

/// The note in the comment frame round the chain.
const FRAME_NOTE: &str = crate::text!(
    "1. Image: the picture, and the square part used. 2. Even out light takes out the light and \
     shadow it was taken in. 3. Make seamless hides the joins where it repeats. 4. Guess height \
     and Guess roughness guess the bumps and the shine. 5. Material from height makes the \
     material."
);

/// The colour of the comment frame round the chain: a muted blue.
const FRAME_COLOUR: [u8; 3] = [70, 110, 150];

impl MaterialEditor {
    /// An Image box's rows: the picture it holds, bringing one in, the
    /// real-world size, the seam check and Make into material.
    pub(super) fn photo_rows(
        &mut self,
        ui: &mut egui::Ui,
        graph: &Graph,
        b: &GraphBox,
        id: BoxId,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        ui.add_space(4.0);
        help(
            ui,
            match &b.photo {
                None => tr("No picture brought in yet.").to_string(),
                Some(p) => crate::trf!(
                    "{}, kept at {} × {} pixels.",
                    p.name(),
                    p.width(),
                    p.height()
                ),
            },
        );
        let words = if b.photo.is_some() {
            tr(ANOTHER_LABEL)
        } else {
            tr(BRING_IN_LABEL)
        };
        let bring = main_button(ui, words).on_hover_text(tr(
            "Pick a photo or a painting (PNG, JPEG, TGA or BMP). Big pictures are made smaller \
             (2048 pixels at most) and kept packed in the project.",
        ));
        if bring.clicked() {
            self.ask_for_photo(id);
        }
        if b.photo.is_some() {
            self.seam_row(ui, id, true);
        }
        self.real_size_row(ui, graph, document, note);
        if b.photo.is_some() {
            ui.add_space(4.0);
            let make = main_button(ui, tr(MAKE_LABEL)).on_hover_text(tr(
                "Add the boxes that turn the picture into a material: Even out light, Make \
                 seamless, Guess height, Guess roughness and Material from height, wired up \
                 inside a comment frame. Each has its own settings.",
            ));
            if make.clicked() {
                self.make_into_material(document, id, note);
            }
        }
    }

    /// The seam check of box `id`'s picture, in plain words: whether its
    /// edges match, so it repeats without a join (an Image box and Make
    /// seamless show it), and with `suggest`, what hides the joins.
    pub(super) fn seam_row(&mut self, ui: &mut egui::Ui, id: BoxId, suggest: bool) {
        let Some(value) = self.pictures.value(id) else {
            return;
        };
        let check = SeamCheck::of(value);
        let words = if check.shows() && suggest {
            crate::trf!(
                "The seam check: {} Make seamless after it hides the joins.",
                check.describe()
            )
        } else {
            crate::trf!("The seam check: {}", check.describe())
        };
        help(ui, words);
    }

    /// The graph's real-world size (Material out's), to set with the
    /// picture: how many centimetres the square part shows across.
    fn real_size_row(
        &mut self,
        ui: &mut egui::Ui,
        graph: &Graph,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let registry = self.registry_of(document);
        let Some(out) = graph.first_of("material_out") else {
            help(
                ui,
                tr(
                    "Its real-world size (how many centimetres the square shows across) is set \
                     on Material out, which Make into material adds.",
                ),
            );
            return;
        };
        let Some(spec) = registry
            .get("material_out")
            .and_then(|k| k.info().setting(matgraph::eval::REAL_SIZE))
        else {
            return;
        };
        let now = graph
            .get(out)
            .and_then(|b| b.settings.get(&spec.key))
            .unwrap_or_else(|| spec.standard());
        let changed = row(ui, tr("Real-world size"), |ui| {
            setting_row(ui, spec, now.clone(), registry.palettes())
        });
        help(
            ui,
            tr(
                "How many centimetres the square part shows across (Material out's, the same \
                 for the whole graph): measure something in the photo, a brick or a plank.",
            ),
        );
        if let Some(value) = changed
            && value != now
        {
            let key = self.field_key(out.0 << 8 | 0xFE);
            let edit = GraphEdit::SetSetting {
                node: out,
                key: spec.key.to_string(),
                value,
            };
            let label = edit.describe();
            if let Err(why) = self.run(document, vec![edit], &label, Some(key), false) {
                *note = Some(why.reason);
            }
        }
    }

    /// Asks the app for a picture for Image box `id` of the graph shown.
    pub(crate) fn ask_for_photo(&mut self, id: BoxId) {
        if let Some(target) = self.target() {
            self.photo_for = Some((target, id));
            self.photo_asked = true;
        }
    }

    /// Puts `photo` on the Image box that asked for it, as one undo step;
    /// what to tell the user.
    pub(crate) fn put_photo(&mut self, document: &mut Document, photo: Photo) -> String {
        let Some((target, id)) = self.photo_for.take() else {
            return tr("Choose an Image box first: the picture goes on it.").to_string();
        };
        let name = photo.name().to_string();
        let edit = GraphEdit::SetPhoto {
            node: id,
            photo: Some(Arc::new(photo)),
        };
        let label = edit.describe();
        let key = self.next_key();
        match document.execute_all_labelled([target.command(edit)], &label, key) {
            Ok(()) => crate::trf!(
                "{} is on the Image box. Choose its square part in Attributes, then Make into \
                 material.",
                name
            ),
            Err(SceneError::Graph(why)) => why.reason,
            Err(other) => other.to_string(),
        }
    }

    /// Adds the boxes turning Image box `id`'s picture into a material
    /// (decision 0284), wired up inside a comment frame, as one undo step.
    pub(crate) fn make_into_material(
        &mut self,
        document: &mut Document,
        id: BoxId,
        note: &mut Option<String>,
    ) {
        let Some(graph) = self.current(document).cloned() else {
            return;
        };
        let registry = self.registry_of(document);
        let Some(mut chain) = chain(&graph, id) else {
            return;
        };
        // Where the boxes end up once tidied, for the frame round them.
        let mut after = (*graph).clone();
        for edit in &chain.edits {
            if let Err(why) = after.apply(&registry, edit) {
                *note = Some(why.reason);
                return;
            }
        }
        let tidied = super::tidy::tidy(&after, &registry);
        for edit in &tidied {
            let _ = after.apply(&registry, edit);
        }
        chain.edits.extend(tidied);
        chain
            .edits
            .extend(frame_round(&after, &registry, &chain.boxes));
        match self.run(document, chain.edits, tr(MAKE_LABEL), None, true) {
            Ok(()) => {
                self.canvas.selected = [chain.made].into();
                *note = Some(chain.said);
            }
            Err(why) => *note = Some(why.reason),
        }
    }
}

/// The boxes Make into material adds after an Image box, before the
/// frame round them.
struct Chain {
    /// The edits adding and wiring them.
    edits: Vec<GraphEdit>,
    /// The boxes the frame goes round: the Image box and those added.
    boxes: Vec<BoxId>,
    /// The Material from height box.
    made: BoxId,
    /// What to tell the user.
    said: String,
}

/// The chain after Image box `id` of `graph`: Even out light (its edges
/// a photo's), Make seamless, Guess height and Guess roughness into
/// Material from height, into Material out (added if the graph has none;
/// left unwired if the graph's already has a material).
fn chain(graph: &Graph, id: BoxId) -> Option<Chain> {
    let at = graph.get(id)?.at;
    let first = graph.next_id();
    let [even, seamless, height, rough, made] = [0, 1, 2, 3, 4].map(|i| BoxId(first + i));
    let place = |column: f32, down: f32| at + Vec2::new(COLUMN * column, down);
    let add = |kind: &str, at: Vec2| GraphEdit::AddBox {
        kind: kind.to_string(),
        at,
    };
    let wire = |from: BoxId, fp: &str, to: BoxId, tp: &str| GraphEdit::Connect {
        from: PlugRef::new(from, fp),
        to: PlugRef::new(to, tp),
    };
    let mut edits = vec![
        add(EVEN_LIGHT, place(1.0, 0.0)),
        add(SEAMLESS, place(2.0, 0.0)),
        add(GUESS_HEIGHT, place(3.0, 0.0)),
        add(GUESS_ROUGHNESS, place(3.0, 200.0)),
        add("material_from_height", place(4.0, 0.0)),
        GraphEdit::SetSetting {
            node: even,
            key: EDGES.into(),
            value: SettingValue::Choice(PHOTO_EDGES),
        },
        wire(id, "colour", even, "colour"),
        wire(even, "colour_out", seamless, "colour"),
        wire(seamless, "colour_out", height, "colour"),
        wire(seamless, "colour_out", rough, "colour"),
        wire(seamless, "colour_out", made, "colour"),
        wire(height, "out", made, "height"),
        wire(rough, "out", made, "roughness"),
    ];
    let mut boxes = vec![id, even, seamless, height, rough, made];
    let mut said = tr(
        "Added the boxes turning the picture into a material, in a comment frame. Each has its \
         own settings and a picture showing what it did.",
    );
    match graph.first_of("material_out") {
        None => {
            let out = BoxId(first + 5);
            edits.push(add("material_out", place(5.0, 0.0)));
            edits.push(wire(made, "material", out, "material"));
            boxes.push(out);
        }
        Some(out) if graph.wire_into(&PlugRef::new(out, "material")).is_none() => {
            edits.push(wire(made, "material", out, "material"));
        }
        Some(_) => {
            said = tr(
                "Added the boxes turning the picture into a material, in a comment frame. \
                 Material out already has a material: connect Material from height to it to \
                 use this one.",
            );
        }
    }
    Some(Chain {
        edits,
        boxes,
        made,
        said: said.to_string(),
    })
}

/// The comment frame round `boxes` of `graph`, saying what each step
/// does; none when no box is there.
fn frame_round(graph: &Graph, registry: &matgraph::Registry, boxes: &[BoxId]) -> Option<GraphEdit> {
    let mut lo = Vec2::splat(f32::INFINITY);
    let mut hi = Vec2::splat(f32::NEG_INFINITY);
    for b in boxes {
        if let Some((a, z)) = box_rect(graph, registry, *b) {
            lo = lo.min(a);
            hi = hi.max(z);
        }
    }
    if !lo.is_finite() {
        return None;
    }
    let mut frame = Frame::new(
        tr(FRAME_TITLE_TEXT),
        lo - Vec2::new(FRAME_MARGIN, FRAME_MARGIN + FRAME_TITLE),
        hi + Vec2::splat(FRAME_MARGIN),
    );
    frame.note = tr(FRAME_NOTE).to_string();
    frame.colour = FRAME_COLOUR;
    Some(GraphEdit::AddFrame(frame))
}
