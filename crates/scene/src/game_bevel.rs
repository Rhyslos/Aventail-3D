//! The game bevel ([decision 0156](../../../docs/decisions/0156-game-bevel.md)):
//! one command that readies a hard-surface shape for the mid-poly way of
//! working, where a model's edges are bevelled and shaded well instead of
//! baked onto a normal map.
//!
//! For each object, as one undo step, it:
//!
//! 1. shades the object by its marked edges, if it was smoothed by angle,
//!    keeping its look as marks first (as marking an edge by hand does,
//!    decision 0126). Smoothed by angle, a 2-row bevel of a right-angled
//!    edge has a 45° step in its middle, sharper than 30°, so its strip
//!    would shade with a crease;
//! 2. gives it a Live bevel (decision 0149) with *Soft bevels* on: edges
//!    by angle 30°, 2 cm, 2 rows. A Live bevel it already has is kept,
//!    with Soft bevels switched on (and switched on, if none is on);
//! 3. gives it Weighted normals (decision 0148) last in its list: one it
//!    already has moves to the end and is switched on.
//!
//! It marks no seams: the bevel already moves a seam to its strip's far
//! border (decision 0104), and a seam doesn't change UVs, only where they
//! may part. Everything it does is ordinary modifiers, settings and
//! recorded edits, shown and changed in the Attributes panel afterwards.

use crate::{Command, Document, ModifierKind, ObjectId, ObjectKind, Operation, SceneError};
use mesh::live_bevel::LiveBevelOptions;
use mesh::weighted_normals::WeightedNormalsOptions;

/// The Live bevel the game bevel adds: the standard one (edges by angle
/// 30°, 2 cm, 2 rows, round, Stop at neighbours) with *Soft bevels* on.
pub fn game_bevel_options() -> LiveBevelOptions {
    LiveBevelOptions {
        soften: true,
        ..LiveBevelOptions::default()
    }
}

/// What a game bevel did, for the status bar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GameBevelDone {
    /// The objects given it.
    pub objects: usize,
    /// How many already had a Live bevel, kept with Soft bevels on.
    pub kept_bevels: usize,
    /// How many were smoothed by angle and now shade by their marked
    /// edges, their look kept as marks.
    pub switched: usize,
}

impl Document {
    /// Gives each of `ids` the game bevel (see the module's notes), as one
    /// undo step labelled `label`, made with the history key `key` (a key
    /// no other step uses). Objects without a shape of their own (groups,
    /// lattice cages) are skipped. Refused, changing nothing, if any
    /// object can't take it (an instance, too many modifiers) or none has
    /// a shape.
    pub fn game_bevel(
        &mut self,
        ids: &[ObjectId],
        label: &str,
        key: u64,
    ) -> Result<GameBevelDone, SceneError> {
        let scene = self.scene();
        let mut targets = Vec::new();
        for id in ids {
            let object = scene.object(*id).ok_or(SceneError::UnknownObject(*id))?;
            match object.kind {
                ObjectKind::Group | ObjectKind::Lattice(_) => {}
                ObjectKind::Instance(_) => return Err(SceneError::Instance),
                _ if !targets.contains(id) => targets.push(*id),
                _ => {}
            }
        }
        if targets.is_empty() {
            return Err(SceneError::NoShape);
        }
        let mut done = GameBevelDone {
            objects: targets.len(),
            ..GameBevelDone::default()
        };
        for id in targets {
            if let Err(why) = self.game_bevel_one(id, label, key, &mut done) {
                // Take back what was done for the objects before.
                self.abandon(key);
                return Err(why);
            }
        }
        Ok(done)
    }

    /// The game bevel's three steps for one object.
    fn game_bevel_one(
        &mut self,
        id: ObjectId,
        label: &str,
        key: u64,
        done: &mut GameBevelDone,
    ) -> Result<(), SceneError> {
        let run =
            |doc: &mut Self, command: Command| doc.execute_labelled(command, label, Some(key));
        let object = self
            .scene()
            .object(id)
            .ok_or(SceneError::UnknownObject(id))?;
        let is_curve = matches!(object.kind, ObjectKind::Curve(_));

        // 1. Shade by marked edges, keeping the look as marks. A curve's
        // shape can't take recorded edits: it keeps the marks it is made
        // with.
        if let mesh::Hardness::ByAngle(angle) = object.hardness {
            if !is_curve {
                run(
                    self,
                    Command::Record {
                        id,
                        operation: Operation::HardEdgesByAngle { angle },
                    },
                )?;
            }
            run(
                self,
                Command::SetHardness {
                    ids: vec![id],
                    hardness: mesh::Hardness::Marked,
                },
            )?;
            done.switched += 1;
        }

        // 2. A Live bevel with Soft bevels on.
        let bevels: Vec<usize> = self
            .scene()
            .modifiers(id)
            .iter()
            .enumerate()
            .filter(|(_, m)| matches!(m.kind, ModifierKind::LiveBevel(_)))
            .map(|(i, _)| i)
            .collect();
        if bevels.is_empty() {
            run(
                self,
                Command::AddModifier {
                    ids: vec![id],
                    kind: ModifierKind::LiveBevel(game_bevel_options()),
                },
            )?;
        } else {
            done.kept_bevels += 1;
            let none_on = bevels.iter().all(|i| !self.scene().modifiers(id)[*i].on);
            for (n, index) in bevels.into_iter().enumerate() {
                let mut modifier = self.scene().modifiers(id)[index].clone();
                if let ModifierKind::LiveBevel(options) = &mut modifier.kind {
                    options.soften = true;
                }
                if none_on && n == 0 {
                    modifier.on = true;
                }
                run(
                    self,
                    Command::ChangeModifier {
                        id,
                        index,
                        modifier,
                    },
                )?;
            }
        }

        // 3. Weighted normals, last and on.
        let list = self.scene().modifiers(id);
        let last = list.len().saturating_sub(1);
        match list
            .iter()
            .rposition(|m| matches!(m.kind, ModifierKind::WeightedNormals(_)))
        {
            None => run(
                self,
                Command::AddModifier {
                    ids: vec![id],
                    kind: ModifierKind::WeightedNormals(WeightedNormalsOptions::default()),
                },
            )?,
            Some(at) => {
                if at != last {
                    run(
                        self,
                        Command::MoveModifier {
                            id,
                            from: at,
                            to: last,
                        },
                    )?;
                }
                let mut modifier = self.scene().modifiers(id)[last].clone();
                if !modifier.on {
                    modifier.on = true;
                    run(
                        self,
                        Command::ChangeModifier {
                            id,
                            index: last,
                            modifier,
                        },
                    )?;
                }
            }
        }
        // A Weighted normals just added goes at the end unless a kind
        // asks to go before it; make sure it is last.
        let list = self.scene().modifiers(id);
        let at = list
            .iter()
            .rposition(|m| matches!(m.kind, ModifierKind::WeightedNormals(_)));
        if let Some(at) = at
            && at + 1 != list.len()
        {
            let to = list.len() - 1;
            run(self, Command::MoveModifier { id, from: at, to })?;
        }
        Ok(())
    }
}
