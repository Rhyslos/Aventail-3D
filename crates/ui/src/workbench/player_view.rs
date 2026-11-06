//! Player-view UVs ([decision 0247](../../../../docs/decisions/0247-player-view-uvs.md)):
//! the automatic UV tools look at the asset from the player's cameras
//! (third person, first person or top-down, at a distance): Suggest seams
//! can hide cuts where the player doesn't look, Pack can give texture
//! where the player looks closest, and the UV Editor can colour texels per
//! screen pixel.

use super::Workbench;
use crate::text::tr;
use bake::player::{DISTANCE_RANGE, PlayerKind, PlayerView, Seen};
use egui::Ui;
use scene::{FaceId, ObjectId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// How the player looks at assets, as chosen and remembered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PlayerChoice {
    /// From behind a character: the standard.
    #[default]
    ThirdPerson,
    /// Through the character's eyes.
    FirstPerson,
    /// From high above at a slant.
    TopDown,
}

impl PlayerChoice {
    /// Every choice, in menu order.
    pub const ALL: [Self; 3] = [Self::ThirdPerson, Self::FirstPerson, Self::TopDown];

    /// The cameras' kind.
    pub fn kind(self) -> PlayerKind {
        match self {
            Self::ThirdPerson => PlayerKind::ThirdPerson,
            Self::FirstPerson => PlayerKind::FirstPerson,
            Self::TopDown => PlayerKind::TopDown,
        }
    }

    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Self::ThirdPerson => tr("Third person"),
            Self::FirstPerson => tr("First person"),
            Self::TopDown => tr("Top-down"),
        }
    }

    /// What it means.
    pub fn help(self) -> &'static str {
        match self {
            Self::ThirdPerson => tr(
                "The camera follows a character at eye height a few metres off (3 m standard): \
                 the tops and sides are seen, the bottom never",
            ),
            Self::FirstPerson => tr(
                "Through the character's eyes, close up (1.5 m standard): everything at eye \
                 height is seen sharpest",
            ),
            Self::TopDown => tr(
                "From high above at a slant, as in strategy and city games (12 m standard): \
                 the tops are seen most, the sides less",
            ),
        }
    }
}

/// The settings section's names.
pub(crate) const PLAYER_VIEW_LABEL: &str = crate::text!("Player view");
pub(crate) const PLAYER_TEXTURE_LABEL: &str =
    crate::text!("Pack gives more texture where the player looks closest");
/// The UV Editor's view switch.
pub(crate) const TEXELS_PER_PIXEL_LABEL: &str = crate::text!("Texels per screen pixel");

/// What the player sees of each polygon of one shape.
pub(crate) type SeenMap = Arc<BTreeMap<FaceId, Seen>>;

/// What the player sees of each shown object, made for one shape and one
/// view and kept until either changes.
#[derive(Debug, Default)]
pub(crate) struct PlayerSeen {
    view: Option<PlayerView>,
    by: HashMap<ObjectId, (Arc<scene::Mesh>, SeenMap)>,
}

impl PlayerSeen {
    /// What the player sees of `mesh` (object `id`), made again only
    /// when the shape or the view changed.
    pub(crate) fn of(
        &mut self,
        id: ObjectId,
        mesh: &Arc<scene::Mesh>,
        view: PlayerView,
    ) -> Arc<BTreeMap<FaceId, Seen>> {
        if self.view != Some(view) {
            self.by.clear();
            self.view = Some(view);
        }
        if let Some((made, seen)) = self.by.get(&id)
            && Arc::ptr_eq(made, mesh)
        {
            return Arc::clone(seen);
        }
        let seen = Arc::new(bake::player::player_view(mesh, &view));
        self.by.insert(id, (Arc::clone(mesh), Arc::clone(&seen)));
        seen
    }

    /// What is kept, by object, for the UV Editor.
    pub(crate) fn kept(&self) -> HashMap<ObjectId, Arc<BTreeMap<FaceId, Seen>>> {
        self.by
            .iter()
            .map(|(id, (_, seen))| (*id, Arc::clone(seen)))
            .collect()
    }
}

impl Workbench {
    /// The player's cameras, from the UV settings.
    pub(crate) fn player_view(&self) -> PlayerView {
        let s = &self.uv_settings;
        let distance = if s.player_distance.is_finite() {
            s.player_distance
                .clamp(*DISTANCE_RANGE.start(), *DISTANCE_RANGE.end())
        } else {
            s.player.kind().standard_distance()
        };
        PlayerView {
            kind: s.player.kind(),
            distance,
            ..PlayerView::default()
        }
    }

    /// What the player sees of object `id`'s shape `mesh`.
    pub(crate) fn player_seen(
        &mut self,
        id: ObjectId,
        mesh: &Arc<scene::Mesh>,
    ) -> Arc<BTreeMap<FaceId, Seen>> {
        let view = self.player_view();
        self.player_seen.of(id, mesh, view)
    }

    /// The shape Suggest seams works on: `mesh` itself, or with how much
    /// the player sees each polygon when cuts are hidden where the player
    /// doesn't look.
    pub(crate) fn shape_for_seams(
        &mut self,
        id: ObjectId,
        mesh: &Arc<scene::Mesh>,
    ) -> Arc<scene::Mesh> {
        if self.uv_settings.seam_hiding != super::unwrapping::SeamHiding::Player {
            return Arc::clone(mesh);
        }
        let seen: BTreeMap<FaceId, f32> = self
            .player_seen(id, mesh)
            .iter()
            .map(|(f, s)| (*f, s.seen))
            .collect();
        Arc::new(uv::seams::with_player_view(mesh, &seen))
    }

    /// How much more texture the piece `faces` of object `id` gets by how
    /// close the player sees it, against the object's average (1 when
    /// Pack doesn't use the player view): from half to twice as much
    /// across.
    pub(crate) fn player_scale(
        &mut self,
        id: ObjectId,
        mesh: &Arc<scene::Mesh>,
        faces: &[FaceId],
    ) -> f32 {
        if !self.uv_settings.player_texture {
            return 1.0;
        }
        let seen = self.player_seen(id, mesh);
        let average = |faces: &mut dyn Iterator<Item = f32>| {
            let (sum, n) = faces.fold((0.0, 0), |(s, n), x| (s + x, n + 1));
            (n > 0).then(|| sum / n as f32)
        };
        let all = average(&mut seen.values().map(|s| s.pixels_per_metre));
        let piece = average(
            &mut faces
                .iter()
                .filter_map(|f| seen.get(f))
                .map(|s| s.pixels_per_metre),
        );
        match (all, piece) {
            (Some(all), Some(piece)) if all > 1e-6 => (piece / all).sqrt().clamp(0.5, 2.0),
            _ => 1.0,
        }
    }
}

impl super::automation::UvSettingsView<'_> {
    /// UV → Settings → Player view.
    pub(crate) fn player_settings(&mut self, ui: &mut Ui) {
        let s = &mut *self.uv_settings;
        ui.label(tr(
            "How the player sees your assets: Suggest seams can hide cuts where they don't look \
             (Seam suggestions → Where the player doesn't look), and Pack can give more texture \
             where they look closest",
        ));
        let before = s.player;
        for choice in PlayerChoice::ALL {
            ui.radio_value(&mut s.player, choice, choice.name())
                .on_hover_text(choice.help());
        }
        if s.player != before {
            s.player_distance = s.player.kind().standard_distance();
        }
        ui.horizontal(|ui| {
            ui.label(tr("Distance"));
            ui.add(
                egui::DragValue::new(&mut s.player_distance)
                    .range(DISTANCE_RANGE)
                    .speed(0.05)
                    .max_decimals(2)
                    .suffix(tr(" m")),
            )
            .on_hover_text(tr(
                "How far the player's camera is from the asset's middle, in metres",
            ));
        });
        ui.checkbox(&mut s.player_texture, tr(PLAYER_TEXTURE_LABEL))
            .on_hover_text(tr(
                "Pieces the player sees up close get up to twice as much texture across, pieces \
                 seen only from far off or never down to half, against the object's average",
            ));
    }
}
