//! The Mesh section: what the object's final mesh is made of and whether
//! anything is wrong with it, checked in the background for heavy meshes.

use super::{LABEL_WIDTH, label, section};
use crate::theme;
use egui::{RichText, Ui, vec2};
use mesh::MeshReport;
use scene::{Mesh, ObjectId};
use std::sync::Arc;

/// Below this many polygons the mesh check is quick enough to run at
/// once; above, it runs in the background so the window never stalls
/// ([decision 0035](../../../docs/decisions/0035-heavy-meshes.md)).
const CHECK_IN_BACKGROUND_FROM: usize = 50_000;

/// Checks objects' meshes, keeping the last report.
#[derive(Default)]
pub(super) struct MeshChecker {
    /// The last report, for which object and which mesh.
    shown: Option<(ObjectId, Arc<Mesh>, MeshReport)>,
    /// A check running in the background.
    running: Option<(ObjectId, Arc<Mesh>, std::thread::JoinHandle<MeshReport>)>,
}

impl Clone for MeshChecker {
    /// A copy starts afresh: reports are worked out again when needed.
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl std::fmt::Debug for MeshChecker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshChecker")
            .field("running", &self.running.is_some())
            .finish()
    }
}

impl MeshChecker {
    /// The report on object `id`'s `mesh`, and whether it is up to date.
    /// For a heavy mesh the check runs in the background, and until it
    /// finishes the object's previous report (if any) is given, out of
    /// date; `ctx` is asked to draw again when it is done.
    pub(super) fn report(
        &mut self,
        id: ObjectId,
        mesh: &Arc<Mesh>,
        ctx: &egui::Context,
    ) -> (Option<MeshReport>, bool) {
        if let Some((shown_id, shown, report)) = &self.shown
            && *shown_id == id
            && Arc::ptr_eq(shown, mesh)
        {
            return (Some(*report), true);
        }
        if self
            .running
            .as_ref()
            .is_some_and(|(.., handle)| handle.is_finished())
            && let Some((done_id, done, handle)) = self.running.take()
            && let Ok(report) = handle.join()
        {
            self.shown = Some((done_id, Arc::clone(&done), report));
            if done_id == id && Arc::ptr_eq(&done, mesh) {
                return (Some(report), true);
            }
        }
        if mesh.face_count() < CHECK_IN_BACKGROUND_FROM {
            let report = MeshReport::new(mesh);
            self.shown = Some((id, Arc::clone(mesh), report));
            return (Some(report), true);
        }
        if self.running.is_none() {
            let (work, ctx) = (Arc::clone(mesh), ctx.clone());
            let handle = std::thread::spawn(move || {
                let report = MeshReport::new(&work);
                ctx.request_repaint();
                report
            });
            self.running = Some((id, Arc::clone(mesh), handle));
        }
        let earlier = self
            .shown
            .as_ref()
            .filter(|(shown_id, ..)| *shown_id == id)
            .map(|(.., report)| *report);
        (earlier, false)
    }
}

/// The Mesh section, for every shape: what its final mesh is made of
/// (counted from the shape with its edits) and whether anything is wrong
/// with it. A plain mesh also says what it is for.
pub(super) fn mesh_section(
    ui: &mut Ui,
    report: Option<&MeshReport>,
    current: bool,
    plain_mesh: bool,
) {
    section(ui, "Mesh");
    if !current {
        ui.label(RichText::new("Checking the mesh…").color(theme::TEXT_WEAK))
            .on_hover_text(
                "A heavy shape is checked in the background, so you can keep working. The \
                 numbers below are from before the last change until it finishes.",
            );
    }
    let Some(report) = report else {
        return;
    };
    egui::Grid::new("attributes_mesh")
        .num_columns(2)
        .min_col_width(LABEL_WIDTH)
        .spacing(vec2(12.0, 6.0))
        .show(ui, |ui| {
            let mut rows = vec![
                ("Points", report.points, "The corners the mesh is made of"),
                ("Edges", report.edges, "The lines between the points"),
                (
                    "Polygons",
                    report.faces,
                    "The faces, with any number of corners",
                ),
                (
                    "Triangles",
                    report.triangles,
                    "The triangles a game engine cuts the faces into: the usual measure of \
                     how heavy a model is",
                ),
            ];
            if report.pieces > 1 {
                rows.push(("Pieces", report.pieces, "Separate parts that don't touch"));
            }
            for (name, count, help) in rows {
                label(ui, name, help);
                ui.label(count.to_string()).widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Label,
                        true,
                        format!("{name}: {count}"),
                    )
                });
                ui.end_row();
            }
            label(
                ui,
                "Surface",
                "Closed: no open borders, like a solid. Open: some edges have a face on \
                 one side only, as on a plane or an open pipe",
            );
            let surface = if report.is_closed() {
                "Closed".to_string()
            } else {
                format!("Open ({})", count(report.border_edges, "border edge"))
            };
            ui.label(surface);
            ui.end_row();
        });
    let problems = [
        (
            report.non_manifold_edges,
            "edge shared by three or more faces",
            "edges shared by three or more faces",
            "No real solid has these, and most tools can't work on them. They usually come \
             from faces stacked on each other.",
        ),
        (
            report.flipped_edges,
            "edge between faces facing opposite ways",
            "edges between faces facing opposite ways",
            "One face on each of these edges points inwards (a flipped normal); it will \
             shade and bake wrongly. Mesh → Fix normals turns them the right way.",
        ),
        (
            report.zero_length_edges,
            "edge with no length",
            "edges with no length",
            "Both ends sit on the same spot. Usually left over from moving points together.",
        ),
        (
            report.flat_faces,
            "face with no area",
            "faces with no area",
            "Faces squashed flat to a line or a point: invisible, but they still count and \
             can break tools.",
        ),
        (
            report.loose_points,
            "loose point",
            "loose points",
            "Points that no face uses.",
        ),
    ];
    let mut any = false;
    for (n, one, many, help) in problems {
        if n > 0 {
            any = true;
            let text = if n == 1 {
                format!("1 {one}")
            } else {
                format!("{n} {many}")
            };
            ui.label(RichText::new(text).color(theme::WARNING))
                .on_hover_text(help);
        }
    }
    if !any {
        ui.label(RichText::new("No problems found").color(theme::TEXT_WEAK))
            .on_hover_text(
                "Checked for edges shared by three or more faces, faces facing the wrong \
                 way, edges with no length, faces with no area and loose points",
            );
    }
    if plain_mesh {
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "A plain mesh: it has no settings, so its shape changes only by editing \
                 its points, edges and polygons (press 1, 2 or 3).",
            )
            .small()
            .color(theme::TEXT_WEAK),
        );
    }
}

/// "1 border edge", "4 border edges".
pub(super) fn count(n: usize, thing: &str) -> String {
    if n == 1 {
        format!("1 {thing}")
    } else {
        format!("{n} {thing}s")
    }
}

#[cfg(test)]
mod checker_tests {
    use super::*;

    #[test]
    fn heavy_meshes_are_checked_in_the_background() {
        let ctx = egui::Context::default();
        let mut checker = MeshChecker::default();
        let id = ObjectId::from_raw(1);
        let light = Arc::new(mesh::PrimitiveKind::Cube.standard().mesh());
        let (report, current) = checker.report(id, &light, &ctx);
        assert!(current && report.unwrap().faces == 6, "light: at once");
        let heavy = Arc::new(mesh::samples::heavy_ball(60_000, 100.0));
        let (report, current) = checker.report(id, &heavy, &ctx);
        assert!(!current, "heavy: in the background");
        assert_eq!(
            report.unwrap().faces,
            6,
            "the object's last report meanwhile"
        );
        // Another object has no report to show meanwhile.
        let (other, _) = checker.report(ObjectId::from_raw(2), &heavy, &ctx);
        assert!(other.is_none());
        let start = std::time::Instant::now();
        loop {
            let (report, current) = checker.report(id, &heavy, &ctx);
            if current {
                assert_eq!(report.unwrap().faces, heavy.face_count());
                break;
            }
            assert!(start.elapsed().as_secs() < 60, "the check never finished");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
