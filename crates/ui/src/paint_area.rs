//! The painting area ([decision 0215](../../../docs/decisions/0215-one-painting-area.md)):
//! in Paint mode the viewport and the UV Editor beside it are one place to
//! paint, so the floating brush bar and the paint tools lie over both,
//! not over the viewport alone. Each view says where it was drawn this
//! frame ([`PaintPanes`]); once both are drawn the workbench joins them
//! ([`PaintPanes::area`]) and draws the bar and the tools over the whole.

use egui::Rect;

/// One view in the painting area, as drawn this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Pane {
    /// The view's rectangle.
    pub(crate) rect: Rect,
    /// The highest the brush bar may sit in it: under its header and
    /// name.
    pub(crate) bar_top: f32,
    /// What it draws along its bottom that the tools must not cover (the
    /// viewport's Shading and navigation buttons, the UV Editor's notes).
    pub(crate) bottom: [Option<Rect>; 2],
}

/// The views that make up the painting area this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct PaintPanes {
    pub(crate) viewport: Option<Pane>,
    pub(crate) uv_editor: Option<Pane>,
}

/// The painting area: where the bar and the tools go.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PaintArea {
    /// The whole area.
    pub(crate) rect: Rect,
    /// The top of the brush bar.
    pub(crate) bar_top: f32,
    /// What lies along the bottom, not to be covered.
    pub(crate) bottom: Vec<Rect>,
}

/// How far apart two views may be (the dock's separator) and still count
/// as side by side.
const TOUCHING: f32 = 12.0;

impl PaintPanes {
    /// The painting area: the viewport and the UV Editor joined when they
    /// sit side by side, else the viewport alone. None without a
    /// viewport.
    pub(crate) fn area(&self) -> Option<PaintArea> {
        let view = self.viewport?;
        let mut panes = vec![view];
        if let Some(uv) = self.uv_editor.filter(|uv| side_by_side(view.rect, uv.rect)) {
            panes.push(uv);
        }
        let left = panes
            .iter()
            .map(|p| p.rect.left())
            .fold(f32::INFINITY, f32::min);
        let right = panes
            .iter()
            .map(|p| p.rect.right())
            .fold(f32::NEG_INFINITY, f32::max);
        let top = panes
            .iter()
            .map(|p| p.rect.top())
            .fold(f32::NEG_INFINITY, f32::max);
        let bottom = panes
            .iter()
            .map(|p| p.rect.bottom())
            .fold(f32::INFINITY, f32::min);
        let bar_top = panes
            .iter()
            .map(|p| p.bar_top)
            .fold(f32::NEG_INFINITY, f32::max);
        Some(PaintArea {
            rect: Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom)),
            bar_top,
            bottom: panes
                .iter()
                .flat_map(|p| p.bottom.into_iter().flatten())
                .collect(),
        })
    }
}

/// Whether two views sit side by side: next to each other left and
/// right, mostly at the same height.
fn side_by_side(a: Rect, b: Rect) -> bool {
    let touching =
        (a.right() - b.left()).abs() <= TOUCHING || (b.right() - a.left()).abs() <= TOUCHING;
    let shared = a.bottom().min(b.bottom()) - a.top().max(b.top());
    touching && shared >= 0.5 * a.height().min(b.height())
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};

    fn pane(x: f32, y: f32, w: f32, h: f32) -> Pane {
        Pane {
            rect: Rect::from_min_size(pos2(x, y), vec2(w, h)),
            bar_top: y + 40.0,
            bottom: [
                Some(Rect::from_min_size(
                    pos2(x + 10.0, y + h - 30.0),
                    vec2(80.0, 24.0),
                )),
                None,
            ],
        }
    }

    #[test]
    fn side_by_side_views_make_one_area() {
        let panes = PaintPanes {
            viewport: Some(pane(0.0, 20.0, 500.0, 600.0)),
            uv_editor: Some(pane(504.0, 0.0, 500.0, 640.0)),
        };
        let area = panes.area().unwrap();
        assert_eq!(
            area.rect,
            Rect::from_min_max(pos2(0.0, 20.0), pos2(1004.0, 620.0))
        );
        assert_eq!(area.bar_top, 60.0, "under both headers");
        assert_eq!(area.bottom.len(), 2);
    }

    #[test]
    fn a_uv_editor_elsewhere_is_left_out() {
        // Below the viewport, not beside it.
        let panes = PaintPanes {
            viewport: Some(pane(0.0, 0.0, 500.0, 400.0)),
            uv_editor: Some(pane(0.0, 404.0, 500.0, 300.0)),
        };
        assert_eq!(panes.area().unwrap().rect, panes.viewport.unwrap().rect);
        // Beside it but far off (another window's corner).
        let panes = PaintPanes {
            viewport: Some(pane(0.0, 0.0, 500.0, 400.0)),
            uv_editor: Some(pane(700.0, 0.0, 300.0, 400.0)),
        };
        assert_eq!(panes.area().unwrap().rect, panes.viewport.unwrap().rect);
        // Beside it only for a sliver of its height.
        let panes = PaintPanes {
            viewport: Some(pane(0.0, 0.0, 500.0, 400.0)),
            uv_editor: Some(pane(504.0, 350.0, 300.0, 400.0)),
        };
        assert_eq!(panes.area().unwrap().rect, panes.viewport.unwrap().rect);
    }

    #[test]
    fn no_viewport_no_area_and_the_viewport_alone() {
        assert!(PaintPanes::default().area().is_none());
        let only_uv = PaintPanes {
            viewport: None,
            uv_editor: Some(pane(0.0, 0.0, 300.0, 300.0)),
        };
        assert!(only_uv.area().is_none());
        let only_view = PaintPanes {
            viewport: Some(pane(0.0, 0.0, 300.0, 300.0)),
            uv_editor: None,
        };
        assert_eq!(
            only_view.area().unwrap().rect,
            only_view.viewport.unwrap().rect
        );
    }
}
