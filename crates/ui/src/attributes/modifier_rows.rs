//! The Attributes panel's part in modifiers
//! ([decision 0142](../../../../docs/decisions/0142-modifier-stack-and-remesh.md),
//! [decision 0168](../../../../docs/decisions/0168-modifiers-in-the-lists.md)):
//! with an object selected, a short Modifiers section naming its modifiers,
//! each a link; with a modifier clicked (there or in the Objects or Edit
//! layers panel), only that modifier: its on/off switch, its settings,
//! whether the viewport shows it, and why it can't do its work, if it
//! can't. Every change is a command; one drag of a value is one undo step.
//! Ordering, copying and removing are in the lists (`crate::modifier_list`).

use super::curve_rows::Rows;
use super::{AttributesPanel, LABEL_WIDTH, label, section, stepper};
use crate::objects::DocRequest;
use crate::text::tr;
use crate::theme;
use crate::units::Units;
use egui::{RichText, Ui, vec2};
use scene::{Command, Document, Modifier, ModifierKind, ObjectId, RemeshOptions, SubdivideOptions};

/// The accessible name of a modifier's on/off switch.
pub(crate) const ON_LABEL: &str = crate::text!("Modifier on");
/// The accessible name of the button going back to the object's settings
/// starts with this (then the object's name).
pub(crate) const BACK_PREFIX: &str = crate::text!("Back to ");

impl AttributesPanel {
    /// The Modifiers section for object `id`, if it has any: their names,
    /// first to last, each a link showing its settings.
    pub(super) fn modifiers_section(&mut self, ui: &mut Ui, doc: &Document, id: ObjectId) {
        let scene = doc.scene();
        let modifiers = scene.modifiers(id);
        if modifiers.is_empty() {
            return;
        }
        section(ui, tr("Modifiers"));
        let problems = scene.modifier_problems(id);
        ui.horizontal_wrapped(|ui| {
            for (index, modifier) in modifiers.iter().enumerate() {
                let mut text = RichText::new(modifier.kind.name());
                if !modifier.on {
                    text = text.color(theme::TEXT_WEAK).strikethrough();
                } else if problems
                    .iter()
                    .any(|(i, why)| *i == index && !scene::has_nothing_to_do(why))
                {
                    text = text.color(theme::WARNING);
                }
                let link = ui
                    .link(text)
                    .on_hover_text(tr("Show this modifier's settings"));
                if link.clicked() {
                    self.picked_modifier = Some((id, modifier.id));
                }
            }
        });
        ui.label(
            RichText::new(tr(
                "Worked out first to last, after the edits, booleans and mirror. Click one, here \
                 or under the object in the Objects panel, to see its settings. Bake into shape \
                 (Ctrl + E) makes those that are on part of the shape for good.",
            ))
            .small()
            .color(theme::TEXT_WEAK),
        );
    }

    /// The panel with a modifier picked: only that modifier (decision
    /// 0168), with a way back to its object's settings.
    pub(super) fn picked_modifier_view(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        id: ObjectId,
        index: usize,
        units: Units,
        out: &mut Vec<DocRequest>,
    ) {
        let scene = doc.scene();
        let (Some(object), Some(modifier)) = (
            scene.object(id).cloned(),
            scene.modifiers(id).get(index).cloned(),
        ) else {
            return;
        };
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let back = ui
                        .small_button(crate::trf!("← {}", object.name))
                        .on_hover_text(tr("Back to the object's own settings"));
                    back.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            true,
                            format!("{}{}", tr(BACK_PREFIX), object.name),
                        )
                    });
                    if back.clicked() {
                        self.picked_modifier = None;
                    }
                });
                ui.add_space(4.0);
                ui.push_id(("modifier", id, modifier.id), |ui| {
                    self.modifier_header(ui, id, index, &modifier, out);
                    ui.add_space(4.0);
                    self.modifier_rows(ui, doc, (id, index), &modifier, units, out);
                    if let Some((_, why)) = scene
                        .modifier_problems(id)
                        .iter()
                        .find(|(i, _)| *i == index)
                    {
                        // Nothing to do (no points close enough to weld) is
                        // a calm grey note; a real problem is in the
                        // warning colour.
                        let (text, color) = if scene::has_nothing_to_do(why) {
                            (crate::trf!("Nothing to do: {}.", why), theme::TEXT_WEAK)
                        } else {
                            (crate::trf!("Not worked out: {}.", why), theme::WARNING)
                        };
                        ui.add(egui::Label::new(RichText::new(text).color(color)).wrap());
                    }
                });
                ui.add_space(6.0);
                ui.label(
                    RichText::new(tr(
                        "Drag it in the Objects panel to change the order; right-click it \
                         there to duplicate it, bake it into the shape or remove \
                         it. Delete removes it.",
                    ))
                    .small()
                    .color(theme::TEXT_WEAK),
                );
            });
    }

    /// A modifier's first line: its switch and name.
    fn modifier_header(
        &mut self,
        ui: &mut Ui,
        id: ObjectId,
        index: usize,
        modifier: &Modifier,
        out: &mut Vec<DocRequest>,
    ) {
        let mut on = modifier.on;
        let switch = ui
            .checkbox(
                &mut on,
                RichText::new(modifier.kind.name()).strong().size(15.0),
            )
            .on_hover_text(tr(
                "Switch this modifier off to see the shape without it; it stays, with its \
                 settings",
            ));
        switch.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, on, tr(ON_LABEL))
        });
        if switch.changed() {
            out.push(change(
                id,
                index,
                Modifier {
                    on,
                    ..modifier.clone()
                },
            ));
        }
    }

    /// A modifier's settings, and whether the viewport shows it.
    fn modifier_rows(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        (id, index): (ObjectId, usize),
        modifier: &Modifier,
        units: Units,
        out: &mut Vec<DocRequest>,
    ) {
        let mut edited = modifier.clone();
        let mut rows = Rows::default();
        egui::Grid::new(("attributes_modifier", id, index))
            .num_columns(2)
            .min_col_width(LABEL_WIDTH)
            .spacing(vec2(12.0, 6.0))
            .show(ui, |ui| {
                if let ModifierKind::Lattice(options) = &modifier.kind {
                    cage_row(ui, doc, options.cage, out);
                }
                match &modifier.kind {
                    ModifierKind::Scatter(settings) => {
                        super::scatter_rows::shapes_row(ui, doc, &settings.shapes, out);
                    }
                    ModifierKind::BendAlongCurve(settings) => {
                        super::wrap_rows::source_row(ui, doc, settings.curve, true, out);
                    }
                    ModifierKind::Shrinkwrap(settings) => {
                        super::wrap_rows::source_row(ui, doc, settings.target, false, out);
                    }
                    _ => {}
                }
                // The polygons selected on this shape, in the Polygons
                // mode (for a scatter's Use selected), and its points, in
                // the Points mode (for Shrinkwrap's and Displace's).
                let chosen = Chosen {
                    polygons: self.polygons_selected.map(|_| {
                        doc.elements(id)
                            .map_or_else(Vec::new, |e| e.polygons().collect())
                    }),
                    points: (self.curve.points_mode && doc.selection() == [id]).then(|| {
                        doc.elements(id)
                            .map_or_else(Vec::new, |e| e.points().collect())
                    }),
                    centres: match &modifier.kind {
                        ModifierKind::Array(a)
                            if a.options.layout == scene::ArrayLayout::Radial =>
                        {
                            ring_centres(doc, id)
                        }
                        _ => Vec::new(),
                    },
                };
                kind_rows(ui, &mut edited.kind, units, &mut rows, &chosen);
                label(
                    ui,
                    tr("Show in viewport"),
                    tr(
                        "Off, the viewport skips this modifier (handy while it is slow); exports \
                         and Bake into shape still use it",
                    ),
                );
                rows.toggle(ui, &mut edited.in_viewport, tr("Show in viewport"));
                ui.end_row();
                if matches!(modifier.kind, ModifierKind::Decimate(_))
                    && let Some(shown) = doc.scene().shown_shape_if_ready(id)
                {
                    super::decimate_rows::shown_triangles_row(
                        ui,
                        mesh::decimate::triangle_count(&shown),
                    );
                }
            });
        if matches!(modifier.kind, ModifierKind::WeightedNormals(_)) && modifier.on {
            super::weighted_rows::dropped_note(ui, doc.scene().modifiers(id), index);
        }
        if rows.began {
            self.gesture += 1;
        }
        if rows.changed && edited != *modifier {
            out.push(DocRequest::RunMerging(
                Command::ChangeModifier {
                    id,
                    index,
                    modifier: edited,
                },
                self.gesture,
            ));
        }
    }
}

/// The elements selected on the shape, for the modifiers that work on
/// chosen ones (`None` outside their mode).
struct Chosen {
    /// In the Polygons mode.
    polygons: Option<Vec<scene::FaceId>>,
    /// In the Points mode.
    points: Option<Vec<scene::VertexId>>,
    /// The objects an array's ring could turn round (only listed for an
    /// array's ring).
    centres: Vec<(ObjectId, String)>,
}

/// The rows for a kind's own settings as a level of detail has them
/// (decision 0165): nothing chosen on the shape.
pub(super) fn level_kind_rows(ui: &mut Ui, kind: &mut ModifierKind, units: Units, rows: &mut Rows) {
    let chosen = Chosen {
        polygons: None,
        points: None,
        centres: Vec::new(),
    };
    kind_rows(ui, kind, units, rows, &chosen);
}

/// The rows for a kind's own settings.
fn kind_rows(ui: &mut Ui, kind: &mut ModifierKind, units: Units, rows: &mut Rows, chosen: &Chosen) {
    match kind {
        ModifierKind::Remesh(options) => remesh_rows(ui, options, units, rows),
        ModifierKind::Subdivide(options) => subdivide_rows(ui, options, rows),
        // The cage's own settings are on the cage (`cage_row` names it).
        ModifierKind::Lattice(_) => {}
        ModifierKind::Decimate(options) => super::decimate_rows::decimate_rows(ui, options, rows),
        ModifierKind::Scatter(settings) => {
            let polygons = chosen.polygons.as_deref();
            super::scatter_rows::scatter_rows(ui, settings, units, rows, polygons);
        }
        ModifierKind::BendAlongCurve(settings) => {
            super::wrap_rows::bend_rows(ui, settings, units, rows);
        }
        ModifierKind::Shrinkwrap(settings) => {
            let points = chosen.points.as_deref();
            super::wrap_rows::shrinkwrap_rows(ui, settings, units, rows, points);
        }
        ModifierKind::WeightedNormals(options) => {
            super::weighted_rows::weighted_rows(ui, options, rows);
        }
        ModifierKind::LiveBevel(options) => {
            super::live_bevel_rows::live_bevel_rows(ui, options, units, rows);
        }
        ModifierKind::Quadify(options) => super::quadify_rows::quadify_rows(ui, options, rows),
        ModifierKind::WeldByDistance(options) => super::weld_rows::weld_rows(ui, options, rows),
        ModifierKind::Array(settings) => {
            super::array_rows::array_rows(ui, settings, units, rows, &chosen.centres);
        }
        ModifierKind::Shell(options) => super::shell_rows::shell_rows(ui, options, units, rows),
        ModifierKind::Struts(options) => super::struts_rows::struts_rows(ui, options, units, rows),
        ModifierKind::Displace(settings) => {
            let points = chosen.points.as_deref();
            super::displace_rows::displace_rows(ui, settings, units, rows, points);
        }
        ModifierKind::BendTwistTaper(options) => {
            super::axis_deform_rows::axis_deform_rows(ui, options, units, rows);
        }
        ModifierKind::Triangulate(options) => {
            super::triangulate_rows::triangulate_rows(ui, options, rows);
        }
    }
}

/// The objects `id`'s array could turn its ring round, by name, in the
/// Objects panel's order: any but itself and those made from it (which
/// the scene refuses as a loop).
fn ring_centres(doc: &Document, id: ObjectId) -> Vec<(ObjectId, String)> {
    let scene = doc.scene();
    scene
        .walk()
        .into_iter()
        .filter(|(other, _)| *other != id && !scene.shape_uses(*other, id))
        .filter_map(|(other, _)| Some((other, scene.object(other)?.name.clone())))
        .collect()
}

/// The Lattice modifier's cage (decision 0144): its name, and a button
/// selecting it, where its divisions and points are changed.
fn cage_row(ui: &mut Ui, doc: &Document, cage: ObjectId, out: &mut Vec<DocRequest>) {
    label(
        ui,
        tr("Cage"),
        tr(
            "The lattice cage bending this shape. Select it to change its divisions, and press 1 \
             to move its points.",
        ),
    );
    ui.horizontal(|ui| {
        let name = doc
            .scene()
            .object(cage)
            .map_or_else(|| tr("(none)").to_string(), |o| o.name.clone());
        ui.label(name);
        if ui
            .small_button(tr("Select cage"))
            .on_hover_text(tr("Select the cage, to change it or move its points"))
            .clicked()
        {
            out.push(DocRequest::Select(vec![cage]));
        }
    });
    ui.end_row();
}

/// Subdivide's settings: Levels, Levels for export, Hard edges stay sharp
/// (decision 0143).
fn subdivide_rows(ui: &mut Ui, options: &mut SubdivideOptions, rows: &mut Rows) {
    label(
        ui,
        tr("Levels"),
        tr(
            "How many times the viewport splits and smooths the shape: each level has four \
             times the polygons. 0 shows the shape as it is.",
        ),
    );
    rows.take(stepper::count(
        ui,
        &mut options.levels,
        (0, SubdivideOptions::MAX_LEVELS),
        tr("Levels"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Levels for export"),
        tr(
            "How many times exports and Bake into shape split and smooth it: often one more \
             than the viewport, for a smoother result where it counts",
        ),
    );
    rows.take(stepper::count(
        ui,
        &mut options.export_levels,
        (0, SubdivideOptions::MAX_LEVELS),
        tr("Levels for export"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Hard edges stay sharp"),
        tr(
            "Keep the edges marked hard sharp, as if fully creased. Only on objects shaded by \
             their marked edges: smoothed by angle, every sharp corner looks hard, so nothing \
             would round.",
        ),
    );
    rows.toggle(
        ui,
        &mut options.hard_edges_sharp,
        tr("Hard edges stay sharp"),
    );
    ui.end_row();
}

/// Remesh's settings: Voxel size and Smooth.
fn remesh_rows(ui: &mut Ui, options: &mut RemeshOptions, units: Units, rows: &mut Rows) {
    label(
        ui,
        tr("Voxel size"),
        tr(
            "How far apart the grid the shape is rebuilt on is: about the size of the new \
             polygons. Smaller keeps more detail and makes more polygons.",
        ),
    );
    rows.take(stepper::length(
        ui,
        &mut options.voxel_size,
        (RemeshOptions::MIN_VOXEL, RemeshOptions::MAX_VOXEL),
        units,
        tr("Voxel size"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Smooth"),
        tr("Relax passes evening out the new surface without shrinking it: 0 for none"),
    );
    rows.take(stepper::count(
        ui,
        &mut options.smooth,
        (0, RemeshOptions::MAX_SMOOTH),
        tr("Smooth"),
    ));
    ui.end_row();
}

fn change(id: ObjectId, index: usize, modifier: Modifier) -> DocRequest {
    DocRequest::Run(Command::ChangeModifier {
        id,
        index,
        modifier,
    })
}
