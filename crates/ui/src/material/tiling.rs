//! The **Tiling** slider ([decision 0270](../../../../docs/decisions/0270-tiling-and-moving-colours.md)):
//! how many times more often a graph's material repeats on a model than
//! at its real-world size, without changing the graph. It sits on a
//! material made by a graph (Attributes → Materials) and on a graph fill
//! layer (the Paint layers panel); both keep it in the layer
//! (`paint::GraphFill::tiling`).
//!
//! The slider runs from ×⅛ to ×32 on a doubling scale, snapping to the
//! doublings (×½, ×1, ×2, ×4…) when close, so the common sizes are easy to
//! hit; any number in between can be typed or dragged to.

use crate::text::tr;
use egui::Ui;
use paint::graph_fill::{MAX_TILING, MIN_TILING};

/// Its name.
pub(crate) const TILING_LABEL: &str = crate::text!("Tiling");

/// How near (in doublings) a value snaps to the nearest doubling.
const SNAP: f32 = 0.08;

/// A Tiling changed by [`control`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Changed {
    /// The new Tiling.
    pub(crate) tiling: f32,
    /// Whether it starts a new undo step (a drag starting, a click, a
    /// typed number) rather than carrying on a drag.
    pub(crate) fresh: bool,
}

/// The slider and its ↺ for Tiling `now`; the new Tiling when changed.
pub(crate) fn control(ui: &mut Ui, now: f32) -> Option<Changed> {
    let mut value = now;
    let response = ui
        .add(
            egui::Slider::new(&mut value, MIN_TILING..=MAX_TILING)
                .logarithmic(true)
                .custom_formatter(|v, _| shown(v as f32))
                .custom_parser(|text| parsed(text).map(f64::from)),
        )
        .on_hover_text(tr(
            "How many times more often the material repeats on the model: ×2 makes it half \
             the size, ×½ twice the size. The graph doesn't change, so the same graph can be \
             used at different sizes on different shapes. What follows the model (From the \
             model boxes) stays where it is.",
        ));
    let fresh = response.drag_started() || !response.dragged();
    let mut changed = response.changed().then(|| Changed {
        tiling: snapped(value),
        fresh,
    });
    if (now - 1.0).abs() > f32::EPSILON
        && ui
            .small_button("↺")
            .on_hover_text(tr("Back to ×1: the material's own real-world size"))
            .clicked()
    {
        changed = Some(Changed {
            tiling: 1.0,
            fresh: true,
        });
    }
    changed.filter(|c| (c.tiling - now).abs() > f32::EPSILON)
}

/// `value` snapped to the nearest doubling when within [`SNAP`] of it,
/// else rounded to two decimals; always within the limits.
pub(crate) fn snapped(value: f32) -> f32 {
    let value = paint::GraphFill::usable_tiling(value);
    let doublings = value.log2();
    let nearest = doublings.round();
    let out = if (doublings - nearest).abs() < SNAP {
        nearest.exp2()
    } else {
        (value * 100.0).round() / 100.0
    };
    out.clamp(MIN_TILING, MAX_TILING)
}

/// Tiling `value` as shown: "×4", "×½", "×1.5".
pub(crate) fn shown(value: f32) -> String {
    let fraction = match value {
        v if (v - 0.5).abs() < 1e-4 => Some("½"),
        v if (v - 0.25).abs() < 1e-4 => Some("¼"),
        v if (v - 0.125).abs() < 1e-4 => Some("⅛"),
        _ => None,
    };
    match fraction {
        Some(f) => format!("×{f}"),
        None => {
            let text = format!("{value:.2}");
            let text = text.trim_end_matches('0').trim_end_matches('.');
            format!("×{text}")
        }
    }
}

/// Tiling typed as "4", "×4", "x4", "1/2", "½" or "0.5"; `None` when it
/// isn't one.
pub(crate) fn parsed(text: &str) -> Option<f32> {
    let text = text.trim().trim_start_matches(['×', 'x', 'X']).trim();
    let value = match text {
        "½" => 0.5,
        "¼" => 0.25,
        "⅛" => 0.125,
        _ => match text.split_once('/') {
            Some((a, b)) => a.trim().parse::<f32>().ok()? / b.trim().parse::<f32>().ok()?,
            None => text.parse::<f32>().ok()?,
        },
    };
    (value.is_finite() && value > 0.0).then(|| value.clamp(MIN_TILING, MAX_TILING))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_snaps_to_doublings_when_close() {
        assert_eq!(snapped(3.9), 4.0);
        assert_eq!(snapped(0.51), 0.5);
        assert_eq!(snapped(1.03), 1.0);
        assert_eq!(snapped(3.0), 3.0);
        assert_eq!(snapped(1.4567), 1.46);
        assert_eq!(snapped(1000.0), MAX_TILING);
        assert_eq!(snapped(0.0), MIN_TILING);
        assert_eq!(snapped(f32::NAN), 1.0);
    }

    #[test]
    fn it_reads_as_times_and_halves() {
        assert_eq!(shown(4.0), "×4");
        assert_eq!(shown(1.0), "×1");
        assert_eq!(shown(0.5), "×½");
        assert_eq!(shown(0.125), "×⅛");
        assert_eq!(shown(1.5), "×1.5");
        assert_eq!(shown(2.75), "×2.75");
    }

    #[test]
    fn typed_tilings_are_read_or_refused() {
        assert_eq!(parsed("4"), Some(4.0));
        assert_eq!(parsed("×4"), Some(4.0));
        assert_eq!(parsed(" x 2.5 "), Some(2.5));
        assert_eq!(parsed("1/2"), Some(0.5));
        assert_eq!(parsed("½"), Some(0.5));
        assert_eq!(parsed("1000"), Some(MAX_TILING));
        assert_eq!(parsed("0"), None);
        assert_eq!(parsed("-2"), None);
        assert_eq!(parsed("1/0"), None);
        assert_eq!(parsed("bricks"), None);
        assert_eq!(parsed(""), None);
        // Every shown value reads back as itself.
        for v in [0.125, 0.25, 0.5, 1.0, 1.5, 4.0, 32.0] {
            assert_eq!(parsed(&shown(v)), Some(v));
        }
    }
}
