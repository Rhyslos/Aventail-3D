//! See it as the player will ([decision 0200](../../../../docs/decisions/0200-see-it-as-the-player-will.md)):
//! the viewport drawn at the size the model would have on a player's
//! screen at game distance, then enlarged with its pixels kept, so the
//! texture shows with the blur and smaller copies (mipmaps) a game uses.

use serde::{Deserialize, Serialize};
use tools::Camera;

/// How the player sees the model.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerView {
    /// How far from the player's camera the model is, in metres.
    pub distance: f32,
    /// The game's screen height in pixels.
    pub screen_height: u32,
    /// The game camera's vertical field of view, in degrees.
    pub field_of_view: f32,
}

impl Default for PlayerView {
    /// Five metres away on a 1080p screen, a 60° view: a third-person
    /// game looking at a prop across a room.
    fn default() -> Self {
        Self {
            distance: 5.0,
            screen_height: 1080,
            field_of_view: 60.0,
        }
    }
}

/// The screen heights offered.
pub(crate) const SCREEN_HEIGHTS: [u32; 4] = [720, 1080, 1440, 2160];

/// The least a view is shrunk to: tiny models still show a few pixels.
const SMALLEST: f32 = 1.0 / 128.0;

impl PlayerView {
    /// How much smaller than `height_px` (the view's height in pixels)
    /// the image is drawn so the model at `camera`'s focus `focus` is as
    /// big as on the player's screen: 1 when the view is already as far
    /// or farther (shown as is).
    pub fn scale(&self, camera: &Camera, focus: glam::Vec3, height_px: f32) -> f32 {
        let distance_cm = (self.distance.clamp(0.1, 10_000.0)) * 100.0;
        let fov = self.field_of_view.clamp(10.0, 150.0).to_radians();
        // Pixels per centimetre at the model, in the game.
        let game =
            self.screen_height.clamp(120, 8640) as f32 / (2.0 * distance_cm * (fov * 0.5).tan());
        // And in the view now.
        let here = match camera.ortho_height {
            Some(h) => height_px / h.max(1e-3),
            None => {
                let d = camera.position.distance(focus).max(1e-3);
                height_px / (2.0 * d * (camera.fov_y * 0.5).tan().max(1e-4))
            }
        };
        if !game.is_finite() || !here.is_finite() || here <= 0.0 {
            return 1.0;
        }
        (game / here).clamp(SMALLEST, 1.0)
    }
}

/// The note at the top of a view in the player view: how far and on what
/// screen, or that the view is already as small as the player sees it.
pub(super) fn label(ui: &egui::Ui, rect: egui::Rect, player: &PlayerView, as_is: bool) {
    let text = if as_is {
        crate::trf!(
            "As the player sees it: already this small or smaller at {} m",
            format_metres(player.distance)
        )
    } else {
        crate::trf!(
            "As the player sees it: {} m away on a {}p screen",
            format_metres(player.distance),
            player.screen_height
        )
    };
    let at = egui::pos2(rect.center().x, rect.top() + 34.0);
    let font = egui::FontId::proportional(12.0);
    let painter = ui.painter_at(rect);
    let galley = painter.layout_no_wrap(text, font, crate::theme::TEXT);
    let back = egui::Rect::from_center_size(at, galley.size() + egui::vec2(16.0, 8.0));
    painter.rect_filled(back, 5.0, crate::theme::PANEL.gamma_multiply(0.9));
    painter.galley(back.min + egui::vec2(8.0, 4.0), galley, crate::theme::TEXT);
}

/// `metres` with at most one decimal, none when whole.
fn format_metres(metres: f32) -> String {
    let rounded = (metres * 10.0).round() / 10.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}")
    } else {
        format!("{rounded:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn looking_from(distance: f32) -> Camera {
        Camera::looking_at(Vec3::new(0.0, -distance, 0.0), Vec3::ZERO)
    }

    #[test]
    fn the_image_shrinks_by_how_much_farther_the_player_is() {
        let player = PlayerView {
            distance: 5.0,
            screen_height: 1000,
            field_of_view: 45.0,
        };
        // 1 m away on a 1000 pixel tall view at the same angle: five
        // times as big as the player sees it.
        let camera = Camera {
            fov_y: 45f32.to_radians(),
            ..looking_from(100.0)
        };
        let s = player.scale(&camera, Vec3::ZERO, 1000.0);
        assert!((s - 0.2).abs() < 1e-4, "{s}");
        // A bigger screen shows more pixels.
        let big = PlayerView {
            screen_height: 2000,
            ..player
        };
        assert!((big.scale(&camera, Vec3::ZERO, 1000.0) - 0.4).abs() < 1e-4);
        // Already farther than the player: shown as is.
        let far = Camera {
            fov_y: 45f32.to_radians(),
            ..looking_from(1000.0)
        };
        assert_eq!(player.scale(&far, Vec3::ZERO, 1000.0), 1.0);
        // A flat view: by how much of the world it shows.
        let flat = Camera {
            ortho_height: Some(100.0),
            ..camera
        };
        let s = player.scale(&flat, Vec3::ZERO, 1000.0);
        let game = 1000.0 / (2.0 * 500.0 * (22.5f32).to_radians().tan());
        assert!((s - game / 10.0).abs() < 1e-4, "{s}");
    }

    #[test]
    fn odd_numbers_never_break_it() {
        let player = PlayerView {
            distance: f32::NAN,
            ..PlayerView::default()
        };
        let s = player.scale(&looking_from(100.0), Vec3::ZERO, 0.0);
        assert!(s.is_finite() && s > 0.0 && s <= 1.0);
        let s = PlayerView::default().scale(&looking_from(0.0), Vec3::ZERO, 900.0);
        assert!(s.is_finite() && s > 0.0 && s <= 1.0);
        assert_eq!(format_metres(5.0), "5");
        assert_eq!(format_metres(2.54), "2.5");
    }
}
