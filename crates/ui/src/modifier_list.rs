//! The modifier list ([decision 0168](../../../docs/decisions/0168-modifiers-in-the-lists.md),
//! [0174](../../../docs/decisions/0174-modifiers-in-objects-only.md)):
//! an object's modifiers as rows under it in the Objects panel, first to
//! last (the order they are worked out in). Their one home: the Edit
//! layers panel lists only layers and edits.
//!
//! - **Click** a modifier: its object is selected and the Attributes panel
//!   shows only that modifier's settings. Clicking the object's own row (or
//!   anything in the viewport) goes back to the object's settings.
//! - **The tick** at the end of a row switches the modifier on or off.
//! - **Drag** a row up or down to change the order.
//! - **Delete** (or Backspace) in the Model mode removes the modifier
//!   picked (handled by the workbench).
//! - **Right-click** for Switch on/off, Duplicate, Move up, Move down,
//!   Bake into shape up to here and Remove.
//!
//! Every change is a command, so Ctrl + Z takes it back.

use crate::icons;
use crate::objects::DocRequest;
use crate::text::tr;
use crate::theme;
use egui::{
    Align2, CursorIcon, FontId, Rect, Sense, Stroke, Ui, WidgetInfo, WidgetType, pos2, vec2,
};
use scene::{Command, Document, MAX_MODIFIERS, Modifier, ModifierId, ObjectId};

/// The modifier picked in a list, by its object and its id (which stays the
/// same as modifiers are added, moved or removed around it).
pub(crate) type Picked = Option<(ObjectId, ModifierId)>;

/// A modifier row's height, as an object's.
const ROW_HEIGHT: f32 = 22.0;

/// The accessible name of a modifier's row starts with this (then its
/// name, such as "Array").
pub(crate) const ROW_PREFIX: &str = crate::text!("Modifier: ");
/// The accessible name of a modifier's tick starts with this.
pub(crate) const TICK_PREFIX: &str = crate::text!("Switch on or off: ");
/// The right-click menu's entry that bakes up to a modifier.
pub(crate) const BAKE_UP_TO: &str = crate::text!("Bake into shape up to here");
/// The right-click menu's entry that copies a modifier.
pub(crate) const DUPLICATE: &str = crate::text!("Duplicate");

/// The modifier picked, as its object and its place in the object's list,
/// if it is still there and its object is the only one selected (a
/// selection of anything else puts the pick away).
pub(crate) fn picked_place(doc: &Document, picked: Picked) -> Option<(ObjectId, usize)> {
    let (id, modifier) = picked?;
    if doc.selection() != [id] {
        return None;
    }
    let index = doc
        .scene()
        .modifiers(id)
        .iter()
        .position(|m| m.id == modifier)?;
    Some((id, index))
}

/// Picks the modifier at `index` of `id`, selecting `id` alone first if it
/// isn't already.
fn pick(
    doc: &Document,
    id: ObjectId,
    modifier: ModifierId,
    picked: &mut Picked,
    out: &mut Vec<DocRequest>,
) {
    if doc.selection() != [id] {
        out.push(DocRequest::Select(vec![id]));
    }
    *picked = Some((id, modifier));
}

/// A list's own state: the row being dragged.
#[derive(Debug, Clone, Default)]
pub(crate) struct ModifierRows {
    /// The object and the place of the modifier being dragged.
    dragging: Option<(ObjectId, usize)>,
}

impl ModifierRows {
    /// Draws `id`'s modifiers, a row each, their icons `indent` from the
    /// left, and handles clicks, ticks, drags and right-clicks.
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        id: ObjectId,
        indent: f32,
        picked: &mut Picked,
        out: &mut Vec<DocRequest>,
    ) {
        let modifiers = doc.scene().modifiers(id).to_vec();
        if modifiers.is_empty() {
            return;
        }
        let problems = doc.scene().modifier_problems(id);
        let chosen = picked_place(doc, *picked);
        let mut rects = Vec::with_capacity(modifiers.len());
        for (index, modifier) in modifiers.iter().enumerate() {
            let problem = problems
                .iter()
                .find(|(i, _)| *i == index)
                .map(|(_, why)| why.as_str());
            let rect = self.row(
                ui,
                doc,
                (id, index),
                modifier,
                modifiers.len(),
                indent,
                chosen == Some((id, index)),
                problem,
                picked,
                out,
            );
            rects.push(rect);
        }
        self.follow_drag(ui, id, &rects, out);
    }

    /// One modifier's row: icon, name, a mark if it can't do its work, and
    /// its tick. Returns where it was drawn.
    #[allow(clippy::too_many_arguments)]
    fn row(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        (id, index): (ObjectId, usize),
        modifier: &Modifier,
        count: usize,
        indent: f32,
        chosen: bool,
        problem: Option<&str>,
        picked: &mut Picked,
        out: &mut Vec<DocRequest>,
    ) -> Rect {
        let name = modifier.kind.name();
        let (rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
        // A fixed id (the modifier's own), so a drag or a double-click is
        // followed however the rows around it change.
        let response = ui.interact(
            rect,
            ui.id().with(("modifier row", id, modifier.id)),
            Sense::click_and_drag(),
        );
        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::SelectableLabel,
                true,
                chosen,
                format!("{}{}", tr(ROW_PREFIX), name),
            )
        });
        let painter = ui.painter_at(rect);
        let accent = theme::accent(ui.ctx()).colors();
        if chosen {
            painter.rect_filled(rect, 3.0, accent.fill);
        } else if response.hovered() && self.dragging.is_none() {
            painter.rect_filled(rect, 3.0, theme::HOVER.gamma_multiply(0.6));
        }

        // The kind's icon, as on the modifiers palette.
        let icon = crate::modifiers::kind_icon(&modifier.kind);
        let icon_rect = Rect::from_min_size(
            pos2(rect.left() + indent, rect.center().y - 7.5),
            vec2(15.0, 15.0),
        );
        let tint = crate::palette::icon_tint(icon);
        icons::paint(
            &painter,
            icon_rect,
            icon,
            if modifier.on {
                tint
            } else {
                tint.gamma_multiply(0.45)
            },
        );

        // The tick, on the right.
        let tick =
            Rect::from_center_size(pos2(rect.right() - 14.0, rect.center().y), vec2(20.0, 18.0));
        let tick_response = ui.interact(
            tick,
            ui.id().with(("modifier tick", id, modifier.id)),
            Sense::click(),
        );
        tick_response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Checkbox,
                true,
                modifier.on,
                format!("{}{}", tr(TICK_PREFIX), name),
            )
        });
        let tick_color = if tick_response.hovered() {
            theme::TEXT_STRONG
        } else {
            theme::TEXT_WEAK
        };
        paint_tick(&painter, tick.center(), modifier.on, tick_color);
        let tip = if modifier.on {
            crate::trf!(
                "Switch {} off: see the shape without it (it keeps its settings)",
                name
            )
        } else {
            crate::trf!("Switch {} on", name)
        };
        if tick_response.on_hover_text(tip).clicked() {
            out.push(DocRequest::Run(Command::ChangeModifier {
                id,
                index,
                modifier: Modifier {
                    on: !modifier.on,
                    ..modifier.clone()
                },
            }));
        }

        // The name, and a mark when it can't do its work.
        let text_left = icon_rect.right() + 7.0;
        let color = match (chosen, modifier.on) {
            (true, _) => theme::TEXT_STRONG,
            (false, true) => theme::TEXT,
            (false, false) => theme::TEXT_WEAK.gamma_multiply(0.7),
        };
        let name_painter = ui.painter_at(Rect::from_min_max(
            pos2(text_left, rect.top()),
            pos2(tick.left() - 4.0, rect.bottom()),
        ));
        let written = name_painter.text(
            pos2(text_left, rect.center().y),
            Align2::LEFT_CENTER,
            name,
            FontId::proportional(13.0),
            color,
        );
        let mut response = response;
        if let Some(why) = problem {
            let calm = scene::has_nothing_to_do(why);
            let mark = pos2(written.right() + 8.0, rect.center().y);
            if mark.x < tick.left() - 4.0 {
                let fill = if calm {
                    theme::TEXT_WEAK
                } else {
                    theme::WARNING
                };
                painter.circle_filled(mark, 3.5, fill);
            }
            response = response.on_hover_text(if calm {
                crate::trf!("Nothing to do: {}.", why)
            } else {
                crate::trf!("Not worked out: {}.", why)
            });
        } else {
            response = response.on_hover_text(tr(
                "Click to see its settings in the Attributes panel. Drag to change the order; \
                 right-click for more.",
            ));
        }

        if response.clicked() || response.secondary_clicked() {
            pick(doc, id, modifier.id, picked, out);
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            pick(doc, id, modifier.id, picked, out);
            self.dragging = Some((id, index));
        }
        response.context_menu(|ui| {
            ui.set_min_width(crate::dialog::MENU_WIDTH);
            menu_entries(ui, id, index, modifier, count, out);
        });
        rect
    }

    /// While a row is dragged: a line where it would land, and the move
    /// when the mouse button is let go.
    fn follow_drag(&mut self, ui: &Ui, id: ObjectId, rects: &[Rect], out: &mut Vec<DocRequest>) {
        let Some((dragged_id, from)) = self.dragging else {
            return;
        };
        if dragged_id != id {
            return;
        }
        let Some(at) = ui.input(|i| i.pointer.latest_pos()) else {
            return;
        };
        // The gap it goes into: before the first row whose middle is below
        // the mouse, or after the last.
        let gap = rects
            .iter()
            .position(|r| at.y < r.center().y)
            .unwrap_or(rects.len());
        let to = if gap > from { gap - 1 } else { gap };
        let line_y = match rects.get(gap) {
            Some(r) => r.top(),
            None => rects.last().map_or(at.y, |r| r.bottom()),
        };
        if let (Some(first), Some(last)) = (rects.first(), rects.last()) {
            let accent = theme::accent(ui.ctx()).colors();
            if to != from {
                ui.painter().hline(
                    first.x_range(),
                    line_y.clamp(first.top(), last.bottom()),
                    Stroke::new(2.0, accent.light),
                );
            }
        }
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        if ui.input(|i| !i.pointer.any_down()) {
            self.dragging = None;
            if to != from {
                out.push(DocRequest::Run(Command::MoveModifier { id, from, to }));
            }
        }
    }
}

/// A small tick box: ticked when the modifier is on.
fn paint_tick(painter: &egui::Painter, centre: egui::Pos2, on: bool, color: egui::Color32) {
    let size = 12.0;
    let box_rect = Rect::from_center_size(centre, vec2(size, size));
    painter.rect_stroke(
        box_rect,
        2.0,
        Stroke::new(1.0, color),
        egui::StrokeKind::Inside,
    );
    if on {
        let points = [
            pos2(box_rect.left() + 2.5, centre.y + 0.2),
            pos2(centre.x - 1.0, box_rect.bottom() - 3.0),
            pos2(box_rect.right() - 2.5, box_rect.top() + 3.0),
        ];
        painter.line(points.to_vec(), Stroke::new(1.6, color));
    }
}

/// A modifier's right-click menu.
fn menu_entries(
    ui: &mut Ui,
    id: ObjectId,
    index: usize,
    modifier: &Modifier,
    count: usize,
    out: &mut Vec<DocRequest>,
) {
    let entry = |ui: &mut Ui, name: &str, shortcut: &str, enabled: bool, help: &str| {
        let button = if shortcut.is_empty() {
            egui::Button::new(name)
        } else {
            egui::Button::new(name).shortcut_text(shortcut)
        };
        ui.add_enabled(enabled, button)
            .on_hover_text(help)
            .on_disabled_hover_text(help)
            .clicked()
    };
    let (word, help) = if modifier.on {
        (
            tr("Switch off"),
            tr("See the shape without it; it keeps its settings"),
        )
    } else {
        (tr("Switch on"), tr("Bring its work back"))
    };
    if entry(ui, word, "", true, help) {
        out.push(DocRequest::Run(Command::ChangeModifier {
            id,
            index,
            modifier: Modifier {
                on: !modifier.on,
                ..modifier.clone()
            },
        }));
        ui.close();
    }
    ui.separator();
    let room = count < MAX_MODIFIERS;
    let duplicate_help = if room {
        tr("Put a copy with the same settings just after it")
    } else {
        tr("A shape can have at most 32 modifiers")
    };
    if entry(ui, tr(DUPLICATE), "", room, duplicate_help) {
        out.push(DocRequest::Run(Command::DuplicateModifier { id, index }));
        ui.close();
    }
    if entry(
        ui,
        tr("Move up"),
        "",
        index > 0,
        tr("Work it out before the one above it"),
    ) {
        out.push(DocRequest::Run(Command::MoveModifier {
            id,
            from: index,
            to: index - 1,
        }));
        ui.close();
    }
    if entry(
        ui,
        tr("Move down"),
        "",
        index + 1 < count,
        tr("Work it out after the one below it"),
    ) {
        out.push(DocRequest::Run(Command::MoveModifier {
            id,
            from: index,
            to: index + 1,
        }));
        ui.close();
    }
    ui.separator();
    if entry(
        ui,
        tr(BAKE_UP_TO),
        "",
        true,
        tr(
            "Make this modifier and those above it part of the shape for good (those switched \
             off stay); those below it stay, worked out on the new shape. Ctrl + E bakes them all.",
        ),
    ) {
        out.push(DocRequest::Run(Command::BakeModifiersUpTo { id, index }));
        ui.close();
    }
    ui.separator();
    if entry(
        ui,
        tr("Remove"),
        tr("Delete"),
        true,
        tr("Take this modifier away (Ctrl + Z brings it back)"),
    ) {
        out.push(DocRequest::Run(Command::RemoveModifier { id, index }));
        ui.close();
    }
}
