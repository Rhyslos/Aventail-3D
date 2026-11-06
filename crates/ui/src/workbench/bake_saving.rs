//! The Bake window's settings kept with the project ([decision 0254](../../../../docs/decisions/0254-bake-settings-in-the-project.md)):
//! which maps, what to bake from, the cage, the Godot material and the
//! rest are written into the project as text (`Scene::bake_settings`),
//! so they are there again when the project is opened. Not undoable,
//! like the UV settings; a change counts as a change to save.

use super::Workbench;
use super::baker::Baker;
use bake::maps::{CAGE_RANGE, MapKind};
use scene::ObjectId;
use serde::{Deserialize, Serialize};

/// The Bake window's settings as kept. Settings missing from the text
/// (from an older or a damaged one) read as their standard value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct BakeSettings {
    /// The maps ticked, by the end of their file names ("normal").
    pub(crate) maps: Vec<String>,
    /// The high-detail object baked from (its id), if any.
    pub(crate) high: Option<u64>,
    pub(crate) by_name: bool,
    pub(crate) show_cage: bool,
    pub(crate) skip_stacked: bool,
    pub(crate) godot: bool,
    pub(crate) roughness: f32,
    pub(crate) metal: bool,
    pub(crate) cage: f32,
    pub(crate) green_down: bool,
}

impl Default for BakeSettings {
    fn default() -> Self {
        Self::of(&Baker::default())
    }
}

impl BakeSettings {
    /// The window's settings now.
    pub(crate) fn of(baker: &Baker) -> Self {
        Self {
            maps: MapKind::ALL
                .iter()
                .zip(baker.maps)
                .filter(|(_, on)| *on)
                .map(|(k, _)| k.file_suffix().to_string())
                .collect(),
            high: baker.high.map(ObjectId::get),
            by_name: baker.by_name,
            show_cage: baker.show_cage,
            skip_stacked: baker.skip_stacked,
            godot: baker.godot,
            roughness: baker.roughness,
            metal: baker.metal,
            cage: baker.cage,
            green_down: baker.green_down,
        }
    }

    /// Gives the window these settings; numbers out of range (from a
    /// damaged text) are brought within it, or the standard if not
    /// numbers.
    pub(crate) fn give(&self, baker: &mut Baker) {
        let standard = Baker::default();
        for (on, kind) in baker.maps.iter_mut().zip(MapKind::ALL) {
            *on = self.maps.iter().any(|m| m == kind.file_suffix());
        }
        baker.high = self.high.map(ObjectId::from_raw);
        baker.by_name = self.by_name;
        baker.show_cage = self.show_cage;
        baker.skip_stacked = self.skip_stacked;
        baker.godot = self.godot;
        baker.roughness = if self.roughness.is_finite() {
            self.roughness.clamp(0.0, 1.0)
        } else {
            standard.roughness
        };
        baker.metal = self.metal;
        baker.cage = if self.cage.is_finite() {
            self.cage.clamp(*CAGE_RANGE.start(), *CAGE_RANGE.end())
        } else {
            standard.cage
        };
        baker.green_down = self.green_down;
    }

    /// The text kept in the project: empty for the standard settings, so
    /// a project nobody baked in keeps nothing.
    pub(crate) fn text(&self) -> String {
        if *self == Self::default() {
            return String::new();
        }
        ron::to_string(self).unwrap_or_default()
    }

    /// The settings a project's text holds: the standard ones for an
    /// empty or unreadable text.
    pub(crate) fn read(text: &str) -> Self {
        if text.trim().is_empty() {
            return Self::default();
        }
        ron::from_str(text).unwrap_or_default()
    }
}

impl Workbench {
    /// Keeps the Bake window's settings with the project when they have
    /// changed. Waits while a mouse button is down, so dragging the cage
    /// slider counts as one change, not one each frame.
    pub(super) fn keep_bake_settings(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.pointer.any_down()) {
            return;
        }
        let text = BakeSettings::of(&self.baker).text();
        if text != self.document.scene().bake_settings() {
            self.document.set_bake_settings(&text);
        }
    }

    /// Gives the Bake window the settings kept in the project (just
    /// opened, or new).
    pub(super) fn load_bake_settings(&mut self) {
        BakeSettings::read(self.document.scene().bake_settings()).give(&mut self.baker);
    }
}
