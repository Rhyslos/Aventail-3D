//! The realism channels' rows (decision 0207): roughness, metal and glow,
//! each on or off with its value, for a fill layer in the Paint layers
//! panel and for the brush in Attributes.

use crate::text::tr;
use egui::{Color32, Ui};
use paint::{Channel, ChannelPart, Channels};

/// A channel's name in the panels.
pub(crate) fn channel_name(channel: Channel) -> &'static str {
    match channel {
        Channel::Roughness => tr("Roughness"),
        Channel::Metal => tr("Metal"),
        Channel::Glow => tr("Glow"),
    }
}

/// What a channel does, for its tooltip.
pub(crate) fn channel_hint(channel: Channel) -> &'static str {
    match channel {
        Channel::Roughness => tr(
            "How rough the surface is: 0% is smooth and shiny (polished metal, wet paint, \
             glass), 100% is rough and dull (chalk, cloth, old wood)",
        ),
        Channel::Metal => tr(
            "Whether the surface is metal: 100% for bare metal, 0% for everything else \
             (paint, rust, wood, stone). In between only for a soft change at an edge",
        ),
        Channel::Glow => tr(
            "Light the surface gives off in this colour, as screens, lamps and lava do. \
             Black gives none",
        ),
    }
}

/// One channel's row in a grid: a tick for whether it is changed and its
/// value (a percentage, or a colour for glow). Whether either changed;
/// `dragging` is set while a slider is held.
pub(crate) fn channel_row(
    ui: &mut Ui,
    channel: Channel,
    on: &mut bool,
    value: &mut [u8; 4],
    dragging: &mut bool,
) -> bool {
    let mut changed = false;
    ui.label(tr(channel_name(channel)))
        .on_hover_text(tr(channel_hint(channel)));
    ui.horizontal(|ui| {
        changed |= ui
            .checkbox(on, "")
            .on_hover_text(tr("Whether it changes this channel"))
            .changed();
        ui.add_enabled_ui(*on, |ui| {
            if channel.is_grey() {
                let mut percent = f32::from(value[0]) / 255.0 * 100.0;
                let response = ui.add(
                    egui::Slider::new(&mut percent, 0.0..=100.0)
                        .suffix("%")
                        .max_decimals(0),
                );
                if response.dragged() || response.has_focus() {
                    *dragging = true;
                }
                if response.changed() {
                    let v = (percent / 100.0 * 255.0).round().clamp(0.0, 255.0) as u8;
                    if v != value[0] {
                        *value = [v, v, v, 255];
                        changed = true;
                    }
                }
            } else {
                let mut c = Color32::from_rgb(value[0], value[1], value[2]);
                if ui.color_edit_button_srgba(&mut c).changed() {
                    let [r, g, b, _] = c.to_srgba_unmultiplied();
                    *value = [r, g, b, 255];
                    changed = true;
                }
            }
        });
    });
    ui.end_row();
    changed
}

/// A fill layer's channel rows: each channel it changes, with its value.
/// Whether any changed.
pub(crate) fn fill_rows(ui: &mut Ui, channels: &mut Channels, dragging: &mut bool) -> bool {
    let mut changed = false;
    for channel in Channel::ALL {
        let part = channels.get_mut(channel);
        let mut on = part.is_some();
        let mut value = match part {
            Some(ChannelPart::Fill(value)) => *value,
            _ => default_value(channel),
        };
        if channel_row(ui, channel, &mut on, &mut value, dragging) {
            *part = on.then_some(ChannelPart::Fill(value));
            changed = true;
        }
    }
    changed
}

/// The value a channel starts at when ticked: half rough, not metal, a
/// warm glow.
pub(crate) fn default_value(channel: Channel) -> [u8; 4] {
    match channel {
        Channel::Roughness => [128, 128, 128, 255],
        Channel::Metal => [0, 0, 0, 255],
        Channel::Glow => [255, 170, 60, 255],
    }
}
