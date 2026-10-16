//! The bake check ([decision 0077](../../../../docs/decisions/0077-bake-check.md)):
//! File → Check for baking… lists, in plain words, what in the shown shapes
//! would spoil a bake in Substance Painter, each with Select (to see it)
//! and, where there is one, a one-click Fix. It checks again whenever the
//! scene changes while it is open.

use super::Workbench;
use super::joining::counted;
use crate::dialog::{self, Dialog};
use crate::palette::Mode;
use crate::theme;
use egui::{RichText, Ui};
use mesh::bake::BakeReport;
use scene::{Element, ElementSelection, ObjectId, ObjectKind, Operation};

/// The bake check window's title.
pub(crate) const BAKE_TITLE: &str = "Check for baking";

/// The bake check window's state.
#[derive(Debug, Clone, Default)]
pub(crate) struct Baking {
    /// Whether the window is showing.
    pub(crate) open: bool,
    /// The problems found, and the scene revision they were found for.
    found: Option<(u64, Vec<Problem>)>,
}

/// One kind of problem on one object.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Problem {
    /// The object.
    pub(crate) id: ObjectId,
    /// Its name.
    pub(crate) name: String,
    /// What is wrong.
    pub(crate) kind: Kind,
}

/// What is wrong, with the elements concerned.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Kind {
    /// Polygons facing the wrong way round.
    Flipped(Vec<scene::FaceId>),
    /// Edges shared by three or more polygons.
    NonManifold(Vec<scene::EdgeKey>),
    /// Polygons with no area.
    Flat(Vec<scene::FaceId>),
    /// Edges with no length, in groups of points to weld.
    ShortEdges(Vec<scene::EdgeKey>, Vec<Vec<scene::VertexId>>),
    /// N-gons bent out of flat.
    Bent(Vec<scene::FaceId>),
    /// N-gons cut into sliver triangles.
    Slivers(Vec<scene::FaceId>),
    /// Points no polygon uses.
    Loose(Vec<scene::VertexId>),
    /// A `_low` object without its `_high` partner, or the other way
    /// round: the name the partner should have.
    Unpaired(String),
}

impl Kind {
    /// What is wrong, in a few words.
    pub(crate) fn text(&self) -> String {
        match self {
            Self::Flipped(f) => format!("{} facing the wrong way", counted("", f.len(), "polygon")),
            Self::NonManifold(e) => format!(
                "{} shared by three or more polygons",
                counted("", e.len(), "edge")
            ),
            Self::Flat(f) => format!("{} with no area", counted("", f.len(), "polygon")),
            Self::ShortEdges(e, _) => format!("{} with no length", counted("", e.len(), "edge")),
            Self::Bent(f) => format!(
                "{} bent out of flat",
                counted("", f.len(), "many-sided polygon")
            ),
            Self::Slivers(f) => format!(
                "{} cut into sliver triangles",
                counted("", f.len(), "many-sided polygon")
            ),
            Self::Loose(p) => format!("{} no polygon uses", counted("", p.len(), "point")),
            Self::Unpaired(partner) => format!("no partner called {partner}"),
        }
        .trim_start()
        .to_string()
    }

    /// How many elements (or 1, for a name).
    fn count(&self) -> usize {
        match self {
            Self::Flipped(f) | Self::Flat(f) | Self::Bent(f) | Self::Slivers(f) => f.len(),
            Self::NonManifold(e) | Self::ShortEdges(e, _) => e.len(),
            Self::Loose(p) => p.len(),
            Self::Unpaired(_) => 1,
        }
    }

    /// Why it spoils a bake, and what to do.
    pub(crate) fn help(&self) -> &'static str {
        match self {
            Self::Flipped(_) => {
                "The bake sees these from behind: the normal map comes out inverted or black \
                 there. Fix turns them the right way round (Mesh → Fix normals)."
            }
            Self::NonManifold(_) => {
                "Three or more polygons on one edge can't be unfolded or baked cleanly. Select \
                 them and delete or split off the extra polygons."
            }
            Self::Flat(_) => {
                "A polygon with no area has no direction, so it bakes as a black speck or a \
                 streak. Select them, then weld their points (M) or delete them."
            }
            Self::ShortEdges(..) => {
                "Points on the same spot make polygons with no area and break the shading \
                 there. Fix welds each group into one point."
            }
            Self::Bent(_) => {
                "A bent polygon with many corners shades unevenly across its triangles, and the \
                 bake shows it. Select them and cut them up with Connect (J) or Slit (X)."
            }
            Self::Slivers(_) => {
                "Very thin triangles shade badly and leave streaks in a bake. Select them and \
                 cut them up with Connect (J) so the triangles are fuller."
            }
            Self::Loose(_) => {
                "A point no polygon uses goes into the file for nothing and can confuse the \
                 baker. Fix deletes them."
            }
            Self::Unpaired(_) => {
                "Substance Painter matches a low-poly object with its high-poly one by name: \
                 crate_low with crate_high. Rename one, or make the missing partner."
            }
        }
    }

    /// The one-click fix, if there is one: the edits to record.
    fn fix(&self) -> Option<Vec<Operation>> {
        match self {
            Self::Flipped(_) => Some(vec![Operation::FixFacing]),
            Self::Loose(points) => Some(vec![Operation::DeleteElements {
                points: points.clone(),
                edges: Vec::new(),
                faces: Vec::new(),
                anchors: None,
            }]),
            Self::ShortEdges(_, groups) => Some(
                groups
                    .iter()
                    .map(|points| Operation::Weld {
                        points: points.clone(),
                        target: None,
                        anchors: None,
                    })
                    .collect(),
            ),
            _ => None,
        }
    }

    /// What Select selects: the mode, and the elements (none for a whole
    /// object).
    fn selection(&self) -> (Mode, Vec<Element>) {
        let faces = |f: &[scene::FaceId]| f.iter().map(|f| Element::Polygon(*f)).collect();
        let edges = |e: &[scene::EdgeKey]| e.iter().map(|e| Element::Edge(*e)).collect();
        match self {
            Self::Flipped(f) | Self::Flat(f) | Self::Bent(f) | Self::Slivers(f) => {
                (Mode::Polygons, faces(f))
            }
            Self::NonManifold(e) | Self::ShortEdges(e, _) => (Mode::Edges, edges(e)),
            Self::Loose(p) => (Mode::Points, p.iter().map(|v| Element::Point(*v)).collect()),
            Self::Unpaired(_) => (Mode::Model, Vec::new()),
        }
    }
}

/// The problems of one shape's `report`, for object `id` called `name`.
pub(crate) fn problems_of(id: ObjectId, name: &str, report: BakeReport) -> Vec<Problem> {
    let groups = report.short_edge_groups();
    let kinds = [
        (!report.flipped.is_empty()).then_some(Kind::Flipped(report.flipped)),
        (!report.non_manifold.is_empty()).then_some(Kind::NonManifold(report.non_manifold)),
        (!report.flat.is_empty()).then_some(Kind::Flat(report.flat)),
        (!report.short_edges.is_empty()).then_some(Kind::ShortEdges(report.short_edges, groups)),
        (!report.bent.is_empty()).then_some(Kind::Bent(report.bent)),
        (!report.slivers.is_empty()).then_some(Kind::Slivers(report.slivers)),
        (!report.loose.is_empty()).then_some(Kind::Loose(report.loose)),
    ];
    kinds
        .into_iter()
        .flatten()
        .map(|kind| Problem {
            id,
            name: name.to_string(),
            kind,
        })
        .collect()
}

/// The suffixes Substance Painter pairs objects by.
const LOW: &str = "_low";
const HIGH: &str = "_high";

/// For each of `names` ending in `_low` or `_high` (any case) whose
/// partner isn't among them, the partner's name.
pub(crate) fn unpaired(names: &[String]) -> Vec<(usize, String)> {
    let base = |name: &str, suffix: &str| {
        let lower = name.to_lowercase();
        lower
            .strip_suffix(suffix)
            .map(|_| name[..name.len() - suffix.len()].to_string())
    };
    let has = |wanted: &str| names.iter().any(|n| n.eq_ignore_ascii_case(wanted));
    names
        .iter()
        .enumerate()
        .filter_map(|(i, name)| {
            let partner = if let Some(b) = base(name, LOW) {
                format!("{b}{HIGH}")
            } else {
                format!("{}{LOW}", base(name, HIGH)?)
            };
            (!has(&partner)).then_some((i, partner))
        })
        .collect()
}

impl Workbench {
    /// File → Check for baking…
    pub(super) fn bake_entry(&mut self, ui: &mut Ui) {
        if ui
            .button("Check for baking…")
            .on_hover_text(
                "List what in the shown shapes would spoil a bake in Substance Painter, in \
                 plain words, with one-click fixes",
            )
            .clicked()
        {
            self.baking.open = true;
            ui.close();
        }
    }

    /// The problems in the shown shapes, found again when the scene has
    /// changed.
    pub(crate) fn bake_problems(&mut self) -> Vec<Problem> {
        let revision = self.document.revision();
        if let Some((at, found)) = &self.baking.found
            && *at == revision
        {
            return found.clone();
        }
        let scene = self.document.scene();
        let mut found = Vec::new();
        let mut names = Vec::new();
        let mut ids = Vec::new();
        for (id, _) in scene.walk() {
            let Some(object) = scene.object(id) else {
                continue;
            };
            if !scene.is_shown(id) || !object.kind.has_shape() {
                continue;
            }
            names.push(object.name.clone());
            ids.push(id);
            // An instance shares its original's shape: checked there.
            if matches!(object.kind, ObjectKind::Instance(_)) {
                continue;
            }
            if let Some(shape) = scene.evaluated(id) {
                found.extend(problems_of(id, &object.name, BakeReport::new(&shape.mesh)));
            }
        }
        for (i, partner) in unpaired(&names) {
            found.push(Problem {
                id: ids[i],
                name: names[i].clone(),
                kind: Kind::Unpaired(partner),
            });
        }
        self.baking.found = Some((revision, found.clone()));
        found
    }

    /// Draws the bake check window, if open.
    pub(super) fn bake_window(&mut self, ctx: &egui::Context) {
        if !self.baking.open {
            return;
        }
        let problems = self.bake_problems();
        let mut done = false;
        let mut select = None;
        let mut fix = None;
        let summary = if problems.is_empty() {
            "Nothing found that would spoil a bake.".to_string()
        } else {
            format!(
                "{} that would spoil a bake in Substance Painter.",
                counted("", problems.len(), "thing").trim_start()
            )
        };
        let outcome = Dialog::new(BAKE_TITLE, 560.0).show(ctx, |d| {
            d.header(true, |_| {});
            d.body(|ui| {
                dialog::heading(ui, &summary);
                if problems.is_empty() {
                    dialog::help(
                        ui,
                        "Checked: polygons facing the wrong way, edges shared by three \
                         polygons, polygons with no area, points on the same spot, bent or \
                         sliver many-sided polygons, loose points, and _low / _high names.",
                    );
                    return;
                }
                dialog::help(
                    ui,
                    "Point at one for why. Select shows where it is; Fix mends it as one step \
                     Ctrl + Z can take back.",
                );
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .show(ui, |ui| {
                        egui::Grid::new("bake_problems")
                            .num_columns(3)
                            .spacing([10.0, 6.0])
                            .show(ui, |ui| {
                                for (i, problem) in problems.iter().enumerate() {
                                    let what = format!("{}: {}", problem.name, problem.kind.text());
                                    ui.label(RichText::new(&what).color(theme::WARNING))
                                        .on_hover_text(problem.kind.help());
                                    let button = ui
                                        .small_button("Select")
                                        .on_hover_text("Select them, to see where they are");
                                    button.widget_info(|| {
                                        egui::WidgetInfo::labeled(
                                            egui::WidgetType::Button,
                                            true,
                                            format!("Select {what}"),
                                        )
                                    });
                                    if button.clicked() {
                                        select = Some(i);
                                    }
                                    if problem.kind.fix().is_some() {
                                        let button = ui
                                            .small_button("Fix")
                                            .on_hover_text(problem.kind.help());
                                        button.widget_info(|| {
                                            egui::WidgetInfo::labeled(
                                                egui::WidgetType::Button,
                                                true,
                                                format!("Fix {what}"),
                                            )
                                        });
                                        if button.clicked() {
                                            fix = Some(i);
                                        }
                                    } else {
                                        ui.label("");
                                    }
                                    ui.end_row();
                                }
                            });
                    });
            });
            d.footer(None, |ui| {
                if dialog::main_button(ui, "Done").clicked() {
                    done = true;
                }
            });
        });
        let (enter, escape) = dialog::enter_or_escape(ctx);
        if outcome.closed || done || enter || escape {
            self.baking.open = false;
        }
        if let Some(problem) = select.and_then(|i| problems.get(i)) {
            self.select_problem(problem);
        }
        if let Some(problem) = fix.and_then(|i| problems.get(i)) {
            self.fix_problem(problem);
        }
    }

    /// Selects what `problem` is about, in its mode.
    pub(crate) fn select_problem(&mut self, problem: &Problem) {
        let (mode, elements) = problem.kind.selection();
        self.document.set_selection([problem.id]);
        self.mode = mode;
        if !elements.is_empty() {
            self.document.select_elements(
                [(problem.id, ElementSelection::of(elements))],
                "Select from the bake check",
                None,
            );
        }
        self.note = Some(format!("{}: {}.", problem.name, problem.kind.text()));
    }

    /// Fixes `problem`, as one undo step.
    pub(crate) fn fix_problem(&mut self, problem: &Problem) {
        let Some(ops) = problem.kind.fix() else {
            return;
        };
        let label = format!("Fix {}", problem.kind.text());
        let report = |this: &Self| {
            this.document
                .scene()
                .evaluated(problem.id)
                .map(|shape| problems_of(problem.id, &problem.name, BakeReport::new(&shape.mesh)))
                .unwrap_or_default()
        };
        let before = report(self);
        let ops = ops.into_iter().map(|op| (problem.id, op)).collect();
        if self.record_each(&label, ops) {
            // A fix can make something else worse (welding can leave three
            // polygons on an edge): say so rather than hide it.
            let worse: Vec<String> = report(self)
                .into_iter()
                .filter(|after| {
                    let count = |k: &Kind| k.count();
                    before
                        .iter()
                        .find(|b| {
                            std::mem::discriminant(&b.kind) == std::mem::discriminant(&after.kind)
                        })
                        .is_none_or(|b| count(&after.kind) > count(&b.kind))
                })
                .map(|p| p.kind.text())
                .collect();
            self.note = Some(if worse.is_empty() {
                format!("{label} on {}. Ctrl + Z takes it back.", problem.name)
            } else {
                format!(
                    "{label} on {}, but now: {}. Ctrl + Z takes it back.",
                    problem.name,
                    worse.join(", ")
                )
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_and_high_objects_are_paired_by_name() {
        let names: Vec<String> = ["crate_low", "crate_high", "Barrel_LOW", "lamp_high", "box"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(
            unpaired(&names),
            vec![(2, "Barrel_high".to_string()), (3, "lamp_low".to_string())]
        );
        assert!(unpaired(&[]).is_empty());
    }

    #[test]
    fn problems_say_what_is_wrong_and_which_have_a_fix() {
        let id = ObjectId::from_raw(1);
        let report = BakeReport {
            flipped: vec![scene::FaceId::from_raw(0)],
            loose: vec![scene::VertexId::from_raw(3), scene::VertexId::from_raw(4)],
            ..BakeReport::default()
        };
        let problems = problems_of(id, "Crate", report);
        assert_eq!(problems.len(), 2);
        assert_eq!(problems[0].kind.text(), "1 polygon facing the wrong way");
        assert_eq!(problems[1].kind.text(), "2 points no polygon uses");
        assert!(problems.iter().all(|p| p.kind.fix().is_some()));
        assert!(Kind::Bent(vec![]).fix().is_none());
        assert_eq!(
            Kind::Unpaired("crate_high".into()).selection().0,
            Mode::Model
        );
    }
}
