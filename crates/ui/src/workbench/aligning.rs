//! Snap to and Align ([decision 0093](../../../../docs/decisions/0093-snap-to-and-align.md)):
//! putting the selected objects against another object, picked with a
//! click.
//!
//! - **Snap to** moves the selected objects so their pivots sit on the
//!   target's pivot (3ds Max's Quick Align, by our own name).
//! - **Align** opens a window: along which world axes, from which point of
//!   the selected objects (lowest, middle, pivot, highest) to which point
//!   of the target, and whether to turn them to match it. It is shown live
//!   while the window is open, and is one undo step.
//!
//! Both move whole objects (shape and pivot together) with
//! [`Command::SetTransforms`]; nothing new in the scene.

use super::Workbench;
use crate::dialog;
use glam::{Mat4, Quat, Vec3};
use scene::{Command, ObjectId, Transform};

/// What the next click on an object is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetUse {
    /// Pivot to object (decision 0068).
    Pivot,
    /// Snap to.
    SnapTo,
    /// Align: the window opens once the target is picked.
    Align,
}

impl TargetUse {
    /// The status bar while waiting for the click.
    pub(crate) fn note(self) -> &'static str {
        match self {
            Self::Pivot => super::pivots::PIVOT_TARGET_NOTE,
            Self::SnapTo => SNAP_TO_NOTE,
            Self::Align => ALIGN_NOTE,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Pivot => "Pivot to object",
            Self::SnapTo => "Snap to",
            Self::Align => "Align",
        }
    }
}

/// Snap to's name in the menus.
pub(crate) const SNAP_TO_LABEL: &str = "Snap to...";
/// What Snap to does.
pub(crate) const SNAP_TO_HELP: &str = "Move the selected objects so their pivots sit on \
     another object's pivot: click this, then click that object";
/// Align's name in the menus.
pub(crate) const ALIGN_LABEL: &str = "Align...";
/// What Align does.
pub(crate) const ALIGN_HELP: &str = "Line the selected objects up with another object: click \
     this, then click that object, and choose the sides or middles to line up in the window";
/// The Align window's title.
pub(crate) const ALIGN_TITLE: &str = "Align";
const SNAP_TO_NOTE: &str = "Snap to: click the object to snap to. Esc cancels.";
const ALIGN_NOTE: &str = "Align: click the object to line up with. Esc cancels.";

/// A point of an object along one axis, to line up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AlignPoint {
    /// The lowest point of its box along the axis.
    Lowest,
    /// The middle of its box.
    Middle,
    /// Its pivot.
    Pivot,
    /// The highest point of its box.
    Highest,
}

const POINTS: [(AlignPoint, &str, &str); 4] = [
    (
        AlignPoint::Lowest,
        "Lowest",
        "The lowest point of the box around it along each axis (its left, back or bottom)",
    ),
    (
        AlignPoint::Middle,
        "Middle",
        "The middle of the box around it",
    ),
    (AlignPoint::Pivot, "Pivot", "Its pivot"),
    (
        AlignPoint::Highest,
        "Highest",
        "The highest point of the box around it along each axis (its right, front or top)",
    ),
];

/// The Align window's choices, remembered for the next time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AlignSettings {
    /// Along the world's X, Y and Z.
    pub(crate) axes: [bool; 3],
    /// The point of the selected objects that moves.
    pub(crate) from: AlignPoint,
    /// The point of the target it moves to.
    pub(crate) to: AlignPoint,
    /// Turn the selected objects to the target's rotation too.
    pub(crate) turn: bool,
}

impl Default for AlignSettings {
    fn default() -> Self {
        Self {
            axes: [true; 3],
            from: AlignPoint::Middle,
            to: AlignPoint::Middle,
            turn: false,
        }
    }
}

/// The Align window, open on a target.
#[derive(Debug, Clone)]
pub(crate) struct AlignSession {
    target: ObjectId,
    /// The settings last shown in the scene, if any were.
    shown: Option<AlignSettings>,
    /// The undo merge key of the shown step.
    key: u64,
}

/// Marks Align's merge keys apart from every other.
const ALIGN_KEY: u64 = 0x414c_4947_0000_0000;

/// `bounds`'s coordinate for `point` along `axis`, or the pivot's.
fn coordinate(point: AlignPoint, bounds: Option<scene::Bounds>, pivot: Vec3, axis: usize) -> f32 {
    match (point, bounds) {
        (AlignPoint::Lowest, Some(b)) => b.min[axis],
        (AlignPoint::Highest, Some(b)) => b.max[axis],
        (AlignPoint::Middle, Some(b)) => b.centre()[axis],
        _ => pivot[axis],
    }
}

impl Workbench {
    /// Starts waiting for the click on a target, for `what`.
    pub(super) fn start_target_pick(&mut self, what: TargetUse) {
        if self.top_selected().is_empty() {
            self.note = Some("Select one or more objects first.".into());
            return;
        }
        self.target_pick = Some(what);
        self.note = Some(what.note().into());
    }

    /// Stops waiting for a target without changing anything.
    pub(super) fn cancel_target_pick(&mut self) {
        if let Some(what) = self.target_pick.take() {
            self.note = Some(format!("{} cancelled.", what.name()));
        }
    }

    /// A target was clicked for `what`.
    pub(super) fn target_picked(&mut self, what: TargetUse, target: ObjectId) {
        match what {
            TargetUse::Pivot => self.pivot_to_object(target),
            TargetUse::SnapTo => self.snap_to(target),
            TargetUse::Align => self.open_align(target),
        }
    }

    /// The selected objects that would move to `target`: not the target,
    /// and not one with the target inside it (it would carry the target
    /// along).
    fn movers(&self, target: ObjectId) -> Vec<ObjectId> {
        let scene = self.document.scene();
        self.top_selected()
            .into_iter()
            .filter(|id| *id != target && !scene.is_inside(target, *id))
            .collect()
    }

    /// Snap to: the selected objects' pivots onto `target`'s.
    pub(crate) fn snap_to(&mut self, target: ObjectId) {
        let scene = self.document.scene();
        let at = scene.world_matrix(target).w_axis.truncate();
        let name = scene
            .object(target)
            .map_or_else(String::new, |o| o.name.clone());
        let changes: Vec<(ObjectId, Transform)> = self
            .movers(target)
            .into_iter()
            .filter_map(|id| Some((id, self.placed_at(id, at)?)))
            .collect();
        if changes.is_empty() {
            self.note = Some(format!("Snap to: already on {name}'s pivot."));
            return;
        }
        let label = format!("Snap to {name}");
        self.note = Some(
            match self.document.execute_labelled(
                Command::SetTransforms(changes),
                label.clone(),
                None,
            ) {
                Ok(()) => format!("{label}. Ctrl + Z takes it back."),
                Err(why) => format!("Couldn't snap: {why}."),
            },
        );
    }

    /// Opens the Align window on `target`, showing the remembered settings.
    pub(crate) fn open_align(&mut self, target: ObjectId) {
        if self.movers(target).is_empty() {
            self.note = Some("Align: pick an object other than the ones selected.".into());
            return;
        }
        self.sessions += 1;
        self.align = Some(AlignSession {
            target,
            shown: None,
            key: ALIGN_KEY | self.sessions,
        });
        self.note = None;
        self.show_align();
    }

    /// Whether the Align window is open.
    #[cfg(test)]
    pub(crate) fn is_aligning(&self) -> bool {
        self.align.is_some()
    }

    /// Shows the current settings in the scene, in place of what was
    /// shown before (one undo step).
    fn show_align(&mut self) {
        let Some(session) = self.align.clone() else {
            return;
        };
        let settings = self.align_settings;
        if session.shown == Some(settings) {
            return;
        }
        if session.shown.is_some() && !self.document.abandon(session.key) {
            // Something else was done meanwhile: keep it all, and stop.
            self.align = None;
            return;
        }
        let name = self
            .document
            .scene()
            .object(session.target)
            .map_or_else(String::new, |o| o.name.clone());
        let label = format!("Align to {name}");
        let ids = self.movers(session.target);
        let steps = self.document.undo_steps();
        if settings.turn {
            let turned: Vec<(ObjectId, Transform)> = ids
                .iter()
                .filter_map(|id| Some((*id, self.turned_like(*id, session.target)?)))
                .collect();
            if !turned.is_empty() {
                let _ = self.document.execute_labelled(
                    Command::SetTransforms(turned),
                    label.clone(),
                    Some(session.key),
                );
            }
        }
        let scene = self.document.scene();
        let target_bounds = scene.bounds_of([session.target]);
        let target_pivot = scene.world_matrix(session.target).w_axis.truncate();
        let moved: Vec<(ObjectId, Transform)> = ids
            .iter()
            .filter_map(|id| {
                let pivot = scene.world_matrix(*id).w_axis.truncate();
                let bounds = scene.bounds_of([*id]);
                let mut to = pivot;
                for axis in 0..3 {
                    if settings.axes[axis] {
                        to[axis] += coordinate(settings.to, target_bounds, target_pivot, axis)
                            - coordinate(settings.from, bounds, pivot, axis);
                    }
                }
                Some((*id, self.placed_at(*id, to)?))
            })
            .collect();
        if !moved.is_empty() {
            let _ = self.document.execute_labelled(
                Command::SetTransforms(moved),
                label,
                Some(session.key),
            );
        }
        let recorded = self.document.undo_steps() > steps;
        if let Some(open) = self.align.as_mut() {
            // Nothing to change records nothing, and is shown as nothing,
            // so the next change doesn't take back a step that isn't
            // Align's.
            open.shown = recorded.then_some(settings);
        }
    }

    /// `id`'s transform turned to `target`'s rotation in the world, its
    /// pivot and size kept, or `None` if nothing changes.
    fn turned_like(&self, id: ObjectId, target: ObjectId) -> Option<Transform> {
        let scene = self.document.scene();
        let (_, turn, _) = scene.world_matrix(target).to_scale_rotation_translation();
        let (size, _, at) = scene.world_matrix(id).to_scale_rotation_translation();
        let world = Mat4::from_scale_rotation_translation(size, turn.normalize(), at);
        let parent = scene
            .parent(id)
            .map_or(Mat4::IDENTITY, |p| scene.world_matrix(p));
        let t = Transform::from_matrix(parent.inverse() * world)?;
        let before = scene.object(id)?.transform;
        let same =
            Quat::from_mat4(&t.matrix()).angle_between(Quat::from_mat4(&before.matrix())) < 1e-5;
        (!same).then_some(t)
    }

    /// The Align window, while open: the settings change the scene live;
    /// Align keeps it, Cancel (or Esc) takes it back.
    pub(super) fn align_window(&mut self, ctx: &egui::Context) {
        let Some(session) = self.align.clone() else {
            return;
        };
        let target = self
            .document
            .scene()
            .object(session.target)
            .map_or_else(String::new, |o| o.name.clone());
        let mut settings = self.align_settings;
        let mut answer = None;
        // Bottom left of the viewport, out of the way of what moves.
        let outcome = dialog::Dialog::new(ALIGN_TITLE, 420.0)
            .id("align")
            .anchor(egui::Align2::LEFT_BOTTOM, [70.0, -190.0])
            .show(ctx, |d| {
                d.header(true, |_| {});
                d.body(|ui| {
                    dialog::help(
                        ui,
                        format!(
                            "Line the selected objects up with {target}. The change shows as \
                             you choose."
                        ),
                    );
                    dialog::heading(ui, "Along");
                    dialog::row(ui, "World axes", |ui| {
                        ui.horizontal(|ui| {
                            for (axis, name) in ["X", "Y", "Z"].into_iter().enumerate() {
                                let on = settings.axes[axis];
                                ui.checkbox(&mut settings.axes[axis], name).widget_info(|| {
                                    egui::WidgetInfo::selected(
                                        egui::WidgetType::Checkbox,
                                        true,
                                        on,
                                        format!("Along {name}"),
                                    )
                                });
                            }
                        });
                    });
                    dialog::heading(ui, "Line up");
                    dialog::row(ui, "This point", |ui| {
                        dialog::segmented(ui, &mut settings.from, &POINTS);
                    });
                    dialog::row(ui, &format!("With {target}'s"), |ui| {
                        dialog::segmented(ui, &mut settings.to, &POINTS);
                    });
                    dialog::help(
                        ui,
                        "To stand an object on another: Z only, this object's Lowest with \
                         the other's Highest.",
                    );
                    dialog::divider(ui);
                    ui.checkbox(&mut settings.turn, "Also turn them to match")
                        .on_hover_text(format!("Give the selected objects {target}'s rotation"));
                });
                d.footer(Some("Esc: cancel"), |ui| {
                    if dialog::main_button(ui, "Align").clicked() {
                        answer = Some(true);
                    }
                    if dialog::plain_button(ui, "Cancel").clicked() {
                        answer = Some(false);
                    }
                });
            });
        let (enter, escape) = dialog::enter_or_escape(ctx);
        if enter {
            answer = Some(true);
        }
        if escape || outcome.closed {
            answer = Some(false);
        }
        if settings != self.align_settings {
            self.align_settings = settings;
            self.show_align();
        }
        match answer {
            Some(true) => {
                self.align = None;
                self.note = Some(format!("Aligned to {target}. Ctrl + Z takes it back."));
            }
            Some(false) => {
                if let Some(session) = self.align.take()
                    && session.shown.is_some()
                {
                    self.document.abandon(session.key);
                }
                self.note = Some("Align cancelled: nothing moved.".into());
            }
            None => {}
        }
    }
}
