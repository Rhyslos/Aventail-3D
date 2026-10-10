//! The Object Manager: the tree of objects in the scene. Click to select
//! (Ctrl adds or removes, Shift selects a run), double-click a name to
//! rename it, click the eye to show or hide, drag rows to nest or reorder,
//! and right-click for the object commands.
//!
//! The panel never changes the scene itself: it hands back
//! [`DocRequest`]s (commands, undo, redo) for the workbench to carry out.
//! Only the selection, which is not an undo step yet, is changed directly.

use crate::icons::{self, Icon};
use crate::theme::{self, tint};
use egui::{
    Align2, Color32, CursorIcon, FontId, Key, KeyboardShortcut, Modifiers, Rect, Sense, Stroke, Ui,
    WidgetInfo, WidgetType, pos2, vec2,
};
use scene::{Command, Document, ObjectId, ObjectKind};
use std::collections::BTreeSet;

/// Height of one row of the tree, in logical pixels.
pub(crate) const ROW_HEIGHT: f32 = 22.0;
/// How far each level of nesting is indented.
const INDENT: f32 = 14.0;

/// Undo (Ctrl+Z, Cmd+Z on macOS), as in Halberd.
pub const UNDO_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Z);
/// Redo (Ctrl+Y), as in Halberd.
pub const REDO_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Y);
/// Redo's second shortcut (Ctrl+Shift+Z), as in Halberd.
pub const REDO_SHORTCUT_2: KeyboardShortcut = KeyboardShortcut::new(
    Modifiers {
        shift: true,
        ..Modifiers::COMMAND
    },
    Key::Z,
);
/// Delete the selection (Backspace does it too), as in Halberd.
pub const DELETE_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);
/// Deselect everything, as in Halberd.
pub const DESELECT_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::Escape);
/// Put the selection into a new group (new here; free in Halberd).
pub const GROUP_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::G);
/// Take the contents out of the selected groups (new here; free in
/// Halberd).
pub const UNGROUP_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::SHIFT, Key::G);

/// Something the Object Manager (or a menu) asks the workbench to do to
/// the document.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DocRequest {
    /// Carry out a command.
    Run(Command),
    /// Take back the last step.
    Undo,
    /// Do the last undone step again.
    Redo,
}

/// A name being typed in place of an object's name.
#[derive(Debug, Clone, PartialEq)]
struct Renaming {
    id: ObjectId,
    text: String,
    /// The text box still needs to take the keyboard and select its text.
    just_started: bool,
}

/// Where dragged rows would land if let go now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DropAt {
    /// Just above this object, beside it.
    Before(ObjectId),
    /// Inside this object, after what is already there.
    Into(ObjectId),
    /// Just below this object, beside it.
    After(ObjectId),
    /// At the bottom of the tree, at the top level.
    End,
}

/// The Object Manager's own state: what is folded away, a name being
/// typed, and rows being dragged.
#[derive(Debug, Clone, Default)]
pub(crate) struct ObjectManager {
    renaming: Option<Renaming>,
    /// Objects whose contents are folded away (everything starts open).
    folded: BTreeSet<ObjectId>,
    /// Where a Shift+click run of selection starts.
    anchor: Option<ObjectId>,
    /// The objects being dragged, if a drag is under way.
    dragging: Option<Vec<ObjectId>>,
}

impl ObjectManager {
    /// Starts typing a new name for `id` (double-click, or Rename).
    pub(crate) fn start_rename(&mut self, doc: &Document, id: ObjectId) {
        if let Some(object) = doc.scene().object(id) {
            self.renaming = Some(Renaming {
                id,
                text: object.name.clone(),
                just_started: true,
            });
        }
    }

    /// Whether a name is being typed.
    pub(crate) fn is_renaming(&self) -> bool {
        self.renaming.is_some()
    }

    /// Opens every object's contents (View → Expand all).
    pub(crate) fn expand_all(&mut self) {
        self.folded.clear();
    }

    /// Folds away every object's contents (View → Collapse all).
    pub(crate) fn collapse_all(&mut self, doc: &Document) {
        let scene = doc.scene();
        self.folded = scene
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| !scene.children(Some(*id)).is_empty())
            .collect();
    }

    /// The rows shown: every object, except those inside a folded one,
    /// with their depth.
    fn rows(&self, doc: &Document) -> Vec<(ObjectId, usize)> {
        let scene = doc.scene();
        let mut rows = Vec::new();
        let mut hidden_below: Option<usize> = None;
        for (id, depth) in scene.walk() {
            match hidden_below {
                Some(level) if depth > level => continue,
                _ => hidden_below = None,
            }
            rows.push((id, depth));
            if self.folded.contains(&id) && !scene.children(Some(id)).is_empty() {
                hidden_below = Some(depth);
            }
        }
        rows
    }

    /// Draws the tree into `ui` and handles clicks, renaming and dragging.
    pub(crate) fn show(&mut self, ui: &mut Ui, doc: &mut Document, out: &mut Vec<DocRequest>) {
        let scene = doc.scene();
        self.folded.retain(|id| scene.contains(*id));
        if self
            .renaming
            .as_ref()
            .is_some_and(|r| !scene.contains(r.id))
        {
            self.renaming = None;
        }
        if scene.is_empty() {
            crate::panels::placeholder(
                ui,
                "No objects yet",
                "Create a group with Create → Group. Primitives such as cubes arrive later in Phase 2.",
            );
            return;
        }
        let rows = self.rows(doc);
        let mut row_rects = Vec::with_capacity(rows.len());
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for (id, depth) in &rows {
                    let rect = self.row(ui, doc, *id, *depth, &rows, out);
                    row_rects.push((Some(*id), rect));
                }
                // The empty space below the rows: click to select nothing,
                // or drop dragged rows there to move them to the top level.
                let rest = ui.available_size().max(vec2(0.0, ROW_HEIGHT));
                let (below, background) = ui.allocate_exact_size(rest, Sense::click());
                background.widget_info(|| {
                    WidgetInfo::labeled(WidgetType::Other, true, "Empty space below the objects")
                });
                if background.clicked() {
                    doc.clear_selection();
                    self.anchor = None;
                }
                row_rects.push((None, below));
            });
        self.follow_drag(ui, doc, &row_rects, out);
    }

    /// One row: fold arrow, icon, name and eye. Returns where it was drawn.
    fn row(
        &mut self,
        ui: &mut Ui,
        doc: &mut Document,
        id: ObjectId,
        depth: usize,
        rows: &[(ObjectId, usize)],
        out: &mut Vec<DocRequest>,
    ) -> Rect {
        let scene = doc.scene();
        let Some(object) = scene.object(id).cloned() else {
            return Rect::NOTHING;
        };
        let selected = doc.is_selected(id);
        let shown = scene.is_shown(id);
        let has_children = !scene.children(Some(id)).is_empty();
        let (rect, response) = ui.allocate_exact_size(
            vec2(ui.available_width(), ROW_HEIGHT),
            Sense::click_and_drag(),
        );
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, &object.name)
        });

        let painter = ui.painter_at(rect);
        let accent = theme::accent(ui.ctx()).colors();
        if selected {
            painter.rect_filled(rect, 3.0, accent.soft);
        } else if response.hovered() && self.dragging.is_none() {
            painter.rect_filled(rect, 3.0, theme::HOVER.gamma_multiply(0.6));
        }

        let mut x = rect.left() + 4.0 + depth as f32 * INDENT;
        // Fold arrow, for objects with something inside.
        let arrow = Rect::from_min_size(pos2(x, rect.center().y - 7.0), vec2(14.0, 14.0));
        if has_children {
            let folded = self.folded.contains(&id);
            let toggle = ui.interact(arrow, ui.id().with(("fold", id)), Sense::click());
            let label = if folded { "Unfold" } else { "Fold" };
            toggle.widget_info(|| {
                WidgetInfo::labeled(WidgetType::Button, true, format!("{label} {}", object.name))
            });
            let color = if toggle.hovered() {
                theme::TEXT_STRONG
            } else {
                theme::TEXT_WEAK
            };
            fold_arrow(&painter, arrow, folded, color);
            if toggle.clicked() {
                if folded {
                    self.folded.remove(&id);
                } else {
                    self.folded.insert(id);
                }
            }
        }
        x += 16.0;

        // The kind's icon.
        let (icon, icon_tint) = match object.kind {
            ObjectKind::TestCube => (Icon::Cube, tint::SHAPE),
            _ => (Icon::Null, tint::ICON),
        };
        let icon_rect = Rect::from_min_size(pos2(x, rect.center().y - 7.5), vec2(15.0, 15.0));
        let icon_color = if shown {
            icon_tint
        } else {
            icon_tint.gamma_multiply(0.45)
        };
        icons::paint(&painter, icon_rect, icon, icon_color);
        x += 22.0;

        // The eye, on the right.
        let eye =
            Rect::from_center_size(pos2(rect.right() - 14.0, rect.center().y), vec2(20.0, 18.0));
        let eye_response = ui.interact(eye, ui.id().with(("eye", id)), Sense::click());
        let eye_label = if object.visible { "Hide" } else { "Show" };
        eye_response.widget_info(|| {
            WidgetInfo::labeled(
                WidgetType::Button,
                true,
                format!("{eye_label} {}", object.name),
            )
        });
        let eye_icon = if object.visible {
            Icon::Shown
        } else {
            Icon::Hidden
        };
        let eye_color = match (eye_response.hovered(), object.visible, shown) {
            (true, _, _) => theme::TEXT_STRONG,
            // On, but a parent is hidden: faint.
            (_, true, false) => theme::TEXT_WEAK.gamma_multiply(0.45),
            (_, true, true) => theme::TEXT_WEAK,
            (_, false, _) => theme::TEXT_WEAK.gamma_multiply(0.75),
        };
        icons::paint(
            &painter,
            Rect::from_center_size(eye.center(), vec2(15.0, 15.0)),
            eye_icon,
            eye_color,
        );
        let tip = if object.visible && !shown {
            format!(
                "{eye_label} {}\n(It is hidden now because something it is inside is hidden.)",
                object.name
            )
        } else {
            format!("{eye_label} {} in the viewport", object.name)
        };
        let eye_clicked = eye_response.on_hover_text(tip).clicked();
        if eye_clicked {
            // The whole selection, if this row is part of it.
            let ids = if selected { doc.selection() } else { vec![id] };
            out.push(DocRequest::Run(Command::SetVisible {
                ids,
                visible: !object.visible,
            }));
        }

        // The name, or a box to type a new one in.
        let name_rect = Rect::from_min_max(
            pos2(x, rect.top() + 1.0),
            pos2((eye.left() - 6.0).max(x + 20.0), rect.bottom() - 1.0),
        );
        let renaming_here = self.renaming.as_ref().is_some_and(|r| r.id == id);
        if renaming_here {
            self.rename_box(ui, name_rect, out);
        } else {
            let color = if selected {
                theme::SELECTED_NAME
            } else if shown {
                theme::TEXT
            } else {
                theme::TEXT_WEAK.gamma_multiply(0.7)
            };
            let name_painter = ui.painter_at(name_rect);
            name_painter.text(
                pos2(name_rect.left(), rect.center().y),
                Align2::LEFT_CENTER,
                &object.name,
                FontId::proportional(13.0),
                color,
            );
        }

        // Clicks on the row itself (the arrow and eye take their own).
        if !renaming_here {
            if response.double_clicked() {
                doc.set_selection([id]);
                self.start_rename(doc, id);
            } else if response.clicked() {
                self.click(ui, doc, id, rows);
            } else if response.secondary_clicked() && !selected {
                doc.set_selection([id]);
                self.anchor = Some(id);
            }
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            let ids = if doc.is_selected(id) {
                doc.selection()
            } else {
                doc.set_selection([id]);
                vec![id]
            };
            self.dragging = Some(ids);
        }
        response.context_menu(|ui| {
            context_menu(ui, doc, self, out);
        });
        rect
    }

    /// A left click on a row: select it alone, add or remove it (Ctrl), or
    /// select the run from the last clicked row to it (Shift).
    fn click(&mut self, ui: &Ui, doc: &mut Document, id: ObjectId, rows: &[(ObjectId, usize)]) {
        let modifiers = ui.input(|i| i.modifiers);
        if modifiers.shift
            && let Some(anchor) = self.anchor
        {
            let position = |target| rows.iter().position(|(r, _)| *r == target);
            if let (Some(a), Some(b)) = (position(anchor), position(id)) {
                let (from, to) = (a.min(b), a.max(b));
                let run = rows[from..=to].iter().map(|(r, _)| *r);
                if modifiers.command {
                    let mut ids = doc.selection();
                    ids.extend(run);
                    doc.set_selection(ids);
                } else {
                    doc.set_selection(run);
                }
                return;
            }
        }
        if modifiers.command {
            doc.toggle_selected(id);
        } else {
            doc.set_selection([id]);
        }
        self.anchor = Some(id);
    }

    /// The text box for a new name. Enter (or clicking elsewhere) keeps
    /// the name; Esc goes back to the old one.
    fn rename_box(&mut self, ui: &mut Ui, rect: Rect, out: &mut Vec<DocRequest>) {
        let Some(renaming) = self.renaming.as_mut() else {
            return;
        };
        let box_id = ui.id().with(("rename", renaming.id));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        let output = egui::TextEdit::singleline(&mut renaming.text)
            .id(box_id)
            .desired_width(rect.width())
            .margin(vec2(4.0, 1.0))
            .show(&mut child);
        let response = output.response;
        response.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, "New name"));
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            // Esc: keep the old name. (The box gives up the keyboard on
            // Esc, but only reports it a frame later.)
            self.renaming = None;
            ui.memory_mut(|m| m.surrender_focus(box_id));
            return;
        }
        if renaming.just_started {
            renaming.just_started = false;
            response.request_focus();
            let mut state = output.state;
            let all = egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(renaming.text.chars().count()),
            );
            state.cursor.set_char_range(Some(all));
            state.store(ui.ctx(), box_id);
        } else if response.lost_focus() {
            out.push(DocRequest::Run(Command::Rename {
                id: renaming.id,
                name: renaming.text.clone(),
            }));
            self.renaming = None;
        }
    }

    /// While rows are dragged: shows where they would land, and moves them
    /// there when the mouse button is let go.
    fn follow_drag(
        &mut self,
        ui: &Ui,
        doc: &Document,
        rows: &[(Option<ObjectId>, Rect)],
        out: &mut Vec<DocRequest>,
    ) {
        let Some(dragged) = self.dragging.clone() else {
            return;
        };
        let pointer = ui.input(|i| i.pointer.latest_pos());
        let target = pointer.and_then(|at| drop_target(doc, &dragged, rows, at));
        if let Some((target, rect)) = target {
            let accent = theme::accent(ui.ctx()).colors();
            let painter = ui.painter();
            let stroke = Stroke::new(2.0, accent.light);
            match target {
                DropAt::Before(_) => {
                    painter.hline(rect.x_range(), rect.top(), stroke);
                }
                DropAt::After(_) | DropAt::End => {
                    painter.hline(rect.x_range(), rect.bottom(), stroke);
                }
                DropAt::Into(_) => {
                    painter.rect_stroke(rect.shrink(1.0), 3.0, stroke, egui::StrokeKind::Inside);
                }
            }
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        } else {
            ui.ctx().set_cursor_icon(CursorIcon::NoDrop);
        }
        if ui.input(|i| !i.pointer.any_down()) {
            self.dragging = None;
            if let Some((target, _)) = target
                && let Some(command) = move_command(doc, dragged, target)
            {
                if let DropAt::Into(parent) = target {
                    self.folded.remove(&parent);
                }
                out.push(DocRequest::Run(command));
            }
        }
    }
}

/// Where rows dragged to `at` would land, with the row it refers to (for
/// drawing), or `None` where they can't go (onto themselves, or into
/// something inside them).
fn drop_target(
    doc: &Document,
    dragged: &[ObjectId],
    rows: &[(Option<ObjectId>, Rect)],
    at: egui::Pos2,
) -> Option<(DropAt, Rect)> {
    let scene = doc.scene();
    let (row, rect) = rows.iter().find(|(_, r)| r.contains(at)).copied()?;
    let Some(id) = row else {
        // The empty space below: the line goes under the last row.
        let last = rows.iter().rev().find(|(r, _)| r.is_some())?;
        return Some((DropAt::End, last.1));
    };
    if dragged.iter().any(|d| scene.is_inside(id, *d)) {
        return None;
    }
    let share = (at.y - rect.top()) / rect.height().max(1.0);
    let target = if share < 0.25 {
        DropAt::Before(id)
    } else if share > 0.75 {
        DropAt::After(id)
    } else {
        DropAt::Into(id)
    };
    Some((target, rect))
}

/// The command that moves `ids` to `target`.
fn move_command(doc: &Document, ids: Vec<ObjectId>, target: DropAt) -> Option<Command> {
    let scene = doc.scene();
    let (parent, index) = match target {
        DropAt::Before(id) => {
            let place = scene.place(id)?;
            (place.parent, place.index)
        }
        DropAt::After(id) => {
            let place = scene.place(id)?;
            (place.parent, place.index + 1)
        }
        DropAt::Into(id) => (Some(id), scene.children(Some(id)).len()),
        DropAt::End => (None, scene.children(None).len()),
    };
    Some(Command::Move { ids, parent, index })
}

/// A small triangle: pointing right when folded, down when open.
fn fold_arrow(painter: &egui::Painter, rect: Rect, folded: bool, color: Color32) {
    let c = rect.center();
    let points = if folded {
        vec![
            c + vec2(-2.5, -4.0),
            c + vec2(3.0, 0.0),
            c + vec2(-2.5, 4.0),
        ]
    } else {
        vec![
            c + vec2(-4.0, -2.5),
            c + vec2(4.0, -2.5),
            c + vec2(0.0, 3.0),
        ]
    };
    painter.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
}

/// An entry with its shortcut written beside it.
fn entry(ui: &mut Ui, label: &str, shortcut: Option<&KeyboardShortcut>, enabled: bool) -> bool {
    let mut button = egui::Button::new(label);
    if let Some(shortcut) = shortcut {
        button = button.shortcut_text(ui.ctx().format_shortcut(shortcut));
    }
    let clicked = ui.add_enabled(enabled, button).clicked();
    if clicked {
        ui.close();
    }
    clicked
}

/// Undo and Redo, naming what they would do.
pub(crate) fn undo_entries(ui: &mut Ui, doc: &Document, out: &mut Vec<DocRequest>) {
    let undo = doc
        .undo_label()
        .map_or("Undo".to_string(), |l| format!("Undo: {l}"));
    if entry(ui, &undo, Some(&UNDO_SHORTCUT), doc.undo_label().is_some()) {
        out.push(DocRequest::Undo);
    }
    let redo = doc
        .redo_label()
        .map_or("Redo".to_string(), |l| format!("Redo: {l}"));
    if entry(ui, &redo, Some(&REDO_SHORTCUT), doc.redo_label().is_some()) {
        out.push(DocRequest::Redo);
    }
}

/// Rename, Delete, Select all and Deselect.
pub(crate) fn selection_entries(
    ui: &mut Ui,
    doc: &mut Document,
    manager: &mut ObjectManager,
    out: &mut Vec<DocRequest>,
) {
    let selection = doc.selection();
    let single = (selection.len() == 1).then(|| selection[0]);
    if entry(ui, "Rename", None, single.is_some())
        && let Some(id) = single
    {
        manager.start_rename(doc, id);
    }
    if entry(ui, "Delete", Some(&DELETE_SHORTCUT), !selection.is_empty()) {
        out.push(DocRequest::Run(Command::Delete(selection.clone())));
    }
    ui.separator();
    if entry(ui, "Select all", None, !doc.scene().is_empty()) {
        let all: Vec<ObjectId> = doc.scene().walk().into_iter().map(|(id, _)| id).collect();
        doc.set_selection(all);
    }
    if entry(
        ui,
        "Deselect",
        Some(&DESELECT_SHORTCUT),
        !selection.is_empty(),
    ) {
        doc.clear_selection();
    }
}

/// New group, Group objects, Ungroup, Show and Hide.
pub(crate) fn object_entries(ui: &mut Ui, doc: &Document, out: &mut Vec<DocRequest>) {
    let selection = doc.selection();
    let any = !selection.is_empty();
    if entry(ui, "New group", None, true) {
        out.push(DocRequest::Run(Command::Create(ObjectKind::Group)));
    }
    if entry(ui, "Group objects", Some(&GROUP_SHORTCUT), any) {
        out.push(DocRequest::Run(Command::Group(selection.clone())));
    }
    let has_contents = selection
        .iter()
        .any(|id| !doc.scene().children(Some(*id)).is_empty());
    if entry(ui, "Ungroup", Some(&UNGROUP_SHORTCUT), has_contents) {
        out.push(DocRequest::Run(Command::Ungroup(selection.clone())));
    }
    ui.separator();
    for (label, visible) in [("Show", true), ("Hide", false)] {
        if entry(ui, label, None, any) {
            out.push(DocRequest::Run(Command::SetVisible {
                ids: selection.clone(),
                visible,
            }));
        }
    }
}

/// The right-click menu on a row.
fn context_menu(
    ui: &mut Ui,
    doc: &mut Document,
    manager: &mut ObjectManager,
    out: &mut Vec<DocRequest>,
) {
    selection_entries(ui, doc, manager, out);
    ui.separator();
    object_entries(ui, doc, out);
}

/// The Object Manager's menu line: Edit, View and Object.
pub(crate) fn menus(
    ui: &mut Ui,
    doc: &mut Document,
    manager: &mut ObjectManager,
    out: &mut Vec<DocRequest>,
) {
    ui.menu_button("Edit", |ui| {
        undo_entries(ui, doc, out);
        ui.separator();
        selection_entries(ui, doc, manager, out);
    });
    ui.menu_button("View", |ui| {
        if entry(ui, "Expand all", None, true) {
            manager.expand_all();
        }
        if entry(ui, "Collapse all", None, true) {
            manager.collapse_all(doc);
        }
    });
    ui.menu_button("Object", |ui| object_entries(ui, doc, out));
}
