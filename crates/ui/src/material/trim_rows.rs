//! A Trim sheet box's rows in Attributes ([decision 0289](../../../../docs/decisions/0289-trim-sheet-box.md)):
//! where each strip lies on the texture, in plain words (from the top, on
//! which of the project's trim rows), what has no row, and a button making
//! the project's trim rows match the strips, so strips of a model can be
//! put on them in the UV Editor.
//!
//! The rows are the scene's (the UV Editor's Trim menu, decision 0106),
//! read through the project's registry; the button changes them with one
//! `scene::Command::SetTrim`, one undo step.

use super::MaterialEditor;
use crate::attributes::section;
use crate::dialog::help;
use crate::text::tr;
use matgraph::boxes::trim_sheet::{Laid, Layout, STRIP_KEYS, STRIPS, describe, layout_of};
use matgraph::{BoxId, Graph, GraphBox, PlugRef};
use scene::{Document, TrimRow, TrimSheet};

/// The button making the project's trim rows match the strips.
pub(crate) const MATCH_LABEL: &str = crate::text!("Make these the project's trim rows");

impl MaterialEditor {
    /// A Trim sheet box's rows: where its strips lie and the button
    /// making them the project's trim rows.
    pub(super) fn trim_rows(
        &mut self,
        ui: &mut egui::Ui,
        graph: &Graph,
        b: &GraphBox,
        id: BoxId,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let registry = self.registry_of(document);
        let laid = strips_of(graph, b, id, registry.trim_rows());
        section(ui, tr("Where the strips lie"));
        help(ui, source_words(&laid));
        for place in &laid.places {
            ui.label(describe(place));
        }
        for strip in &laid.past_rows {
            help(
                ui,
                crate::trf!(
                    "Strip {} has no trim row to lie on: the project has {}. Add rows in the UV \
                     Editor's Trim menu, or plug its material into an earlier Strip.",
                    strip + 1,
                    laid.rows
                ),
            );
        }
        if laid.laid != Laid::OnRows && !laid.places.is_empty() {
            let button = ui.button(tr(MATCH_LABEL)).on_hover_text(tr(
                "Sets the UV Editor's trim rows to these strips (replacing any it has), so \
                 strips of a model put on a row with Put on trim row show that strip.",
            ));
            if button.clicked() {
                match make_rows(document, &laid) {
                    Ok(words) | Err(words) => *note = Some(words),
                }
            }
        }
        help(
            ui,
            tr(
                "On a model the sheet is laid once across the texture square, whatever its \
                 real-world size or Tiling, so the strips stay on the rows.",
            ),
        );
    }
}

/// Makes the project's trim rows match `laid`'s strips, one undo step;
/// what to tell the status bar, or why it couldn't.
pub(crate) fn make_rows(document: &mut Document, laid: &Layout) -> Result<String, String> {
    document
        .execute_labelled(
            scene::Command::SetTrim(sheet_of(laid)),
            tr("Change the trim sheet"),
            None,
        )
        .map_err(|why| why.to_string())?;
    Ok(tr(
        "The project's trim rows now match the strips. In the UV mode, put strips of a model \
         on them with UV → Lay out → Put on trim row.",
    )
    .to_string())
}

/// Where box `id`'s strips lie, from its settings, which Strips have a
/// wire in, and the project's trim `rows`.
pub(crate) fn strips_of(
    graph: &Graph,
    b: &GraphBox,
    id: BoxId,
    rows: &matgraph::TrimRows,
) -> Layout {
    let mut plugged = [false; STRIPS];
    for (i, key) in STRIP_KEYS.iter().enumerate() {
        plugged[i] = graph.wire_into(&PlugRef::new(id, *key)).is_some();
    }
    layout_of(&b.settings, &plugged, rows)
}

/// Where the strips come from, in plain words.
fn source_words(laid: &Layout) -> String {
    match laid.laid {
        Laid::OnRows => crate::trf!(
            "On the project's {} trim rows, made in the UV Editor's Trim menu: Strip 1 on the \
             top row, Strip 2 on the next.",
            laid.rows
        ),
        Laid::NoRows => tr(
            "The project has no trim rows yet, so the strips share the texture by their own \
             heights. Make these the project's trim rows to lay strips of a model on them.",
        )
        .to_string(),
        Laid::OwnHeights => tr("The strips share the texture by their own heights.").to_string(),
    }
}

/// A trim sheet with a row where each strip lies, named after it.
pub(crate) fn sheet_of(laid: &Layout) -> TrimSheet {
    TrimSheet {
        rows: laid
            .places
            .iter()
            .map(|p| TrimRow {
                name: crate::trf!("Strip {}", p.strip + 1),
                // The scene counts from the bottom.
                bottom: (1.0 - p.to).clamp(0.0, 1.0),
                top: (1.0 - p.from).clamp(0.0, 1.0),
            })
            .collect(),
    }
}
