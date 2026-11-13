//! The seams the Blend seams tool shows over the 3D views ([decision 0293](../../../../docs/decisions/0293-blend-seams.md)):
//! every seam of the material faintly, the marked ones in the accent
//! colour, and the one under the mouse in white. A seam both of whose
//! sides face away from the camera is left out, so the back of the model
//! doesn't clutter the front.

use crate::theme;
use egui::{Color32, Rect, Stroke, Ui, pos2};
use glam::{Vec2, Vec3};
use std::sync::Arc;
use tools::Camera;

/// How a seam is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SeamLook {
    /// A seam that could be marked.
    Faint,
    /// A seam the Seam blend layer blends.
    Marked,
    /// The seam line a click would mark.
    Hovered,
}

/// One seam edge in the world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SeamLine {
    /// Its two ends.
    pub(crate) ends: [Vec3; 2],
    /// Which way its two sides face.
    pub(crate) facing: [Vec3; 2],
    /// How it is drawn.
    pub(crate) look: SeamLook,
}

/// What the views draw: the seams (faint and marked, made again only
/// when they change) and those under the mouse.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct SeamLines {
    /// The material's seams.
    pub(crate) all: Arc<Vec<SeamLine>>,
    /// The seam line under the mouse.
    pub(crate) hovered: Vec<SeamLine>,
}

/// Whether either side of `line` faces `camera`.
fn faces(line: &SeamLine, camera: &Camera) -> bool {
    let middle = (line.ends[0] + line.ends[1]) / 2.0;
    let toward = if camera.is_flat() {
        camera.forward()
    } else {
        middle - camera.position
    };
    line.facing.iter().any(|f| f.dot(toward) < 0.0)
}

/// Draws `lines` over the view at `rect` seen through `camera`.
pub(crate) fn paint(ui: &Ui, rect: Rect, camera: &Camera, lines: &SeamLines) {
    let painter = ui.painter_at(rect);
    let size = Vec2::new(rect.width(), rect.height());
    let accent = theme::accent(ui.ctx()).colors().fill;
    let order = [SeamLook::Faint, SeamLook::Marked, SeamLook::Hovered];
    for look in order {
        let (width, colour) = match look {
            SeamLook::Faint => (1.0, Color32::from_white_alpha(70)),
            SeamLook::Marked => (2.5, accent),
            SeamLook::Hovered => (3.0, Color32::WHITE),
        };
        for line in lines.all.iter().chain(&lines.hovered) {
            if line.look != look || !faces(line, camera) {
                continue;
            }
            let (Some(a), Some(b)) = (
                camera.project(line.ends[0], size),
                camera.project(line.ends[1], size),
            ) else {
                continue;
            };
            let (a, b) = (
                rect.min + egui::vec2(a.x, a.y),
                rect.min + egui::vec2(b.x, b.y),
            );
            if look != SeamLook::Faint {
                painter.line_segment(
                    [a + egui::vec2(0.0, 1.0), b + egui::vec2(0.0, 1.0)],
                    Stroke::new(width + 1.0, Color32::from_black_alpha(140)),
                );
            }
            painter.line_segment([pos2(a.x, a.y), pos2(b.x, b.y)], Stroke::new(width, colour));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seam_facing_away_on_both_sides_is_hidden() {
        let camera = Camera::looking_at(Vec3::new(0.0, -10.0, 0.0), Vec3::ZERO);
        let line = |facing: [Vec3; 2]| SeamLine {
            ends: [Vec3::ZERO, Vec3::X],
            facing,
            look: SeamLook::Faint,
        };
        assert!(
            faces(&line([Vec3::NEG_Y, Vec3::Y]), &camera),
            "one side faces it"
        );
        assert!(!faces(&line([Vec3::Y, Vec3::Y]), &camera), "the back");
    }
}
