//! "Why does this look wrong?" ([decision 0203](../../../../docs/decisions/0203-why-does-this-look-wrong.md)):
//! a window listing, for each painted object, the usual reasons a
//! painted model looks off (seams that show, stretched pixels, too few
//! pixels, flat lighting), each explained in plain words with Show (to
//! see where) and, where there is one, a one-click Fix. Opened from the
//! Paint layers panel or the command search; it looks again whenever the
//! scene changes while it is open.

use super::Workbench;
use crate::dialog::{self, Dialog};
use crate::drawing::SceneDrawing;
use crate::palette::Mode;
use crate::text::tr;
use crate::theme;
use egui::RichText;
use paint::advice::Finding;
use scene::{Element, ElementSelection, ObjectId};

/// The window's title, and the button that opens it.
pub(crate) const LOOK_WRONG_TITLE: &str = crate::text!("Why does this look wrong?");

/// The window's state.
#[derive(Debug, Clone, Default)]
pub(crate) struct LookWrong {
    /// Whether the window is showing.
    pub(crate) open: bool,
    /// What was found, and the scene revision and texture settings it
    /// was found for.
    found: Option<((u64, u32, u32), Vec<Found>)>,
}

/// One finding on one object.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Found {
    pub(crate) id: ObjectId,
    pub(crate) name: String,
    pub(crate) material: String,
    pub(crate) finding: Finding,
}

impl Found {
    /// What it is, in a line.
    pub(crate) fn text(&self) -> String {
        match &self.finding {
            Finding::Seams { edges, .. } => {
                crate::trf!("the paint breaks at {} seams", edges)
            }
            Finding::Stretched { worst, .. } => {
                crate::trf!(
                    "stretched pixels (up to {}× one way)",
                    format!("{worst:.1}")
                )
            }
            Finding::Blurry {
                pixels_per_metre,
                aim,
                ..
            } => crate::trf!(
                "too few pixels ({} px/m, aiming for {})",
                pixels_per_metre.round(),
                aim.round()
            ),
            Finding::FlatLight { .. } => tr("flat lighting: hardly any light and dark").to_string(),
        }
    }

    /// Why it shows, and what to do, in plain words.
    pub(crate) fn help(&self) -> &'static str {
        match &self.finding {
            Finding::Seams { .. } => tr(
                "A seam is where the shape is cut open to lie flat on the texture. Here the two \
                 sides of the cut were painted differently, so a hard line shows on the model. \
                 Paint across the seam on the model (not in the UV Editor): the brush paints \
                 both sides at once. The Blur brush along the seam softens it too.",
            ),
            Finding::Stretched { .. } => tr(
                "The UVs squash these polygons, so each texture pixel covers much more of the \
                 surface one way than the other and painted detail smears into streaks. Show \
                 picks them and the Stretch view colours them; unwrapping them again (UV → \
                 Unwrap) or relaxing them evens it out.",
            ),
            Finding::Blurry { .. } => tr(
                "These polygons get fewer texture pixels per metre than the project aims for, \
                 so the paint looks soft or blocky up close. Fix raises the texture size, \
                 repainting your strokes sharp at the new size. Making their UV pieces bigger \
                 helps too.",
            ),
            Finding::FlatLight { .. } => tr(
                "The painting's lightest and darkest parts are close together, so the model \
                 looks flat, especially in games that don't light textures. Paint lighter tops \
                 and edges and darker undersides and creases. Fix adds a Light and shade layer \
                 to start from.",
            ),
        }
    }

    /// The polygons it's about.
    fn faces(&self) -> Vec<scene::FaceId> {
        let faces = match &self.finding {
            Finding::Seams { faces, .. }
            | Finding::Stretched { faces, .. }
            | Finding::Blurry { faces, .. } => faces.clone(),
            Finding::FlatLight { .. } => Vec::new(),
        };
        faces.into_iter().map(scene::FaceId::from_raw).collect()
    }

    /// Whether it has a one-click fix.
    pub(crate) fn has_fix(&self) -> bool {
        matches!(
            self.finding,
            Finding::Blurry { .. } | Finding::FlatLight { .. }
        )
    }
}

impl Workbench {
    /// Looks again, if the window is open and the scene or texture
    /// settings changed since the last look.
    pub(super) fn update_look_wrong(&mut self, drawing: &SceneDrawing) {
        if std::mem::take(&mut self.paint_layers.look_wrong_asked) {
            self.look_wrong.open = true;
        }
        if !self.look_wrong.open {
            return;
        }
        let texture = self.document.scene().texture();
        let key = (
            self.document.revision(),
            texture.size,
            texture.density.round() as u32,
        );
        if self
            .look_wrong
            .found
            .as_ref()
            .is_some_and(|(k, _)| *k == key)
        {
            return;
        }
        let mut found = Vec::new();
        let materials: Vec<(String, std::sync::Arc<paint::TextureSet>)> = self
            .document
            .scene()
            .paint()
            .sets()
            .map(|(m, s)| (m.to_string(), std::sync::Arc::clone(s)))
            .collect();
        for (material, set) in materials {
            let Some(composite) = paint::Composite::of(&set) else {
                continue;
            };
            let image = composite.image();
            for (key, surface, slots) in self.material_shapes(&material, drawing) {
                let id = ObjectId::from_raw(key);
                let scene = self.document.scene();
                let Some(object) = scene.object(id) else {
                    continue;
                };
                let m = scene.shape_matrix(id);
                let scale = (m.x_axis.truncate().length()
                    + m.y_axis.truncate().length()
                    + m.z_axis.truncate().length())
                    / 3.0;
                let name = object.name.clone();
                for finding in
                    paint::advice::look_for(&surface, &slots, image, scale, texture.density)
                {
                    found.push(Found {
                        id,
                        name: name.clone(),
                        material: material.clone(),
                        finding,
                    });
                }
            }
        }
        self.look_wrong.found = Some((key, found));
    }

    /// What was found at the last look.
    pub(crate) fn look_wrong_found(&self) -> Vec<Found> {
        self.look_wrong
            .found
            .as_ref()
            .map(|(_, f)| f.clone())
            .unwrap_or_default()
    }

    /// Draws the window, if open.
    pub(super) fn look_wrong_window(&mut self, ctx: &egui::Context) {
        if !self.look_wrong.open {
            return;
        }
        let found = self.look_wrong_found();
        let mut done = false;
        let mut show = None;
        let mut fix = None;
        let summary = if found.is_empty() {
            tr("Nothing found that looks wrong.").to_string()
        } else {
            crate::trf!("{} things that may look wrong.", found.len())
        };
        let outcome =
            Dialog::new(tr(LOOK_WRONG_TITLE), 560.0).show(ctx, |d| {
                d.header(true, |_| {});
                d.body(|ui| {
                dialog::heading(ui, &summary);
                if found.is_empty() {
                    dialog::help(
                        ui,
                        tr("Checked each painted object for seams painted differently on their \
                         two sides, stretched pixels, fewer pixels per metre than the project \
                         aims for, and flat lighting."),
                    );
                    return;
                }
                dialog::help(
                    ui,
                    tr("Point at one for why and what to do. Show picks where it is; Fix mends it \
                     as one step Ctrl + Z can take back."),
                );
                ui.add_space(4.0);
                egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                    egui::Grid::new("look_wrong_found")
                        .num_columns(3)
                        .spacing([10.0, 6.0])
                        .show(ui, |ui| {
                            for (i, f) in found.iter().enumerate() {
                                let what = format!("{}: {}", f.name, f.text());
                                ui.label(RichText::new(&what).color(theme::WARNING))
                                    .on_hover_text(f.help());
                                if f.faces().is_empty() {
                                    ui.label("");
                                } else {
                                    let button = ui
                                        .small_button(tr("Show"))
                                        .on_hover_text(tr("Pick the polygons, to see where"));
                                    button.widget_info(|| {
                                        egui::WidgetInfo::labeled(
                                            egui::WidgetType::Button,
                                            true,
                                            crate::trf!("Show {}", what),
                                        )
                                    });
                                    if button.clicked() {
                                        show = Some(i);
                                    }
                                }
                                if f.has_fix() {
                                    let button = ui.small_button(tr("Fix")).on_hover_text(f.help());
                                    button.widget_info(|| {
                                        egui::WidgetInfo::labeled(
                                            egui::WidgetType::Button,
                                            true,
                                            crate::trf!("Fix {}", what),
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
                    if dialog::main_button(ui, tr("Done")).clicked() {
                        done = true;
                    }
                });
            });
        let (enter, escape) = dialog::enter_or_escape(ctx);
        if outcome.closed || done || enter || escape {
            self.look_wrong.open = false;
        }
        if let Some(f) = show.and_then(|i| found.get(i)) {
            self.show_found(f);
        }
        if let Some(f) = fix.and_then(|i| found.get(i)) {
            self.fix_found(f);
        }
    }

    /// Picks the polygons `found` is about (in the Polygons mode), and
    /// colours stretch for stretched pixels.
    pub(crate) fn show_found(&mut self, found: &Found) {
        let faces = found.faces();
        self.document.set_selection([found.id]);
        self.mode = Mode::Polygons;
        if !faces.is_empty() {
            self.document.select_elements(
                [(
                    found.id,
                    ElementSelection::of(faces.into_iter().map(Element::Polygon)),
                )],
                tr("Show what looks wrong"),
                None,
            );
        }
        if matches!(found.finding, Finding::Stretched { .. }) {
            self.viewport.options_mut().surface = crate::viewport::Surface::Stretch;
        }
        self.note = Some(format!("{}: {}.", found.name, found.text()));
    }

    /// Mends `found`, as one undo step: a bigger texture for too few
    /// pixels, a Light and shade layer for flat lighting.
    pub(crate) fn fix_found(&mut self, found: &Found) {
        match &found.finding {
            Finding::Blurry {
                pixels_per_metre,
                aim,
                ..
            } => {
                let mut texture = self.document.scene().texture();
                let wanted = texture.size as f32 * (aim * 0.5 / pixels_per_metre.max(1e-3));
                let size = scene::TEXTURE_SIZES
                    .iter()
                    .copied()
                    .filter(|s| *s <= MAX_FIX_SIZE)
                    .find(|s| *s as f32 >= wanted)
                    .unwrap_or(MAX_FIX_SIZE);
                if size <= texture.size {
                    self.note = Some(
                        tr(
                            "The texture is already as big as Fix makes it: make the UV pieces \
                         bigger instead.",
                        )
                        .to_string(),
                    );
                    return;
                }
                texture.size = size;
                if let Err(why) = self.document.execute_labelled(
                    scene::Command::SetTexture(texture),
                    tr("Raise the texture size"),
                    None,
                ) {
                    self.note = Some(crate::trf!("Couldn't change the texture size: {}.", why));
                }
            }
            Finding::FlatLight { .. } => {
                let Some(set) = self.document.scene().paint().set(&found.material) else {
                    return;
                };
                let mut set = (**set).clone();
                let Some(image) = paint::Image::new(set.size) else {
                    return;
                };
                let mut layer = paint::Layer::paint(tr("Light and shade"), image);
                layer.lighting = Some(paint::Lighting::default());
                layer.blend = paint::Blend::Overlay;
                layer.opacity = 0.6;
                if set.insert_layer(layer, None).is_none() {
                    return;
                }
                match self.document.execute_labelled(
                    scene::Command::SetTextureSet {
                        material: found.material.clone(),
                        set: Some(set),
                    },
                    tr("Add a light and shade layer"),
                    None,
                ) {
                    Ok(_) => {
                        if !self
                            .paint_layers
                            .gradients_to_make
                            .contains(&found.material)
                        {
                            self.paint_layers
                                .gradients_to_make
                                .push(found.material.clone());
                        }
                    }
                    Err(why) => {
                        self.note = Some(crate::trf!("Couldn't add the layer: {}.", why));
                    }
                }
            }
            _ => {}
        }
    }
}

/// The largest texture Fix raises the size to: bigger ones are heavy on
/// memory and most game textures stop here.
const MAX_FIX_SIZE: u32 = 4096;
