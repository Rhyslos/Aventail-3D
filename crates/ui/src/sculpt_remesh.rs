//! Remesh for sculpting's settings and where they are shown ([decision 0312](../../../docs/decisions/0312-remesh-for-sculpting.md)):
//! the voxel size in the brush bar (with about how many polygons it
//! makes), and the Sculpt tab's Remesh section with Keep detail and Fix
//! holes.

use crate::paint_bar::{Shown, fill_field, weak};
use crate::sculpt_masks::SculptAction;
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui};
use mesh::remesh::{HEAVY_POLYGONS, MAX_SCULPT_POLYGONS};

/// Accessible name of the voxel size.
pub(crate) const VOXEL_LABEL: &str = crate::text!("Remesh voxel size");
/// Accessible name of the brush bar's Remesh button.
pub(crate) const REMESH_BUTTON: &str = crate::text!("Remesh the sculpt");
/// The voxel sizes the bar takes, in centimetres.
pub(crate) const VOXEL_RANGE: (f32, f32) = (0.05, 50.0);

/// Remesh for sculpting's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RemeshSettings {
    /// How far apart the new surface's points are, in the world's
    /// centimetres.
    pub(crate) voxel_size: f32,
    /// Lay the new surface back on the old one's detail.
    pub(crate) keep_detail: bool,
    /// Close an open shape's holes first.
    pub(crate) fix_holes: bool,
    /// About how many polygons it would make now, set by the workbench
    /// (`None` with nothing to remesh).
    pub(crate) polygons: Option<u64>,
}

impl Default for RemeshSettings {
    fn default() -> Self {
        Self {
            voxel_size: 2.0,
            keep_detail: true,
            fix_holes: true,
            polygons: None,
        }
    }
}

impl RemeshSettings {
    /// About how many polygons, in words ("about 120 thousand polygons"), and
    /// whether that is heavy.
    fn estimate(&self) -> Option<(String, bool)> {
        let polygons = self.polygons?;
        let text = if polygons >= 1_000_000 {
            crate::trf!(
                "about {} million polygons",
                format!("{:.1}", polygons as f64 / 1e6)
            )
        } else if polygons >= 1_000 {
            crate::trf!(
                "about {} thousand polygons",
                (polygons as f64 / 1e3).round()
            )
        } else {
            crate::trf!("about {} polygons", polygons)
        };
        Some((text, polygons > HEAVY_POLYGONS))
    }

    /// What the estimate warns, if anything.
    fn warning(&self) -> Option<String> {
        let polygons = self.polygons?;
        if polygons > MAX_SCULPT_POLYGONS {
            Some(crate::trf!(
                "Too small: past {} million polygons remeshing is refused. Make the voxel size \
                 bigger.",
                MAX_SCULPT_POLYGONS / 1_000_000
            ))
        } else if polygons > HEAVY_POLYGONS {
            Some(crate::trf!(
                "Heavy: more than {} million polygons are slow to sculpt and to bake. You'll be \
                 asked before it runs.",
                HEAVY_POLYGONS / 1_000_000
            ))
        } else {
            None
        }
    }
}

/// The voxel size as a bar filled to its value.
fn voxel_field(ui: &mut Ui, settings: &mut RemeshSettings) -> egui::Response {
    let response = fill_field(
        ui,
        tr(VOXEL_LABEL),
        &mut settings.voxel_size,
        VOXEL_RANGE,
        Shown {
            scale: 1.0,
            suffix: tr(" cm"),
            decimals: 2,
            eased: true,
            size: false,
        },
    );
    settings.voxel_size = settings.voxel_size.clamp(VOXEL_RANGE.0, VOXEL_RANGE.1);
    let mut help = tr(
        "Remesh for sculpting's voxel size: how far apart the new surface's points are. \
         Smaller makes more polygons, for finer detail. Shown as a grid on the shape while you \
         change it; Ctrl + R remeshes",
    )
    .to_string();
    if let Some((estimate, _)) = settings.estimate() {
        help.push_str(&crate::trf!(" (now {}).", estimate));
    }
    response.on_hover_text(help)
}

/// The brush bar's Remesh group: the voxel size, and a button.
pub(crate) fn bar_group(ui: &mut Ui, settings: &mut RemeshSettings) -> Option<SculptAction> {
    weak(ui, tr("Voxel"));
    voxel_field(ui, settings);
    let heavy = settings.estimate().is_some_and(|(_, heavy)| heavy);
    let text = if heavy {
        RichText::new(tr("Remesh")).color(theme::WARNING)
    } else {
        RichText::new(tr("Remesh"))
    };
    let mut hover = tr(SculptAction::Remesh.explanation()).to_string();
    if let Some(warning) = settings.warning() {
        hover.push(' ');
        hover.push_str(&warning);
    }
    let button = ui.add(egui::Button::new(text)).on_hover_text(hover);
    button.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tr(REMESH_BUTTON))
    });
    button.clicked().then_some(SculptAction::Remesh)
}

/// The Sculpt tab's Remesh section.
pub(crate) fn rows(ui: &mut Ui, settings: &mut RemeshSettings, asked: &mut Option<SculptAction>) {
    ui.horizontal_wrapped(|ui| {
        weak(ui, tr("Voxel size"));
        voxel_field(ui, settings);
    });
    if let Some((estimate, heavy)) = settings.estimate() {
        let colour = if heavy {
            theme::WARNING
        } else {
            theme::TEXT_WEAK
        };
        ui.label(RichText::new(crate::trf!("Makes {}.", estimate)).color(colour));
    }
    if let Some(warning) = settings.warning() {
        ui.label(RichText::new(warning).color(theme::WARNING));
    }
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut settings.keep_detail, tr("Keep detail"))
            .on_hover_text(tr(
                "Lays the new surface back on the old one, so the sculpting the voxels can hold \
                 stays sharp. Off: the new surface is a little softer",
            ));
        ui.checkbox(&mut settings.fix_holes, tr("Fix holes"))
            .on_hover_text(tr(
                "Closes an open shape's holes first, so it has an inside to remesh. Off: an open \
                 shape is refused",
            ));
    });
    let action = SculptAction::Remesh;
    if ui
        .button(tr(action.name()))
        .on_hover_text(tr(action.explanation()))
        .clicked()
    {
        *asked = Some(action);
    }
}

/// What the Remesh section says when closed: the voxel size.
pub(crate) fn aside(settings: &RemeshSettings) -> String {
    crate::trf!("{:.2} cm", settings.voxel_size)
}
