//! The workbench's part in the modular kit helpers ([decision 0160](../../../../docs/decisions/0160-kit-helpers.md)):
//! the Create menu's Modular kit list, new shapes made kit-sized, and
//! Size to kit grid. The pivot presets are with the other ready-made
//! pivot places, in `pivots`.

use super::Workbench;
use crate::kit::KitChoice;
use crate::objects::DocRequest;
use crate::text::tr;
use egui::Ui;
use scene::{Command, ObjectId, ObjectKind, Primitive, PrimitiveKind};

/// The size the standard shapes are made at, in centimetres: Kit-sized
/// shapes grows or shrinks them from this to the kit grid's step.
const STANDARD_SIZE: f32 = 100.0;

impl Workbench {
    /// A new primitive of `kind`: its standard settings, or, with
    /// Kit-sized shapes on, made at the kit grid's size with its outside
    /// sizes whole steps of it.
    pub(super) fn new_primitive(&self, kind: PrimitiveKind) -> DocRequest {
        let grid = self.document.scene().kit_grid();
        let primitive = if grid.sized_shapes && grid.is_valid() {
            kind.standard()
                .scaled(grid.step / STANDARD_SIZE)
                .fitted_to_grid(grid.step)
        } else {
            kind.standard()
        };
        DocRequest::Run(Command::Create(ObjectKind::Primitive(primitive)))
    }

    /// The selected primitives, with their settings fitted to the kit
    /// grid, where that changes them.
    fn kit_fitted(&self) -> Vec<(ObjectId, Primitive)> {
        let scene = self.document.scene();
        let step = scene.kit_grid().step;
        self.document
            .selection()
            .into_iter()
            .filter_map(|id| match &scene.object(id)?.kind {
                ObjectKind::Primitive(p) => {
                    let fitted = p.fitted_to_grid(step);
                    (fitted != p.clamped()).then_some((id, fitted))
                }
                _ => None,
            })
            .collect()
    }

    /// Whether Size to kit grid can be used now, or why not.
    pub(super) fn kit_sizing_ready(&self) -> Result<(), String> {
        let scene = self.document.scene();
        let selection = self.document.selection();
        if selection.is_empty() {
            return Err(tr("Select one or more shapes first").into());
        }
        let any_shape = selection.iter().any(|id| {
            scene
                .object(*id)
                .is_some_and(|o| matches!(o.kind, ObjectKind::Primitive(_)))
        });
        if !any_shape {
            return Err(tr(
                "Only shapes with settings (a cube, a cylinder…) can be sized: baked meshes \
                 and groups have no sizes to round",
            )
            .into());
        }
        if self.kit_fitted().is_empty() {
            return Err(tr("The selected shapes already fit the kit grid").into());
        }
        Ok(())
    }

    /// Size to kit grid: the selected shapes' outside sizes rounded to
    /// whole steps of the kit grid, as one undo step.
    pub(super) fn size_to_kit_grid(&mut self) {
        if let Err(why) = self.kit_sizing_ready() {
            self.note = Some(crate::trf!("Size to kit grid: {}.", why));
            return;
        }
        let changes = self.kit_fitted();
        let key = self.new_step_key();
        let label = tr("Size to kit grid");
        // A lattice cage nobody has changed, round a shape being sized, is
        // fitted round it again afterwards (a moved or shared cage stays
        // as it is): otherwise the shape would grow out of its cage.
        let scene = self.document.scene();
        let cages: Vec<(scene::ObjectId, scene::ObjectId)> = changes
            .iter()
            .flat_map(|(id, _)| {
                scene
                    .modifiers(*id)
                    .iter()
                    .filter_map(move |m| match &m.kind {
                        scene::ModifierKind::Lattice(o) if scene.cage_fits(o.cage, *id) => {
                            Some((o.cage, *id))
                        }
                        _ => None,
                    })
            })
            .collect();
        // One step: if one shape is refused, none is sized.
        let commands = changes
            .iter()
            .map(|(id, primitive)| Command::SetPrimitive {
                ids: vec![*id],
                primitive: *primitive,
            })
            .chain(cages.iter().map(|(cage, shape)| Command::FitCage {
                cage: *cage,
                shape: *shape,
            }))
            .collect::<Vec<_>>();
        if let Err(why) = self.document.execute_all_labelled(commands, label, key) {
            self.note = Some(crate::trf!("Couldn't size to the kit grid: {}.", why));
            return;
        }
        let grid = crate::units::Units::from(self.document.scene().units())
            .format(self.document.scene().kit_grid().step);
        let scene = self.document.scene();
        self.note = Some(match changes.as_slice() {
            [(id, _)] => crate::trf!(
                "{} sized to the {} kit grid. Ctrl + Z takes it back.",
                scene.object(*id).map_or("", |o| o.name.as_str()),
                grid
            ),
            many => crate::trf!(
                "{} shapes sized to the {} kit grid. Ctrl + Z takes it back.",
                many.len(),
                grid
            ),
        });
    }

    /// The Create menu's Modular kit list.
    pub(super) fn kit_menu(&mut self, ui: &mut Ui, requests: &mut Vec<DocRequest>) {
        let grid = self.document.scene().kit_grid();
        let units = self.units();
        let ready = self.kit_sizing_ready();
        let mut snap = self.viewport.options().snap.kit_grid;
        let chosen = crate::kit::kit_entries(ui, grid, units, &mut snap, ready);
        self.viewport.options_mut().snap.kit_grid = snap;
        match chosen {
            Some(KitChoice::Grid { grid, began }) => {
                if began {
                    self.kit_key = self.new_step_key();
                }
                requests.push(DocRequest::RunMerging(
                    Command::SetKitGrid(grid),
                    self.kit_key,
                ));
            }
            Some(KitChoice::SizeNow) => self.size_to_kit_grid(),
            None => {}
        }
    }
}
