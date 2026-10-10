//! Navigation controls: which mouse buttons and keys move the view.
//!
//! People arrive from different 3D programs with different habits, so the
//! viewport offers a few schemes to choose from (the picker in the
//! viewport's bottom-right corner). Every scheme zooms with the mouse
//! wheel, and the left button without keys is always left free for the
//! selection and modelling tools.
//!
//! | Scheme | Orbit | Pan | Zoom |
//! |---|---|---|---|
//! | Mouse only (standard) | right | middle | wheel |
//! | Alt + mouse | Alt + left | Alt + middle | Alt + right |
//! | Mouse only, middle zooms | right | Shift + middle | middle |
//! | Middle pans, Alt orbits | Alt + middle | middle | wheel |
//! | Middle orbits | middle | Shift + middle | wheel |
//!
//! With **Alt + mouse everywhere** switched on (the standard), the Alt +
//! mouse moves also work in every other scheme, wherever that scheme
//! doesn't use the same keys itself.

use crate::navigation::{DragKind, MouseButton};
use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Keys held while a drag starts, as far as navigation cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    /// Alt (Option on macOS).
    pub alt: bool,
    /// Shift.
    pub shift: bool,
}

impl Modifiers {
    /// No keys held.
    pub const NONE: Self = Self {
        alt: false,
        shift: false,
    };
    /// Only Alt held.
    pub const ALT: Self = Self {
        alt: true,
        shift: false,
    };
    /// Only Shift held.
    pub const SHIFT: Self = Self {
        alt: false,
        shift: true,
    };
}

/// A set of mouse controls for moving the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Scheme {
    /// Alt + left orbits, Alt + middle pans, Alt + right zooms.
    AltMouse,
    /// Right drag orbits, middle drag pans; no keys needed. The standard,
    /// because it is the easiest to start with.
    #[default]
    MouseOnly,
    /// Right drag orbits, middle drag zooms, Shift + middle pans.
    MouseOnlyMiddleZooms,
    /// Middle drag pans, Alt + middle orbits.
    MiddlePansAltOrbits,
    /// Middle drag orbits, Shift + middle pans.
    MiddleOrbits,
}

impl Scheme {
    /// Every scheme, in the order the picker lists them.
    pub const ALL: [Scheme; 5] = [
        Self::MouseOnly,
        Self::AltMouse,
        Self::MouseOnlyMiddleZooms,
        Self::MiddlePansAltOrbits,
        Self::MiddleOrbits,
    ];

    /// The scheme's name, as the picker shows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::AltMouse => "Alt + mouse",
            Self::MouseOnly => "Mouse only",
            Self::MouseOnlyMiddleZooms => "Mouse only, middle zooms",
            Self::MiddlePansAltOrbits => "Middle pans, Alt orbits",
            Self::MiddleOrbits => "Middle orbits",
        }
    }

    /// The controls in one short line, for the picker and the status bar.
    pub fn summary(self) -> &'static str {
        match self {
            Self::AltMouse => "Alt + left: orbit · Alt + middle: pan · Alt + right or wheel: zoom",
            Self::MouseOnly => "Right drag: orbit · Middle drag: pan · Wheel: zoom",
            Self::MouseOnlyMiddleZooms => {
                "Right drag: orbit · Middle drag: zoom · Shift + middle: pan"
            }
            Self::MiddlePansAltOrbits => "Alt + middle: orbit · Middle drag: pan · Wheel: zoom",
            Self::MiddleOrbits => "Middle drag: orbit · Shift + middle: pan · Wheel: zoom",
        }
    }

    /// What a drag with `button` and `keys` does in this scheme alone.
    pub fn drag(self, button: MouseButton, keys: Modifiers) -> Option<DragKind> {
        use DragKind::{Dolly, Orbit, Pan};
        use MouseButton::{Middle, Primary, Secondary};
        let Modifiers { alt, shift } = keys;
        match self {
            Self::AltMouse => match (button, alt, shift) {
                (Primary, true, false) => Some(Orbit),
                (Middle, true, false) => Some(Pan),
                (Secondary, true, false) => Some(Dolly),
                _ => None,
            },
            Self::MouseOnly => match (button, alt, shift) {
                (Secondary, false, false) => Some(Orbit),
                (Middle, false, false) => Some(Pan),
                _ => None,
            },
            Self::MouseOnlyMiddleZooms => match (button, alt, shift) {
                (Secondary, false, false) => Some(Orbit),
                (Middle, false, false) => Some(Dolly),
                (Middle, false, true) => Some(Pan),
                _ => None,
            },
            Self::MiddlePansAltOrbits => match (button, alt, shift) {
                (Middle, false, false) => Some(Pan),
                (Middle, true, false) => Some(Orbit),
                _ => None,
            },
            Self::MiddleOrbits => match (button, alt, shift) {
                (Middle, false, false) => Some(Orbit),
                (Middle, false, true) => Some(Pan),
                _ => None,
            },
        }
    }
}

/// The user's navigation choices, remembered between sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NavigationSettings {
    /// The mouse controls in use.
    pub scheme: Scheme,
    /// Whether the Alt + mouse moves also work in the other schemes.
    pub alt_everywhere: bool,
    /// Whether orbiting turns the other way (the scene moves against the
    /// mouse instead of with it).
    pub invert_orbit: bool,
    /// Whether orbiting turns around, and zooming moves towards, the point
    /// under the mouse, instead of the middle of the view.
    pub around_cursor: bool,
}

impl Default for NavigationSettings {
    fn default() -> Self {
        Self {
            scheme: Scheme::default(),
            alt_everywhere: true,
            invert_orbit: false,
            around_cursor: true,
        }
    }
}

impl NavigationSettings {
    /// Which navigation a mouse drag starts, if any: the scheme's own
    /// controls first, then the Alt + mouse ones when they work everywhere.
    pub fn drag(&self, button: MouseButton, keys: Modifiers) -> Option<DragKind> {
        self.scheme.drag(button, keys).or_else(|| {
            (self.alt_everywhere && keys.alt)
                .then(|| Scheme::AltMouse.drag(button, keys))
                .flatten()
        })
    }

    /// The mouse movement to apply for a drag of `kind`, turned around
    /// for orbiting when the orbit direction is inverted.
    pub fn movement(&self, kind: DragKind, delta: Vec2) -> Vec2 {
        if kind == DragKind::Orbit && self.invert_orbit {
            -delta
        } else {
            delta
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use DragKind::{Dolly, Orbit, Pan};
    use MouseButton::{Middle, Primary, Secondary};

    const SHIFT_ALT: Modifiers = Modifiers {
        alt: true,
        shift: true,
    };

    fn only(scheme: Scheme) -> NavigationSettings {
        NavigationSettings {
            scheme,
            alt_everywhere: false,
            ..NavigationSettings::default()
        }
    }

    #[test]
    fn alt_mouse_matches_the_original_controls() {
        let s = only(Scheme::AltMouse);
        assert_eq!(s.drag(Primary, Modifiers::ALT), Some(Orbit));
        assert_eq!(s.drag(Middle, Modifiers::ALT), Some(Pan));
        assert_eq!(s.drag(Secondary, Modifiers::ALT), Some(Dolly));
        for button in [Primary, Middle, Secondary] {
            assert_eq!(s.drag(button, Modifiers::NONE), None);
        }
    }

    #[test]
    fn every_scheme_does_what_its_summary_says() {
        let table = [
            (Scheme::MouseOnly, Secondary, Modifiers::NONE, Orbit),
            (Scheme::MouseOnly, Middle, Modifiers::NONE, Pan),
            (
                Scheme::MouseOnlyMiddleZooms,
                Secondary,
                Modifiers::NONE,
                Orbit,
            ),
            (Scheme::MouseOnlyMiddleZooms, Middle, Modifiers::NONE, Dolly),
            (Scheme::MouseOnlyMiddleZooms, Middle, Modifiers::SHIFT, Pan),
            (Scheme::MiddlePansAltOrbits, Middle, Modifiers::NONE, Pan),
            (Scheme::MiddlePansAltOrbits, Middle, Modifiers::ALT, Orbit),
            (Scheme::MiddleOrbits, Middle, Modifiers::NONE, Orbit),
            (Scheme::MiddleOrbits, Middle, Modifiers::SHIFT, Pan),
        ];
        for (scheme, button, keys, want) in table {
            assert_eq!(
                only(scheme).drag(button, keys),
                Some(want),
                "{scheme:?} {button:?} {keys:?}"
            );
        }
    }

    #[test]
    fn a_plain_left_drag_is_always_left_for_the_tools() {
        for scheme in Scheme::ALL {
            let s = NavigationSettings {
                scheme,
                ..NavigationSettings::default()
            };
            assert_eq!(s.drag(Primary, Modifiers::NONE), None, "{scheme:?}");
            assert_eq!(s.drag(Primary, Modifiers::SHIFT), None, "{scheme:?}");
        }
    }

    #[test]
    fn alt_mouse_works_everywhere_when_switched_on() {
        for scheme in Scheme::ALL {
            let s = NavigationSettings {
                scheme,
                ..NavigationSettings::default()
            };
            assert_eq!(s.drag(Primary, Modifiers::ALT), Some(Orbit), "{scheme:?}");
            assert_eq!(s.drag(Secondary, Modifiers::ALT), Some(Dolly), "{scheme:?}");
            let off = NavigationSettings {
                alt_everywhere: false,
                ..s
            };
            if scheme != Scheme::AltMouse {
                assert_eq!(off.drag(Primary, Modifiers::ALT), None, "{scheme:?}");
            }
        }
    }

    #[test]
    fn a_scheme_keeps_its_own_meaning_for_alt() {
        // Alt + middle orbits in this scheme, even though Alt + mouse pans.
        let s = NavigationSettings {
            scheme: Scheme::MiddlePansAltOrbits,
            ..NavigationSettings::default()
        };
        assert_eq!(s.drag(Middle, Modifiers::ALT), Some(Orbit));
    }

    #[test]
    fn unlisted_key_combinations_do_nothing() {
        for scheme in Scheme::ALL {
            for button in [Primary, Middle, Secondary] {
                assert_eq!(only(scheme).drag(button, SHIFT_ALT), None, "{scheme:?}");
            }
        }
    }

    #[test]
    fn schemes_have_distinct_names_and_summaries() {
        for (i, a) in Scheme::ALL.iter().enumerate() {
            assert!(!a.summary().is_empty());
            for b in &Scheme::ALL[i + 1..] {
                assert_ne!(a.name(), b.name());
                assert_ne!(a.summary(), b.summary());
            }
        }
    }

    #[test]
    fn names_stay_neutral() {
        // The app names schemes by what they do, not after other programs.
        for scheme in Scheme::ALL {
            let text = format!("{} {}", scheme.name(), scheme.summary()).to_lowercase();
            for word in ["cinema", "c4d", "max", "blender", "maya"] {
                assert!(!text.contains(word), "{scheme:?} mentions {word}");
            }
        }
    }

    #[test]
    fn inverting_turns_only_the_orbit_around() {
        let s = NavigationSettings {
            invert_orbit: true,
            ..NavigationSettings::default()
        };
        let d = Vec2::new(3.0, -2.0);
        assert_eq!(s.movement(Orbit, d), -d);
        assert_eq!(s.movement(Pan, d), d);
        assert_eq!(s.movement(Dolly, d), d);
        assert_eq!(NavigationSettings::default().movement(Orbit, d), d);
    }

    #[test]
    fn the_standard_settings_are_mouse_only_with_alt_everywhere() {
        let s = NavigationSettings::default();
        assert_eq!(s.scheme, Scheme::MouseOnly);
        assert_eq!(Scheme::ALL[0], Scheme::MouseOnly, "listed first");
        assert!(s.alt_everywhere);
        assert!(!s.invert_orbit);
        assert!(s.around_cursor, "orbit and zoom around the mouse: on");
    }

    #[test]
    fn settings_survive_saving_and_old_files() {
        let s = NavigationSettings {
            scheme: Scheme::MiddleOrbits,
            alt_everywhere: false,
            invert_orbit: true,
            around_cursor: false,
        };
        let text = ron::to_string(&s).unwrap();
        assert_eq!(ron::from_str::<NavigationSettings>(&text).unwrap(), s);
        let empty: NavigationSettings = ron::from_str("()").unwrap();
        assert_eq!(empty, NavigationSettings::default());
    }
}
