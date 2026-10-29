//! The shelf's cards and its Materials, Smart masks and Stencils tabs
//! ([decision 0229](../../../docs/decisions/0229-the-shelf.md)), as the
//! mockup draws them: a small square picture with the name under it, the
//! chosen one outlined in the accent colour.

use crate::paint_layers::{ShelfAsk, material_kind_name, smart_kind_hint, smart_kind_name};
use crate::text::tr;
use crate::theme;
use egui::{Color32, Painter, Rect, Response, RichText, Sense, Ui, vec2};
use paint::{MaterialFill, MaterialKind, SmartKind};

/// A card's size: the picture and the name under it.
pub(crate) const CARD: egui::Vec2 = vec2(76.0, 70.0);
/// The picture's side.
pub(crate) const PICTURE: f32 = 46.0;

/// One card called `name` (accessible as `label`), its picture drawn by
/// `draw` in the square it's given.
pub(crate) fn card(
    ui: &mut Ui,
    name: &str,
    label: String,
    chosen: bool,
    draw: impl FnOnce(&Painter, Rect),
) -> Response {
    let (rect, response) = ui.allocate_exact_size(CARD, Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            true,
            chosen,
            label.clone(),
        )
    });
    let painter = ui.painter_at(rect);
    let accent = theme::accent(ui.ctx()).colors();
    if response.hovered() {
        painter.rect_filled(rect, 4.0, theme::TEXT_WEAK.gamma_multiply(0.12));
    }
    let picture = Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 4.0 + PICTURE / 2.0),
        vec2(PICTURE, PICTURE),
    );
    painter.rect_filled(picture, 4.0, ui.visuals().extreme_bg_color);
    draw(&painter.with_clip_rect(picture), picture);
    let edge = if chosen {
        egui::Stroke::new(2.0, accent.light)
    } else {
        egui::Stroke::new(1.0, theme::BORDER)
    };
    painter.rect_stroke(picture, 4.0, edge, egui::StrokeKind::Outside);
    painter.text(
        egui::pos2(rect.center().x, rect.bottom() - 9.0),
        egui::Align2::CENTER_CENTER,
        name,
        egui::FontId::proportional(10.5),
        if chosen {
            theme::TEXT_STRONG
        } else {
            theme::TEXT
        },
    );
    response.on_hover_text(name)
}

/// Lays `cards` out in a row that scrolls sideways.
fn row(ui: &mut Ui, id: &str, cards: impl FnOnce(&mut Ui)) {
    egui::ScrollArea::horizontal().id_salt(id).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            cards(ui);
        });
    });
}

/// A note under the header, in the weak colour.
fn note(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::TEXT_WEAK));
}

/// The Materials tab: a card per ready-made material. A click adds a
/// material layer of that kind above the chosen layer.
pub(crate) fn materials(ui: &mut Ui, lead: impl FnOnce(&mut Ui)) -> Option<ShelfAsk> {
    ui.horizontal(|ui| {
        lead(ui);
        note(
            ui,
            tr("Click one to lay it over the object, above the chosen layer."),
        );
    });
    let mut asked = None;
    row(ui, "shelf_materials", |ui| {
        for kind in MaterialKind::ALL {
            let name = material_kind_name(kind);
            let response = card(
                ui,
                name,
                crate::trf!("{} {}", tr("Material"), name),
                false,
                |p, r| {
                    material_picture(p, r, kind);
                },
            );
            if response.clicked() {
                asked = Some(ShelfAsk::Material(kind));
            }
        }
    });
    asked
}

/// A material's picture: its two colours in bands, as its pattern.
pub(crate) fn material_picture(painter: &Painter, rect: Rect, kind: MaterialKind) {
    let fill = MaterialFill::of(kind);
    let [r, g, b, _] = fill.colour;
    let [r2, g2, b2, _] = fill.second;
    painter.rect_filled(rect, 4.0, Color32::from_rgb(r, g, b));
    let second = Color32::from_rgb(r2, g2, b2);
    match kind {
        // Fine streaks one way.
        MaterialKind::Metal => {
            for i in 0..12 {
                let y = rect.top() + 2.0 + i as f32 * 3.8;
                painter.hline(rect.x_range(), y, egui::Stroke::new(1.0, second));
            }
        }
        // Grain along it.
        MaterialKind::Wood => {
            for i in 0..6 {
                let x = rect.left() + 4.0 + i as f32 * 8.0;
                painter.line_segment(
                    [
                        egui::pos2(x, rect.top()),
                        egui::pos2(x + 4.0, rect.bottom()),
                    ],
                    egui::Stroke::new(2.0, second),
                );
            }
        }
        // Blotches.
        MaterialKind::Stone => {
            for (dx, dy, radius) in [
                (12.0, 12.0, 6.0),
                (32.0, 18.0, 8.0),
                (18.0, 34.0, 7.0),
                (36.0, 38.0, 4.0),
            ] {
                painter.circle_filled(rect.min + vec2(dx, dy), radius, second);
            }
        }
        // One colour, a faint sheen.
        MaterialKind::Paint => {
            painter.rect_filled(
                Rect::from_min_size(rect.min, vec2(rect.width(), rect.height() * 0.35)),
                4.0,
                Color32::from_white_alpha(18),
            );
        }
    }
}

/// The Smart masks tab: a card per kind. A click gives the chosen layer
/// a smart mask of that kind (or, with no layer, adds a fill shown
/// through one). The realism masks (wear, dirt, rust) show only with the
/// realism channels on, as they paint roughness and metal too.
pub(crate) fn smart_masks(
    ui: &mut Ui,
    realism: bool,
    lead: impl FnOnce(&mut Ui),
) -> Option<ShelfAsk> {
    ui.horizontal(|ui| {
        lead(ui);
        note(
            ui,
            tr("Click one to give the chosen layer that smart mask."),
        );
    });
    let mut asked = None;
    row(ui, "shelf_smart_masks", |ui| {
        for kind in SmartKind::ALL {
            if !realism && matches!(kind, SmartKind::Wear | SmartKind::Dirt | SmartKind::Rust) {
                continue;
            }
            let name = smart_kind_name(kind);
            let response = card(
                ui,
                name,
                crate::trf!("{} {}", tr("Smart mask"), name),
                false,
                |p, r| {
                    smart_picture(p, r, kind);
                },
            );
            if response.on_hover_text(smart_kind_hint(kind)).clicked() {
                asked = Some(ShelfAsk::SmartMask(kind));
            }
        }
    });
    asked
}

/// A smart mask's picture: a box with white where it would show.
pub(crate) fn smart_picture(painter: &Painter, rect: Rect, kind: SmartKind) {
    let white = Color32::from_gray(230);
    let grey = Color32::from_gray(90);
    let shape = rect.shrink(9.0);
    painter.rect_filled(shape, 2.0, grey);
    match kind {
        SmartKind::Edges => {
            painter.rect_stroke(
                shape,
                2.0,
                egui::Stroke::new(3.0, white),
                egui::StrokeKind::Inside,
            );
        }
        SmartKind::Cavities => {
            painter.vline(
                shape.center().x,
                shape.y_range(),
                egui::Stroke::new(3.0, white),
            );
            painter.hline(
                shape.x_range(),
                shape.center().y,
                egui::Stroke::new(3.0, white),
            );
        }
        SmartKind::FacingUp => {
            painter.rect_filled(
                Rect::from_min_size(shape.min, vec2(shape.width(), 8.0)),
                2.0,
                white,
            );
        }
        SmartKind::Height => {
            painter.rect_filled(
                Rect::from_min_size(shape.min, vec2(shape.width(), shape.height() * 0.55)),
                2.0,
                white,
            );
        }
        SmartKind::Wear => {
            for i in 0..5 {
                let x = shape.left() + 2.0 + i as f32 * 6.0;
                painter.line_segment(
                    [egui::pos2(x, shape.top()), egui::pos2(x + 3.0, shape.top())],
                    egui::Stroke::new(3.0, white),
                );
                painter.line_segment(
                    [
                        egui::pos2(shape.right(), x - shape.left() + shape.top()),
                        egui::pos2(shape.right(), x - shape.left() + shape.top() + 3.0),
                    ],
                    egui::Stroke::new(3.0, white),
                );
            }
        }
        SmartKind::Dirt => {
            painter.rect_filled(
                Rect::from_min_max(egui::pos2(shape.left(), shape.bottom() - 9.0), shape.max),
                2.0,
                Color32::from_rgb(120, 98, 72),
            );
        }
        SmartKind::Rust => {
            for (dx, dy, radius) in [(5.0, 5.0, 4.0), (22.0, 8.0, 3.0), (8.0, 22.0, 3.5)] {
                painter.circle_filled(
                    shape.min + vec2(dx, dy),
                    radius,
                    Color32::from_rgb(160, 72, 30),
                );
            }
        }
    }
}

/// What a click in the Stencils tab asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StencilClick {
    /// Choose a picture file.
    Choose,
    /// Paint with the picture: the projection brush.
    Use,
}

/// The Stencils tab: the projection brush's picture, if one is chosen,
/// and a card to choose one.
pub(crate) fn stencils(
    ui: &mut Ui,
    picture: Option<(&str, Option<egui::TextureId>)>,
    projecting: bool,
    lead: impl FnOnce(&mut Ui),
) -> Option<StencilClick> {
    ui.horizontal(|ui| {
        lead(ui);
        note(
            ui,
            tr("A picture laid over the views, for the projection brush (3) to paint onto the model."),
        );
    });
    let mut click = None;
    row(ui, "shelf_stencils", |ui| {
        if let Some((name, texture)) = picture {
            let response = card(
                ui,
                name,
                crate::trf!("{} {}", tr("Stencil"), name),
                projecting,
                |p, r| match texture {
                    Some(id) => {
                        p.image(
                            id,
                            r.shrink(2.0),
                            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    None => {
                        crate::icons::paint(
                            p,
                            r.shrink(12.0),
                            crate::icons::Icon::PaintProjection,
                            theme::TEXT,
                        );
                    }
                },
            );
            if response
                .on_hover_text(tr("Paint with this picture (the projection brush)"))
                .clicked()
            {
                click = Some(StencilClick::Use);
            }
        }
        let choose = card(
            ui,
            tr("Choose…"),
            tr(CHOOSE_STENCIL).to_string(),
            false,
            |p, r| {
                p.text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    "+",
                    egui::FontId::proportional(24.0),
                    theme::TEXT_WEAK,
                );
            },
        );
        if choose
            .on_hover_text(tr("Choose a picture file to paint onto the model"))
            .clicked()
        {
            click = Some(StencilClick::Choose);
        }
    });
    click
}

/// The accessible name of the Stencils tab's choose card.
pub(crate) const CHOOSE_STENCIL: &str = crate::text!("Choose a stencil picture");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_material_and_smart_mask_has_a_name_and_a_hint() {
        for kind in MaterialKind::ALL {
            assert!(!material_kind_name(kind).is_empty());
        }
        for kind in SmartKind::ALL {
            assert!(!smart_kind_name(kind).is_empty());
            assert!(!smart_kind_hint(kind).is_empty());
        }
    }
}
