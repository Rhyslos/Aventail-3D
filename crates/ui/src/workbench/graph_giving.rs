//! Graph materials on shapes and as texture sets (Phase 12, milestone 6,
//! [decision 0262](../../../../docs/decisions/0262-baked-export.md)).
//!
//! - **Giving a graph to a shape** without painting: its material's
//!   texture set becomes one graph fill layer (`material::giving`), which
//!   `paint_graphs` makes on the model at the material's real-world size
//!   and makes again whenever the graph or the shape changes. One undo
//!   step; saved with the project as any texture set; exported by the
//!   texture export with every engine preset.
//! - **Export textures…** in the Material mode: a folder is asked for
//!   (`WorkbenchAction::ChooseGraphTexturesFolder`), then the graph's
//!   material is worked out at the texture budget's size and written as a
//!   seamless texture set on another thread (`material::export_set`).
//! - **The Export window's check**: graph materials not made yet, ones
//!   that can't be, and what each graph bakes (animated boxes at their
//!   current setting).

use super::{Workbench, WorkbenchAction};
use crate::material::giving::{Given, given_graph, with_graph_given};
use crate::paint_layers::own_texture;
use crate::text::tr;
use matgraph::{GraphId, Registry};
use scene::{Command, ObjectId};
use std::path::PathBuf;

impl Workbench {
    /// Each frame after the panels: the Material mode's and Attributes'
    /// asks to give a graph or export its textures, and a finished
    /// texture set export.
    pub(super) fn carry_out_graph_asks(&mut self, actions: &mut Vec<WorkbenchAction>) {
        if std::mem::take(&mut self.material.give_asked)
            && let Some(id) = self.material.graph
        {
            self.give_graph_to_selected(id);
        }
        if std::mem::take(&mut self.material.export_asked) {
            actions.push(WorkbenchAction::ChooseGraphTexturesFolder);
        }
        if let Some((owner, material, id)) = self.attributes.give_graph.take() {
            match id {
                Some(id) => {
                    self.give_graph(&[(owner, material)], id);
                }
                None => self.take_graph_back(&material),
            }
        }
        if let Some(said) = self.material.export_set.collect() {
            self.note = Some(said);
        }
    }

    /// Gives graph `id`'s material to the first material of each selected
    /// shape (an instance shows its original's, so it is left out).
    pub(crate) fn give_graph_to_selected(&mut self, id: GraphId) {
        let scene = self.document.scene();
        let targets: Vec<(ObjectId, String)> = self
            .document
            .selection()
            .into_iter()
            .filter(|o| {
                scene
                    .object(*o)
                    .is_some_and(|object| own_texture::paints(&object.kind))
            })
            .filter_map(|o| Some((o, scene.material_list(o).first()?.name.clone())))
            .collect();
        if targets.is_empty() {
            self.note = Some(
                tr("Select a shape first (in Objects, or in the viewport) to give it the material.")
                    .to_string(),
            );
            return;
        }
        self.give_graph(&targets, id);
    }

    /// Gives graph `id`'s material to each of `targets` (a shape and its
    /// material), as one undo step: a material made by a graph changes
    /// graph, a bare one becomes made by it, a painted one gets it under
    /// its paint. The viewport then shows painted textures, and the
    /// status bar says what happened. Returns how each went.
    pub(crate) fn give_graph(&mut self, targets: &[(ObjectId, String)], id: GraphId) -> Vec<Given> {
        let Some(graph) = self.document.scene().graphs().get(id).cloned() else {
            self.note = Some(tr("That graph isn't in the project any more.").to_string());
            return Vec::new();
        };
        let key = self.new_step_key();
        let mut done: Vec<(String, Given)> = Vec::new();
        let mut owned_note = None;
        for (owner, material) in targets {
            // A shape sharing the standard material with another gets its
            // own first (decision 0232), in the same undo step.
            let material = own_texture::own_if_shared(
                &mut self.document,
                *owner,
                material,
                key,
                &mut owned_note,
            );
            if done.iter().any(|(m, _)| *m == material) {
                continue;
            }
            let scene = self.document.scene();
            let size = scene.texture().size;
            let Some((set, given)) = with_graph_given(
                scene.paint().set(&material).map(|s| &**s),
                id,
                graph.name(),
                size,
            ) else {
                self.note = Some(crate::trf!(
                    "Couldn't give {} to {}: its textures have as many layers as they can \
                     hold. Merge or delete some in the Paint mode first.",
                    graph.name(),
                    material
                ));
                continue;
            };
            let label = crate::trf!("Give {} to {}", graph.name(), material);
            let command = Command::SetTextureSet {
                material: material.clone(),
                set: Some(set),
            };
            match self.document.execute_labelled(command, label, Some(key)) {
                Ok(()) => done.push((material, given)),
                Err(why) => {
                    self.note = Some(crate::trf!(
                        "Couldn't give {} to {}: {}.",
                        graph.name(),
                        material,
                        why
                    ));
                }
            }
        }
        if done.is_empty() {
            return Vec::new();
        }
        // Painted textures show the material.
        self.viewport.options_mut().surface = crate::viewport::Surface::Painted;
        let names: Vec<&str> = done.iter().map(|(m, _)| m.as_str()).collect();
        let mut said = crate::trf!(
            "Gave {} to {}: it is made on the shape at its real-world size and follows the \
             graph as you change it. Ctrl + Z takes it back.",
            graph.name(),
            names.join(", ")
        );
        if done.iter().any(|(_, g)| *g == Given::UnderPaint) {
            said.push_str(tr(
                " The paint already there stays on top of it (the graph is the bottom layer in \
                 the Paint mode).",
            ));
        }
        if let Some(owned) = owned_note {
            said = format!("{owned} {said}");
        }
        self.note = Some(said);
        done.into_iter().map(|(_, g)| g).collect()
    }

    /// Takes the graph back from `material` when it is made by one: its
    /// textures go, and it shows its plain colour again. One undo step.
    pub(crate) fn take_graph_back(&mut self, material: &str) {
        let made_by = self
            .document
            .scene()
            .paint()
            .set(material)
            .and_then(|s| given_graph(s));
        if made_by.is_none() {
            return;
        }
        let label = crate::trf!("Take the graph back from {}", material);
        let command = Command::SetTextureSet {
            material: material.to_string(),
            set: None,
        };
        match self.document.execute_labelled(command, label, None) {
            Ok(()) => {
                self.note = Some(crate::trf!(
                    "{} is no longer made by a graph: it shows its own colour again. Ctrl + Z \
                     takes this back.",
                    material
                ));
            }
            Err(why) => {
                self.note = Some(crate::trf!("Couldn't take the graph back: {}.", why));
            }
        }
    }

    /// Exports the Material mode's graph as a texture set into `folder`
    /// (from the folder window): at the texture budget's size, made for
    /// the Export window's Made for. Says why not when it can't.
    pub fn export_graph_textures_into(&mut self, folder: PathBuf) {
        let Some(graph) = self.material.current(&self.document).cloned() else {
            self.note = Some(tr("There is no graph to export.").to_string());
            return;
        };
        let (stops, checks) = crate::material::export_set::checks(Registry::standard(), &graph);
        if stops {
            self.note = Some(crate::trf!(
                "Couldn't export {}'s textures: {}",
                graph.name(),
                checks.join(" ")
            ));
            return;
        }
        let size = self.material.budget_size(&self.document);
        let options = crate::material::export_set::set_options(
            &graph,
            super::ExportSettings::of(self.exporting.choice),
        );
        self.material
            .export_set
            .start(folder, (*graph).clone(), size, options);
        self.note = Some(crate::trf!("Exporting {}'s textures…", graph.name()));
    }

    /// The Export window's check on graph materials going out (decision
    /// 0262): ones not made yet or that can't be, and what each graph
    /// bakes differently from how the app shows it. Each a plain-words
    /// line saying what to do.
    pub(super) fn graph_export_checks(&self, materials: &[String]) -> Vec<String> {
        let registry = Registry::standard();
        let scene = self.document.scene();
        let mut out = Vec::new();
        let mut graphs_seen = Vec::new();
        for material in materials {
            let Some(set) = scene.paint().set(material) else {
                continue;
            };
            for (path, _) in set.listed(true) {
                let Some(fill) = set.layer(&path).and_then(|l| l.graph.clone()) else {
                    continue;
                };
                let Some(graph) = scene.graphs().get(fill.graph) else {
                    out.push(crate::trf!(
                        "{}: its graph is no longer in the project, so its last textures go \
                         out as they are. Undo brings the graph back.",
                        material
                    ));
                    continue;
                };
                if let Some(why) = self.graph_fill_problem(material, &path) {
                    out.push(crate::trf!(
                        "{}: the graph {} can't be made ({}), so its textures would go out \
                         empty. Open it in the Material mode to see why.",
                        material,
                        graph.name(),
                        why
                    ));
                } else if fill.made == 0 || self.graph_fill_making(material, &path) {
                    out.push(crate::trf!(
                        "{}: {} is still being made on the shape. Wait a moment for it to \
                         finish, or the textures go out as they were last made.",
                        material,
                        graph.name()
                    ));
                }
                if !graphs_seen.contains(&fill.graph) {
                    graphs_seen.push(fill.graph);
                    for note in matgraph::bake_notes(registry, graph, true) {
                        out.push(format!(
                            "{}: {}",
                            graph.name(),
                            note.describe(registry, graph)
                        ));
                    }
                }
            }
        }
        out
    }
}
