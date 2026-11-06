//! What a wire carries, and which kinds fit which plugs.
//!
//! There are four kinds. A plug takes its own kind; some other kinds are
//! **converted** with a note (a shape can be used as a height: white is
//! high), and the rest are **refused** with the reason in plain words.

use words::{text, tr};

/// What a wire carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ValueKind {
    /// A black-and-white shape: white inside, black outside (a mask).
    Shape,
    /// A height: greys, light high and dark low.
    Height,
    /// A colour picture.
    Colour,
    /// A whole material: its colour, height, roughness, metal, normal,
    /// occlusion and curvature together, on one wire.
    Material,
}

impl ValueKind {
    /// Every kind, in the order the interface lists them.
    pub const ALL: [Self; 4] = [Self::Shape, Self::Height, Self::Colour, Self::Material];

    /// Its plain name ("shape"), in the language in use.
    pub fn name(self) -> &'static str {
        tr(match self {
            Self::Shape => text!("shape"),
            Self::Height => text!("height"),
            Self::Colour => text!("colour"),
            Self::Material => text!("whole material"),
        })
    }

    /// What it is, as the end of a sentence ("a black-and-white shape").
    pub fn described(self) -> &'static str {
        tr(match self {
            Self::Shape => text!("a black-and-white shape"),
            Self::Height => text!("a height (greys: light is high)"),
            Self::Colour => text!("a colour"),
            Self::Material => text!("a whole material"),
        })
    }

    /// Whether it is held as greys (a shape or a height).
    pub fn is_grey(self) -> bool {
        matches!(self, Self::Shape | Self::Height)
    }
}

/// How a value of one kind goes into a plug of another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fit {
    /// The same kind: it goes in as it is.
    Fits,
    /// Another kind, converted; the note says how, in plain words.
    Converts(String),
}

/// How `from` goes into a plug taking `to`, or why it can't, in plain
/// words (the end of a sentence: "a colour has no inside or outside").
pub fn fit(from: ValueKind, to: ValueKind) -> Result<Fit, &'static str> {
    use ValueKind::{Colour, Height, Material, Shape};
    let converts = |note: &'static str| Ok(Fit::Converts(tr(note).to_string()));
    match (from, to) {
        (a, b) if a == b => Ok(Fit::Fits),
        (Shape, Height) => converts(text!(
            "The shape is used as a height: white parts high, black parts low."
        )),
        (Height, Shape) => converts(text!(
            "The height is used as a shape: light greys count as inside."
        )),
        (Shape | Height, Colour) => converts(text!("The greys are used as a colour.")),
        (Material, Height) => converts(text!("Only the material's height is used.")),
        (Material, Colour) => converts(text!("Only the material's colour is used.")),
        (Colour, Shape) => Err(tr(text!("a colour has no inside or outside"))),
        (Colour, Height) => Err(tr(text!("a colour has no high and low parts"))),
        (Material, Shape) => Err(tr(text!(
            "a whole material is several maps together, with no single shape"
        ))),
        (_, Material) => Err(tr(text!(
            "a whole material is several maps together (colour, height, roughness and more), \
             and one map alone isn't one"
        ))),
        // Every pair is listed above; this keeps the match whole.
        _ => Err(tr(text!("they carry different things"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_fits_itself() {
        for k in ValueKind::ALL {
            assert_eq!(fit(k, k), Ok(Fit::Fits));
        }
    }

    #[test]
    fn greys_convert_and_colours_are_refused_as_shapes() {
        use ValueKind::*;
        assert!(matches!(fit(Shape, Height), Ok(Fit::Converts(_))));
        assert!(matches!(fit(Height, Shape), Ok(Fit::Converts(_))));
        assert!(matches!(fit(Height, Colour), Ok(Fit::Converts(_))));
        assert!(matches!(fit(Material, Colour), Ok(Fit::Converts(_))));
        let why = fit(Colour, Shape).unwrap_err();
        assert!(why.contains("inside or outside"), "{why}");
        for k in [Shape, Height, Colour] {
            assert!(fit(k, Material).is_err());
        }
        assert!(fit(Material, Shape).is_err());
    }

    #[test]
    fn every_pair_has_an_answer_in_words() {
        for a in ValueKind::ALL {
            for b in ValueKind::ALL {
                match fit(a, b) {
                    Ok(Fit::Fits) => assert_eq!(a, b),
                    Ok(Fit::Converts(note)) => assert!(note.len() > 10),
                    Err(why) => assert!(why.len() > 10),
                }
            }
        }
    }
}
