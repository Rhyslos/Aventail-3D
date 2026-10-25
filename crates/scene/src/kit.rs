//! The project's kit grid ([decision 0160](../../../docs/decisions/0160-kit-helpers.md)):
//! the step a modular kit's pieces are sized to (1 m as standard), and
//! whether new shapes are made at that size, saved with the project and
//! changed with [`crate::Command::SetKitGrid`].
//!
//! It is what *Size to kit grid* rounds a shape's outside sizes to
//! ([`mesh::Primitive::fitted_to_grid`]), what *Kit-sized shapes* makes
//! new shapes at, and, when the Snap setting asks for it, the grid moves
//! snap to. The scene only keeps the settings; the interface does the
//! rest.

use words::{tr, trf};

/// A project's kit grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KitGrid {
    /// The step, in centimetres: from [`mesh::MIN_GRID_STEP`] (1 cm) to
    /// [`mesh::MAX_GRID_STEP`] (100 m).
    pub step: f32,
    /// Whether new shapes are made kit-sized: at the step's size rather
    /// than 1 m, their outside sizes whole steps (*Kit-sized shapes*).
    pub sized_shapes: bool,
}

/// The steps offered in menus, in centimetres: 1 m, 2 m and 4 m. Any
/// other step can be typed.
pub const KIT_GRID_CHOICES: [f32; 3] = [100.0, 200.0, 400.0];

impl Default for KitGrid {
    fn default() -> Self {
        Self {
            step: 100.0,
            sized_shapes: false,
        }
    }
}

impl KitGrid {
    /// Whether a project can keep it: a real step within limits.
    pub fn is_valid(&self) -> bool {
        mesh::is_grid_step(self.step)
    }

    /// The change from `before` to this, for undo steps: "Kit grid 2 m"
    /// (in metres from 1 m up, else centimetres), or "Kit-sized shapes on".
    pub fn describe(&self, before: &KitGrid) -> String {
        if self.step == before.step && self.sized_shapes != before.sized_shapes {
            return if self.sized_shapes {
                tr("Kit-sized shapes on").to_string()
            } else {
                tr("Kit-sized shapes off").to_string()
            };
        }
        if self.step >= 100.0 {
            trf!("Kit grid {} m", self.step / 100.0)
        } else {
            trf!("Kit grid {} cm", self.step)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Document};

    #[test]
    fn the_kit_grid_is_one_metre_until_changed_and_undoes() {
        let mut doc = Document::new();
        assert_eq!(doc.scene().kit_grid(), KitGrid::default());
        assert_eq!(doc.scene().kit_grid().step, 100.0);
        let two = KitGrid {
            step: 200.0,
            ..KitGrid::default()
        };
        doc.execute(Command::SetKitGrid(two)).unwrap();
        assert_eq!(doc.scene().kit_grid(), two);
        assert_eq!(doc.undo_label(), Some("Kit grid 2 m"));
        doc.undo();
        assert_eq!(doc.scene().kit_grid(), KitGrid::default());
        doc.redo();
        assert_eq!(doc.scene().kit_grid(), two);
        // The same grid again is no step.
        let steps = doc.undo_steps();
        doc.execute(Command::SetKitGrid(two)).unwrap();
        assert_eq!(doc.undo_steps(), steps);
    }

    #[test]
    fn a_kit_grid_out_of_limits_is_refused() {
        let mut doc = Document::new();
        for step in [0.0, -100.0, 0.5, f32::NAN, f32::INFINITY, 1e7] {
            assert!(
                doc.execute(Command::SetKitGrid(KitGrid {
                    step,
                    ..KitGrid::default()
                }))
                .is_err(),
                "{step}"
            );
        }
        assert_eq!(doc.scene().kit_grid(), KitGrid::default());
        assert_eq!(doc.undo_steps(), 0);
        for step in KIT_GRID_CHOICES {
            assert!(
                KitGrid {
                    step,
                    ..KitGrid::default()
                }
                .is_valid()
            );
        }
    }

    #[test]
    fn the_description_reads_in_metres_or_centimetres() {
        let standard = KitGrid::default();
        let with = |step| KitGrid { step, ..standard };
        assert_eq!(with(400.0).describe(&standard), "Kit grid 4 m");
        assert_eq!(with(250.0).describe(&standard), "Kit grid 2.5 m");
        assert_eq!(with(50.0).describe(&standard), "Kit grid 50 cm");
        let sized = KitGrid {
            sized_shapes: true,
            ..standard
        };
        assert_eq!(sized.describe(&standard), "Kit-sized shapes on");
        assert_eq!(standard.describe(&sized), "Kit-sized shapes off");
    }

    #[test]
    fn a_copy_of_the_scene_keeps_its_kit_grid() {
        let scene = crate::Scene::new().with_kit_grid(KitGrid {
            step: 400.0,
            sized_shapes: true,
        });
        assert_eq!(scene.clone().kit_grid().step, 400.0);
        assert!(scene.clone().kit_grid().sized_shapes);
        assert_ne!(scene, crate::Scene::new());
    }
}
