//! The viewport's options, kept with the settings: the grid's detail,
//! the surface (plain, checker or stretch), the shading, symmetry, and
//! the grid and checker images the renderer draws with.

use crate::selecting::SelectKeys;
use crate::text::tr;
use crate::transforming::{Axes, SnapSettings};
use serde::{Deserialize, Serialize};
use tools::NavigationSettings;

/// How detailed the grid is: how far apart its finest lines are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GridDetail {
    /// Lines every metre (and stronger ones every 10 m): the calmest.
    #[default]
    Metre,
    /// Each metre square cut into four: lines every 50 cm as well.
    HalfMetre,
    /// Thin lines every 10 cm as well: for placing things precisely.
    TenCentimetres,
}

/// What shapes' surfaces show ([decision 0102](../../../../docs/decisions/0102-checker-and-stretch.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Surface {
    /// Their materials' colours.
    #[default]
    Plain,
    /// A checker laid on by their UVs, to see how a texture would sit.
    Checker,
    /// How much their UVs stretch: green where even, through yellow to
    /// red.
    Stretch,
    /// The numbered checker (decision 0107): cells labelled A1 to H8 in
    /// a gradient of the app's accent colour, laid on by their UVs.
    NumberedChecker,
    /// Their painted textures (decision 0177), laid on by their UVs over
    /// their materials' colours.
    Painted,
}

impl Surface {
    /// Every choice, in the order offered.
    pub const ALL: [Surface; 5] = [
        Self::Plain,
        Self::Painted,
        Self::Checker,
        Self::NumberedChecker,
        Self::Stretch,
    ];

    /// Its name in the Display menu.
    pub fn label(self) -> &'static str {
        match self {
            Self::Plain => tr("Colours"),
            Self::Checker => tr("Checker"),
            Self::Stretch => tr("Stretch"),
            Self::NumberedChecker => tr("Numbered checker"),
            Self::Painted => tr("Painted"),
        }
    }

    /// What it shows, for its tooltip.
    pub fn explanation(self) -> &'static str {
        match self {
            Self::Plain => tr("Each surface in its material's colour"),
            Self::Painted => tr(
                "Each material's painted texture, laid on by the UVs over its colour. Where \
                 nothing is painted, the colour shows. The UV Editor shows it behind the \
                 UVs too.",
            ),
            Self::Checker => tr(
                "A checker laid on by the UVs: where the squares stay square and even, a \
                 texture will too. The tint runs red across and green up the texture, so a \
                 piece turned or mirrored shows.",
            ),
            Self::NumberedChecker => tr(
                "A checker of labelled cells laid on by the UVs, A1 at the top left of the \
                 texture to H8 at the bottom right, in colours from the app's accent: see \
                 at once which part of the texture each part of the shape uses, and which \
                 way round it lies (the labels turn and mirror with it). The UV Editor shows \
                 it behind the UVs too.",
            ),
            Self::Stretch => tr(
                "How evenly the UVs lie: green where even, yellow where 1.5 times off, red \
                 where twice off (stretched one way, or too big or small against the rest \
                 of the object), purple where a polygon has no room on the texture",
            ),
        }
    }
}

/// How painted surfaces are lit and shown ([decision 0198](../../../../docs/decisions/0198-paint-previews.md)):
/// the ways a painter checks a texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PaintPreview {
    /// The viewport's soft studio light.
    #[default]
    Lit,
    /// The texture's own colours with no light, as most hand-painted
    /// games show them.
    Unlit,
    /// Cel shading: lit and shaded bands with a hard edge.
    Toon,
    /// Close to how Godot shows it as standard.
    Godot,
    /// The colours as greys, to check light and dark.
    Values,
    /// Blurred, as if squinting.
    Squint,
    /// Godot's look with its toon shading.
    GodotToon,
}

impl PaintPreview {
    /// Every preview, in the order offered.
    pub const ALL: [PaintPreview; 7] = [
        Self::Lit,
        Self::Unlit,
        Self::Toon,
        Self::Godot,
        Self::GodotToon,
        Self::Values,
        Self::Squint,
    ];

    /// Its name in the menus.
    pub fn label(self) -> &'static str {
        match self {
            Self::Lit => tr("Lit"),
            Self::Unlit => tr("Unlit"),
            Self::Toon => tr("Toon"),
            Self::Godot => tr("Godot"),
            Self::Values => tr("Value check"),
            Self::Squint => tr("Squint"),
            Self::GodotToon => tr("Godot toon"),
        }
    }

    /// What it shows, for its tooltip.
    pub fn explanation(self) -> &'static str {
        match self {
            Self::Lit => tr("The viewport's soft studio light over the painting"),
            Self::Unlit => tr(
                "The painting's own colours, with no light: how most hand-painted games \
                 show it, since the light and shade are painted in",
            ),
            Self::Toon => tr(
                "Cartoon (cel) shading: a lit side and a cooler shaded side with a hard \
                 line between, and a thin rim of light around the outline, as toon games \
                 draw it",
            ),
            Self::GodotToon => tr(
                "Godot's look with its own toon shading (a material's Diffuse Mode set to \
                 Toon): the sun's light in a soft step",
            ),
            Self::Godot => tr(
                "Close to Godot's standard look: a sun, the sky's light and Filmic tone \
                 mapping. Its own lights and settings will still change it a little",
            ),
            Self::Values => tr(
                "The painting in greys, to check light against dark: a texture reads well \
                 when its parts differ in lightness, not only in colour",
            ),
            Self::Squint => tr(
                "The painting blurred, as if squinting at it: only big shapes and colours \
                 remain, as from far away in a game",
            ),
        }
    }
}

/// How many checker squares go across the texture: one per 128 pixels,
/// at least 4.
pub fn checker_squares(texture_size: u32) -> f32 {
    (texture_size as f32 / 128.0).max(4.0)
}

/// The numbered checker image's size in pixels (decision 0107): its
/// labels stay sharp up close.
pub const GRID_IMAGE_SIZE: usize = 1024;

/// The numbered checker image in `colors` (`ViewportView::grid_colors`):
/// [`GRID_IMAGE_SIZE`] pixels square, four bytes a pixel (sRGB), its first
/// row the top of the texture square.
pub fn grid_image(colors: [[u8; 3]; 2]) -> Vec<u8> {
    uv::grid::numbered_checker(GRID_IMAGE_SIZE, colors[0], colors[1])
}

/// How shapes are drawn in the viewport ([decision 0063](../../../../docs/decisions/0063-wireframe.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Shading {
    /// Solid, softly lit surfaces with faint edges: the calmest.
    #[default]
    Shaded,
    /// Solid surfaces with every edge drawn clearly on top, to see how a
    /// shape is built while still seeing its form.
    ShadedWithWires,
    /// Only the edges, seen through the shape: every edge at once, front
    /// and back.
    Wireframe,
}

impl Shading {
    /// Every look, in the order offered.
    pub const ALL: [Shading; 3] = [Self::Shaded, Self::ShadedWithWires, Self::Wireframe];

    /// Its name, in the menu and on the button.
    pub fn label(self) -> &'static str {
        match self {
            Self::Shaded => tr("Shaded"),
            Self::ShadedWithWires => tr("Shaded with wires"),
            Self::Wireframe => tr("Wireframe"),
        }
    }

    /// What it shows, for its tooltip.
    pub fn explanation(self) -> &'static str {
        match self {
            Self::Shaded => tr("Solid, softly lit surfaces with faint edges"),
            Self::ShadedWithWires => tr(
                "Solid surfaces with every edge drawn clearly on top, to see how the shape \
                 is built",
            ),
            Self::Wireframe => tr(
                "Only the edges, seen through the shape: front and back at once. Clicking \
                 still picks the shape.",
            ),
        }
    }

    /// Whether surfaces are drawn.
    pub fn shows_faces(self) -> bool {
        self != Self::Wireframe
    }
}

/// The angle new objects are smoothed by (decision 0126): the same as
/// Hard edges by angle starts at.
pub const AUTO_HARD_ANGLE: f32 = scene::STANDARD_SMOOTHING_ANGLE;

impl ViewOptions {
    /// These options with an old "fine grid lines on" setting turned into
    /// the finest grid level.
    #[must_use]
    pub fn migrated(mut self) -> Self {
        if self.legacy_fine_grid {
            self.grid_detail = GridDetail::TenCentimetres;
            self.legacy_fine_grid = false;
        }
        self
    }
}

impl GridDetail {
    /// Every level, from calm to detailed.
    pub const ALL: [GridDetail; 3] = [Self::Metre, Self::HalfMetre, Self::TenCentimetres];

    /// The menu entry's name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Metre => tr("Every 1 m"),
            Self::HalfMetre => tr("Every 50 cm (quarters)"),
            Self::TenCentimetres => tr("Every 10 cm (fine)"),
        }
    }

    /// The finest lines' spacing, as written in the viewport's corner.
    pub fn spacing_text(self) -> &'static str {
        match self {
            Self::Metre => "1 m",
            Self::HalfMetre => tr("50 cm"),
            Self::TenCentimetres => tr("10 cm"),
        }
    }

    /// The finest lines' spacing for the renderer, in centimetres, or
    /// `None` for only the 1 m and 10 m lines.
    pub fn fine_spacing(self) -> Option<f32> {
        match self {
            Self::Metre => None,
            Self::HalfMetre => Some(50.0),
            Self::TenCentimetres => Some(10.0),
        }
    }

    /// The next level, round again after the finest (the palette button).
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Metre => Self::HalfMetre,
            Self::HalfMetre => Self::TenCentimetres,
            Self::TenCentimetres => Self::Metre,
        }
    }

    /// A plain-language description, for tooltips.
    pub fn explanation(self) -> &'static str {
        match self {
            Self::Metre => tr("Grid lines every metre: the calmest ground"),
            Self::HalfMetre => tr("Each metre square cut into four: lines every 50 cm"),
            Self::TenCentimetres => tr("Thin lines every 10 cm, for placing things precisely"),
        }
    }
}

/// How the viewport is shown and steered: choices the user can switch,
/// which are remembered between sessions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewOptions {
    /// How detailed the grid is. Lines every metre as standard, for a calm
    /// ground.
    pub grid_detail: GridDetail,
    /// Settings saved before the grid had three levels said only whether
    /// the 10 cm lines were on; read so that choice carries over.
    #[serde(rename = "fine_grid", skip_serializing)]
    pub legacy_fine_grid: bool,
    /// Which mouse controls move the view.
    pub navigation: NavigationSettings,
    /// Whether all four views (perspective, top, front, right) are shown
    /// at once, rather than one filling the viewport.
    pub four_views: bool,
    /// Whether the box, lasso and brush only select what can be seen, as
    /// a click does. On as standard (decision 0126; before, off, decision
    /// 0029): the saved name changed with it, so everyone starts with the
    /// new standard once.
    #[serde(rename = "select_visible")]
    pub select_visible_only: bool,
    /// Which keys add to and take from the selection.
    pub select_keys: SelectKeys,
    /// How the move, rotate and scale gizmos snap.
    pub snap: SnapSettings,
    /// Axes that can't change while moving, rotating or scaling (X, Y,
    /// Z).
    pub locked: [bool; 3],
    /// Whether the gizmo follows the world's axes or the object's own.
    pub axes: Axes,
    /// Whether hard edges are *not* marked in the Points, Edges and
    /// Polygons modes (they are as standard; decision 0053).
    pub hide_hard_edges: bool,
    /// Whether the selected objects show which way each polygon faces,
    /// as short lines (off as standard; decision 0053).
    pub show_normals: bool,
    /// Whether every object's collision shapes are outlined, not only the
    /// selected objects' (off as standard; decision 0151).
    pub show_collision: bool,
    /// Whether every shown object's polygons are drawn cut into the
    /// triangles the export makes, as faint lines (off as standard;
    /// decision 0161).
    pub show_triangles: bool,
    /// Whether seams are marked in the Points, Edges and Polygons modes
    /// (off as standard; always in the UV mode; decision 0096).
    pub show_seams: bool,
    /// The live mirror's settings, remembered for the next object
    /// mirrored (decision 0088).
    pub symmetry: Symmetry,
    /// Whether Dissolve goes ahead across a sharp corner without asking
    /// first ("Don't warn me again"; it asks as standard, decision 0057).
    pub dissolve_without_warning: bool,
    /// How shapes are drawn: shaded, shaded with wires, or wireframe.
    pub shading: Shading,
    /// What shapes' surfaces show: colours, a checker or the stretch.
    pub surface: Surface,
    /// How painted surfaces are lit and shown (decision 0198).
    pub paint_preview: PaintPreview,
    /// Whether Cavity is *off*: ridges not brightened and creases not
    /// darkened. Cavity is on as standard (decision 0065).
    pub hide_cavity: bool,
    /// The keyboard the Keys window draws, once picked there (`None`:
    /// guessed from the system language; decision 0061).
    pub keyboard: Option<crate::KeyboardLayout>,
    /// Whether clicking the view dice (or snapping to it with Ctrl while
    /// orbiting) turns a perspective view orthographic, until it is
    /// orbited away (off as standard; decision 0084).
    pub flat_from_dice: bool,
    /// Whether the selection is drawn larger, with a dark rim, in the
    /// viewport and the UV Editor (Settings → Interface → High contrast;
    /// off as standard; decision 0126).
    pub high_contrast: bool,
    /// Whether the app looks for new versions and downloads them, to
    /// offer (Settings → Interface → Updates; on as standard; decision
    /// 0169). Only an installed copy updates itself.
    pub check_for_updates: bool,
    /// The build the person chose to skip ("Skip this version"): it isn't
    /// offered again; a newer one is (decision 0173).
    pub skipped_update: Option<u32>,
    /// What clearing the selection does in the Points, Edges and Polygons
    /// modes (decision 0175): off (standard), the mode stays and a click on
    /// a shape selects it; on, the app goes back to the Model mode.
    pub leave_element_modes_when_empty: bool,
}

impl Default for ViewOptions {
    fn default() -> Self {
        Self {
            grid_detail: GridDetail::default(),
            legacy_fine_grid: false,
            navigation: NavigationSettings::default(),
            four_views: false,
            select_visible_only: true,
            select_keys: SelectKeys::default(),
            snap: SnapSettings::default(),
            locked: [false; 3],
            axes: Axes::default(),
            hide_hard_edges: false,
            show_normals: false,
            show_collision: false,
            show_triangles: false,
            show_seams: false,
            symmetry: Symmetry::default(),
            dissolve_without_warning: false,
            shading: Shading::default(),
            surface: Surface::default(),
            paint_preview: PaintPreview::default(),
            hide_cavity: false,
            keyboard: None,
            flat_from_dice: false,
            high_contrast: false,
            check_for_updates: true,
            skipped_update: None,
            leave_element_modes_when_empty: false,
        }
    }
}

/// The live mirror's settings ([decision 0088](../../../../docs/decisions/0088-live-mirror.md)):
/// what the Symmetry button gives the selected objects, and what its menu
/// shows. Each object keeps its own mirror; these are remembered for the
/// next one.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Symmetry {
    /// Across which of the world's axes, X, Y and Z (X as standard: left
    /// and right).
    pub axes: [bool; 3],
    /// How far the mirror is moved from the world centre along X, Y and Z,
    /// in centimetres.
    pub offset: [f32; 3],
    /// How near points at the seam must be to become one, in centimetres
    /// (0.1 as standard).
    pub weld_distance: f32,
}

/// The standard weld distance, in centimetres.
pub(crate) const WELD_DISTANCE: f32 = scene::LiveMirror::WELD_DISTANCE;

impl Default for Symmetry {
    fn default() -> Self {
        Self {
            axes: [true, false, false],
            offset: [0.0; 3],
            weld_distance: WELD_DISTANCE,
        }
    }
}

impl Symmetry {
    /// The mirror these settings make, if any axis is chosen.
    pub fn mirror(&self) -> Option<scene::LiveMirror> {
        // On the object (decision 0126): the offset is from its pivot,
        // along its own axes.
        let mirror = scene::LiveMirror {
            axes: self.axes,
            centre: glam::Vec3::from_array(self.offset),
            weld_distance: self
                .weld_distance
                .clamp(0.0, scene::LiveMirror::MAX_WELD_DISTANCE),
            on_object: true,
        };
        mirror.is_valid().then_some(mirror)
    }

    /// The settings of an object's `mirror`.
    pub fn of(mirror: scene::LiveMirror) -> Self {
        Self {
            axes: mirror.axes,
            offset: mirror.centre.to_array(),
            weld_distance: mirror.weld_distance,
        }
    }
}
