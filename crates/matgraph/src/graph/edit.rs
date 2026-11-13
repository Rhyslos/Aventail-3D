//! Every change to a graph, checked before it is made.

use super::{BoxId, Frame, FrameId, Graph, GraphBox, MAX_BOXES, MAX_FRAMES, MAX_NAME, MAX_PLACE};
use super::{PlugRef, Wire};
use crate::kind::{Fit, fit};
use crate::refusal::{Fix, Refusal};
use crate::registry::Registry;
use crate::settings::SettingValue;
use glam::Vec2;
use words::{tr, trf};

/// One change to a graph. The scene records each as an undo step
/// (`scene::Command::EditGraph`).
#[derive(Debug, Clone, PartialEq)]
pub enum GraphEdit {
    /// Adds a box of kind `kind` (an id in the registry) at `at`, with
    /// its standard settings. It gets the id [`Graph::next_id`].
    AddBox {
        /// The kind's id.
        kind: String,
        /// Where it sits.
        at: Vec2,
    },
    /// Removes boxes and every wire into or out of them.
    RemoveBoxes(Vec<BoxId>),
    /// Removes a box but keeps the wire through it: what fed its first
    /// input feeds what its outputs fed, where the kinds fit.
    DissolveBox(BoxId),
    /// Moves boxes by `by`.
    MoveBoxes {
        /// The boxes.
        ids: Vec<BoxId>,
        /// How far.
        by: Vec2,
    },
    /// Changes one of a box's settings.
    SetSetting {
        /// The box.
        node: BoxId,
        /// The setting's key.
        key: String,
        /// Its new value.
        value: SettingValue,
    },
    /// Wires output `from` into input `to`, replacing any wire there.
    Connect {
        /// The output.
        from: PlugRef,
        /// The input.
        to: PlugRef,
    },
    /// Takes the wire out of input `to`.
    Disconnect(PlugRef),
    /// Adds a comment frame. It gets the id [`Graph::next_id`].
    AddFrame(Frame),
    /// Changes a comment frame (title, colour, rectangle, note).
    ChangeFrame {
        /// The frame.
        id: FrameId,
        /// What it becomes.
        frame: Frame,
    },
    /// Removes a comment frame (the boxes inside stay).
    RemoveFrame(FrameId),
    /// Shows or hides one of a box's settings in the painter (milestone
    /// 5): shown, every layer using the graph can set it its own way.
    ShowInPainter {
        /// The box.
        node: BoxId,
        /// The setting's key.
        key: String,
        /// Shown or not.
        shown: bool,
    },
    /// Replaces what is drawn on a Shape box's canvas (milestone 11):
    /// every change on the shape canvas is one of these, so undo works.
    SetDrawing {
        /// The box.
        node: BoxId,
        /// What it becomes.
        drawing: std::sync::Arc<shapes::Drawing>,
    },
    /// Marks one of a box's settings "change in the game", or unmarks it
    /// (milestone 15): kept for the engine translators, baked until they
    /// come. Only numbers, lengths, whole numbers, colours and switches
    /// can be marked ([`super::changes_in_game`]).
    ChangeInGame {
        /// The box.
        node: BoxId,
        /// The setting's key.
        key: String,
        /// Marked or not.
        on: bool,
    },
    /// Sets the moment of the graph's animation that exports and the
    /// painter bake, in seconds (milestone 15): from 0 to
    /// [`super::MAX_MOMENT`].
    SetMoment(f32),
    /// Changes the graph's seed.
    SetSeed(u32),
    /// Renames the graph.
    Rename(String),
}

impl GraphEdit {
    /// The undo step's wording ("Connect boxes").
    pub fn describe(&self) -> String {
        match self {
            Self::AddBox { .. } => tr("Add a box").to_string(),
            Self::RemoveBoxes(ids) if ids.len() == 1 => tr("Remove a box").to_string(),
            Self::RemoveBoxes(ids) => trf!("Remove {} boxes", ids.len()),
            Self::DissolveBox(_) => tr("Remove a box, keeping its wire").to_string(),
            Self::MoveBoxes { .. } => tr("Move boxes").to_string(),
            Self::SetSetting { .. } => tr("Change a box's setting").to_string(),
            Self::Connect { .. } => tr("Connect boxes").to_string(),
            Self::Disconnect(_) => tr("Disconnect boxes").to_string(),
            Self::AddFrame(_) => tr("Add a comment frame").to_string(),
            Self::ChangeFrame { .. } => tr("Change a comment frame").to_string(),
            Self::RemoveFrame(_) => tr("Remove a comment frame").to_string(),
            Self::ShowInPainter { shown: true, .. } => {
                tr("Show a setting in the painter").to_string()
            }
            Self::ShowInPainter { shown: false, .. } => {
                tr("Hide a setting from the painter").to_string()
            }
            Self::SetDrawing { .. } => tr("Change a drawn shape").to_string(),
            Self::ChangeInGame { on: true, .. } => {
                tr("Mark a setting to change in the game").to_string()
            }
            Self::ChangeInGame { on: false, .. } => {
                tr("Bake a setting rather than change it in the game").to_string()
            }
            Self::SetMoment(_) => tr("Choose the moment to bake").to_string(),
            Self::SetSeed(_) => tr("Change the graph's seed").to_string(),
            Self::Rename(_) => tr("Rename the graph").to_string(),
        }
    }
}

impl Graph {
    /// Makes `edit`, checked against `registry`, or says why it can't be
    /// made (the graph is then unchanged).
    pub fn apply(&mut self, registry: &Registry, edit: &GraphEdit) -> Result<(), Refusal> {
        match edit {
            GraphEdit::AddBox { kind, at } => self.add_box(registry, kind, *at),
            GraphEdit::RemoveBoxes(ids) => {
                for id in ids {
                    self.missing(*id)?;
                }
                self.boxes.retain(|id, _| !ids.contains(id));
                self.wires
                    .retain(|w| !ids.contains(&w.from.node) && !ids.contains(&w.to.node));
                Ok(())
            }
            GraphEdit::DissolveBox(id) => self.dissolve(registry, *id),
            GraphEdit::MoveBoxes { ids, by } => {
                for id in ids {
                    let at = self.missing(*id)?.at + *by;
                    if !at.is_finite() || at.abs().max_element() > MAX_PLACE {
                        return Err(Refusal::new(tr(
                            "That's too far from the rest of the graph.",
                        )));
                    }
                }
                for id in ids {
                    if let Some(b) = self.boxes.get_mut(id) {
                        b.at += *by;
                    }
                }
                Ok(())
            }
            GraphEdit::SetSetting { node, key, value } => {
                let info = self.info_of(registry, *node)?;
                let spec = info.setting(key).ok_or_else(|| {
                    Refusal::new(trf!("{} has no setting called {}.", info.name.show(), key))
                })?;
                let value = spec.check(*value)?;
                if let Some(b) = self.boxes.get_mut(node) {
                    b.settings.set(key.clone(), value);
                }
                Ok(())
            }
            GraphEdit::Connect { from, to } => {
                self.check_connection(registry, from, to)?;
                self.wires.retain(|w| w.to != *to);
                self.wires.push(Wire {
                    from: from.clone(),
                    to: to.clone(),
                });
                self.wires.sort();
                Ok(())
            }
            GraphEdit::Disconnect(to) => {
                if self.wire_into(to).is_none() {
                    return Err(Refusal::new(tr("Nothing is plugged in there.")));
                }
                self.wires.retain(|w| w.to != *to);
                Ok(())
            }
            GraphEdit::AddFrame(frame) => {
                if self.frames.len() >= MAX_FRAMES {
                    return Err(Refusal::new(trf!(
                        "A graph can hold {} comment frames; remove one first.",
                        MAX_FRAMES
                    )));
                }
                frame_ok(frame)?;
                let id = FrameId(self.take_id());
                self.frames.insert(id, frame.clone());
                Ok(())
            }
            GraphEdit::ChangeFrame { id, frame } => {
                frame_ok(frame)?;
                let slot = self
                    .frames
                    .get_mut(id)
                    .ok_or_else(|| Refusal::new(tr("That comment frame isn't there any more.")))?;
                *slot = frame.clone();
                Ok(())
            }
            GraphEdit::RemoveFrame(id) => self
                .frames
                .remove(id)
                .map(|_| ())
                .ok_or_else(|| Refusal::new(tr("That comment frame isn't there any more."))),
            GraphEdit::ShowInPainter { node, key, shown } => {
                let info = self.info_of(registry, *node)?;
                if info.setting(key).is_none() {
                    return Err(Refusal::new(trf!(
                        "{} has no setting called {}.",
                        info.name.show(),
                        key
                    )));
                }
                if let Some(b) = self.boxes.get_mut(node) {
                    if *shown {
                        b.shown.insert(key.clone());
                    } else {
                        b.shown.remove(key);
                    }
                }
                Ok(())
            }
            GraphEdit::SetDrawing { node, drawing } => {
                let info = self.info_of(registry, *node)?;
                let drawn = registry.get(&info.id).is_some_and(|k| k.takes_drawing());
                if !drawn {
                    return Err(Refusal::new(trf!(
                        "{} isn't drawn on the shape canvas; a Shape box is.",
                        info.name.show()
                    )));
                }
                // Checked again as a file would be (it was made through
                // checked changes, but may come from anywhere).
                shapes::Drawing::from_parts(drawing.to_parts())
                    .map_err(|why| Refusal::new(tr(why.describe())))?;
                if let Some(b) = self.boxes.get_mut(node) {
                    b.drawing = Some(drawing.clone());
                }
                Ok(())
            }
            GraphEdit::ChangeInGame { node, key, on } => {
                let info = self.info_of(registry, *node)?;
                let spec = info.setting(key).ok_or_else(|| {
                    Refusal::new(trf!("{} has no setting called {}.", info.name.show(), key))
                })?;
                if *on && !super::changes_in_game(spec) {
                    return Err(Refusal::new(trf!(
                        "{}'s {} can't change in the game: it picks how the box works, so it is \
                         always baked. Numbers, colours and switches can be marked.",
                        info.name.show(),
                        spec.name.show()
                    )));
                }
                if let Some(b) = self.boxes.get_mut(node) {
                    if *on {
                        b.in_game.insert(key.clone());
                    } else {
                        b.in_game.remove(key);
                    }
                }
                Ok(())
            }
            GraphEdit::SetMoment(seconds) => {
                if !seconds.is_finite() || !(0.0..=super::MAX_MOMENT).contains(seconds) {
                    return Err(Refusal::new(trf!(
                        "The moment to bake must be from 0 to {} seconds.",
                        super::MAX_MOMENT
                    )));
                }
                self.moment = *seconds;
                Ok(())
            }
            GraphEdit::SetSeed(seed) => {
                self.seed = *seed;
                Ok(())
            }
            GraphEdit::Rename(name) => {
                if name.trim().is_empty() || name.len() > MAX_NAME {
                    return Err(Refusal::new(trf!(
                        "A graph needs a name of up to {} letters.",
                        MAX_NAME
                    )));
                }
                self.name.clone_from(name);
                Ok(())
            }
        }
    }

    /// The next id, counted on.
    fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Box `id`, or a refusal saying it is gone.
    fn missing(&self, id: BoxId) -> Result<&GraphBox, Refusal> {
        self.get(id)
            .ok_or_else(|| Refusal::new(tr("That box isn't in the graph any more.")))
    }

    fn add_box(&mut self, registry: &Registry, kind: &str, at: Vec2) -> Result<(), Refusal> {
        let info = registry
            .get(kind)
            .map(|t| t.info())
            .ok_or_else(|| Refusal::new(trf!("There's no box called {}.", kind)))?;
        if self.boxes.len() >= MAX_BOXES {
            return Err(Refusal::new(trf!(
                "A graph can hold {} boxes; make some into one of your own box, or remove some.",
                MAX_BOXES
            )));
        }
        if !at.is_finite() || at.abs().max_element() > MAX_PLACE {
            return Err(Refusal::new(tr(
                "That's too far from the rest of the graph.",
            )));
        }
        if info.one_per_graph
            && let Some(there) = self.first_of(kind)
        {
            return Err(Refusal::new(trf!(
                "A graph makes one material, so it has one {} box; it already has it.",
                info.name.show()
            ))
            .with_fixes([Fix::LookAt(there)]));
        }
        let id = BoxId(self.take_id());
        self.boxes.insert(id, GraphBox::new(kind, at));
        Ok(())
    }

    /// [`GraphEdit::DissolveBox`].
    fn dissolve(&mut self, registry: &Registry, id: BoxId) -> Result<(), Refusal> {
        self.missing(id)?;
        let info = self.info_of(registry, id).ok();
        // What fed its first input, and that input's kind.
        let through = info.and_then(|info| {
            let first = info.inputs.first()?;
            let wire = self.wire_into(&PlugRef::new(id, first.key.to_string()))?;
            let kind = self
                .get(wire.from.node)
                .and_then(|b| registry.get(&b.kind))
                .and_then(|t| t.info().output(&wire.from.plug))?
                .kind;
            Some((wire.from.clone(), kind))
        });
        let fed: Vec<PlugRef> = self
            .wires
            .iter()
            .filter(|w| w.from.node == id)
            .map(|w| w.to.clone())
            .collect();
        self.boxes.remove(&id);
        self.wires.retain(|w| w.from.node != id && w.to.node != id);
        if let Some((source, kind)) = through {
            for to in fed {
                let takes = self
                    .get(to.node)
                    .and_then(|b| registry.get(&b.kind))
                    .and_then(|t| t.info().input(&to.plug))
                    .map(|p| p.kind);
                if takes.is_some_and(|k| matches!(fit(kind, k), Ok(Fit::Fits | Fit::Converts(_)))) {
                    self.wires.push(Wire {
                        from: source.clone(),
                        to,
                    });
                }
            }
            self.wires.sort();
        }
        Ok(())
    }
}

/// Why a frame can't be kept, as a refusal.
fn frame_ok(frame: &Frame) -> Result<(), Refusal> {
    match frame.problem() {
        Some(why) => Err(Refusal::new(trf!(
            "That comment frame can't be kept: {}.",
            tr(why)
        ))),
        None => Ok(()),
    }
}
