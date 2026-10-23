//! Booleans in the interface ([decision 0135](../../../docs/decisions/0135-live-booleans.md)):
//! which objects a Union, Subtract or Intersect would use (the first one
//! selected is kept, the others cut it), the modifiers palette's three
//! buttons, and the Attributes panel's rows for a kept object and for a
//! cutter.
//!
//! No keys or mouse actions of their own: they are started from the Mesh
//! menu, the modifiers palette or command search (the project lead's
//! choice, 2026-10-23).

use crate::commands::CommandId;
use crate::objects::DocRequest;
use crate::palette::{ButtonState, icon_button};
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui};
use scene::{Boolean, BooleanKind, BooleanRefusal, Command, Document, ObjectId};

/// The three boolean commands, in menu order.
pub(crate) const COMMANDS: [CommandId; 3] =
    [CommandId::Union, CommandId::Subtract, CommandId::Intersect];

/// The kind a boolean command makes.
pub(crate) fn kind_of(command: CommandId) -> Option<BooleanKind> {
    match command {
        CommandId::Union => Some(BooleanKind::Union),
        CommandId::Subtract => Some(BooleanKind::Subtract),
        CommandId::Intersect => Some(BooleanKind::Intersect),
        _ => None,
    }
}

/// The command that makes `kind`.
fn command_of(kind: BooleanKind) -> CommandId {
    match kind {
        BooleanKind::Union => CommandId::Union,
        BooleanKind::Subtract => CommandId::Subtract,
        BooleanKind::Intersect => CommandId::Intersect,
    }
}

/// A kind's name, as the interface shows it.
pub(crate) fn kind_name(kind: BooleanKind) -> &'static str {
    command_of(kind).name()
}

/// Why a boolean can't be set up, in the interface's words.
fn refusal_text(why: BooleanRefusal) -> &'static str {
    match why {
        BooleanRefusal::NoCutter => {
            tr("Select the shape to keep first, then Ctrl + click the shapes that cut it")
        }
        BooleanRefusal::CutsItself => tr("A shape can't cut itself"),
        BooleanRefusal::Loop => tr(
            "The shape to keep already cuts one of the others (or is shown by it): each would \
             wait for the other",
        ),
        BooleanRefusal::NotAShape => tr("A group has no shape: select shapes only"),
        BooleanRefusal::Instance => tr(
            "The first selected is an instance: select its original, or bake it into a shape \
             first",
        ),
        BooleanRefusal::TooMany => tr("A shape can have at most 64 cutters"),
    }
}

/// The kept object and its cutters for a boolean of `kind` on the
/// selection: the object selected first is kept, the others cut it, in
/// the order they were selected. Or why it can't be done.
pub(crate) fn plan(
    doc: &Document,
    kind: BooleanKind,
) -> Result<(ObjectId, Vec<ObjectId>), &'static str> {
    let order = doc.selection_order();
    let Some((&kept, cutters)) = order.split_first() else {
        return Err(tr(
            "Select the shape to keep first, then Ctrl + click the shapes that cut it",
        ));
    };
    if cutters.is_empty() {
        return Err(tr(
            "Ctrl + click one or more shapes to cut with: the shape selected first is kept",
        ));
    }
    let scene = doc.scene();
    let mut after = scene.booleans(kept).to_vec();
    match after.last_mut() {
        Some(last) if last.kind == kind => last.cutters.extend(cutters.iter().copied()),
        _ => after.push(Boolean {
            kind,
            cutters: cutters.to_vec(),
        }),
    }
    for boolean in &mut after {
        let mut seen = Vec::new();
        boolean.cutters.retain(|c| {
            let fresh = !seen.contains(c);
            seen.push(*c);
            fresh
        });
    }
    scene.check_booleans(kept, &after).map_err(refusal_text)?;
    Ok((kept, cutters.to_vec()))
}

/// The command for a boolean of `kind` on the selection.
pub(crate) fn command(doc: &Document, kind: BooleanKind) -> Result<Command, &'static str> {
    let (kept, cutters) = plan(doc, kind)?;
    Ok(Command::AddBoolean {
        kept,
        cutters,
        kind,
    })
}

/// The modifiers palette's boolean buttons; returns the kind clicked.
pub(crate) fn palette_buttons(ui: &mut Ui, doc: &Document) -> Option<BooleanKind> {
    let mut picked = None;
    for kind in BooleanKind::ALL {
        let info = command_of(kind).info();
        let (state, note) = match plan(doc, kind) {
            Ok(_) => (ButtonState::Normal, info.help),
            Err(why) => (ButtonState::Unavailable, why),
        };
        if icon_button(ui, info.icon, info.name, note, state).clicked()
            && state != ButtonState::Unavailable
        {
            picked = Some(kind);
        }
    }
    picked
}

/// The Attributes panel's Booleans section for object `id`: its booleans
/// (each kind changeable, each cutter by name with a button to take it
/// out, and a button to remove the boolean) and why the result can't be
/// worked out, if it can't; for a cutter, what it cuts.
pub(crate) fn attributes_section(
    ui: &mut Ui,
    doc: &Document,
    id: ObjectId,
    out: &mut Vec<DocRequest>,
) {
    let scene = doc.scene();
    let booleans = scene.booleans(id).to_vec();
    let cuts = scene.cut_by(id);
    if booleans.is_empty() && cuts.is_empty() {
        return;
    }
    let name = |id: ObjectId| {
        scene
            .object(id)
            .map_or_else(|| id.to_string(), |o| o.name.clone())
    };
    crate::attributes::section(ui, tr("Booleans"));
    for (index, boolean) in booleans.iter().enumerate() {
        ui.horizontal(|ui| {
            let mut kind = boolean.kind;
            egui::ComboBox::from_id_salt(("boolean_kind", id, index))
                .selected_text(kind_name(kind))
                .show_ui(ui, |ui| {
                    for choice in BooleanKind::ALL {
                        ui.selectable_value(&mut kind, choice, kind_name(choice))
                            .on_hover_text(command_of(choice).info().help);
                    }
                })
                .response
                .on_hover_text(tr("What the cutters do to this shape"))
                .widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr(KIND_LABEL))
                });
            if kind != boolean.kind {
                let mut changed = booleans.clone();
                changed[index].kind = kind;
                out.push(set(id, changed));
            }
            if ui
                .small_button(tr("Remove"))
                .on_hover_text(tr(
                    "Take this boolean away: the shape is whole again, and its cutters are \
                     ordinary shapes",
                ))
                .clicked()
            {
                let mut changed = booleans.clone();
                changed.remove(index);
                out.push(set(id, changed));
            }
        });
        for cutter in &boolean.cutters {
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                if ui
                    .link(name(*cutter))
                    .on_hover_text(tr("Select this cutter, to move or edit it"))
                    .clicked()
                {
                    out.push(DocRequest::Select(vec![*cutter]));
                }
                if ui
                    .small_button(tr("Take out"))
                    .on_hover_text(tr("Stop this shape cutting: it becomes an ordinary shape"))
                    .clicked()
                {
                    let mut changed = booleans.clone();
                    changed[index].cutters.retain(|c| c != cutter);
                    out.push(set(id, changed));
                }
            });
        }
    }
    if let Some(problem) = scene.boolean_problem(id) {
        ui.label(
            RichText::new(crate::trf!(
                "Shown uncut: {}. Booleans need closed solids.",
                problem
            ))
            .color(theme::WARNING),
        );
    } else if !booleans.is_empty() {
        ui.label(
            RichText::new(tr(
                "Move, turn, scale or edit a cutter and the cut follows. Bake into shape \
                 (Ctrl + E) makes it part of the shape for good.",
            ))
            .small()
            .color(theme::TEXT_WEAK),
        );
    }
    if !cuts.is_empty() {
        let names: Vec<String> = cuts.iter().map(|c| name(*c)).collect();
        ui.label(crate::trf!("Cuts: {}", names.join(", ")));
        ui.label(
            RichText::new(tr(
                "A cutter is drawn as an outline and left out of exports: the shape it cuts \
                 carries the result. Hidden, it still cuts.",
            ))
            .small()
            .color(theme::TEXT_WEAK),
        );
    }
}

/// The accessible name of a boolean's kind box.
pub(crate) const KIND_LABEL: &str = crate::text!("Boolean kind");

fn set(id: ObjectId, booleans: Vec<Boolean>) -> DocRequest {
    DocRequest::Run(Command::SetBooleans { id, booleans })
}
