//! How much memory the undo history may use: the choices offered in
//! Edit → Undo memory, and how memory sizes are written.

use scene::HistoryLimit;
use serde::{Deserialize, Serialize};

/// A choice of undo memory, as offered in the Edit menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UndoMemory {
    /// 256 MB: for computers short of memory.
    Mb256,
    /// 512 MB.
    Mb512,
    /// 1 GB.
    Gb1,
    /// 2 GB: the standard.
    #[default]
    Gb2,
    /// 4 GB.
    Gb4,
    /// 8 GB: for very long sessions on a computer with plenty of memory.
    Gb8,
    /// No limit: every step is kept until the app closes.
    Unlimited,
}

const MB: usize = 1 << 20;
const GB: usize = 1 << 30;

impl UndoMemory {
    /// Every choice, smallest first.
    pub const ALL: [UndoMemory; 7] = [
        Self::Mb256,
        Self::Mb512,
        Self::Gb1,
        Self::Gb2,
        Self::Gb4,
        Self::Gb8,
        Self::Unlimited,
    ];

    /// The limit the document keeps to.
    pub fn limit(self) -> HistoryLimit {
        match self {
            Self::Mb256 => HistoryLimit::Bytes(256 * MB),
            Self::Mb512 => HistoryLimit::Bytes(512 * MB),
            Self::Gb1 => HistoryLimit::Bytes(GB),
            Self::Gb2 => HistoryLimit::Bytes(2 * GB),
            Self::Gb4 => HistoryLimit::Bytes(4 * GB),
            Self::Gb8 => HistoryLimit::Bytes(8 * GB),
            Self::Unlimited => HistoryLimit::Unlimited,
        }
    }

    /// The menu entry's name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Mb256 => "256 MB",
            Self::Mb512 => "512 MB",
            Self::Gb1 => "1 GB",
            Self::Gb2 => "2 GB (standard)",
            Self::Gb4 => "4 GB",
            Self::Gb8 => "8 GB",
            Self::Unlimited => "Unlimited",
        }
    }

    /// The limit, short, for sentences ("2 GB", "no limit").
    pub fn short(self) -> &'static str {
        match self {
            Self::Gb2 => "2 GB",
            Self::Unlimited => "no limit",
            other => other.label(),
        }
    }

    /// What choosing it means, for its tooltip.
    pub fn explanation(self) -> &'static str {
        match self {
            Self::Mb256 | Self::Mb512 => {
                "For a computer short of memory: fewer steps can be undone"
            }
            Self::Gb1 => "Enough for most sessions on a computer with 4 GB or more",
            Self::Gb2 => "Enough for a long session on a computer with 8 GB or more",
            Self::Gb4 | Self::Gb8 => "For very long sessions on a computer with plenty of memory",
            Self::Unlimited => {
                "Keep every step until the app closes. A very long session with large \
                 meshes can then use a lot of memory"
            }
        }
    }
}

/// A memory size in words: "0 KB", "12 KB", "3.4 MB", "1.25 GB".
pub fn format_bytes(bytes: usize) -> String {
    let b = bytes as f64;
    if bytes >= GB {
        format!("{:.2} GB", b / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", b / MB as f64)
    } else {
        format!("{} KB", bytes.div_ceil(1024))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_gigabytes_is_the_standard_and_matches_the_scene() {
        assert_eq!(UndoMemory::default(), UndoMemory::Gb2);
        assert_eq!(UndoMemory::default().limit(), HistoryLimit::STANDARD);
    }

    #[test]
    fn choices_grow_and_end_unlimited() {
        let sizes: Vec<usize> = UndoMemory::ALL
            .iter()
            .filter_map(|m| match m.limit() {
                HistoryLimit::Bytes(b) => Some(b),
                HistoryLimit::Unlimited => None,
            })
            .collect();
        assert_eq!(sizes.len(), 6);
        assert!(sizes.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(UndoMemory::Unlimited.limit(), HistoryLimit::Unlimited);
    }

    #[test]
    fn sizes_read_naturally() {
        assert_eq!(format_bytes(0), "0 KB");
        assert_eq!(format_bytes(1), "1 KB");
        assert_eq!(format_bytes(12 * 1024), "12 KB");
        assert_eq!(format_bytes(3 * MB + MB / 2), "3.5 MB");
        assert_eq!(format_bytes(GB + GB / 4), "1.25 GB");
    }

    #[test]
    fn the_choice_survives_saving() {
        let text = ron::to_string(&UndoMemory::Gb8).unwrap();
        assert_eq!(ron::from_str::<UndoMemory>(&text).unwrap(), UndoMemory::Gb8);
    }
}
