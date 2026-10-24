//! Modifiers in the interface ([decision 0142](../../../docs/decisions/0142-modifier-stack-and-remesh.md)):
//! one command per kind of modifier, which the modifiers palette, the
//! Mesh menu's Modifiers submenu and command search offer; what a click
//! adds a modifier to (the selected shapes); and the palette's buttons,
//! grouped by family.
//!
//! The Attributes panel's Modifiers section is in
//! `attributes::modifier_rows`. No keys or mouse actions of their own.

use crate::commands::CommandId;
use crate::palette::{ButtonState, icon_button};
use crate::text::tr;
use egui::Ui;
use scene::{
    Command, Document, MAX_MODIFIERS, ModifierFamily, ModifierKind, ObjectId, ObjectKind,
    RemeshOptions,
};

/// One command per kind of modifier, in the order of
/// [`ModifierKind::standard_kinds`].
pub(crate) const COMMANDS: [CommandId; 1] = [CommandId::Remesh];

/// The modifier (with its standard settings) a command adds.
pub(crate) fn kind_of(command: CommandId) -> Option<ModifierKind> {
    match command {
        CommandId::Remesh => Some(ModifierKind::Remesh(RemeshOptions::default())),
        _ => None,
    }
}

/// The objects a modifier would be added to: the selected shapes. Or why
/// there are none.
pub(crate) fn targets(doc: &Document) -> Result<Vec<ObjectId>, &'static str> {
    let selection = doc.selection();
    if selection.is_empty() {
        return Err(tr("Select one or more shapes first"));
    }
    let scene = doc.scene();
    let mut out = Vec::new();
    for id in selection {
        let Some(object) = scene.object(id) else {
            continue;
        };
        match object.kind {
            ObjectKind::Group => continue,
            ObjectKind::Instance(_) => {
                return Err(tr(
                    "An instance shows its original's shape, modifiers and all: select the \
                     original, or bake the instance into a shape first",
                ));
            }
            _ => {}
        }
        if object.modifiers.len() >= MAX_MODIFIERS {
            return Err(tr("A shape can have at most 32 modifiers"));
        }
        out.push(id);
    }
    if out.is_empty() {
        return Err(tr("A group has no shape of its own: select shapes"));
    }
    Ok(out)
}

/// The command adding the modifier `command` stands for to the selected
/// shapes, or why it can't.
pub(crate) fn command(doc: &Document, command: CommandId) -> Result<Command, &'static str> {
    let kind = kind_of(command).ok_or(tr("That isn't a modifier"))?;
    Ok(Command::AddModifier {
        ids: targets(doc)?,
        kind,
    })
}

/// The modifiers palette's modifier buttons, a family at a time with a
/// little room between families; returns the command clicked.
pub(crate) fn palette_buttons(ui: &mut Ui, doc: &Document) -> Option<CommandId> {
    let mut picked = None;
    let ready = targets(doc);
    for family in ModifierFamily::ALL {
        let members: Vec<CommandId> = COMMANDS
            .into_iter()
            .filter(|c| kind_of(*c).is_some_and(|k| k.family() == family))
            .collect();
        if members.is_empty() {
            continue;
        }
        ui.add_space(6.0);
        for command in members {
            let info = command.info();
            let (state, note) = match ready {
                Ok(_) => (ButtonState::Normal, info.help),
                Err(why) => (ButtonState::Unavailable, why),
            };
            if icon_button(ui, info.icon, info.name, note, state).clicked()
                && state != ButtonState::Unavailable
            {
                picked = Some(command);
            }
        }
    }
    picked
}

/// The Mesh menu's Modifiers submenu's commands, family by family: each
/// family's name and its commands.
pub(crate) fn menu_groups() -> Vec<(&'static str, Vec<CommandId>)> {
    ModifierFamily::ALL
        .into_iter()
        .map(|family| {
            let members = COMMANDS
                .into_iter()
                .filter(|c| kind_of(*c).is_some_and(|k| k.family() == family))
                .collect::<Vec<_>>();
            (family.name(), members)
        })
        .filter(|(_, members)| !members.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_one_command() {
        let kinds = ModifierKind::standard_kinds();
        assert_eq!(kinds.len(), COMMANDS.len());
        for (kind, command) in kinds.iter().zip(COMMANDS) {
            assert_eq!(kind_of(command).as_ref(), Some(kind));
            assert_eq!(command.name(), kind.name());
            assert!(crate::commands::OFF_THE_RING.contains(&command));
        }
        assert_eq!(kind_of(CommandId::Move), None);
        let listed: usize = menu_groups().iter().map(|(_, m)| m.len()).sum();
        assert_eq!(listed, COMMANDS.len());
    }

    #[test]
    fn modifiers_go_on_the_selected_shapes() {
        let mut doc = Document::new();
        assert!(targets(&doc).is_err());
        doc.execute(Command::Create(ObjectKind::Group)).unwrap();
        assert!(targets(&doc).is_err(), "a group alone");
        let group = doc.selection()[0];
        doc.execute(Command::Create(ObjectKind::Primitive(
            scene::PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
        let cube = doc.selection()[0];
        doc.set_selection([group, cube]);
        assert_eq!(targets(&doc), Ok(vec![cube]), "groups are left out");
        doc.execute(Command::Duplicate {
            ids: vec![cube],
            instances: true,
        })
        .unwrap();
        assert!(targets(&doc).is_err(), "an instance");
        doc.set_selection([cube]);
        let add = command(&doc, CommandId::Remesh).unwrap();
        doc.execute(add).unwrap();
        assert_eq!(doc.scene().modifiers(cube).len(), 1);
    }
}
