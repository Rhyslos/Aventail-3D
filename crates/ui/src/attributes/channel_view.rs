//! Showing one channel ([decision 0213](../../../../docs/decisions/0213-paint-interface-overhaul.md)):
//! at the top of the Paint tab, buttons that make the views show the
//! whole painting or one of its channels on its own (the colour, or with
//! the realism channels the roughness, metal, height or glow), as texture
//! painters do. They choose the paint preview: **All** goes back to the
//! one in use before.

use super::AttributesPanel;
use crate::text::tr;
use crate::theme;
use crate::viewport::PaintPreview;
use egui::{RichText, Ui};

/// What the views show of the painting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ChannelView {
    /// Everything, in the chosen preview (lit, Godot, realistic…).
    #[default]
    All,
    /// The colours alone, with no light.
    Colour,
    /// The roughness as greys.
    Roughness,
    /// The metal as greys.
    Metal,
    /// The height as greys.
    Height,
    /// The glow on black.
    Glow,
}

impl ChannelView {
    /// Every choice, in the order shown.
    pub(crate) const ALL: [ChannelView; 6] = [
        Self::All,
        Self::Colour,
        Self::Roughness,
        Self::Metal,
        Self::Height,
        Self::Glow,
    ];

    /// Its button's name.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => tr("All"),
            Self::Colour => tr("Colour"),
            Self::Roughness => tr("Rough"),
            Self::Metal => tr("Metal"),
            Self::Height => tr("Height"),
            Self::Glow => tr("Glow"),
        }
    }

    /// Whether it needs the realism channels.
    pub(crate) fn needs_realism(self) -> bool {
        !matches!(self, Self::All | Self::Colour)
    }

    /// The paint preview that shows it (`None` for All: the one before).
    pub(crate) fn preview(self) -> Option<PaintPreview> {
        match self {
            Self::All => None,
            Self::Colour => Some(PaintPreview::Unlit),
            Self::Roughness => Some(PaintPreview::Roughness),
            Self::Metal => Some(PaintPreview::Metal),
            Self::Height => Some(PaintPreview::Height),
            Self::Glow => Some(PaintPreview::Glow),
        }
    }

    /// What `preview` shows: one channel, or all (any other preview).
    /// Unlit counts as All: it's a look of its own too.
    pub(crate) fn of(preview: PaintPreview) -> Self {
        match preview {
            PaintPreview::Roughness => Self::Roughness,
            PaintPreview::Metal => Self::Metal,
            PaintPreview::Height => Self::Height,
            PaintPreview::Glow => Self::Glow,
            _ => Self::All,
        }
    }
}

/// Accessible name of the row.
pub(crate) const CHANNELS_LABEL: &str = crate::text!("Show in the views");

impl AttributesPanel {
    /// The row of channel buttons; a click goes to `channel_asked` for the
    /// workbench, which changes the views.
    pub(crate) fn channel_row(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr(CHANNELS_LABEL)).color(theme::TEXT_WEAK))
                .on_hover_text(tr(
                    "Show the whole painting, or one of its channels on its own: handy to \
                     check where you've painted roughness, metal, height or glow",
                ));
            for view in ChannelView::ALL {
                if view.needs_realism() && !self.realism {
                    continue;
                }
                let on = self.channel_shown == view;
                let button = ui.add(egui::Button::selectable(on, view.label()));
                let name = crate::trf!("Show {}", view.label());
                button.widget_info(|| {
                    egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, &name)
                });
                if button
                    .on_hover_text(view.preview().map_or(
                        tr("The whole painting, in the preview chosen in the Shading button"),
                        |p| p.explanation(),
                    ))
                    .clicked()
                {
                    self.channel_asked = Some(view);
                }
            }
        });
        ui.add_space(4.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_channel_has_its_preview_and_back() {
        for view in ChannelView::ALL {
            match view.preview() {
                None => assert_eq!(view, ChannelView::All),
                Some(p) if view == ChannelView::Colour => assert_eq!(p, PaintPreview::Unlit),
                Some(p) => assert_eq!(ChannelView::of(p), view),
            }
        }
        assert_eq!(ChannelView::of(PaintPreview::Godot), ChannelView::All);
        assert!(!ChannelView::Colour.needs_realism());
        assert!(ChannelView::Height.needs_realism());
    }
}
