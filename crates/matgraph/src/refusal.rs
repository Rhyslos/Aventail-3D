//! Saying why something won't work, and how to make it work.
//!
//! Every refusal in a graph (a wire between plugs that don't fit, a loop,
//! a box that can't run, a setting out of range, a size that can't be
//! used) is a [`Refusal`]: the reason in plain words, and the ways to make
//! it work as data ([`Fix`]), which the interface offers as choices.
//! Nothing changes until one is picked.

use crate::graph::{BoxId, Graph, PlugRef};
use crate::registry::Registry;
use std::fmt;
use words::{text, tr, trf};

/// Why something can't be done, and what would make it work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The reason, in plain words, in the language in use.
    pub reason: String,
    /// Ways to make it work, best first (may be empty).
    pub fixes: Vec<Fix>,
}

impl Refusal {
    /// A refusal with no fixes.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            fixes: Vec::new(),
        }
    }

    /// The same refusal offering `fixes` too.
    pub fn with_fixes(mut self, fixes: impl IntoIterator<Item = Fix>) -> Self {
        self.fixes.extend(fixes);
        self
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.reason)
    }
}

impl std::error::Error for Refusal {}

/// One way to make a refused thing work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fix {
    /// Put a box of this kind between the two plugs (its type id).
    InsertBox {
        /// The box type's id.
        kind: String,
    },
    /// Add a box of this kind and connect it to the empty plug (its type
    /// id).
    AddBox {
        /// The box type's id.
        kind: String,
    },
    /// Connect this output instead.
    ConnectInstead {
        /// The output to connect.
        from: PlugRef,
    },
    /// Work the graph out at this size instead (pixels across).
    UseSize(u32),
    /// Use the nearest value the setting allows.
    UseNearest,
    /// Look at this box: it is the one that can't run.
    LookAt(BoxId),
    /// Remove this box.
    RemoveBox(BoxId),
    /// Switch this bypassed box back on (milestone 16).
    StopBypass(BoxId),
    /// Set the refusing box's palette setting `key` to the project's
    /// palette `name` (milestone 19).
    UsePalette {
        /// The setting's key.
        key: String,
        /// The palette's name.
        name: String,
    },
    /// Give the project the painter's starter palettes, when it has none
    /// (milestone 19).
    AddPalettes,
}

impl Fix {
    /// The fix as a choice to offer, in plain words ("Put a Material from
    /// height box between them").
    pub fn describe(&self, registry: &Registry, graph: &Graph) -> String {
        let box_name = |id: BoxId| {
            graph
                .get(id)
                .and_then(|b| registry.get(&b.kind))
                .map_or_else(|| tr("a box").to_string(), |t| t.info().name.show())
        };
        let kind_name = |kind: &String| {
            registry
                .get(kind)
                .map_or_else(|| kind.clone(), |t| t.info().name.show())
        };
        match self {
            Self::InsertBox { kind } => trf!("Put a {} box between them", kind_name(kind)),
            Self::AddBox { kind } => trf!("Add a {} box and connect it", kind_name(kind)),
            Self::ConnectInstead { from } => trf!("Connect {} instead", box_name(from.node)),
            Self::UseSize(size) => trf!("Use {} pixels across", size),
            Self::UseNearest => tr(text!("Use the nearest value it allows")).to_string(),
            Self::LookAt(id) => trf!("Look at {}", box_name(*id)),
            Self::RemoveBox(id) => trf!("Remove {}", box_name(*id)),
            Self::StopBypass(id) => trf!("Switch {} back on", box_name(*id)),
            Self::UsePalette { name, .. } => trf!("Use the palette {}", name),
            Self::AddPalettes => tr(text!("Add the starter palettes to this project")).to_string(),
        }
    }
}
