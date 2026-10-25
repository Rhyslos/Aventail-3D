//! The status bar's two texts side by side: what is selected on the left,
//! the latest note (or how to move the view) on the right. When both don't
//! fit, each is cut short with "…" and shows in full when the mouse rests
//! on it, so a long note never runs over the text beside it.

use egui::{Label, RichText, TextStyle, Ui, WidgetText};

/// The gap kept between the two texts, in points.
const GAP: f32 = 24.0;
/// The share of the bar a long note may take when both are too long: the
/// note says what just happened, so it gets the larger part.
const NOTE_SHARE: f32 = 0.6;

/// How wide `text` would be on one line.
fn width_of(ui: &Ui, text: &WidgetText) -> f32 {
    text.clone()
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            TextStyle::Body,
        )
        .size()
        .x
}

/// How much room each text gets, as (left, note), from their own widths
/// and the bar's `full` width: each its own width when both fit; when they
/// don't, the note keeps what it needs up to [`NOTE_SHARE`] of the bar
/// (more if the left text is short), and the left text the rest.
pub(super) fn shares(full: f32, left: f32, note: f32) -> (f32, f32) {
    let room = (full - GAP).max(0.0);
    if left + note <= room {
        return (left, note);
    }
    let note_room = note.min((room * NOTE_SHARE).max(room - left)).max(0.0);
    ((room - note_room).max(0.0), note_room)
}

/// Draws `left` on the left of the bar and `note` on its right (with a
/// turning wheel before it when `busy`), each cut short if need be.
pub(super) fn two_sides(ui: &mut Ui, left: String, note: String, busy: bool) {
    let left = WidgetText::from(left);
    let note = WidgetText::from(RichText::new(note).weak());
    let wheel = if busy { 12.0 + GAP / 2.0 } else { 0.0 };
    let full = (ui.available_width() - wheel).max(0.0);
    let (left_room, note_room) = shares(full, width_of(ui, &left), width_of(ui, &note));
    let height = ui.available_height();
    ui.allocate_ui_with_layout(
        egui::vec2(left_room, height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_max_width(left_room);
            ui.add(Label::new(left).truncate());
        },
    );
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(note_room, height),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.set_max_width(note_room);
                ui.add(Label::new(note).truncate());
            },
        );
        // A turning wheel while a save or export is done on another
        // thread (decision 0163).
        if busy {
            ui.add(egui::Spinner::new().size(12.0));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_texts_keep_their_width_when_they_fit() {
        assert_eq!(shares(1000.0, 300.0, 400.0), (300.0, 400.0));
    }

    #[test]
    fn a_long_note_takes_the_room_the_left_text_leaves() {
        // A short left text: the note gets the rest of the bar.
        let (left, note) = shares(1000.0, 200.0, 2000.0);
        assert_eq!(left, 200.0);
        assert_eq!(note, 1000.0 - GAP - 200.0);
        // Both long: the note gets its larger share, and they never
        // overlap.
        let (left, note) = shares(1000.0, 900.0, 2000.0);
        assert!((note - (1000.0 - GAP) * NOTE_SHARE).abs() < 1e-3);
        assert!(left + note <= 1000.0 - GAP + 1e-3);
        // A short note keeps its width; the left text is cut.
        let (left, note) = shares(1000.0, 1500.0, 100.0);
        assert_eq!(note, 100.0);
        assert!((left - (1000.0 - GAP - 100.0)).abs() < 1e-3);
    }

    #[test]
    fn a_tiny_bar_gives_nothing_below_zero() {
        let (left, note) = shares(10.0, 300.0, 400.0);
        assert!(left >= 0.0 && note >= 0.0);
    }
}
