//! Changes made on the shape canvas from keys, buttons and notes: the
//! mirror line, removing, notes with their fixes, and making pen lines
//! and basic shapes into pieces.

use super::{NoteFix, ShapeNote, report};
use crate::material::MaterialEditor;
use crate::text::tr;
use scene::Document;
use shapes::PieceId;

impl MaterialEditor {
    /// M: the mirror line on or off.
    pub(crate) fn toggle_mirror(&mut self, document: &mut Document, note: &mut Option<String>) {
        let Some(drawing) = self.shape_drawing(document) else {
            return;
        };
        let mut mirror = drawing.mirror();
        mirror.on = !mirror.on;
        let label = if mirror.on {
            tr("Turn the mirror line on")
        } else {
            tr("Turn the mirror line off")
        };
        match drawing.with_mirror(mirror) {
            Ok(d) => report(self.set_drawing(document, d, label, None), note),
            Err(why) => *note = Some(tr(why.describe()).into()),
        }
    }

    /// Delete: the chosen point while editing points, otherwise the
    /// selected pieces.
    pub(super) fn remove_chosen(&mut self, document: &mut Document, note: &mut Option<String>) {
        let Some(drawing) = self.shape_drawing(document) else {
            return;
        };
        let Some(s) = self.shape.as_ref() else { return };
        let changed = match (s.editing, s.point) {
            (Some(id), Some(index)) => {
                let Some(piece) = drawing.piece(id) else {
                    return;
                };
                let path = piece.placed().without_knot(index);
                drawing
                    .with_path(id, path)
                    .map(|d| (d, tr("Remove a point")))
            }
            _ if s.selected.is_empty() => return,
            _ => {
                let ids: Vec<PieceId> = s.selected.iter().copied().collect();
                drawing.without(&ids).map(|d| (d, tr("Remove pieces")))
            }
        };
        match changed {
            Ok((d, label)) => {
                let done = self.set_drawing(document, d, label, None);
                if done.is_ok()
                    && let Some(s) = self.shape.as_mut()
                    && s.point.take().is_none()
                {
                    s.selected.clear();
                    s.editing = None;
                }
                report(done, note);
            }
            Err(why) => self.shape_note(tr(why.describe()), Vec::new(), note),
        }
    }

    /// Shows a note on the canvas, repeated in the status bar.
    pub(crate) fn shape_note(
        &mut self,
        reason: &str,
        fixes: Vec<NoteFix>,
        note: &mut Option<String>,
    ) {
        *note = Some(reason.to_string());
        if let Some(s) = self.shape.as_mut() {
            s.note = Some(ShapeNote {
                reason: reason.to_string(),
                fixes,
            });
        }
    }

    /// Enter, or a click on the pen line's first point (`close`): makes
    /// the line a piece. An open line meant to fill is refused, with ways
    /// to make it work.
    pub(crate) fn finish_pen(
        &mut self,
        document: &mut Document,
        close: bool,
        note: &mut Option<String>,
    ) {
        let Some(pen) = self.shape.as_ref().and_then(|s| s.pen.clone()) else {
            return;
        };
        if !close && pen.knots.len() >= 2 {
            let fixes = vec![NoteFix::CloseIt, NoteFix::KeepAsLine, NoteFix::ThrowAway];
            let reason = tr(shapes::Problem::OpenPath.describe());
            self.shape_note(reason, fixes, note);
            return;
        }
        self.add_pen_piece(document, close, note);
    }

    /// Makes the pen line a piece: closed (to add or cut out) or as a
    /// line.
    pub(crate) fn add_pen_piece(
        &mut self,
        document: &mut Document,
        closed: bool,
        note: &mut Option<String>,
    ) {
        let Some(pen) = self.shape.as_ref().and_then(|s| s.pen.clone()) else {
            return;
        };
        let kind = match (closed, pen.cut) {
            (false, _) => shapes::PieceKind::Line,
            (true, true) => shapes::PieceKind::Cut,
            (true, false) => shapes::PieceKind::Add,
        };
        let path = shapes::Path::new(pen.knots.clone(), closed);
        self.add_piece(document, path, kind, note);
        if let Some(s) = self.shape.as_mut() {
            s.pen = None;
        }
    }

    /// Adds a piece made of `path` doing `kind` on top, named for what it
    /// is, and selects it; a refusal shows as a note.
    pub(crate) fn add_piece(
        &mut self,
        document: &mut Document,
        path: shapes::Path,
        kind: shapes::PieceKind,
        note: &mut Option<String>,
    ) {
        let Some(drawing) = self.shape_drawing(document) else {
            return;
        };
        let n = drawing.next_id().0;
        let (name, label) = match kind {
            shapes::PieceKind::Add => (crate::trf!("Shape {}", n), tr("Draw a shape")),
            shapes::PieceKind::Cut => (crate::trf!("Cut-out {}", n), tr("Draw a cut-out")),
            shapes::PieceKind::Line => (crate::trf!("Line {}", n), tr("Draw a line")),
        };
        match drawing.with_piece(&name, path, kind) {
            Ok(d) => {
                let id = d.next_id().0 - 1;
                let done = self.set_drawing(document, d, label, None);
                if done.is_ok()
                    && let Some(s) = self.shape.as_mut()
                {
                    s.selected = [PieceId(id)].into();
                    s.note = None;
                }
                report(done, note);
            }
            Err(why) => self.shape_note(tr(why.describe()), Vec::new(), note),
        }
    }

    /// Carries out a fix chosen in the note.
    pub(crate) fn carry_out_note_fix(
        &mut self,
        document: &mut Document,
        fix: NoteFix,
        note: &mut Option<String>,
    ) {
        if let Some(s) = self.shape.as_mut() {
            s.note = None;
        }
        match fix {
            NoteFix::CloseIt => self.add_pen_piece(document, true, note),
            NoteFix::KeepAsLine => self.add_pen_piece(document, false, note),
            NoteFix::ThrowAway => {
                if let Some(s) = self.shape.as_mut() {
                    s.pen = None;
                }
            }
        }
    }
}
