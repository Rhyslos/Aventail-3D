//! The paint brush in a view ([decision 0179](../../../../docs/decisions/0179-the-round-brush.md)):
//! in Paint mode a left press paints and a left drag carries on painting,
//! turned into [`BrushEvent`]s for the workbench; the brush's circle is
//! drawn under the mouse. The UV Editor uses the same capture, with places
//! on the texture instead of in a view (decision 0180).
//!
//! The controls are Substance Painter's (the project lead's choice):
//! **Ctrl + right drag** changes the size (sideways) and the hardness (up
//! and down); **Ctrl + left drag** changes the flow (sideways). Navigation
//! keeps its own buttons (Alt + drag, right and middle drag in the
//! schemes that use them).

use crate::components::ViewPointer;
use crate::theme;
use egui::{Color32, PointerButton, Pos2, Rect, Response, Stroke, Ui};

/// Something the brush did in a view, for the workbench: `P` is where
/// the mouse was (in a 3D view, the view and the mouse in it; in the UV
/// Editor, the place on the texture).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BrushEvent<P = ViewPointer> {
    /// A stroke began with the mouse at `pointer`.
    Begin(P),
    /// Shift + press: a straight line from where the last stroke ended
    /// to `pointer`, which then carries on as a stroke (decision 0184).
    Line(P),
    /// The stroke went on to `pointer`.
    Move(P),
    /// The stroke ended.
    End,
    /// Ctrl + drag changed the brush: its size by a factor, its hardness
    /// and flow by amounts (0 to 1 scale).
    Adjust {
        /// The size multiplied by this.
        size: f32,
        /// Added to the hardness.
        hardness: f32,
        /// Added to the flow.
        flow: f32,
    },
}

/// How far a Ctrl + drag must move to change the hardness or flow by all
/// of its range, in points.
const ADJUST_RANGE: f32 = 300.0;

/// The brush's state in the views.
#[derive(Debug, Clone, Default)]
pub(crate) struct BrushCapture {
    /// A stroke is under way.
    painting: bool,
    /// A Ctrl + drag is changing the brush: with which button, and where
    /// the mouse was last.
    adjusting: Option<(PointerButton, Pos2)>,
    /// Where an adjustment began, to draw the brush there.
    adjust_origin: Option<Pos2>,
    /// Whether a Ctrl + left drag strokes (Sculpt mode inverts the brush
    /// with Ctrl, decision 0304) instead of changing the flow.
    ctrl_strokes: bool,
}

impl BrushCapture {
    /// Lets a Ctrl + left drag stroke (Sculpt mode) or change the flow.
    pub(crate) fn set_ctrl_strokes(&mut self, on: bool) {
        self.ctrl_strokes = on;
    }

    /// Whether a Ctrl + drag is changing the brush (navigation keeps out).
    pub(crate) fn adjusting(&self) -> bool {
        self.adjusting.is_some()
    }

    /// Whether a Ctrl + drag is about to start (the view mustn't take it
    /// for navigating).
    pub(crate) fn wants(ui: &Ui, response: &Response) -> bool {
        ui.input(|i| i.modifiers.command)
            && (response.drag_started_by(PointerButton::Secondary)
                || response.drag_started_by(PointerButton::Primary))
    }

    /// Turns the mouse over `response` into brush events, with places
    /// from `pointer_at`.
    pub(crate) fn capture<P>(
        &mut self,
        ui: &Ui,
        response: &Response,
        pointer_at: impl Fn(Pos2) -> P,
        out: &mut Vec<BrushEvent<P>>,
    ) {
        let (ctrl, alt, shift) =
            ui.input(|i| (i.modifiers.command, i.modifiers.alt, i.modifiers.shift));
        // Ctrl + drag: change the brush.
        let by_left = !response.drag_started_by(PointerButton::Secondary);
        if Self::wants(ui, response)
            && self.adjusting.is_none()
            && !self.painting
            && !(self.ctrl_strokes && by_left)
        {
            let button = if by_left {
                PointerButton::Primary
            } else {
                PointerButton::Secondary
            };
            if let Some(at) = ui.input(|i| i.pointer.press_origin()) {
                self.adjusting = Some((button, at));
                self.adjust_origin = Some(at);
            }
        }
        if let Some((button, last)) = self.adjusting {
            match response.interact_pointer_pos() {
                Some(now) if response.dragged_by(button) => {
                    let d = now - last;
                    let event = match button {
                        PointerButton::Secondary => BrushEvent::Adjust {
                            size: (d.x / 150.0).exp(),
                            hardness: -d.y / ADJUST_RANGE,
                            flow: 0.0,
                        },
                        _ => BrushEvent::Adjust {
                            size: 1.0,
                            hardness: 0.0,
                            flow: d.x / ADJUST_RANGE,
                        },
                    };
                    if d != egui::Vec2::ZERO {
                        out.push(event);
                    }
                    self.adjusting = Some((button, now));
                }
                _ => {
                    self.adjusting = None;
                    self.adjust_origin = None;
                }
            }
            return;
        }
        // A left press paints at once; holding it paints on. Only a press
        // made over this view: when the app is slow, a press over a panel
        // and the move into the view can come in one frame, and that
        // press mustn't paint where the mouse ended up.
        let pressed = ui.input(|i| i.pointer.primary_pressed());
        let origin = ui.input(|i| i.pointer.press_origin());
        let pressed_here = origin.is_some_and(|at| {
            response.interact_rect.contains(at)
                && ui
                    .ctx()
                    .layer_id_at(at)
                    .is_none_or(|layer| layer == response.layer_id)
        });
        if !self.painting && pressed && (!ctrl || self.ctrl_strokes) && !alt && pressed_here {
            if let Some(at) = origin {
                self.painting = true;
                out.push(if shift {
                    BrushEvent::Line(pointer_at(at))
                } else {
                    BrushEvent::Begin(pointer_at(at))
                });
            }
            return;
        }
        if self.painting {
            // The left button's own presses and releases this frame: when
            // the app is slow, a release and the next press can come in
            // one frame, and the button then looks held throughout. Two
            // strokes were joined into one (a review found it).
            let clicks: Vec<(bool, Pos2)> = ui.input(|i| {
                i.events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::PointerButton {
                            pos,
                            button: PointerButton::Primary,
                            pressed,
                            ..
                        } => Some((*pressed, *pos)),
                        _ => None,
                    })
                    .collect()
            });
            if let Some(r) = clicks.iter().position(|(pressed, _)| !pressed) {
                out.push(BrushEvent::Move(pointer_at(clicks[r].1)));
                self.painting = false;
                out.push(BrushEvent::End);
                let again = clicks[r + 1..]
                    .iter()
                    .find(|(pressed, _)| *pressed)
                    .map(|(_, at)| *at);
                if let Some(at) = again
                    && (!ctrl || self.ctrl_strokes)
                    && !alt
                    && response.interact_rect.contains(at)
                {
                    self.painting = true;
                    out.push(if shift {
                        BrushEvent::Line(pointer_at(at))
                    } else {
                        BrushEvent::Begin(pointer_at(at))
                    });
                    if let Some(now) = ui.input(|i| i.pointer.latest_pos())
                        && now != at
                    {
                        out.push(BrushEvent::Move(pointer_at(now)));
                    }
                    if !ui.input(|i| i.pointer.primary_down()) {
                        self.painting = false;
                        out.push(BrushEvent::End);
                    }
                }
                return;
            }
            let down = ui.input(|i| i.pointer.primary_down());
            if let Some(at) = ui.input(|i| i.pointer.latest_pos())
                && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO)
            {
                out.push(BrushEvent::Move(pointer_at(at)));
            }
            if !down {
                self.painting = false;
                out.push(BrushEvent::End);
            }
        }
    }

    /// Draws the brush's circle of `radius` points under the mouse (or
    /// where a Ctrl + drag began), with its hard part inside. A radius of
    /// 0 is a tool with no size (the polygon fill): a small cross only,
    /// as a circle would promise a size it doesn't paint (a review found
    /// it).
    pub(crate) fn paint(
        &self,
        ui: &Ui,
        rect: Rect,
        hover: Option<Pos2>,
        radius: f32,
        hardness: f32,
    ) {
        let Some(at) = self.adjust_origin.or(hover) else {
            return;
        };
        if !rect.contains(at) {
            return;
        }
        let painter = ui.painter_at(rect);
        let light = Color32::from_white_alpha(220);
        if radius <= 0.0 {
            let shadow = Stroke::new(3.0, Color32::from_black_alpha(140));
            let line = Stroke::new(1.0, light);
            for stroke in [shadow, line] {
                painter.line_segment(
                    [at - egui::vec2(6.0, 0.0), at + egui::vec2(6.0, 0.0)],
                    stroke,
                );
                painter.line_segment(
                    [at - egui::vec2(0.0, 6.0), at + egui::vec2(0.0, 6.0)],
                    stroke,
                );
            }
            return;
        }
        painter.circle_stroke(
            at,
            radius + 1.0,
            Stroke::new(1.0, Color32::from_black_alpha(140)),
        );
        painter.circle_stroke(at, radius, Stroke::new(1.0, light));
        let inner = radius * hardness.clamp(0.0, 1.0);
        if inner > 2.0 && inner < radius - 2.0 {
            painter.circle_stroke(
                at,
                inner,
                Stroke::new(1.0, theme::TEXT_WEAK.gamma_multiply(0.8)),
            );
        }
        painter.circle_filled(at, 1.0, light);
    }
}

/// Mirror painting's other dab (decision 0187): a dashed circle of
/// `radius` points at `at`, so it reads as a reflection, not a second
/// brush.
pub(crate) fn paint_mirror_mark(ui: &Ui, rect: Rect, at: Pos2, radius: f32) {
    if !rect.contains(at) || !radius.is_finite() {
        return;
    }
    let painter = ui.painter_at(rect);
    let radius = radius.max(0.5);
    let light = Color32::from_white_alpha(200);
    let shadow = Color32::from_black_alpha(120);
    // Dashes: one every 12 points of the circle, at least eight.
    let dashes = ((std::f32::consts::TAU * radius / 12.0).round() as usize).clamp(8, 64);
    for i in 0..dashes {
        let a = std::f32::consts::TAU * i as f32 / dashes as f32;
        let b = a + std::f32::consts::TAU * 0.5 / dashes as f32;
        let point = |angle: f32, r: f32| at + egui::vec2(angle.cos(), angle.sin()) * r;
        painter.line_segment(
            [point(a, radius + 1.0), point(b, radius + 1.0)],
            Stroke::new(1.0, shadow),
        );
        painter.line_segment(
            [point(a, radius), point(b, radius)],
            Stroke::new(1.0, light),
        );
    }
    painter.circle_filled(at, 1.0, light);
}

/// The sculpt brush's circle on the surface (decision 0310): lying along
/// it at `centre`, facing `normal`, `radius` across, all in the world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SurfaceCircle {
    /// Its middle.
    pub(crate) centre: glam::Vec3,
    /// Which way the surface faces there.
    pub(crate) normal: glam::Vec3,
    /// Its radius.
    pub(crate) radius: f32,
    /// Where the falloff is down to half, as a share of the radius (the
    /// inner ring).
    pub(crate) inner: f32,
    /// A copy symmetry makes: dashed, so it reads as a reflection.
    pub(crate) dashed: bool,
}

/// Draws `circle` as `camera` sees it in the view at `rect`.
pub(crate) fn paint_surface_circle(
    ui: &Ui,
    rect: Rect,
    camera: &tools::Camera,
    circle: &SurfaceCircle,
) {
    let size = glam::Vec2::new(rect.width(), rect.height());
    let normal = circle.normal.normalize_or(glam::Vec3::Z);
    let across = normal.any_orthonormal_vector();
    let along = normal.cross(across);
    let painter = ui.painter_at(rect);
    let light = Color32::from_white_alpha(if circle.dashed { 200 } else { 220 });
    let shadow = Color32::from_black_alpha(130);
    const STEPS: usize = 48;
    let ring = |share: f32| -> Vec<Option<Pos2>> {
        (0..=STEPS)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / STEPS as f32;
                let p = circle.centre
                    + (across * a.cos() + along * a.sin()) * circle.radius * share;
                camera
                    .project(p, size)
                    .map(|s| rect.min + egui::vec2(s.x, s.y))
            })
            .collect()
    };
    let outer = ring(1.0);
    for (i, pair) in outer.windows(2).enumerate() {
        if circle.dashed && i % 2 == 1 {
            continue;
        }
        if let [Some(a), Some(b)] = pair {
            painter.line_segment([*a, *b], Stroke::new(3.0, shadow));
            painter.line_segment([*a, *b], Stroke::new(1.0, light));
        }
    }
    if !circle.dashed && circle.inner > 0.05 && circle.inner < 0.95 {
        let inner = ring(circle.inner);
        for pair in inner.windows(2) {
            if let [Some(a), Some(b)] = pair {
                painter.line_segment(
                    [*a, *b],
                    Stroke::new(1.0, theme::TEXT_WEAK.gamma_multiply(0.8)),
                );
            }
        }
    }
    if let Some(middle) = camera.project(circle.centre, size) {
        painter.circle_filled(rect.min + egui::vec2(middle.x, middle.y), 1.0, light);
    }
}

/// Draws a box or lasso mask being drawn (decision 0310): `points` in the
/// view at `rect` (a box: its two corners).
pub(crate) fn paint_mask_outline(ui: &Ui, rect: Rect, points: &[glam::Vec2], is_box: bool) {
    let at = |p: &glam::Vec2| rect.min + egui::vec2(p.x, p.y);
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.5, Color32::from_white_alpha(220));
    let fill = Color32::from_black_alpha(50);
    if is_box {
        if let [a, b, ..] = points {
            let area = Rect::from_two_pos(at(a), at(b));
            painter.rect_filled(area, 0.0, fill);
            painter.rect_stroke(area, 0.0, stroke, egui::StrokeKind::Middle);
        }
        return;
    }
    let shown: Vec<Pos2> = points.iter().map(at).collect();
    if shown.len() >= 2 {
        painter.add(egui::Shape::closed_line(shown, stroke));
    }
}

impl super::View {
    /// Whether the view is being navigated (orbit, pan, dolly).
    pub(super) fn navigating(&self) -> bool {
        self.drag.is_some()
    }
}
