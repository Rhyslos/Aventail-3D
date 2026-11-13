//! Turning the preview's picture (decision 0291), with the controls the
//! project lead chose on 2026-11-13: the **same as the 3D view** (right
//! drag turns, middle drag moves, the wheel zooms, Alt + mouse when that
//! setting is on, in whichever navigation scheme is chosen), and **Shift +
//! right drag turns the light**. On Flat the wheel zooms out to show more
//! repeats (the material seen from afar) and a middle drag slides it.
//!
//! Turning never works the graph out again or lays the material again:
//! only the camera (or the flat picture's place) changes, so the renderer
//! draws the same scene from somewhere else.

use super::laying;
use super::preview::PreviewLook;
use super::preview_options::Lighting;
use crate::drawing::SceneDrawing;
use crate::text::tr;
use crate::theme;
use crate::viewport::{GridDetail, Shading, Surface, ViewportView};
use glam::{Mat4, Vec2, Vec3};
use scene::ObjectId;
use std::sync::Arc;
use tools::{DragKind, Modifiers, MouseButton, NavigationSettings, Navigator};

/// The way the camera first looks at the shape: from the front right, a
/// little above, as the viewport opens.
const LOOK_FROM: Vec3 = Vec3::new(0.55, -1.0, 0.5);
/// How far the light turns per point of mouse movement, in degrees: a
/// drag across a 300-point preview turns it half way round.
pub(crate) const LIGHT_SPEED: f32 = 0.6;
/// How many repeats Flat shows across at first, and the most and fewest
/// zooming gives.
pub(crate) const FLAT_REPEATS: f32 = 2.0;
pub(crate) const FLAT_FEWEST: f32 = 1.0;
pub(crate) const FLAT_MOST: f32 = 64.0;

/// What a drag on the picture does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreviewDrag {
    /// Moves the camera, as in the 3D view.
    Camera(DragKind),
    /// Turns the light (Shift + right drag).
    Light,
    /// Moves the compare slider's line (a left drag while comparing with
    /// the slider).
    Split,
}

/// What a drag with `button` and `keys` does on the preview: Shift +
/// right turns the light; otherwise what it does in the 3D view with
/// `settings`; a plain left drag moves the compare line when `slider`.
pub(crate) fn drag_kind(
    button: MouseButton,
    keys: Modifiers,
    settings: &NavigationSettings,
    slider: bool,
) -> Option<PreviewDrag> {
    if button == MouseButton::Secondary && keys.shift && !keys.alt {
        return Some(PreviewDrag::Light);
    }
    if let Some(kind) = settings.drag(button, keys) {
        return Some(PreviewDrag::Camera(kind));
    }
    (button == MouseButton::Primary && slider && keys == Modifiers::NONE)
        .then_some(PreviewDrag::Split)
}

/// What the camera was framed on: a change frames the new shape afresh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FramedOn {
    pub(crate) object: ObjectId,
    pub(crate) mesh: usize,
    pub(crate) transform: Mat4,
}

/// The preview's camera and the flat picture's place.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Turning {
    /// What the camera is framed on, the view it opened with and the
    /// view now.
    framed: Option<(FramedOn, Navigator, Navigator)>,
    /// How many repeats Flat shows across.
    pub(crate) flat_repeats: f32,
    /// Where Flat's picture starts, in repeats.
    pub(crate) flat_offset: Vec2,
    /// Whether a light turn is in progress (the picture says how far).
    pub(crate) turning_light: bool,
}

impl Default for Turning {
    fn default() -> Self {
        Self {
            framed: None,
            flat_repeats: FLAT_REPEATS,
            flat_offset: Vec2::ZERO,
            turning_light: false,
        }
    }
}

impl Turning {
    /// The camera looking at `mesh` placed by `transform` (kept by
    /// `on`): the view turned so far, or a fresh one framing the shape
    /// when it is new. `Err` for a shape with nothing to frame.
    pub(crate) fn camera(
        &mut self,
        on: FramedOn,
        mesh: &mesh::Mesh,
        transform: Mat4,
    ) -> Result<&mut Navigator, String> {
        let framed = match self.framed.take() {
            Some(kept) if kept.0 == on => kept,
            _ => {
                let start = framing(mesh, transform)?;
                (on, start, start)
            }
        };
        Ok(&mut self.framed.insert(framed).2)
    }

    /// The view now, if a shape is framed.
    pub(crate) fn navigator(&self) -> Option<&Navigator> {
        self.framed.as_ref().map(|(_, _, now)| now)
    }

    /// The view now, to move, if a shape is framed.
    pub(crate) fn navigator_mut(&mut self) -> Option<&mut Navigator> {
        self.framed.as_mut().map(|(_, _, now)| now)
    }

    /// Whether the view has been turned, moved or zoomed on `shape`.
    pub(crate) fn moved(&self, shape: super::preview::PreviewShape) -> bool {
        if shape == super::preview::PreviewShape::Flat {
            return self.flat_repeats != FLAT_REPEATS || self.flat_offset != Vec2::ZERO;
        }
        self.framed
            .as_ref()
            .is_some_and(|(_, start, now)| start != now)
    }

    /// Back to the views it opened with.
    pub(crate) fn reset(&mut self) {
        if let Some((_, start, now)) = &mut self.framed {
            *now = *start;
        }
        self.flat_repeats = FLAT_REPEATS;
        self.flat_offset = Vec2::ZERO;
    }

    /// A drag of `kind` by `delta` points on Flat's picture `width` points
    /// across: middle drag slides it, a zoom drag shows more or fewer
    /// repeats; turning does nothing on a flat picture.
    pub(crate) fn flat_drag(&mut self, kind: DragKind, delta: Vec2, width: f32) {
        match kind {
            DragKind::Pan if width > 0.0 && delta.is_finite() => {
                self.flat_offset -= delta * self.flat_repeats / width;
                self.flat_offset = within_one(self.flat_offset);
            }
            DragKind::Dolly => {
                self.flat_zoom((delta.x - delta.y) * tools::navigation::DOLLY_DRAG_SPEED);
            }
            _ => {}
        }
    }

    /// Zooms Flat in (positive `amount`) or out, keeping the middle of
    /// the picture where it is: the repeats shown change by e^-amount,
    /// from [`FLAT_FEWEST`] to [`FLAT_MOST`].
    pub(crate) fn flat_zoom(&mut self, amount: f32) {
        if !amount.is_finite() {
            return;
        }
        let before = self.flat_repeats;
        let after = (before * (-amount).exp()).clamp(FLAT_FEWEST, FLAT_MOST);
        let middle = self.flat_offset + Vec2::splat(before * 0.5);
        self.flat_offset = within_one(middle - Vec2::splat(after * 0.5));
        self.flat_repeats = after;
    }

    /// The part of the texture Flat shows: from its offset, its repeats
    /// across and down.
    pub(crate) fn flat_uv(&self) -> egui::Rect {
        let min = egui::pos2(self.flat_offset.x, self.flat_offset.y);
        egui::Rect::from_min_size(min, egui::vec2(self.flat_repeats, self.flat_repeats))
    }
}

/// `offset` moved by whole repeats to within the first: the same
/// picture, as the texture repeats.
fn within_one(offset: Vec2) -> Vec2 {
    Vec2::new(offset.x.rem_euclid(1.0), offset.y.rem_euclid(1.0))
}

/// The view `mesh` placed by `transform` opens with: looking at its
/// middle from the front right, far enough that it fits with a little
/// room. `Err` for a shape with nothing to frame.
pub(crate) fn framing(mesh: &mesh::Mesh, transform: Mat4) -> Result<Navigator, String> {
    let (middle, radius) =
        laying::around(mesh, transform).ok_or_else(|| tr("the shape is empty").to_string())?;
    let camera = tools::Camera::looking_at(middle + LOOK_FROM, middle);
    let distance = radius / (camera.fov_y * 0.5).sin() * 1.1;
    let camera = tools::Camera::looking_at(middle + LOOK_FROM.normalize() * distance, middle);
    Ok(Navigator::new(camera, middle))
}

/// The view of `scene` for the preview: from `navigator`'s camera,
/// filling `rect`, in `look` with `lighting`'s light and backdrop, with no
/// grid.
pub(crate) fn preview_view(
    scene: Arc<SceneDrawing>,
    navigator: &Navigator,
    rect: egui::Rect,
    pixels_per_point: f32,
    (look, lighting): (PreviewLook, &Lighting),
) -> ViewportView {
    let ppp = if pixels_per_point.is_finite() && pixels_per_point > 0.0 {
        pixels_per_point
    } else {
        1.0
    };
    let px = |v: f32| (v * ppp).round().clamp(1.0, 4096.0) as u32;
    let size_px = [px(rect.width()), px(rect.height())];
    let camera = navigator.camera();
    let scene_look = lighting.backdrop.look();
    let [sky_top, sky_horizon, ground] = scene_look.backdrop();
    ViewportView {
        slot: 0,
        size_px,
        view_projection: camera.view_projection(Vec2::new(size_px[0] as f32, size_px[1] as f32)),
        camera_position: camera.position,
        focus: navigator.pivot(),
        flat_height: None,
        grid_plane: [Vec3::X, Vec3::Y],
        sky_top,
        sky_horizon,
        ground,
        grid_detail: GridDetail::default(),
        scene,
        selection_color: [1.0, 1.0, 1.0],
        grid_colors: theme::grid_colors(),
        checker_per_object: false,
        picture: None,
        mip_level: 0,
        overlay: Arc::default(),
        shading: Shading::default(),
        show_edges: false,
        show_grid: false,
        cavity: false,
        surface: Surface::Painted,
        paint_preview: look.paint_preview(),
        light_turn: lighting.light_turn,
        environment: lighting.environment,
        exposure: lighting.usable_exposure(),
        pixels_per_point: ppp,
        high_contrast: false,
        look: scene_look,
        outline_selection: false,
    }
}

/// How far the mouse wheel turned this frame over the preview, in
/// points (positive away from you), read from the wheel's own events as
/// the 3D view does, so zooming answers at once.
pub(crate) fn wheel(ui: &egui::Ui, page_height: f32) -> f32 {
    let line = ui.ctx().options(|o| o.input_options.line_scroll_speed);
    ui.input(|input| {
        input
            .events
            .iter()
            .map(|event| match event {
                egui::Event::MouseWheel { unit, delta, .. } => match unit {
                    egui::MouseWheelUnit::Point => delta.y,
                    egui::MouseWheelUnit::Line => delta.y * line,
                    egui::MouseWheelUnit::Page => delta.y * page_height,
                },
                _ => 0.0,
            })
            .sum::<f32>()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewport::PaintPreview;

    fn shift() -> Modifiers {
        Modifiers::SHIFT
    }

    #[test]
    fn drags_do_what_they_do_in_the_3d_view_and_shift_right_turns_the_light() {
        let standard = NavigationSettings::default();
        let right = MouseButton::Secondary;
        let middle = MouseButton::Middle;
        let left = MouseButton::Primary;
        let none = Modifiers::NONE;
        assert_eq!(
            drag_kind(right, none, &standard, false),
            Some(PreviewDrag::Camera(DragKind::Orbit))
        );
        assert_eq!(
            drag_kind(middle, none, &standard, false),
            Some(PreviewDrag::Camera(DragKind::Pan))
        );
        assert_eq!(
            drag_kind(right, shift(), &standard, false),
            Some(PreviewDrag::Light)
        );
        // Alt + mouse, switched on by the standard settings.
        assert_eq!(
            drag_kind(left, Modifiers::ALT, &standard, false),
            Some(PreviewDrag::Camera(DragKind::Orbit))
        );
        assert_eq!(
            drag_kind(right, Modifiers::ALT, &standard, false),
            Some(PreviewDrag::Camera(DragKind::Dolly))
        );
        // ...and not when switched off.
        let no_alt = NavigationSettings {
            alt_everywhere: false,
            ..standard
        };
        assert_eq!(drag_kind(left, Modifiers::ALT, &no_alt, false), None);
        // A plain left drag only moves the compare line.
        assert_eq!(drag_kind(left, none, &standard, false), None);
        assert_eq!(
            drag_kind(left, none, &standard, true),
            Some(PreviewDrag::Split)
        );
        assert_eq!(drag_kind(left, shift(), &standard, true), None);
        // Another scheme: middle orbits there.
        let middle_orbits = NavigationSettings {
            scheme: tools::Scheme::MiddleOrbits,
            ..standard
        };
        assert_eq!(
            drag_kind(middle, none, &middle_orbits, false),
            Some(PreviewDrag::Camera(DragKind::Orbit))
        );
        assert_eq!(drag_kind(right, none, &middle_orbits, false), None);
        assert_eq!(
            drag_kind(right, shift(), &middle_orbits, false),
            Some(PreviewDrag::Light),
            "Shift + right turns the light in every scheme"
        );
    }

    #[test]
    fn the_camera_frames_a_new_shape_and_keeps_its_turn_otherwise() {
        let cube = laying::cube();
        let on = FramedOn {
            object: ObjectId::from_raw(7),
            mesh: Arc::as_ptr(&cube) as usize,
            transform: Mat4::IDENTITY,
        };
        let mut turning = Turning::default();
        assert!(!turning.moved(super::super::preview::PreviewShape::Cube));
        let start = *turning.camera(on, &cube, Mat4::IDENTITY).unwrap();
        turning
            .camera(on, &cube, Mat4::IDENTITY)
            .unwrap()
            .orbit(Vec2::new(40.0, 0.0));
        assert!(turning.moved(super::super::preview::PreviewShape::Cube));
        assert_ne!(*turning.navigator().unwrap(), start);
        // The same shape: still turned.
        let again = *turning.camera(on, &cube, Mat4::IDENTITY).unwrap();
        assert_ne!(again, start);
        // Moved: framed afresh.
        let moved = FramedOn {
            transform: Mat4::from_translation(Vec3::X * 300.0),
            ..on
        };
        let fresh = *turning.camera(moved, &cube, moved.transform).unwrap();
        assert!(fresh.pivot().x > 250.0);
        let shape = super::super::preview::PreviewShape::Cube;
        assert!(!turning.moved(shape));
        // Reset goes back.
        turning
            .camera(moved, &cube, moved.transform)
            .unwrap()
            .wheel(-200.0);
        let out = turning.navigator().unwrap().distance() / fresh.distance();
        assert!(out > 1.5, "{out}");
        assert!(turning.moved(shape));
        turning.reset();
        assert_eq!(*turning.navigator().unwrap(), fresh);
        // Nothing to frame: why.
        assert!(
            turning
                .camera(on, &mesh::Mesh::default(), Mat4::IDENTITY)
                .is_err()
        );
    }

    #[test]
    fn flat_zooms_about_its_middle_within_limits_and_slides() {
        let mut turning = Turning::default();
        let shape = super::super::preview::PreviewShape::Flat;
        assert!(!turning.moved(shape));
        turning.flat_zoom(-(2.0f32).ln());
        assert!((turning.flat_repeats - 4.0).abs() < 1e-4);
        // The middle stays: 1 repeat in at 2 shown; at 4, from -1 → 0.
        assert!(turning.flat_offset.distance(Vec2::ZERO) < 1e-4);
        assert!(turning.moved(shape));
        turning.flat_zoom(-100.0);
        assert_eq!(turning.flat_repeats, FLAT_MOST);
        turning.flat_zoom(100.0);
        assert_eq!(turning.flat_repeats, FLAT_FEWEST);
        turning.flat_zoom(f32::NAN);
        assert_eq!(turning.flat_repeats, FLAT_FEWEST);
        // Zoomed about the middle: half a repeat in at one repeat shown.
        assert!((turning.flat_offset.x - 0.5).abs() < 1e-4);
        turning.flat_offset = Vec2::ZERO;
        // A middle drag slides the picture the other way, within a repeat.
        turning.flat_drag(DragKind::Pan, Vec2::new(-50.0, 0.0), 100.0);
        assert!((turning.flat_offset.x - 0.5).abs() < 1e-4);
        assert_eq!(turning.flat_uv().width(), FLAT_FEWEST);
        turning.flat_drag(DragKind::Orbit, Vec2::new(30.0, 30.0), 100.0);
        assert!(
            (turning.flat_offset.x - 0.5).abs() < 1e-4,
            "nothing to turn"
        );
        turning.reset();
        assert!(!turning.moved(shape));
    }

    #[test]
    fn the_view_has_the_lighting_chosen_and_frames_the_whole_shape() {
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 200.0));
        let cube = laying::cube();
        let navigator = framing(&cube, Mat4::IDENTITY).unwrap();
        let lighting = Lighting {
            backdrop: super::super::preview_options::Backdrop::Dusk,
            light_turn: 120.0,
            exposure: 1.0,
            ..Lighting::default()
        };
        let view = preview_view(
            Arc::default(),
            &navigator,
            rect,
            2.0,
            (PreviewLook::Toon, &lighting),
        );
        assert_eq!(view.size_px, [400, 400]);
        assert_eq!(view.surface, Surface::Painted);
        assert_eq!(view.paint_preview, PaintPreview::Toon);
        assert_eq!(view.light_turn, 120.0);
        assert_eq!(view.exposure, 1.0);
        assert_eq!(view.look, crate::viewport::SceneLook::DUSK);
        assert_eq!(view.sky_top, crate::viewport::SceneLook::DUSK.backdrop()[0]);
        assert!(!view.show_grid && !view.show_edges && !view.outline_selection);
        let bounds = cube.bounds().unwrap();
        for i in 0..8 {
            let pick = |bit: usize, lo: f32, hi: f32| if i & bit == 0 { lo } else { hi };
            let corner = Vec3::new(
                pick(1, bounds.min.x, bounds.max.x),
                pick(2, bounds.min.y, bounds.max.y),
                pick(4, bounds.min.z, bounds.max.z),
            );
            let clip = view.view_projection * corner.extend(1.0);
            assert!(clip.w > 0.0);
            let (x, y) = (clip.x / clip.w, clip.y / clip.w);
            assert!(x.abs() < 1.0 && y.abs() < 1.0, "corner {i} at {x}, {y}");
        }
        assert!(framing(&mesh::Mesh::default(), Mat4::IDENTITY).is_err());
    }
}
