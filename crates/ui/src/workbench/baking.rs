//! The bake check ([decision 0077](../../../../docs/decisions/0077-bake-check.md)):
//! File → Check for baking… (or UV → Check for baking…) lists, in plain
//! words, what in the shown shapes would spoil a bake in Substance Painter,
//! each with Select (to see it) and, where there is one, a one-click Fix.
//! It checks again whenever the scene changes while it is open.
//!
//! The UV part ([decision 0105](../../../../docs/decisions/0105-bake-readiness-for-uvs.md))
//! checks each shape's UVs ([`uv::check::UvReport`]) and the pieces
//! sharing each texture ([`uv::check::check_sheet`]), grouped as Pack
//! groups them.

use super::Workbench;
use super::joining::counted;
use crate::dialog::{self, Dialog};
use crate::palette::Mode;
use crate::theme;
use egui::{RichText, Ui};
use mesh::bake::BakeReport;
use scene::{Element, ElementSelection, ObjectId, ObjectKind, Operation};
use std::collections::BTreeMap;

/// The bake check window's title.
pub(crate) const BAKE_TITLE: &str = "Check for baking";

/// The bake check window's state.
#[derive(Debug, Clone, Default)]
pub(crate) struct Baking {
    /// Whether the window is showing.
    pub(crate) open: bool,
    /// The problems found, and what they were found for.
    found: Option<(Checked, Vec<Problem>)>,
}

/// What a check depends on besides the scene: the scene's revision, the
/// UV settings (padding, stacking, textures per material or object) and
/// the hard-edge angle when hard edges go by angle.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Checked {
    revision: u64,
    uv: super::unwrapping::UvSettings,
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
    /// No UVs at all.
    NoUvs,
    /// Hard edges where the UVs don't split.
    HardJoined(Vec<scene::EdgeKey>),
    /// Polygons folded over or squashed flat in the UVs.
    Folded(Vec<scene::FaceId>),
    /// Polygons of pieces outside the texture square.
    Outside(Vec<scene::FaceId>),
    /// Polygons of pieces lying over other pieces.
    Overlapping(Vec<scene::FaceId>),
    /// Polygons of pieces stacked on identical ones, and whether packing
    /// stacks identical pieces on purpose (UV → Packing).
    Stacked(Vec<scene::FaceId>, bool),
    /// Polygons of pieces closer to others than the padding (pixels).
    TooClose(Vec<scene::FaceId>, u32),
    /// Polygons of pieces whose texel density is far from the object's
    /// usual one (texture widths per unit of length).
    Uneven(Vec<scene::FaceId>, f32),
    /// Polygons on the trim sheet sharing a texture with pieces that
    /// aren't.
    TrimShared(Vec<scene::FaceId>),
}

/// A one-click fix.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Fix {
    /// Record these edits.
    Record(Vec<Operation>),
    /// Seams suggested for the whole shape, and the shape unwrapped.
    FirstUvs,
    /// These edges marked as seams, and the pieces around them unwrapped.
    Seams(Vec<scene::EdgeKey>),
    /// The pieces holding these polygons unwrapped.
    Unwrap(Vec<scene::FaceId>),
    /// The pieces on the object's textures packed.
    Pack,
    /// The pieces holding these polygons scaled to this texel density.
    Density(Vec<scene::FaceId>, f32),
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
            Self::NoUvs => "no UVs".to_string(),
            Self::HardJoined(e) => format!(
                "{} that {} UV seams",
                counted("", e.len(), "hard edge"),
                if e.len() == 1 { "isn't" } else { "aren't" }
            ),
            Self::Folded(f) => format!(
                "{} folded over or squashed flat in the UVs",
                counted("", f.len(), "polygon")
            ),
            Self::Outside(f) => format!(
                "{} outside the texture square",
                counted("", f.len(), "polygon")
            ),
            Self::Overlapping(f) => {
                format!("{} lying over other UVs", counted("", f.len(), "polygon"))
            }
            Self::Stacked(f, _) => format!(
                "{} stacked on identical pieces",
                counted("", f.len(), "polygon")
            ),
            Self::TooClose(f, padding) => format!(
                "{} closer than {padding} px to other pieces",
                counted("", f.len(), "polygon")
            ),
            Self::Uneven(f, _) => format!(
                "{} with uneven texel density",
                counted("", f.len(), "polygon")
            ),
            Self::TrimShared(f) => format!(
                "{} on the trim sheet sharing a texture with other pieces",
                counted("", f.len(), "polygon")
            ),
        }
        .trim_start()
        .to_string()
    }

    /// How many elements (or 1, for a name).
    fn count(&self) -> usize {
        match self {
            Self::Flipped(f)
            | Self::Flat(f)
            | Self::Bent(f)
            | Self::Slivers(f)
            | Self::Folded(f)
            | Self::Outside(f)
            | Self::Overlapping(f)
            | Self::Stacked(f, _)
            | Self::TooClose(f, _)
            | Self::Uneven(f, _)
            | Self::TrimShared(f) => f.len(),
            Self::NonManifold(e) | Self::ShortEdges(e, _) | Self::HardJoined(e) => e.len(),
            Self::Loose(p) => p.len(),
            Self::Unpaired(_) | Self::NoUvs => 1,
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
            Self::NoUvs => {
                "Without UVs there is nowhere on the texture for the bake to go. Fix places \
                 seams where they show least, unwraps the shape and packs it with whatever \
                 shares its texture (UV → Suggest seams, Unwrap, Pack)."
            }
            Self::HardJoined(_) => {
                "A hard edge turns the shading sharply, and a normal map can only hold that \
                 turn where the UVs are split too: otherwise the bake shows a dark or light \
                 line along it. Fix marks them as seams and unwraps the pieces around them."
            }
            Self::Folded(_) => {
                "Polygons folded over or squashed flat get no pixels of their own, so the \
                 bake smears or comes out back to front there. Fix unwraps their pieces."
            }
            Self::Outside(_) => {
                "A bake fills only the texture square: parts outside get nothing, or wrap \
                 round onto other parts. Fix packs the pieces into the square (UV → Pack)."
            }
            Self::Overlapping(_) => {
                "Pieces lying over each other share pixels, so one bakes over the other. Fix \
                 packs them apart (UV → Pack)."
            }
            Self::Stacked(_, true) => {
                "These pieces lie exactly on identical ones and share their pixels, to save \
                 room (UV → Packing → Stack identical pieces is on). That is right for \
                 identical parts, but shadow bakes (ambient occlusion) see only one of them. \
                 To bake them apart, switch that off and Pack."
            }
            Self::Stacked(_, false) => {
                "These pieces lie exactly on identical ones (two copies of a shape, say) and \
                 share their pixels, so one bakes over the other. Fix packs them apart (UV → \
                 Pack). To share pixels on purpose, switch on UV → Packing → Stack identical \
                 pieces."
            }
            Self::TooClose(..) => {
                "Pieces need a gap of empty pixels: the bake spreads each piece's colours \
                 into it so seams don't show, and a game shows smaller copies of the texture \
                 at a distance (mipmaps), where close pieces blur into each other. Fix packs \
                 them with the gap set in UV → Packing."
            }
            Self::TrimShared(_) => {
                "These polygons lie on the trim sheet, but share a material (and so a texture) \
                 with pieces that don't: the trim texture would land on those too. Select \
                 them and give them a material of their own (Materials, in the Attributes \
                 panel), so the trim sheet is a texture set of its own."
            }
            Self::Uneven(..) => {
                "These pieces get much more or much less texture than the rest of the \
                 object, so they look sharper or blurrier. Fix scales them to the object's \
                 usual texel density (UV → Match texel density does this for a selection)."
            }
        }
    }

    /// The one-click fix, if there is one.
    pub(crate) fn fix(&self) -> Option<Fix> {
        match self {
            Self::Flipped(_) => Some(Fix::Record(vec![Operation::FixFacing])),
            Self::Loose(points) => Some(Fix::Record(vec![Operation::DeleteElements {
                points: points.clone(),
                edges: Vec::new(),
                faces: Vec::new(),
                anchors: None,
            }])),
            Self::ShortEdges(_, groups) => Some(Fix::Record(
                groups
                    .iter()
                    .map(|points| Operation::Weld {
                        points: points.clone(),
                        target: None,
                        anchors: None,
                    })
                    .collect(),
            )),
            Self::NoUvs => Some(Fix::FirstUvs),
            Self::HardJoined(edges) => Some(Fix::Seams(edges.clone())),
            Self::Folded(faces) => Some(Fix::Unwrap(faces.clone())),
            Self::Outside(_) | Self::Overlapping(_) | Self::TooClose(..) => Some(Fix::Pack),
            Self::Stacked(_, false) => Some(Fix::Pack),
            Self::Uneven(faces, density) => Some(Fix::Density(faces.clone(), *density)),
            _ => None,
        }
    }

    /// What Select selects: the mode, and the elements (none for a whole
    /// object).
    fn selection(&self) -> (Mode, Vec<Element>) {
        let faces = |f: &[scene::FaceId]| f.iter().map(|f| Element::Polygon(*f)).collect();
        let edges = |e: &[scene::EdgeKey]| e.iter().map(|e| Element::Edge(*e)).collect();
        match self {
            Self::Flipped(f)
            | Self::Flat(f)
            | Self::Bent(f)
            | Self::Slivers(f)
            | Self::Folded(f)
            | Self::Outside(f)
            | Self::Overlapping(f)
            | Self::Stacked(f, _)
            | Self::TooClose(f, _)
            | Self::Uneven(f, _)
            | Self::TrimShared(f) => (Mode::Polygons, faces(f)),
            Self::NonManifold(e) | Self::ShortEdges(e, _) | Self::HardJoined(e) => {
                (Mode::Edges, edges(e))
            }
            Self::Loose(p) => (Mode::Points, p.iter().map(|v| Element::Point(*v)).collect()),
            Self::Unpaired(_) | Self::NoUvs => (Mode::Model, Vec::new()),
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

/// The UV problems of one shape's `report` (`None`: it has no UVs), for
/// object `id` called `name`.
pub(crate) fn uv_problems_of(
    id: ObjectId,
    name: &str,
    report: Option<uv::check::UvReport>,
) -> Vec<Problem> {
    let kinds = match report {
        None => vec![Some(Kind::NoUvs)],
        Some(r) => vec![
            (!r.hard_joined.is_empty()).then_some(Kind::HardJoined(r.hard_joined)),
            (!r.folded.is_empty()).then_some(Kind::Folded(r.folded)),
            (!r.outside.is_empty()).then_some(Kind::Outside(r.outside)),
            match (r.uneven.is_empty(), r.usual_density) {
                (false, Some(d)) => Some(Kind::Uneven(r.uneven, d)),
                _ => None,
            },
        ],
    };
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

/// The trim sheet's rows, as (bottom, top).
pub(crate) fn trim_rows(scene: &scene::Scene) -> Vec<(f32, f32)> {
    scene
        .trim()
        .rows
        .iter()
        .map(|r| (r.bottom, r.top))
        .collect()
}

/// Whether `piece` lies on one of `rows`.
pub(crate) fn on_trim(mesh: &scene::Mesh, piece: &[scene::FaceId], rows: &[(f32, f32)]) -> bool {
    rows.iter()
        .any(|(bottom, top)| uv::trim::on_row(mesh, piece, *bottom, *top))
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
    /// UV → Check for baking… (its one home since decision 0126; it was in
    /// File too).
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
        let checked = Checked {
            revision: self.document.revision(),
            uv: self.uv_settings,
        };
        if let Some((at, found)) = &self.baking.found
            && *at == checked
        {
            return found.clone();
        }
        let scene = self.document.scene();
        let mut found = Vec::new();
        let mut names = Vec::new();
        let mut ids = Vec::new();
        let mut shapes = Vec::new();
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
                let hard = self.shown_hard_edges(id, &shape.mesh);
                let report = uv::check::UvReport::new(&shape.mesh, &hard, &trim_rows(scene));
                found.extend(uv_problems_of(id, &object.name, report));
                shapes.push((id, object.name.clone(), shape.mesh));
            }
        }
        found.extend(self.sheet_problems(&shapes));
        for (i, partner) in unpaired(&names) {
            found.push(Problem {
                id: ids[i],
                name: names[i].clone(),
                kind: Kind::Unpaired(partner),
            });
        }
        self.baking.found = Some((checked, found.clone()));
        found
    }

    /// The problems among the pieces sharing each texture (grouped as
    /// Pack groups them) of `shapes`.
    fn sheet_problems(
        &self,
        shapes: &[(ObjectId, String, std::sync::Arc<scene::Mesh>)],
    ) -> Vec<Problem> {
        let texture = self.document.scene().texture();
        let padding = self.uv_settings.padding.min(super::unwrapping::MAX_PADDING);
        // Each texture's pieces: which shape, and its polygons.
        let mut sheets: BTreeMap<
            super::unwrapping::TextureGroup,
            Vec<(usize, Vec<scene::FaceId>)>,
        > = BTreeMap::new();
        let rows = trim_rows(self.document.scene());
        // Pieces on the trim sheet repeat and overlap on purpose: kept
        // apart, per texture.
        let mut trims: BTreeMap<super::unwrapping::TextureGroup, Vec<(usize, Vec<scene::FaceId>)>> =
            BTreeMap::new();
        for (k, (id, _, mesh)) in shapes.iter().enumerate() {
            for island in mesh::uvs::islands(mesh) {
                let group = self.texture_group(*id, mesh, &island);
                let into = if on_trim(mesh, &island, &rows) {
                    &mut trims
                } else {
                    &mut sheets
                };
                into.entry(group).or_default().push((k, island));
            }
        }
        // Trim pieces on a texture other pieces use.
        let mut shared: BTreeMap<usize, Vec<scene::FaceId>> = BTreeMap::new();
        for (group, members) in &trims {
            if sheets.contains_key(group) {
                for (k, faces) in members {
                    shared.entry(*k).or_default().extend(faces.iter().copied());
                }
            }
        }
        // For each shape, the polygons with each problem.
        let mut over: BTreeMap<usize, Vec<scene::FaceId>> = BTreeMap::new();
        let mut stacked: BTreeMap<usize, Vec<scene::FaceId>> = BTreeMap::new();
        let mut close: BTreeMap<usize, Vec<scene::FaceId>> = BTreeMap::new();
        for members in sheets.values() {
            let pieces: Vec<uv::check::SheetPiece> = members
                .iter()
                .map(|(k, faces)| uv::check::SheetPiece {
                    mesh: &shapes[*k].2,
                    faces: faces.clone(),
                })
                .collect();
            let report = uv::check::check_sheet(&pieces, texture.size, padding);
            for (set, into) in [
                (&report.overlapping, &mut over),
                (&report.stacked, &mut stacked),
                (&report.too_close, &mut close),
            ] {
                for i in set {
                    let (k, faces) = &members[*i];
                    into.entry(*k).or_default().extend(faces.iter().copied());
                }
            }
        }
        let mut out = Vec::new();
        for (k, (id, name, _)) in shapes.iter().enumerate() {
            let mut add = |kind: Kind| {
                out.push(Problem {
                    id: *id,
                    name: name.clone(),
                    kind,
                });
            };
            if let Some(mut f) = over.remove(&k) {
                f.sort();
                add(Kind::Overlapping(f));
            }
            if let Some(mut f) = stacked.remove(&k) {
                f.sort();
                add(Kind::Stacked(f, self.uv_settings.stack));
            }
            if let Some(mut f) = close.remove(&k) {
                f.sort();
                add(Kind::TooClose(f, padding));
            }
            if let Some(mut f) = shared.remove(&k) {
                f.sort();
                add(Kind::TrimShared(f));
            }
        }
        out
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
                         sliver many-sided polygons, loose points, _low / _high names; and \
                         the UVs: hard edges that aren't seams, folded, overlapping or \
                         stacked pieces, pieces outside the square or closer than the \
                         padding, and uneven texel density.",
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
        let Some(fix) = problem.kind.fix() else {
            return;
        };
        let label = format!("Fix {}", problem.kind.text());
        let mine = |this: &mut Self| -> Vec<Problem> {
            this.bake_problems()
                .into_iter()
                .filter(|p| p.id == problem.id)
                .collect()
        };
        let before = mine(self);
        let key = self.new_step_key();
        // Every shape sharing a texture with this one.
        let sharing = |this: &Self| -> Vec<(ObjectId, std::sync::Arc<scene::Mesh>)> {
            let ids = if this.uv_settings.per_object {
                vec![problem.id]
            } else {
                this.bake_shapes()
            };
            ids.into_iter()
                .filter_map(|id| Some((id, this.document.scene().evaluated(id)?.mesh)))
                .collect()
        };
        let done = match fix {
            Fix::Pack => {
                let shapes = sharing(self);
                self.pack_shapes(shapes, true, Some(key))
            }
            fix => {
                let Some(mesh) = self.document.scene().evaluated(problem.id).map(|e| e.mesh) else {
                    return;
                };
                let first_uvs = fix == Fix::FirstUvs;
                let ops = self.fix_ops(problem.id, &mesh, fix);
                let ops = ops.into_iter().map(|op| (problem.id, op)).collect();
                let done = self.record_each_in(&label, ops, key);
                // New UVs start with every piece in the middle of the
                // square: packed with what shares the texture, in the same
                // step.
                if done && first_uvs {
                    let shapes = sharing(self);
                    self.pack_shapes(shapes, true, Some(key));
                }
                done
            }
        };
        if done {
            // A fix can make something else worse (welding can leave three
            // polygons on an edge, scaling pieces can make them overlap):
            // say so rather than hide it.
            let worse: Vec<String> = mine(self)
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

    /// The edits a fix makes on `mesh`.
    fn fix_ops(&self, id: scene::ObjectId, mesh: &scene::Mesh, fix: Fix) -> Vec<Operation> {
        use super::unwrapping::UvTool;
        let all: Vec<scene::FaceId> = mesh.faces().map(|(f, _)| f).collect();
        match fix {
            Fix::Record(ops) => ops,
            Fix::FirstUvs => {
                let hard = self.shown_hard_edges(id, mesh);
                let seams = uv::seams::suggest_with(mesh, &hard, self.suggest_options());
                let mut ops = Vec::new();
                if !seams.is_empty() {
                    ops.push(super::seams::seams_op(mesh, seams, true));
                }
                ops.push(self.uv_tool_op(UvTool::Unwrap, mesh, all));
                ops
            }
            Fix::Seams(edges) => {
                let topology = mesh.topology();
                let around: std::collections::BTreeSet<scene::FaceId> = edges
                    .iter()
                    .flat_map(|e| topology.edge_faces(*e).collect::<Vec<_>>())
                    .collect();
                vec![
                    super::seams::seams_op(mesh, edges, true),
                    self.uv_tool_op(UvTool::Unwrap, mesh, around.into_iter().collect()),
                ]
            }
            Fix::Unwrap(faces) => vec![self.uv_tool_op(UvTool::Unwrap, mesh, faces)],
            Fix::Density(faces, density) => vec![Operation::SetTexelDensity {
                anchors: super::unwrapping::anchors(mesh, &faces),
                faces,
                density,
            }],
            Fix::Pack => Vec::new(),
        }
    }

    /// The shown objects the bake check looks at.
    fn bake_shapes(&self) -> Vec<ObjectId> {
        let scene = self.document.scene();
        scene
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| {
                scene.is_shown(*id)
                    && scene.object(*id).is_some_and(|o| {
                        matches!(o.kind, ObjectKind::Primitive(_) | ObjectKind::Mesh(_))
                    })
            })
            .collect()
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
        assert_eq!(Kind::Stacked(vec![], true).fix(), None);
        assert_eq!(Kind::Stacked(vec![], false).fix(), Some(Fix::Pack));
        assert_eq!(Kind::Overlapping(vec![]).fix(), Some(Fix::Pack));
        assert_eq!(Kind::NoUvs.selection().0, Mode::Model);
        assert_eq!(uv_problems_of(id, "Crate", None)[0].kind.text(), "no UVs");
        assert!(Kind::Bent(vec![]).fix().is_none());
        assert_eq!(
            Kind::Unpaired("crate_high".into()).selection().0,
            Mode::Model
        );
    }
}
