//! The shared-paint note ([decision 0214](../../../../docs/decisions/0214-shared-paint-warning.md)):
//! when the shape being painted shares its material's texture with other
//! shapes whose UVs lie over its own, paint on one shows on the others.
//! The Paint layers panel says so, names them, and offers two fixes.

use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui};
use scene::ObjectId;

/// Paint shared by accident: what the workbench found for the shape
/// being painted.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SharedPaint {
    /// The shape being painted.
    pub(crate) owner: ObjectId,
    /// The material whose texture they share.
    pub(crate) material: String,
    /// The other shapes whose UVs lie over the owner's on that texture,
    /// with their names.
    pub(crate) others: Vec<(ObjectId, String)>,
}

/// What the note's buttons asked for, for the workbench.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SharedFix {
    /// Give the shape its own copy of the material, and so its own
    /// texture.
    OwnTexture,
    /// Lay all the shapes' UVs side by side on the shared texture.
    PackTogether,
}

/// The "Own texture" button's text.
pub(crate) const OWN_TEXTURE: &str = crate::text!("Give it its own texture");
/// The "Pack" button's text.
pub(crate) const PACK_TOGETHER: &str = crate::text!("Pack them onto one texture");

/// Draws the note for `shared` when `material` is the one shown; returns
/// the fix clicked.
pub(crate) fn show(ui: &mut Ui, shared: &SharedPaint, material: &str) -> Option<SharedFix> {
    if shared.material != material || shared.others.is_empty() {
        return None;
    }
    let names = list_of(shared.others.iter().map(|(_, n)| n.as_str()));
    let mut fix = None;
    egui::Frame::new()
        .fill(theme::FIELD)
        .stroke(egui::Stroke::new(1.0, theme::WARNING))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.label(
                RichText::new(crate::trf!(
                    "Paint here shows on {} too: they share this material's texture, and their \
                     UVs lie on top of this shape's.",
                    names
                ))
                .color(theme::WARNING),
            );
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(tr(OWN_TEXTURE))
                    .on_hover_text(tr(
                        "Give this shape a copy of the material, with a copy of its painting, so \
                         from now on its paint stays on it. One more texture in the game",
                    ))
                    .clicked()
                {
                    fix = Some(SharedFix::OwnTexture);
                }
                if ui
                    .button(tr(PACK_TOGETHER))
                    .on_hover_text(tr(
                        "Lay the UVs of all these shapes side by side on the shared texture, so \
                         each has its own part of it. Their paint moves with them. One texture \
                         for them all, as games like",
                    ))
                    .clicked()
                {
                    fix = Some(SharedFix::PackTogether);
                }
            });
        });
    ui.add_space(4.0);
    fix
}

/// "A", "A and B", "A, B and C", "A, B and 3 others".
fn list_of<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let names: Vec<&str> = names.collect();
    match names.as_slice() {
        [] => String::new(),
        [one] => (*one).to_string(),
        [a, b] => crate::trf!("{} and {}", a, b),
        [a, b, c] => crate::trf!("{}, {} and {}", a, b, c),
        [a, b, rest @ ..] => crate::trf!("{}, {} and {} others", a, b, rest.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_listed_in_plain_words() {
        let n = |v: &[&str]| list_of(v.iter().copied());
        assert_eq!(n(&[]), "");
        assert_eq!(n(&["Cube"]), "Cube");
        assert_eq!(n(&["Cube", "Cylinder"]), "Cube and Cylinder");
        assert_eq!(n(&["A", "B", "C"]), "A, B and C");
        assert_eq!(n(&["A", "B", "C", "D", "E"]), "A, B and 3 others");
    }
}
