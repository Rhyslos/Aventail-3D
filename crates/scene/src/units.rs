//! The project's units ([decision 0068](../../../docs/decisions/0068-pivot-editing-and-scene-units.md)):
//! the length every size in the project is shown and typed in, saved with
//! the project. The scene itself always stores centimetres; the units
//! only change how lengths read (and, later, how exports are scaled).

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
