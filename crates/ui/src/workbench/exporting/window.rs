//! The Export window's contents: what the export is made for, the general
//! settings, the custom settings, and the game project it can export
//! straight into.

use super::{
    EXPORT_TITLE, ExportChoice, ExportSection, SECTIONS_NOTE, TargetRequest, plural, thousands,
};
use crate::dialog::{self, Badge, Dialog, Section};
use crate::text::tr;
use crate::theme;
use crate::workbench::{Workbench, WorkbenchAction, file_name};
use egui::{RichText, Ui};
use fileio::engine::Engine;
use fileio::export::{
    ExportFormat, ExportOptions, ExportPreset, ExportSummary, ExportWhat, FileUnits, UpAxis,
};
use std::path::{Path, PathBuf};

impl Workbench {
    /// Draws the Export window, if open
    /// ([decision 0087](../../../../../docs/decisions/0087-one-look-for-every-window.md)):
    /// Made for in the title bar, a side list of sections (General and
    /// Game project; Materials, Mesh, Rig and Animations join it with
    /// their features), and Export… in the footer.
    pub(in crate::workbench) fn export_window(
        &mut self,
        ctx: &egui::Context,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        if !self.exporting.open {
            return;
        }
        let mut choice = self.exporting.choice;
        // Until the person sets it, Levels of detail is on when anything
        // going out has levels made.
        choice.lod_levels = self.levels_go_out(choice, choice.what);
        let collision = super::goes_out::collision_line(
            &self.document,
            choice.what,
            choice.preset == Some(ExportPreset::Godot),
        );
        let mut section = self.exporting.section;
        let selected = !self.document.selection().is_empty();
        let summary = self.export_summary();
        let textures = self.export_textures();
        let target = self.document.scene().export_target().cloned();
        let engine = self.detected_engine();
        let into = self.target_request(None);
        let fallback = self.fallback_preset();
        let nothing = summary.objects == 0;
        let mut go = false;
        let mut cancel = false;
        let mut bake = false;
        let mut target_change: Option<Option<scene::ExportTarget>> = None;
        let mut choose_folder = false;
        let mut go_to_game_project = false;
        let sections = [
            Section {
                id: ExportSection::General,
                name: tr("General"),
                badge: None,
                apart: false,
                label: None,
            },
            Section {
                id: ExportSection::GameProject,
                name: tr("Game project"),
                badge: Some(match &target {
                    Some(t) if t.on_save => Badge::On,
                    _ => Badge::Off,
                }),
                apart: true,
                label: None,
            },
        ];
        let outcome = Dialog::new(tr(EXPORT_TITLE), 700.0)
            .body_height(490.0)
            .show(ctx, |d| {
                d.header(true, |ui| {
                    ui.label(RichText::new(tr("Made for")).color(theme::TEXT_WEAK));
                    made_for(ui, &mut choice);
                });
                d.sections(
                    &mut section,
                    &sections,
                    Some(tr(SECTIONS_NOTE)),
                    |ui, shown| match shown {
                        ExportSection::General => {
                            let open_game_project = general(
                                ui,
                                &mut choice,
                                selected,
                                (summary, &collision, &textures),
                                GameProjectView {
                                    target: target.as_ref(),
                                    engine: engine.as_ref(),
                                    into: into.as_ref(),
                                },
                                &mut bake,
                            );
                            go_to_game_project |= open_game_project;
                        }
                        ExportSection::GameProject => game_project(
                            ui,
                            target.as_ref(),
                            engine.as_ref(),
                            fallback,
                            &mut target_change,
                            &mut choose_folder,
                        ),
                    },
                );
                d.footer(
                    nothing.then_some(tr("Nothing to export: nothing shown is a shape.")),
                    |ui| {
                        let response = ui
                            .add_enabled_ui(!nothing, |ui| dialog::main_button(ui, tr("Export…")));
                        if response.inner.clicked() {
                            go = true;
                        }
                        if dialog::plain_button(ui, tr("Cancel")).clicked() {
                            cancel = true;
                        }
                    },
                );
            });
        if go_to_game_project {
            section = ExportSection::GameProject;
        }
        let (enter, escape) = dialog::enter_or_escape(ctx);
        if enter && !nothing {
            go = true;
        }
        if outcome.closed || escape {
            cancel = true;
        }
        if !selected && choice.what == ExportWhat::Selected {
            choice.what = ExportWhat::Everything;
        }
        // Changing a setting by hand leaves the preset.
        if let Some(preset) = choice.preset
            && !choice.follows(preset)
        {
            choice.preset = None;
        }
        self.exporting.choice = choice;
        self.exporting.section = section;
        if bake {
            self.baking.open = true;
        }
        if let Some(change) = target_change {
            self.run_target(change);
        }
        if choose_folder {
            let start = target
                .as_ref()
                .map(|t| PathBuf::from(&t.folder))
                .or_else(|| {
                    self.file
                        .path
                        .as_deref()
                        .and_then(Path::parent)
                        .map(Path::to_path_buf)
                });
            actions.push(WorkbenchAction::ChooseExportFolder { start });
        }
        if cancel {
            self.exporting.open = false;
        }
        if go {
            self.exporting.open = false;
            actions.push(self.export_action(choice, None));
        }
    }
}

/// The Made for drop-down in the Export window's title bar: each preset,
/// with its kind of file on the right, then Custom settings.
pub(super) fn made_for(ui: &mut Ui, choice: &mut ExportChoice) {
    let (name, hint) = match choice.preset {
        Some(preset) => (preset.name(), preset.settings().format.name()),
        None => (tr(CUSTOM_SETTINGS), choice.format.name()),
    };
    let response = dialog::drop_down(ui, tr("Made for"), name, hint, 230.0, |ui| {
        for preset in ExportPreset::ALL {
            if dialog::drop_down_entry(
                ui,
                choice.preset == Some(preset),
                preset.name(),
                preset.settings().format.name(),
            )
            .on_hover_text(preset.description())
            .clicked()
            {
                *choice = choice.with_preset(preset);
            }
        }
        ui.separator();
        if dialog::drop_down_entry(
            ui,
            choice.preset.is_none(),
            tr(CUSTOM_SETTINGS),
            tr("any program"),
        )
        .on_hover_text(tr("Pick the kind of file and how it's set up yourself"))
        .clicked()
        {
            choice.preset = None;
        }
    });
    response.on_hover_text(tr(
        "Where the file is going: this picks the right settings for it",
    ));
}

/// The name of the Made for choice without a preset.
pub(crate) const CUSTOM_SETTINGS: &str = crate::text!("Custom settings");

/// What the General section shows about the game project folder.
struct GameProjectView<'a> {
    target: Option<&'a scene::ExportTarget>,
    engine: Option<&'a (Engine, PathBuf)>,
    into: Option<&'a TargetRequest>,
}

/// The Export window's General section: what goes out, the file's
/// settings, a word on the game project folder, and the numbers. Gives
/// back whether "Game project" was clicked to go there.
fn general(
    ui: &mut Ui,
    choice: &mut ExportChoice,
    selected: bool,
    (summary, collision, textures): (ExportSummary, &str, &[fileio::export::TextureReport]),
    game: GameProjectView<'_>,
    bake: &mut bool,
) -> bool {
    dialog::heading(ui, tr("General"));
    dialog::help(
        ui,
        match choice.preset {
            Some(preset) => format!("{}: {}.", preset.name(), preset.description()),
            None => tr("Any program: pick the kind of file and how it's set up below.").to_string(),
        },
    );
    ui.add_space(4.0);
    dialog::row(ui, tr("Objects"), |ui| {
        let selected_help = if selected {
            tr("The selected objects, with what's inside them")
        } else {
            tr("Select some objects first")
        };
        dialog::segmented(
            ui,
            &mut choice.what,
            &[
                (
                    ExportWhat::Everything,
                    tr("Everything shown"),
                    tr("Every object you can see in the viewport"),
                ),
                (ExportWhat::Selected, tr("Selected only"), selected_help),
            ],
        );
    });
    dialog::row(ui, tr("Includes"), |ui| {
        ui.add_enabled_ui(false, |ui| {
            dialog::chip(ui, &mut true, tr("Mesh"));
            dialog::chip(ui, &mut true, tr("Materials"));
        })
        .response
        .on_disabled_hover_text(tr(
            "Shapes and their materials always go out. Skeletons and animations will be \
             listed here once the app has them, to leave out when you don't need them.",
        ));
        let levels = dialog::chip(ui, &mut choice.lod_levels, tr("Levels of detail"));
        if levels.changed() {
            // Set by hand: kept from now on, instead of following the scene.
            choice.lod_chosen = true;
        }
        levels.on_hover_text(tr(
            "Each object's levels of detail (the Levels panel, Window → Levels) go out beside it \
             as meshes of their own, named like Rock_LOD1, with the distances they show from. \
             It starts on when an object has levels, until you set it yourself. Godot makes \
             its own levels when it imports a file, so it doesn't need these",
        ));
    });
    texture_row(ui, choice);
    match choice.preset {
        Some(preset) => {
            dialog::row(ui, tr("File settings"), |ui| {
                dialog::locked(ui, &settings_line(preset)).on_hover_text(tr(
                    "Set by Made for. Choose Custom settings to change them.",
                ));
            });
            if preset == ExportPreset::SubstancePainter {
                dialog::row(ui, tr("Before baking"), |ui| {
                    if ui
                        .button(tr("Check for baking…"))
                        .on_hover_text(tr(
                            "List what would spoil the bake, in plain words, with one-click \
                             fixes",
                        ))
                        .clicked()
                    {
                        *bake = true;
                    }
                });
            }
        }
        None => custom_settings(ui, choice),
    }
    dialog::help(
        ui,
        tr(
            "Shading goes out as you see it: with Automatic hard edges on (Mesh menu), by angle; \
         otherwise by the edges you marked hard.",
        ),
    );
    dialog::divider(ui);
    let go_there = game_project_note(ui, game);
    dialog::divider(ui);
    dialog::heading(ui, tr("What goes out"));
    ui.add_space(2.0);
    dialog::tiles(
        ui,
        &[
            (
                thousands(summary.objects),
                plural(summary.objects, tr("object"), tr("objects")),
            ),
            (
                thousands(summary.triangles),
                plural(summary.triangles, tr("triangle"), tr("triangles")),
            ),
            (
                thousands(summary.materials),
                plural(summary.materials, tr("material"), tr("materials")),
            ),
        ],
    );
    ui.add_space(4.0);
    dialog::help(ui, collision);
    texture_budget(ui, choice, textures);
    go_there
}

/// The Textures row (decision 0204): PNG, TGA or left out, and the
/// padding round the UV pieces.
fn texture_row(ui: &mut Ui, choice: &mut ExportChoice) {
    use fileio::export::TextureFormat;
    let name = |f: TextureFormat| match f {
        TextureFormat::Png => tr("PNG"),
        TextureFormat::Tga => tr("TGA"),
        TextureFormat::None => tr("Leave out"),
    };
    dialog::row(ui, tr("Textures"), |ui| {
        let label = ui.label("");
        egui::ComboBox::from_id_salt("export_textures")
            .selected_text(name(choice.textures))
            .show_ui(ui, |ui| {
                for f in TextureFormat::ALL {
                    ui.selectable_value(&mut choice.textures, f, name(f));
                }
            })
            .response
            .labelled_by(label.id)
            .on_hover_text(tr(
                "How your painted textures go out: inside a glTF file (always PNG, as glTF \
                 wants), or beside an OBJ file and named in its materials",
            ));
        if choice.textures != TextureFormat::None {
            ui.label(tr("Padding"));
            ui.add(
                egui::DragValue::new(&mut choice.padding)
                    .range(0..=fileio::export::textures::MAX_PADDING)
                    .suffix(tr(" px")),
            )
            .on_hover_text(tr(
                "How far each piece's colours are spread past its edges, so the game's \
                 smaller copies of the texture don't pull in the colour around it. 8 px suits \
                 most textures; more for ones seen from far away",
            ));
        }
    });
}

/// The texture budget (decision 0204): each painted texture going out,
/// its size, kind and what it takes on the graphics card.
fn texture_budget(ui: &mut Ui, choice: &ExportChoice, textures: &[fileio::export::TextureReport]) {
    if textures.is_empty() || choice.textures == fileio::export::TextureFormat::None {
        return;
    }
    let format = match (choice.format, choice.textures) {
        (fileio::export::ExportFormat::Glb, _) | (_, fileio::export::TextureFormat::Png) => {
            tr("PNG")
        }
        _ => tr("TGA"),
    };
    ui.add_space(6.0);
    dialog::heading(ui, tr("Texture budget"));
    let mut total = 0;
    for t in textures {
        total += t.gpu_bytes;
        dialog::help(
            ui,
            crate::trf!(
                "{}: {} × {} {}, red, green, blue and alpha, {} on the graphics card",
                t.material,
                t.size,
                t.size,
                format,
                crate::format_bytes(t.gpu_bytes)
            ),
        );
    }
    dialog::help(
        ui,
        crate::trf!(
            "In all: {} on the graphics card, with the smaller copies games make",
            crate::format_bytes(total)
        ),
    );
}

/// A preset's file settings in one line: "glTF · metres · Y up ·
/// triangles".
pub(crate) fn settings_line(preset: ExportPreset) -> String {
    let settings = preset.settings();
    let options = settings.format.options(&ExportOptions {
        units: settings.units,
        up: settings.up,
        triangulate: settings.triangulate,
        ..ExportOptions::default()
    });
    format!(
        "{} · {} · {} · {}",
        settings.format.name(),
        match options.units {
            FileUnits::Metres => tr("metres"),
            FileUnits::Centimetres => tr("centimetres"),
        },
        match options.up {
            UpAxis::Y => tr("Y up"),
            UpAxis::Z => tr("Z up"),
        },
        if options.triangulate {
            tr("triangles")
        } else {
            tr("polygons kept")
        }
    )
}

/// The file's settings, to choose by hand (Custom settings).
fn custom_settings(ui: &mut Ui, choice: &mut ExportChoice) {
    dialog::row(ui, tr("File type"), |ui| {
        dialog::segmented(
            ui,
            &mut choice.format,
            &[
                (
                    ExportFormat::Glb,
                    tr("glTF (.glb)"),
                    tr(
                        "For Godot, Unity, Unreal Engine and Substance Painter: each object at its \
                     pivot",
                    ),
                ),
                (
                    ExportFormat::Obj,
                    tr("OBJ (.obj)"),
                    tr("Plain, read by almost every 3D program"),
                ),
            ],
        );
    });
    let fixed = choice.format.fixes_space();
    ui.add_enabled_ui(!fixed, |ui| {
        dialog::row(ui, tr("Units"), |ui| {
            dialog::segmented(
                ui,
                &mut choice.units,
                &[
                    (
                        FileUnits::Metres,
                        tr("Metres"),
                        tr("1 unit in the file is 1 m: Godot, Unity, Substance Painter"),
                    ),
                    (
                        FileUnits::Centimetres,
                        tr("Centimetres"),
                        tr("1 unit in the file is 1 cm: Unreal Engine"),
                    ),
                ],
            );
        });
        dialog::row(ui, tr("Up axis"), |ui| {
            dialog::segmented(
                ui,
                &mut choice.up,
                &[
                    (
                        UpAxis::Y,
                        tr("Y up"),
                        tr("Godot, Unity, Unreal's OBJ import and Substance Painter"),
                    ),
                    (
                        UpAxis::Z,
                        tr("Z up"),
                        tr("As in this app; Blender and 3ds Max's own axes"),
                    ),
                ],
            );
        });
        dialog::row(ui, tr("Triangles"), |ui| {
            ui.checkbox(&mut choice.triangulate, tr("Cut polygons into triangles"))
                .on_hover_text(tr(
                    "Cut every polygon into triangles the same way the viewport does, so a \
                     bake in Substance Painter matches the game exactly",
                ));
        });
    });
    if fixed {
        dialog::help(
            ui,
            tr("glTF is always metres, Y up and triangles: every program reads it that way."),
        );
    }
}

/// The General section's word on the game project folder: whether every
/// save exports into it, and where. Gives back whether "Game project" was
/// clicked.
fn game_project_note(ui: &mut Ui, game: GameProjectView<'_>) -> bool {
    let on = game.target.is_some_and(|t| t.on_save);
    dialog::heading_with_pill(ui, tr("Game project"), on);
    let engine_name = game.engine.map(|(engine, _)| engine.name());
    match (game.target, game.into) {
        (Some(target), Some(into)) => {
            if target.on_save {
                dialog::help(
                    ui,
                    crate::trf!(
                        "Every time you save the project, the model is also exported into \
                         your game project, so {} picks up the change by itself. It goes to:",
                        engine_name.unwrap_or(tr("the game engine"))
                    ),
                );
            } else {
                dialog::help(
                    ui,
                    tr(
                        "Exporting on every save is off. File → Export to game project exports \
                     into it when you choose. It goes to:",
                    ),
                );
            }
            let file = format!(
                "{}.{}",
                into.name,
                into.preset.settings().format.extension()
            );
            let place = match game.engine {
                Some((engine, project)) => {
                    let inside = into
                        .folder
                        .strip_prefix(project)
                        .ok()
                        .map(|rest| {
                            rest.components()
                                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let mut parts = vec![format!("{} ({})", file_name(project), engine.name())];
                    parts.extend(inside);
                    parts.push(file);
                    parts.join(" › ")
                }
                None => into.folder.join(file).display().to_string(),
            };
            dialog::place(ui, &place).on_hover_text(into.folder.display().to_string());
        }
        _ => {
            dialog::help(
                ui,
                tr(
                    "Pick a folder inside your Godot, Unity, Unreal Engine or Solis project, and every \
                 save exports the model into it too, so the engine picks up the change by \
                 itself.",
                ),
            );
        }
    }
    let mut clicked = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let words = if game.target.is_some() {
            tr("Change the folder or switch it off under ")
        } else {
            tr("Set it up under ")
        };
        ui.label(
            RichText::new(words)
                .font(dialog::help_font())
                .color(theme::TEXT_WEAK),
        );
        clicked = ui
            .link(RichText::new(tr("Game project")).font(dialog::help_font()))
            .clicked();
        ui.label(
            RichText::new(tr(" in the list."))
                .font(dialog::help_font())
                .color(theme::TEXT_WEAK),
        );
    });
    clicked
}

/// The Export window's Game project section: the folder and its engine,
/// Export on every save, and choosing or clearing it. A change is put in
/// `change`; `choose` asks for a folder.
fn game_project(
    ui: &mut Ui,
    target: Option<&scene::ExportTarget>,
    engine: Option<&(Engine, PathBuf)>,
    fallback: ExportPreset,
    change: &mut Option<Option<scene::ExportTarget>>,
    choose: &mut bool,
) {
    dialog::heading_with_pill(ui, tr("Game project"), target.is_some_and(|t| t.on_save));
    dialog::help(
        ui,
        tr(
            "Export straight into a Godot, Unity, Unreal Engine or Solis project: pick the folder inside \
         it that your models go in. Every save then exports the model there too, named after \
         the project, so the engine picks up the change by itself.",
        ),
    );
    ui.add_space(4.0);
    match target {
        None => {
            dialog::row(ui, tr("Folder"), |ui| {
                ui.label(RichText::new(tr("None chosen")).color(theme::TEXT_WEAK));
            });
        }
        Some(target) => {
            dialog::row(ui, tr("Folder"), |ui| {
                dialog::place(ui, &target.folder);
            });
            dialog::row(ui, tr("Engine"), |ui| {
                ui.label(match engine {
                    Some((engine, project)) => crate::trf!(
                        "In the {} project {}: exported as {} with its settings.",
                        engine.name(),
                        file_name(project),
                        engine.preset().settings().format.name()
                    ),
                    None => crate::trf!(
                        "Not inside a Godot, Unity, Unreal Engine or Solis project: exported with \
                         {}'s settings (Made for picks which).",
                        fallback.name()
                    ),
                });
            });
            dialog::row(ui, tr("On save"), |ui| {
                let mut on_save = target.on_save;
                if ui
                    .checkbox(&mut on_save, tr("Export on every save"))
                    .on_hover_text(tr(
                        "Each time you save the project, export it into this folder too, named \
                         after the project, so the engine picks up the change",
                    ))
                    .changed()
                {
                    *change = Some(Some(scene::ExportTarget {
                        on_save,
                        ..target.clone()
                    }));
                }
            });
        }
    }
    ui.add_space(4.0);
    dialog::row(ui, "", |ui| {
        if ui
            .button(tr("Choose folder…"))
            .on_hover_text(tr(
                "Pick the folder inside your game project the models go in",
            ))
            .clicked()
        {
            *choose = true;
        }
        if target.is_some()
            && ui
                .button(tr("Clear"))
                .on_hover_text(tr("Stop exporting into a game project"))
                .clicked()
        {
            *change = Some(None);
        }
    });
}
