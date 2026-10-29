//! Folding sections for the Paint tab ([decision 0228](../../../../docs/decisions/0228-paint-tab-sections.md)),
//! as the mockup draws them: a line across, an arrow, the section's name,
//! and on the right a few words on what's set inside, so a closed section
//! still says what it holds.

use crate::text::tr;
use crate::theme;
use egui::{Sense, Ui, vec2};

/// Height of a section's header.
const HEADER: f32 = 26.0;

/// A section called `title` (an English text, translated here), open to
/// start with when `open`, with `aside` on the right; `body` draws what's
/// inside while it's open. Its open state is kept by `id`.
pub(crate) fn fold(
    ui: &mut Ui,
    id: &str,
    title: &'static str,
    aside: &str,
    open: bool,
    body: impl FnOnce(&mut Ui),
) {
    let id = ui.make_persistent_id(("paint_fold", id));
    let mut state =
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, open);
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), HEADER), Sense::click());
    let shown = state.is_open();
    let title = tr(title);
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::CollapsingHeader, true, shown, title)
    });
    let painter = ui.painter();
    painter.hline(
        rect.x_range(),
        rect.top() + 0.5,
        egui::Stroke::new(1.0, theme::BORDER),
    );
    let strong = if response.hovered() {
        theme::TEXT_STRONG
    } else {
        theme::TEXT
    };
    arrow(
        painter,
        egui::pos2(rect.left() + 6.0, rect.center().y),
        shown,
        theme::TEXT_WEAK,
    );
    let name = painter.text(
        egui::pos2(rect.left() + 16.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(13.0),
        strong,
    );
    if !aside.is_empty() {
        let space =
            egui::Rect::from_x_y_ranges(name.right() + 12.0..=rect.right() - 4.0, rect.y_range());
        painter.with_clip_rect(space).text(
            egui::pos2(rect.right() - 4.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            aside,
            egui::FontId::proportional(11.0),
            theme::TEXT_WEAK,
        );
    }
    if response.clicked() {
        state.toggle(ui);
    }
    state.show_body_unindented(ui, |ui| {
        ui.add_space(2.0);
        body(ui);
        ui.add_space(6.0);
    });
}

/// A small arrow at `centre`, drawn (the interface font has no arrow
/// glyphs): pointing down when `open`, right when not.
pub(crate) fn arrow(
    painter: &egui::Painter,
    centre: egui::Pos2,
    open: bool,
    colour: egui::Color32,
) {
    let c = centre;
    let points = if open {
        vec![
            c + vec2(-4.0, -2.0),
            c + vec2(4.0, -2.0),
            c + vec2(0.0, 3.0),
        ]
    } else {
        vec![
            c + vec2(-2.0, -4.0),
            c + vec2(3.0, 0.0),
            c + vec2(-2.0, 4.0),
        ]
    };
    painter.add(egui::Shape::convex_polygon(
        points,
        colour,
        egui::Stroke::NONE,
    ));
}
