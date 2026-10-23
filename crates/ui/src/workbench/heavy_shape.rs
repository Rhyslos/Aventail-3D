//! Help → Developer → Add a heavy test shape: a bumpy ball of millions
//! of polygons, for trying how the app copes with heavy shapes
//! (DESIGN.md §4.8).

use super::Workbench;
use scene::{Command, ObjectKind};

/// How many polygons Help → Add a heavy test shape makes.
pub(crate) const HEAVY_TEST_POLYGONS: usize = 3_000_000;

impl Workbench {
    /// Adds a bumpy ball of [`HEAVY_TEST_POLYGONS`] polygons, 2 m tall, as
    /// one undo step: for trying how the app copes with heavy shapes
    /// (DESIGN.md §4.8).
    pub(crate) fn add_heavy_test_shape(&mut self) {
        self.add_test_shape(HEAVY_TEST_POLYGONS);
    }

    /// Adds a bumpy ball of about `polygons` polygons, as one undo step.
    pub(crate) fn add_test_shape(&mut self, polygons: usize) {
        /// Marks this step's merge key apart from every other.
        const HEAVY_SHAPE_KEY: u64 = 0x4845_4156_0000_0000;
        self.sessions += 1;
        let key = Some(HEAVY_SHAPE_KEY | self.sessions);
        let label = "Add heavy test shape";
        let ball = mesh::samples::heavy_ball(polygons, 100.0);
        let created = self.document.execute_labelled(
            Command::Create(ObjectKind::Mesh(std::sync::Arc::new(ball))),
            label,
            key,
        );
        if created.is_ok()
            && let Some(id) = self.document.selection().first().copied()
        {
            let _ = self.document.execute_labelled(
                Command::Rename {
                    id,
                    name: "Heavy test shape".into(),
                },
                label,
                key,
            );
        }
    }
}
