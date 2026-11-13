//! The seams the Blend seams tool shows in the UV Editor ([decision 0295](../../../../docs/decisions/0295-picking-seams-in-the-uv-editor.md)):
//! the same seams as over the 3D views, drawn flat on the texture. Each
//! seam is one place on the model but two places on the texture, so it
//! shows as two lines, one along each piece it joins: faint, in the
//! accent colour when marked, and white under the mouse. Both of a seam's
//! lines always light together, so you see which pieces meet there.

use super::UvEditor;
use crate::theme;
use crate::viewport::SeamLook;
use egui::{Color32, Painter, Rect, Stroke, vec2};
use glam::Vec2;
use std::sync::Arc;

/// One side of a seam edge on the texture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct UvSeamLine {
    /// Its two ends, on the texture (0 to 1 across, V up).
    pub(crate) uvs: [Vec2; 2],
    /// How it is drawn.
    pub(crate) look: SeamLook,
}

/// What the UV Editor draws: the seams' sides (faint and marked, made
/// again only when they change) and those of the seam line under the
/// mouse, in either view.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct UvSeamLines {
    /// The material's seams on the objects shown, both sides of each.
    pub(crate) all: Arc<Vec<UvSeamLine>>,
    /// Both sides of the seam line under the mouse.
    pub(crate) hovered: Vec<UvSeamLine>,
}

impl UvEditor {
    /// Shows the seams the Blend seams tool picks from, or none.
    pub(crate) fn set_seam_lines(&mut self, lines: Option<UvSeamLines>) {
        self.seam_lines = lines;
    }

    /// The seams shown, for tests.
    #[cfg(test)]
    pub(crate) fn seam_lines(&self) -> Option<&UvSeamLines> {
        self.seam_lines.as_ref()
    }

    /// Draws the seams over the UVs, the faint ones first and the line
    /// under the mouse on top.
    pub(super) fn paint_seams(&self, painter: &Painter, rect: Rect) {
        let Some(lines) = &self.seam_lines else {
            return;
        };
        let accent = theme::accent(painter.ctx()).colors().fill;
        for look in [SeamLook::Faint, SeamLook::Marked, SeamLook::Hovered] {
            let (width, colour) = match look {
                SeamLook::Faint => (1.0, Color32::from_white_alpha(70)),
                SeamLook::Marked => (2.5, accent),
                SeamLook::Hovered => (3.0, Color32::WHITE),
            };
            for line in lines.all.iter().chain(&lines.hovered) {
                if line.look != look {
                    continue;
                }
                let [a, b] = line.uvs.map(|uv| self.to_screen(rect, uv));
                if !rect.expand(width).intersects(Rect::from_two_pos(a, b)) {
                    continue;
                }
                if look != SeamLook::Faint {
                    painter.line_segment(
                        [a + vec2(0.0, 1.0), b + vec2(0.0, 1.0)],
                        Stroke::new(width + 1.0, Color32::from_black_alpha(140)),
                    );
                }
                painter.line_segment([a, b], Stroke::new(width, colour));
            }
        }
    }
}
