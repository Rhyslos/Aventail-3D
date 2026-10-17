//! The project's trim sheet ([decision 0106](../../../docs/decisions/0106-trim-sheets.md)):
//! a shared texture made of horizontal rows (a strip of bolts, a panel
//! edge, a rubber seal), each running the full width of the texture
//! square and repeating left to right. Strips of a model are laid
//! straight onto a row rather than unwrapped into a texture of their own.

/// The most rows a trim sheet keeps.
pub const MAX_TRIM_ROWS: usize = 64;

/// The longest name a row keeps, in bytes.
pub const MAX_ROW_NAME: usize = 64;

/// One row of a trim sheet: a band of the texture square from `bottom`
/// to `top` (as shares of its height: 0 is the bottom, 1 the top).
#[derive(Debug, Clone, PartialEq)]
pub struct TrimRow {
    /// What the row holds, for the menus ("Bolts", "Panel edge").
    pub name: String,
    /// Where the row starts, from the bottom of the square (0 to 1).
    pub bottom: f32,
    /// Where it ends (above `bottom`, at most 1).
    pub top: f32,
}

impl TrimRow {
    /// How tall the row is, as a share of the square's height.
    pub fn height(&self) -> f32 {
        self.top - self.bottom
    }
}

/// The project's trim sheet: its rows, bottom to top as the user made
/// them (they may be in any order, but don't overlap).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrimSheet {
    /// The rows.
    pub rows: Vec<TrimRow>,
}

impl TrimSheet {
    /// A sheet of `n` rows of equal height, named "Row 1" (at the top)
    /// to "Row n".
    pub fn even(n: usize) -> Self {
        let n = n.clamp(1, MAX_TRIM_ROWS);
        let h = 1.0 / n as f32;
        Self {
            rows: (0..n)
                .map(|i| TrimRow {
                    name: format!("Row {}", i + 1),
                    top: 1.0 - h * i as f32,
                    bottom: (1.0 - h * (i + 1) as f32).max(0.0),
                })
                .collect(),
        }
    }

    /// Whether the sheet is one a project can keep: at most
    /// [`MAX_TRIM_ROWS`] rows, each named (at most [`MAX_ROW_NAME`]
    /// bytes, on one line) and lying within the square with some height,
    /// none overlapping another.
    pub fn is_valid(&self) -> bool {
        if self.rows.len() > MAX_TRIM_ROWS {
            return false;
        }
        let each = self.rows.iter().all(|r| {
            !r.name.trim().is_empty()
                && r.name.len() <= MAX_ROW_NAME
                && !r.name.chars().any(char::is_control)
                && r.bottom.is_finite()
                && r.top.is_finite()
                && r.bottom >= 0.0
                && r.top <= 1.0
                && r.top > r.bottom
        });
        let mut sorted: Vec<&TrimRow> = self.rows.iter().collect();
        sorted.sort_by(|a, b| a.bottom.total_cmp(&b.bottom));
        each && sorted.windows(2).all(|w| w[0].top <= w[1].bottom + 1e-6)
    }

    /// The row that holds the band from `bottom` to `top` exactly (to
    /// within `slack`), if any.
    pub fn row_at(&self, bottom: f32, top: f32, slack: f32) -> Option<usize> {
        self.rows
            .iter()
            .position(|r| (r.bottom - bottom).abs() <= slack && (r.top - top).abs() <= slack)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Document};

    #[test]
    fn even_sheets_fill_the_square_and_bad_ones_are_refused() {
        let four = TrimSheet::even(4);
        assert!(four.is_valid());
        assert_eq!(four.rows.len(), 4);
        assert_eq!(four.rows[0].name, "Row 1");
        assert!((four.rows[0].top - 1.0).abs() < 1e-6);
        assert!(four.rows[3].bottom.abs() < 1e-6);
        assert!((four.rows[1].height() - 0.25).abs() < 1e-6);
        assert!(TrimSheet::default().is_valid());
        let row = |name: &str, bottom: f32, top: f32| TrimRow {
            name: name.into(),
            bottom,
            top,
        };
        for bad in [
            vec![row("", 0.0, 0.5)],
            vec![row("A\nB", 0.0, 0.5)],
            vec![row("A", 0.5, 0.5)],
            vec![row("A", -0.1, 0.5)],
            vec![row("A", 0.0, 1.5)],
            vec![row("A", f32::NAN, 0.5)],
            vec![row("A", 0.0, 0.6), row("B", 0.5, 1.0)],
        ] {
            assert!(!TrimSheet { rows: bad.clone() }.is_valid(), "{bad:?}");
        }
        assert!(!TrimSheet::even(MAX_TRIM_ROWS + 5).rows.is_empty());
        assert_eq!(four.row_at(0.5, 0.75, 1e-4), Some(1));
        assert_eq!(four.row_at(0.5, 0.7, 1e-4), None);
    }

    #[test]
    fn the_trim_sheet_changes_as_one_undoable_step_and_bad_ones_are_refused() {
        let mut doc = Document::new();
        assert!(doc.scene().trim().rows.is_empty());
        let sheet = TrimSheet::even(3);
        doc.execute(Command::SetTrim(sheet.clone())).unwrap();
        assert_eq!(doc.scene().trim(), &sheet);
        doc.undo();
        assert!(doc.scene().trim().rows.is_empty());
        doc.redo();
        assert_eq!(doc.scene().trim(), &sheet);
        let steps = doc.undo_steps();
        let mut bad = sheet.clone();
        bad.rows[0].top = 2.0;
        assert!(doc.execute(Command::SetTrim(bad)).is_err());
        doc.execute(Command::SetTrim(sheet)).unwrap();
        assert_eq!(doc.undo_steps(), steps, "the same again records nothing");
    }
}
