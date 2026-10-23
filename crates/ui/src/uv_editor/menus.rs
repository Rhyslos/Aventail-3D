//! The UV Editor's Texture and Trim menus (decisions 0100 and 0106), and
//! the texture and texel density line.

use egui::Ui;
use scene::Document;

/// Marks texture setting changes apart from every other undo step, so a
/// drag of the density field is one step.
const TEXTURE_KEY: u64 = 0x5458_5452_0000_0001;

/// The menu entries for the project's texture settings (decision 0100):
/// its size, and the texel density aimed for.
pub(crate) fn texture_menu(ui: &mut Ui, doc: &mut Document) {
    let mut texture = doc.scene().texture();
    let mut changed = false;
    ui.label(egui::RichText::new("Texture size").strong());
    for size in scene::TEXTURE_SIZES {
        if ui
            .add(egui::Button::selectable(
                texture.size == size,
                format!("{size} × {size} px"),
            ))
            .on_hover_text("The size of the texture this project's UVs are made for")
            .clicked()
        {
            texture.size = size;
            changed = true;
        }
    }
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(TEXEL_DENSITY_LABEL);
        changed |= ui
            .add(
                egui::DragValue::new(&mut texture.density)
                    .range(1.0..=scene::MAX_DENSITY)
                    .speed(1.0)
                    .suffix(" px/m"),
            )
            .on_hover_text(
                "The texel density aimed for: texture pixels per metre of surface. 512 px/m \
                 is common for props seen up close in third-person games.",
            )
            .changed();
    });
    if changed && texture.is_valid() {
        let _ = doc.execute_labelled(
            scene::Command::SetTexture(texture),
            "Change the texture settings",
            Some(TEXTURE_KEY),
        );
    }
}

/// Marks trim sheet changes' undo merge keys, so a drag or a run of
/// typing in the Trim menu is one undo step.
const TRIM_KEY: u64 = 0x5452_494d_0000_0001;

/// The UV Editor's Trim menu (decision 0106): the trim sheet's rows, top
/// to bottom, each with its name, where it starts from the top of the
/// texture and its height (in the texture's pixels, as an image editor
/// shows them), to change or remove; quick even layouts; adding a row.
pub(crate) fn trim_menu(ui: &mut Ui, doc: &mut Document) {
    let size = doc.scene().texture().size as f32;
    let mut sheet = doc.scene().trim().clone();
    let mut changed = false;
    ui.set_max_width(340.0);
    ui.label(egui::RichText::new("Trim sheet").strong());
    ui.label(egui::RichText::new("Rows of a shared texture, repeating left to right.").weak());
    ui.label(egui::RichText::new("Lay strips on them: UV → Put on trim row.").weak());
    ui.separator();
    if sheet.rows.is_empty() {
        ui.label("No rows yet.");
    }
    // Top to bottom, as the texture looks in an image editor.
    let mut order: Vec<usize> = (0..sheet.rows.len()).collect();
    order.sort_by(|a, b| sheet.rows[*b].top.total_cmp(&sheet.rows[*a].top));
    let mut remove = None;
    egui::Grid::new("trim_rows")
        .num_columns(4)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            if !sheet.rows.is_empty() {
                ui.label(egui::RichText::new("Name").weak());
                ui.label(egui::RichText::new("From top").weak());
                ui.label(egui::RichText::new("Height").weak());
                ui.label("");
                ui.end_row();
            }
            for i in order {
                let row = &mut sheet.rows[i];
                let mut name = row.name.clone();
                if ui
                    .add_sized([110.0, 20.0], egui::TextEdit::singleline(&mut name))
                    .on_hover_text("What the row holds: bolts, a panel edge, a seal")
                    .changed()
                    && !name.trim().is_empty()
                {
                    name.truncate(scene::MAX_ROW_NAME);
                    row.name = name;
                    changed = true;
                }
                let mut from_top = ((1.0 - row.top) * size).round();
                let mut height = (row.height() * size).round();
                if ui
                    .add(
                        egui::DragValue::new(&mut from_top)
                            .range(0.0..=size - 1.0)
                            .speed(1.0)
                            .suffix(" px"),
                    )
                    .on_hover_text("Where the row starts, in pixels from the top of the texture")
                    .changed()
                {
                    let h = row.height();
                    row.top = 1.0 - from_top / size;
                    row.bottom = row.top - h;
                    changed = true;
                }
                if ui
                    .add(
                        egui::DragValue::new(&mut height)
                            .range(1.0..=size)
                            .speed(1.0)
                            .suffix(" px"),
                    )
                    .on_hover_text("How tall the row is, in pixels")
                    .changed()
                {
                    row.bottom = row.top - height / size;
                    changed = true;
                }
                if ui
                    .small_button("×")
                    .on_hover_text("Remove this row")
                    .clicked()
                {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove {
        sheet.rows.remove(i);
        changed = true;
    }
    ui.separator();
    // A new row below the lowest, as tall as it, if there is room.
    let lowest = sheet
        .rows
        .iter()
        .map(|r| (r.bottom, r.height()))
        .min_by(|a, b| a.0.total_cmp(&b.0));
    let (below, tall) = lowest.unwrap_or((1.0, 0.125));
    let room = below >= tall - 1e-6 && sheet.rows.len() < scene::MAX_TRIM_ROWS;
    if ui
        .add_enabled(room, egui::Button::new("Add a row"))
        .on_hover_text("A new row under the lowest one, as tall as it")
        .on_disabled_hover_text("There's no room under the lowest row: make it shorter first")
        .clicked()
    {
        sheet.rows.push(scene::TrimRow {
            name: format!("Row {}", sheet.rows.len() + 1),
            top: below,
            bottom: (below - tall).max(0.0),
        });
        changed = true;
    }
    ui.horizontal(|ui| {
        ui.label("Even rows:");
        for n in [2, 4, 8, 16] {
            if ui
                .small_button(n.to_string())
                .on_hover_text(format!("{n} rows of the same height, filling the texture"))
                .clicked()
            {
                sheet = scene::TrimSheet::even(n);
                changed = true;
            }
        }
    });
    if !sheet.rows.is_empty()
        && ui
            .button("Remove every row")
            .on_hover_text("No trim sheet. Pieces already on its rows stay where they are.")
            .clicked()
    {
        sheet.rows.clear();
        changed = true;
    }
    // A change that would make rows overlap isn't made: the numbers spring
    // back.
    if changed && sheet.is_valid() {
        let _ = doc.execute_labelled(
            scene::Command::SetTrim(sheet),
            "Change the trim sheet",
            Some(TRIM_KEY),
        );
    }
}

/// The label of the texel density field.
pub(crate) const TEXEL_DENSITY_LABEL: &str = "Texel density";

/// The texture and texel density line: "2048 px · 498 px/m (aim 512)".
pub(crate) fn density_text(
    texture: scene::TextureSettings,
    density: Option<f32>,
    selected: bool,
) -> String {
    let aim = texture.density.round();
    match density {
        Some(d) => format!(
            "{} px · {}{} px/m (aim {aim})",
            texture.size,
            if selected { "selected " } else { "" },
            d.round()
        ),
        None => format!("{} px · aim {aim} px/m", texture.size),
    }
}
