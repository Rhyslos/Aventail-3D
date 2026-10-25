//! Which keys were held for a click. egui's `modifiers` are those held at
//! the end of the frame's input; when a frame takes a while (a heavy
//! scene), a quick Ctrl + click and the release of Ctrl can arrive in
//! the same frame, and by its end Ctrl is no longer held, so the click
//! would replace the selection instead of adding to it. The button's own
//! event carries the keys held as it went down or up, so those are read.

use egui::{Event, Modifiers, PointerButton};

/// The keys held for the primary button's click this frame: those held as
/// it went down (this frame, or remembered from the frame it did), else
/// as it came up, else those held now. Called every frame by the
/// workbench, so a press is always remembered.
pub(crate) fn click_modifiers(ctx: &egui::Context) -> Modifiers {
    let id = egui::Id::new("click_keys_pressed");
    let (pressed, released, now) = ctx.input(|i| {
        (
            of_button(&i.events, true),
            of_button(&i.events, false),
            i.modifiers,
        )
    });
    if let Some(keys) = pressed {
        ctx.data_mut(|d| d.insert_temp(id, keys));
        return keys;
    }
    match released {
        Some(at_release) => ctx
            .data(|d| d.get_temp::<Modifiers>(id))
            .unwrap_or(at_release),
        None => now,
    }
}

/// The keys of the primary button's last press (`pressed`) or release in
/// `events`.
fn of_button(events: &[Event], pressed: bool) -> Option<Modifiers> {
    events.iter().rev().find_map(|e| match e {
        Event::PointerButton {
            button: PointerButton::Primary,
            pressed: p,
            modifiers,
            ..
        } if *p == pressed => Some(*modifiers),
        _ => None,
    })
}

/// The keys of the primary button's last press in `events`, or of its last
/// release when it wasn't pressed in them.
#[cfg(test)]
fn of_events(events: &[Event]) -> Option<Modifiers> {
    of_button(events, true).or_else(|| of_button(events, false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn button(pressed: bool, modifiers: Modifiers) -> Event {
        Event::PointerButton {
            pos: egui::pos2(1.0, 1.0),
            button: PointerButton::Primary,
            pressed,
            modifiers,
        }
    }

    #[test]
    fn the_keys_held_at_the_press_win() {
        let events = [
            button(true, Modifiers::COMMAND),
            button(false, Modifiers::COMMAND),
            Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ];
        assert_eq!(of_events(&events), Some(Modifiers::COMMAND));
    }

    #[test]
    fn a_release_alone_gives_its_keys() {
        assert_eq!(
            of_events(&[button(false, Modifiers::SHIFT)]),
            Some(Modifiers::SHIFT)
        );
    }

    #[test]
    fn a_press_is_remembered_until_its_release() {
        // Ctrl held for the press, let go before the button comes up a
        // frame later: the click still had Ctrl.
        let ctx = egui::Context::default();
        let frame = |events: Vec<Event>| {
            let mut keys = Modifiers::NONE;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| keys = click_modifiers(ui.ctx()),
            );
            output.textures_delta.clear();
            keys
        };
        assert_eq!(
            frame(vec![button(true, Modifiers::COMMAND)]),
            Modifiers::COMMAND
        );
        assert_eq!(
            frame(vec![button(false, Modifiers::NONE)]),
            Modifiers::COMMAND
        );
        // No button: the keys held now (none).
        assert_eq!(frame(Vec::new()), Modifiers::NONE);
    }

    #[test]
    fn without_button_events_there_are_none() {
        assert_eq!(of_events(&[]), None);
        let other = Event::PointerButton {
            pos: egui::pos2(1.0, 1.0),
            button: PointerButton::Secondary,
            pressed: true,
            modifiers: Modifiers::COMMAND,
        };
        assert_eq!(of_events(&[other]), None);
    }
}
