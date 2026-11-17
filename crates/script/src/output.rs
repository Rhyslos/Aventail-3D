//! What a run says, sent to whoever started it as it happens: the lines
//! a script prints, its warnings, notes and mistakes, how far it has got,
//! and how it ended. Printed output is cut to [`MAX_LINES`] a run and
//! [`MAX_LINE_BYTES`] a line, so a script can't fill the app's memory
//! with text.

use crate::problem::Problem;
use std::cell::Cell;
use std::sync::mpsc::Sender;
use words::trf;

/// The most lines a run may print (said and warned together).
pub const MAX_LINES: usize = 1_000;
/// The most bytes kept of one printed line.
pub const MAX_LINE_BYTES: usize = 4 << 10;

/// One line in the output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// What the script said (`print`, `Script:say`).
    Said(String),
    /// A warning the script gave (`Script:warn`).
    Warning(String),
    /// A note from the app (a header line it couldn't read, a version
    /// note, lines left out).
    Note(String),
    /// A mistake, with its place.
    Mistake(Problem),
}

/// How a run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ending {
    /// It ran to its end.
    Finished,
    /// It had mistakes before running, or stopped on one while running.
    Failed,
    /// Someone pressed Stop (or Esc).
    Stopped,
    /// It ran past its time limit.
    OutOfTime,
    /// It used more memory than it may.
    OutOfMemory,
}

/// What a run that ended well changed, to carry out on the real document
/// as one undo step ([`scene::Document::execute_batch`]), while the
/// document is still held as the run found it.
#[derive(Debug, Clone, PartialEq)]
pub struct Landing {
    /// The changes, in the order the script made them.
    pub changes: Vec<scene::Change>,
    /// The run's copy of the scene once they were made: the real
    /// document's scene is the same after landing them.
    pub scene: scene::Scene,
}

/// Something a run says, in the order it happens.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A line for the output.
    Line(Line),
    /// How far it has got (0 to 1), and what it's doing.
    Progress(f32, String),
    /// What it changed: sent just before [`Event::Ended`] with
    /// [`Ending::Finished`], when it changed anything.
    Land(Box<Landing>),
    /// How it ended: the last event of a run.
    Ended(Ending),
}

/// Where a run's events go, with the count of lines printed.
pub(crate) struct Output {
    sender: Sender<Event>,
    printed: Cell<usize>,
}

impl Output {
    pub(crate) fn new(sender: Sender<Event>) -> Self {
        Self {
            sender,
            printed: Cell::new(0),
        }
    }

    /// Sends an event (dropped if no one is listening any more).
    pub(crate) fn send(&self, event: Event) {
        let _ = self.sender.send(event);
    }

    /// A line the script said.
    pub(crate) fn say(&self, text: &str) {
        if let Some(text) = self.counted(text) {
            self.send(Event::Line(Line::Said(text)));
        }
    }

    /// A warning the script gave.
    pub(crate) fn warn(&self, text: &str) {
        if let Some(text) = self.counted(text) {
            self.send(Event::Line(Line::Warning(text)));
        }
    }

    /// A note from the app.
    pub(crate) fn note(&self, text: String) {
        self.send(Event::Line(Line::Note(text)));
    }

    /// A mistake.
    pub(crate) fn mistake(&self, problem: Problem) {
        self.send(Event::Line(Line::Mistake(problem)));
    }

    /// How far the script has got.
    pub(crate) fn progress(&self, done: f64, what: &str) {
        let done = if done.is_nan() {
            0.0
        } else {
            done.clamp(0.0, 1.0) as f32
        };
        self.send(Event::Progress(done, capped_bytes(what, 200)));
    }

    /// `text`, cut to a line's length, if there's still room for it; the
    /// first line past the limit says so instead.
    fn counted(&self, text: &str) -> Option<String> {
        let n = self.printed.get() + 1;
        self.printed.set(n);
        if n == MAX_LINES + 1 {
            self.note(trf!(
                "The script printed more than {} lines; the rest are left out.",
                MAX_LINES
            ));
        }
        (n <= MAX_LINES).then(|| capped_bytes(text, MAX_LINE_BYTES))
    }
}

/// `text` cut to at most `most` bytes, at a character, with "…" if cut.
fn capped_bytes(text: &str, most: usize) -> String {
    if text.len() <= most {
        return text.to_string();
    }
    let mut end = most;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printing_is_cut_by_lines_and_bytes() {
        let (sender, events) = std::sync::mpsc::channel();
        let output = Output::new(sender);
        output.say(&"é".repeat(MAX_LINE_BYTES));
        for _ in 0..MAX_LINES + 10 {
            output.say("again");
        }
        let all: Vec<Event> = events.try_iter().collect();
        assert_eq!(all.len(), MAX_LINES + 1);
        let Event::Line(Line::Said(first)) = &all[0] else {
            panic!("{all:?}")
        };
        assert!(first.len() <= MAX_LINE_BYTES + 3);
        assert!(matches!(&all[MAX_LINES], Event::Line(Line::Note(n)) if n.contains("left out")));
    }

    #[test]
    fn progress_is_kept_between_nought_and_one() {
        let (sender, events) = std::sync::mpsc::channel();
        let output = Output::new(sender);
        output.progress(f64::NAN, "x");
        output.progress(7.0, "y");
        let all: Vec<Event> = events.try_iter().collect();
        assert_eq!(
            all,
            vec![
                Event::Progress(0.0, "x".into()),
                Event::Progress(1.0, "y".into())
            ]
        );
    }
}
