//! Settings saved with a project: its units ([decision 0068](../../../docs/decisions/0068-pivot-editing-and-scene-units.md)),
//! the length every size in the project is shown and typed in (the scene
//! itself always stores centimetres; the units only change how lengths
//! read), and its game project folder ([decision 0076](../../../docs/decisions/0076-engine-export-paths.md)).

/// The length a project is measured in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Units {
    /// Metres: 1 m, 2.5 m. The standard; what Godot and Unity use.
    #[default]
    Metres,
    /// Centimetres: 100 cm, 250 cm. What Unreal uses.
    Centimetres,
}

impl Units {
    /// The name, in lower case, for undo steps: "metres".
    pub fn name(self) -> &'static str {
        match self {
            Self::Metres => "metres",
            Self::Centimetres => "centimetres",
        }
    }
}

/// Where a project exports to in a game project (decision 0076): a
/// folder inside a Godot, Unity or Unreal Engine project, and whether
/// every save exports there too. The scene only keeps the folder's path
/// as text; reading and writing files is `fileio`'s and the app's work.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExportTarget {
    /// The folder, as the computer names it.
    pub folder: String,
    /// Whether saving the project exports into the folder too.
    pub on_save: bool,
}

/// The longest folder path a project keeps, in bytes.
pub const MAX_FOLDER_LEN: usize = 4096;

impl ExportTarget {
    /// Whether the folder's path is one a project can keep: not empty,
    /// not too long, on one line.
    pub fn is_valid(&self) -> bool {
        !self.folder.trim().is_empty()
            && self.folder.len() <= MAX_FOLDER_LEN
            && !self.folder.chars().any(char::is_control)
    }
}
