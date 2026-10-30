//! The paint brush's settings in the Attributes panel, in Paint mode
//! ([decision 0179](../../../../docs/decisions/0179-the-round-brush.md)):
//! like Substance Painter's Properties panel, the settings of the tool in
//! use, above the selection's own.

use super::folds::fold;
use super::{AttributesPanel, label, section, stepper};
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui};
use paint::{Brush, BrushKind};

/// The paint tool in use (Substance Painter's keys: 1 and 2 the brush
/// painting and erasing, 4 the polygon fill).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PaintTool {
    /// The round brush.
    #[default]
    Brush,
    /// The polygon fill tool (decision 0182).
    Fill,
}

/// What the brush paints on (decision 0205).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PaintOn {
    /// The material's texture, through the UVs.
    #[default]
    Texture,
    /// The shape's vertex colours: a colour on each polygon corner.
    VertexColours,
}

/// The Paint on row (decision 0205): texture or vertex colours.
fn paint_on_row(ui: &mut Ui, settings: &mut BrushSettings, from_texture: &mut bool) {
    label(
        ui,
        tr("Paint on"),
        tr(
            "Texture paints the material's texture through the UVs. Vertex colours paints a \
             colour on each polygon corner of the shape itself: no UVs or texture needed, \
             cheap in a game, and soft or crisp across polygons",
        ),
    );
    ui.horizontal(|ui| {
        ui.selectable_value(&mut settings.paint_on, PaintOn::Texture, tr("Texture"));
        ui.selectable_value(
            &mut settings.paint_on,
            PaintOn::VertexColours,
            tr("Vertex colours"),
        );
    });
    ui.end_row();
    if settings.paint_on == PaintOn::VertexColours {
        ui.label("");
        if ui
            .button(tr("From the texture"))
            .on_hover_text(tr(
                "Give every corner of the selected objects the colour of their painted texture \
                 there: a painted model turned into vertex colours, for games without textures",
            ))
            .clicked()
        {
            *from_texture = true;
        }
        ui.end_row();
    }
}

/// The brush's settings, kept between strokes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BrushSettings {
    /// The paint tool in use.
    pub(crate) tool: PaintTool,
    /// What the polygon fill tool fills.
    pub(crate) fill_by: paint::FillBy,
    /// The second colour, behind the first in the brush bar (decision
    /// 0226): a click on it swaps the two.
    pub(crate) second_color: [u8; 4],
    /// The fill is the bucket (decision 0224): it fills the whole layer,
    /// every polygon of the material clicked, whatever `fill_by` says.
    pub(crate) bucket: bool,
    /// The brush.
    pub(crate) brush: Brush,
    /// In which space its size is measured (decision 0217): on screen
    /// (points), on the object (centimetres) or on the texture (texels).
    pub(crate) size_space: SizeSpace,
    /// Its radius on the surface, in centimetres, when measured there.
    pub(crate) surface_radius: f32,
    /// Its radius on the texture, in texels, when measured there.
    pub(crate) texture_radius: f32,
    /// How the pen's pressure changes the size (decision 0184).
    pub(crate) size_pressure: crate::pen::PressureCurve,
    /// How the pen's pressure changes the flow.
    pub(crate) flow_pressure: crate::pen::PressureCurve,
    /// How long the lazy brush's string is, 0 to 1 of its reach.
    pub(crate) smoothing: f32,
    /// Whether smoothing is on (D).
    pub(crate) smoothing_on: bool,
    /// Whether mirror painting is on (L, decision 0187).
    pub(crate) mirror_on: bool,
    /// Across which of the object's own axes mirror painting paints.
    pub(crate) mirror_axis: MirrorAxis,
    /// Whether a stroke stays inside the trim row it starts in (decision
    /// 0202).
    pub(crate) trim_row: bool,
    /// What the brush paints on (decision 0205).
    pub(crate) paint_on: PaintOn,
    /// Which realism channels the brush and fill tool paint, and with
    /// what (decision 0207), by `paint::Channel::index`: on or off, and
    /// the value.
    pub(crate) channels: [(bool, [u8; 4]); 4],
    /// Whether the brush and the polygon fill lay colour (with the realism
    /// channels on, unticking it paints only the ticked channels, such as
    /// height alone; a review found that impossible).
    pub(crate) colour_on: bool,
}

impl BrushSettings {
    /// The realism channels painted and their values (decision 0207).
    pub(crate) fn channel_values(&self) -> Vec<(paint::Channel, [u8; 4])> {
        paint::Channel::ALL
            .into_iter()
            .filter_map(|c| {
                let (on, value) = self.channels[c.index() as usize];
                on.then_some((c, value))
            })
            .collect()
    }
}

/// The brush's realism channel rows (decision 0207), while the texture
/// set uses them.
fn channel_rows(ui: &mut Ui, settings: &mut BrushSettings) {
    // Colour first: untick it to paint only the channels below.
    label(
        ui,
        tr(COLOUR_SWITCH),
        tr("Lay the colour too. Untick it to paint only the ticked channels below, such as \
            bumps with Height alone, without changing the colour"),
    );
    ui.checkbox(&mut settings.colour_on, "");
    ui.end_row();
    let mut held = false;
    for channel in paint::Channel::ALL {
        let (on, value) = &mut settings.channels[channel.index() as usize];
        crate::paint_layers::channel_rows::channel_row(ui, channel, on, value, &mut held);
    }
}

/// The Colour switch's name in the Lays down section.
pub(crate) const COLOUR_SWITCH: &str = crate::text!("Colour");

impl AttributesPanel {
    /// The colour a stroke lays: the brush's, or none (alpha 0) while the
    /// realism channels are on, painting a texture, with Colour unticked.
    pub(crate) fn stroke_colour(&self) -> [u8; 4] {
        let mut colour = self.brush.brush.color;
        if self.realism && self.brush.paint_on == PaintOn::Texture && !self.brush.colour_on {
            colour[3] = 0;
        }
        colour
    }
}

/// The axis mirror painting reflects across (decision 0187): the object's
/// own axis, through its pivot. In the UV Editor, mirror painting always
/// flips left and right across the texture's middle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum MirrorAxis {
    /// Left and right: the usual choice for faces, bodies and vehicles.
    #[default]
    X,
    /// Front and back (Z is up in the app, decision 0012).
    Y,
    /// Up and down.
    Z,
}

impl MirrorAxis {
    /// Every axis, in the order shown.
    pub(crate) const ALL: [MirrorAxis; 3] = [MirrorAxis::X, MirrorAxis::Y, MirrorAxis::Z];

    /// Which coordinate it flips: 0 for X, 1 for Y, 2 for Z.
    pub(crate) fn index(self) -> usize {
        match self {
            MirrorAxis::X => 0,
            MirrorAxis::Y => 1,
            MirrorAxis::Z => 2,
        }
    }

    /// The reflection across this axis, in the object's own space: the
    /// coordinate it flips turned the other way.
    pub(crate) fn reflection(self) -> glam::Mat4 {
        let mut scale = glam::Vec3::ONE;
        scale[self.index()] = -1.0;
        glam::Mat4::from_scale(scale)
    }

    /// Its name with the way it mirrors, as the manual says it.
    pub(crate) fn described(self) -> &'static str {
        match self {
            MirrorAxis::X => tr("X (left and right)"),
            MirrorAxis::Y => tr("Y (front and back)"),
            MirrorAxis::Z => tr("Z (up and down)"),
        }
    }

    /// Its name in the panel.
    pub(crate) fn name(self) -> &'static str {
        match self {
            MirrorAxis::X => "X",
            MirrorAxis::Y => "Y",
            MirrorAxis::Z => "Z",
        }
    }
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            tool: PaintTool::Brush,
            fill_by: paint::FillBy::Polygon,
            second_color: [40, 40, 40, 255],
            bucket: false,
            brush: Brush::default(),
            size_space: SizeSpace::Viewport,
            surface_radius: 5.0,
            texture_radius: 16.0,
            // Pressure changes the size by default, as in Substance
            // Painter; flow stays even.
            size_pressure: crate::pen::PressureCurve {
                on: true,
                least: 0.1,
                curve: crate::pen::Curve::Even,
            },
            flow_pressure: crate::pen::PressureCurve {
                on: false,
                least: 0.2,
                curve: crate::pen::Curve::Even,
            },
            smoothing: 0.3,
            smoothing_on: false,
            mirror_on: false,
            mirror_axis: MirrorAxis::X,
            trim_row: true,
            paint_on: PaintOn::Texture,
            // As in Substance Painter, paint lays down roughness and
            // metal with its colour: half rough, not metal.
            colour_on: true,
            channels: [
                (true, [128, 128, 128, 255]),
                (true, [0, 0, 0, 255]),
                (false, [255, 170, 60, 255]),
                // Height off: raised by a quarter when ticked.
                (false, [192, 192, 192, 255]),
            ],
        }
    }
}

/// Where projection's picture lies over the views (decision 0186).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProjectionSettings {
    /// Screen points per picture pixel.
    pub(crate) scale: f32,
    /// How far the picture's middle is from each view's middle, in
    /// points.
    pub(crate) offset: [f32; 2],
    /// How strongly the picture shows over the views, 0 to 1.
    pub(crate) shown: f32,
}

impl Default for ProjectionSettings {
    fn default() -> Self {
        Self {
            scale: 1.0,
            offset: [0.0, 0.0],
            shown: 0.5,
        }
    }
}

/// The Mirror row (decision 0187): on or off, and across which axis.
fn mirror_row(ui: &mut Ui, settings: &mut BrushSettings) {
    label(
        ui,
        tr("Mirror"),
        tr(
            "Paint the other side of the object at the same time, reflected across its own \
            axis through its pivot. L turns it on and off. In the UV Editor it flips left and \
            right across the middle of the texture",
        ),
    );
    ui.horizontal(|ui| {
        ui.checkbox(&mut settings.mirror_on, "")
            .on_hover_text(tr("Mirror painting on or off (L)"));
        ui.add_enabled_ui(settings.mirror_on, |ui| {
            for axis in MirrorAxis::ALL {
                ui.selectable_value(&mut settings.mirror_axis, axis, axis.name())
                    .on_hover_text(crate::trf!(
                        "Mirror across the object's {} axis",
                        axis.name()
                    ));
            }
        });
    });
    ui.end_row();
}

/// The Trim row row (decision 0202), when the project has trim rows:
/// whether strokes stay inside the row they start in.
fn trim_row_row(ui: &mut Ui, settings: &mut BrushSettings) {
    label(
        ui,
        tr("Trim row"),
        tr(
            "Keep each stroke inside the trim row it starts in, so paint on one strip never \
             spills into the rows above and below it",
        ),
    );
    ui.checkbox(&mut settings.trim_row, tr("Stay in the row"));
    ui.end_row();
}

/// The brush's size in its space, as a number with its unit.
pub(crate) fn size_number(ui: &mut Ui, settings: &mut BrushSettings) {
    match settings.size_space {
        SizeSpace::Viewport => {
            stepper::number(
                ui,
                &mut settings.brush.radius,
                (
                    *paint::brush::RADIUS_RANGE.start(),
                    *paint::brush::RADIUS_RANGE.end(),
                ),
                (1.0, 0),
                tr("Brush size on screen"),
            );
            ui.label(RichText::new(tr("px")).color(theme::TEXT_WEAK));
        }
        SizeSpace::Object => {
            stepper::number(
                ui,
                &mut settings.surface_radius,
                SURFACE_RADIUS,
                (0.5, 2),
                tr("Brush size on the surface"),
            );
            ui.label(RichText::new(tr("cm")).color(theme::TEXT_WEAK));
        }
        SizeSpace::Texture => {
            stepper::number(
                ui,
                &mut settings.texture_radius,
                TEXTURE_RADIUS,
                (1.0, 0),
                tr("Brush size on the texture"),
            );
            ui.label(RichText::new(tr("texels")).color(theme::TEXT_WEAK));
        }
    }
}

/// An alignment's name in the Paint tab.
pub(crate) fn alignment_name(alignment: paint::Alignment) -> &'static str {
    match alignment {
        paint::Alignment::Camera => tr("Camera"),
        paint::Alignment::Wrap => tr("Wrap"),
        paint::Alignment::Planar => tr("Planar"),
        paint::Alignment::Uv => tr("UV"),
    }
}

/// What an alignment does, for its hover text.
fn alignment_help(alignment: paint::Alignment) -> &'static str {
    match alignment {
        paint::Alignment::Camera => tr(
            "Round as seen from the camera: stretches on surfaces turned away. Best for painting \
             what you see as you see it",
        ),
        paint::Alignment::Wrap => tr(
            "Round on the surface, bending round edges and curves without stretching: the \
             usual choice for painting on a model",
        ),
        paint::Alignment::Planar => tr(
            "Round on the surface where it lands, like a stamp pressed flat: fades where the \
             surface curves away, so it doesn't run round edges",
        ),
        paint::Alignment::Uv => tr(
            "Round on the texture: follows the UVs, stretching where they stretch and \
             stopping at seams. For painting by the layout",
        ),
    }
}

/// The smallest and largest radius on the surface, in centimetres.
pub(crate) const SURFACE_RADIUS: (f32, f32) = (0.05, 500.0);

/// The smallest and largest radius on the texture, in texels.
pub(crate) const TEXTURE_RADIUS: (f32, f32) = (0.5, 4096.0);

/// In which space the brush's size is measured ([decision 0217](../../../../docs/decisions/0217-alignment-and-size-space.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SizeSpace {
    /// On screen, in points: the brush looks the same size however far
    /// the camera is.
    #[default]
    Viewport,
    /// On the object, in centimetres: the same real size on every
    /// object, getting bigger on screen as you come closer.
    Object,
    /// On the texture, in texels: the same number of texture pixels
    /// wherever it lands, however the camera is or the UVs are scaled.
    Texture,
}

impl SizeSpace {
    /// Every space, in the order the Paint tab shows them.
    pub(crate) const ALL: [SizeSpace; 3] = [Self::Viewport, Self::Object, Self::Texture];

    /// Its name in the Paint tab.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Viewport => tr("Viewport"),
            Self::Object => tr("Object"),
            Self::Texture => tr("Texture"),
        }
    }
}

impl BrushSettings {
    /// Picks the polygon fill tool (4). It fills with colour: coming from
    /// the eraser or an effect brush doesn't leave it erasing, which
    /// nothing on screen would show (it is ticked again in the Paint tab
    /// to fill with nothing).
    pub(crate) fn choose_fill(&mut self) {
        self.tool = PaintTool::Fill;
        self.bucket = false;
        if self.brush.kind != BrushKind::Paint {
            self.brush.kind = BrushKind::Paint;
        }
    }

    /// Swaps the brush's colour and the second colour (decision 0226).
    pub(crate) fn swap_colours(&mut self) {
        std::mem::swap(&mut self.brush.color, &mut self.second_color);
    }

    /// The bucket (8, decision 0224): a fill of the whole layer, painting
    /// (not erasing), as the polygon fill starts.
    pub(crate) fn choose_bucket(&mut self) {
        self.choose_fill();
        self.bucket = true;
    }

    /// What a fill fills: every polygon of the material for the bucket,
    /// otherwise the polygon fill's choice.
    pub(crate) fn fill_area(&self) -> paint::FillBy {
        if self.bucket {
            paint::FillBy::Material
        } else {
            self.fill_by
        }
    }

    /// The size grown (or shrunk) by `factor`, in its own space ([ and ],
    /// Ctrl + drag).
    pub(crate) fn scale_size(&mut self, factor: f32) {
        match self.size_space {
            SizeSpace::Viewport => {
                self.brush.radius *= factor;
                self.brush = self.brush.clamped();
            }
            SizeSpace::Object => {
                self.surface_radius =
                    (self.surface_radius * factor).clamp(SURFACE_RADIUS.0, SURFACE_RADIUS.1);
            }
            SizeSpace::Texture => {
                self.texture_radius =
                    (self.texture_radius * factor).clamp(TEXTURE_RADIUS.0, TEXTURE_RADIUS.1);
            }
        }
    }

    /// The brush's radius in world units (centimetres) where one screen
    /// point spans `per_point` world units and a world unit spans
    /// `texels_per_unit` texels (`None`: no texture there, measured as on
    /// screen).
    pub(crate) fn world_radius(&self, per_point: f32, texels_per_unit: Option<f32>) -> f32 {
        match (self.size_space, texels_per_unit) {
            (SizeSpace::Object, _) => self.surface_radius,
            (SizeSpace::Texture, Some(per_unit)) if per_unit > 1e-9 => {
                self.texture_radius / per_unit
            }
            _ => self.brush.radius * per_point,
        }
    }

    /// The brush's radius on screen, in points: [`Self::world_radius`]
    /// over `per_point`.
    pub(crate) fn screen_radius(&self, per_point: f32, texels_per_unit: Option<f32>) -> f32 {
        match self.size_space {
            SizeSpace::Viewport => self.brush.radius,
            _ => self.world_radius(per_point, texels_per_unit) / per_point.max(1e-9),
        }
    }

    /// The brush's radius in the UV Editor, in texture widths, where the
    /// editor shows `zoom` points a texture width, a texture width spans
    /// `units_per_uv` world units, and the texture is `size` texels
    /// across.
    pub(crate) fn flat_radius(&self, zoom: f32, units_per_uv: f32, size: Option<f32>) -> f32 {
        match (self.size_space, size) {
            (SizeSpace::Object, _) => self.surface_radius / units_per_uv.max(1e-6),
            (SizeSpace::Texture, Some(size)) if size > 0.0 => self.texture_radius / size,
            _ => self.brush.radius / zoom.max(1e-6),
        }
    }
}

/// What the brush is painting on, for the panel to show: the material
/// and the layer, and how big a texture pixel is there.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct BrushTarget {
    /// The material whose texture set is painted.
    pub(crate) material: String,
    /// The layer painted into.
    pub(crate) layer: String,
    /// How wide one texture pixel is on the surface under the brush, in
    /// centimetres, if known.
    pub(crate) texel: Option<f32>,
    /// The brush's radius on the surface there, in centimetres.
    pub(crate) radius: Option<f32>,
}

impl AttributesPanel {
    /// The Paint brush section: what it paints with and on.
    pub(super) fn brush_section(&mut self, ui: &mut Ui, palettes: &[paint::palette::Palette]) {
        if self.brush.tool == PaintTool::Fill {
            self.fill_section(ui);
        } else {
            self.round_brush_section(ui);
        }
        // With the floating brush bar, the colour picker is its square's
        // pop-up (decision 0213).
        if self.brush.brush.kind == BrushKind::Paint && !self.brush_bar_shown {
            let [r, g, b, _] = self.brush.brush.color;
            let mut colour = [r, g, b];
            self.colour.show(
                ui,
                &mut colour,
                palettes,
                Some(&mut self.picking),
                &mut self.palette_requests,
                true,
            );
            self.brush.brush.color = [colour[0], colour[1], colour[2], 255];
        }
    }

    /// The round brush's settings, in folding sections as the mockup has
    /// them (decision 0228): Tip, Lays down, Pen and smoothing, Symmetry
    /// and trim, Paint through and effects, and the keys.
    fn round_brush_section(&mut self, ui: &mut Ui) {
        // Size, flow, hardness, spacing and mirror are on the floating
        // brush bar when it shows (decision 0213).
        let bar = self.brush_bar_shown;
        let realism = self.realism && self.brush.paint_on == PaintOn::Texture;
        section(ui, kind_name(self.brush.brush.kind));
        let tip_aside = tip_name(self.brush.brush.tip).to_string();
        {
            let from_texture = &mut self.vertex_from_texture;
            let settings = &mut self.brush;
            fold(ui, "tip", crate::text!("Tip"), &tip_aside, true, |ui| {
                ui.horizontal_top(|ui| {
                    // The tip's picture: a dab as it lands.
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
                    crate::paint_bar::dab(ui.painter(), rect.center(), 17.0, settings.brush.hardness);
                    egui::Grid::new("attributes_brush")
                        .num_columns(2)
                        .spacing(egui::vec2(8.0, 6.0))
                        .show(ui, |ui| {
                            label(
                                ui,
                                tr("Shape"),
                                tr("The shape of the brush's dabs. The shelf under the views \
                                    holds ready-made brushes"),
                            );
                            egui::ComboBox::from_id_salt("brush_tip")
                                .selected_text(tip_name(settings.brush.tip))
                                .show_ui(ui, |ui| {
                                    for tip in paint::Tip::ALL {
                                        ui.selectable_value(&mut settings.brush.tip, tip, tip_name(tip));
                                    }
                                });
                            ui.end_row();
                            if !bar {
                                label(
                                    ui,
                                    tr("Size"),
                                    tr("The brush's radius. [ and ] change it, as does Ctrl + right drag (sideways)"),
                                );
                                ui.horizontal(|ui| size_number(ui, settings));
                                ui.end_row();
                                number_rows(ui, settings);
                            }
                        });
                });
                egui::Grid::new("attributes_brush_place")
                    .num_columns(2)
                    .spacing(egui::vec2(8.0, 6.0))
                    .show(ui, |ui| {
                        label(
                            ui,
                            tr("Size space"),
                            tr("Where the brush's size is measured. Viewport: on screen, so the brush \
                                looks the same size as you zoom. Object: on the surface in \
                                centimetres, the same real size on every object. Texture: in texture \
                                pixels, the same number of pixels wherever it lands"),
                        );
                        ui.horizontal(|ui| {
                            for space in SizeSpace::ALL {
                                ui.selectable_value(&mut settings.size_space, space, space.label());
                            }
                        });
                        ui.end_row();
                        label(
                            ui,
                            tr("Alignment"),
                            tr("How each dab is laid on the surface. Camera: round as seen from the \
                                camera, stretching where the surface turns away. Wrap: round on the \
                                surface, bending round edges and curves. Planar: round on the \
                                surface where it lands, fading where the surface curves away. UV: \
                                round on the texture, following the UVs and stopping at seams"),
                        );
                        egui::ComboBox::from_id_salt("brush_alignment")
                            .selected_text(alignment_name(settings.brush.alignment))
                            .show_ui(ui, |ui| {
                                for alignment in paint::Alignment::ALL {
                                    ui.selectable_value(
                                        &mut settings.brush.alignment,
                                        alignment,
                                        alignment_name(alignment),
                                    )
                                    .on_hover_text(alignment_help(alignment));
                                }
                            });
                        ui.end_row();
                        paint_on_row(ui, settings, from_texture);
                    });
            });
        }
        // The eraser takes every channel away: nothing to choose.
        let kind = self.brush.brush.kind;
        if realism && !kind.is_effect() && kind != BrushKind::Erase {
            self.lays_down(ui);
        }
        self.pen_rows(ui);
        self.symmetry_rows(ui);
        let through = self.brush.brush.through;
        let effects_aside = match (through, kind.is_effect() || kind == BrushKind::Projection) {
            (true, _) => tr("paints through"),
            (false, true) => kind_name(kind),
            (false, false) => "",
        };
        fold(
            ui,
            "through",
            crate::text!("Paint through and effects"),
            effects_aside,
            false,
            |ui| {
                ui.horizontal(|ui| {
                    label(
                        ui,
                        tr("Paint through"),
                        tr("Paint the parts hidden behind others and facing away too"),
                    );
                    ui.checkbox(&mut self.brush.brush.through, "");
                });
                self.effect_rows(ui);
            },
        );
        self.brush.brush = self.brush.brush.clamped();
        key_rows(ui, BRUSH_KEYS);
        self.target_lines(ui);
    }

    /// The Lays down section (decision 0228): the realism channels the
    /// brush lays down besides its colour.
    fn lays_down(&mut self, ui: &mut Ui) {
        let settings = &mut self.brush;
        fold(
            ui,
            "lays_down",
            crate::text!("Lays down"),
            tr("besides colour"),
            true,
            |ui| {
                egui::Grid::new("attributes_lays_down")
                    .num_columns(2)
                    .spacing(egui::vec2(8.0, 6.0))
                    .show(ui, |ui| channel_rows(ui, settings));
            },
        );
    }

    /// The Symmetry and trim section: mirror painting (also on the brush
    /// bar) and the trim sheet row.
    fn symmetry_rows(&mut self, ui: &mut Ui) {
        let trim_rows = self.trim_rows;
        let settings = &mut self.brush;
        let aside = if settings.mirror_on {
            crate::trf!("{} {}", tr("Mirror"), settings.mirror_axis.name())
        } else {
            tr("off").to_string()
        };
        fold(
            ui,
            "symmetry",
            crate::text!("Symmetry and trim"),
            &aside,
            false,
            |ui| {
                egui::Grid::new("attributes_symmetry")
                    .num_columns(2)
                    .spacing(egui::vec2(8.0, 6.0))
                    .show(ui, |ui| {
                        mirror_row(ui, settings);
                        if trim_rows {
                            trim_row_row(ui, settings);
                        }
                    });
                if !trim_rows {
                    ui.label(
                        RichText::new(tr(
                            "With a trim sheet (UV → Trim sheet), a row here keeps the brush \
                             inside one strip of it.",
                        ))
                        .small()
                        .color(theme::TEXT_WEAK),
                    );
                }
            },
        );
    }

    /// The settings of the effect brushes, projection and clone (decision
    /// 0186).
    fn effect_rows(&mut self, ui: &mut Ui) {
        match self.brush.brush.kind {
            BrushKind::HueShift => {
                ui.horizontal(|ui| {
                    label(
                        ui,
                        tr("Hue shift"),
                        tr("How far round the colour wheel each pass turns the hue: \
                            negative turns it the other way"),
                    );
                    let mut turn = self.brush.brush.hue_shift * 360.0;
                    if ui
                        .add(
                            egui::Slider::new(&mut turn, -180.0..=180.0)
                                .suffix("°")
                                .max_decimals(0),
                        )
                        .changed()
                    {
                        self.brush.brush.hue_shift = turn / 360.0;
                    }
                });
            }
            BrushKind::Projection => {
                ui.horizontal(|ui| {
                    if ui
                        .button(tr("Picture…"))
                        .on_hover_text(tr(
                            "Choose a picture to lay over the views and paint onto the model",
                        ))
                        .clicked()
                    {
                        self.choose_stencil = true;
                    }
                    let name = self
                        .stencil_name
                        .clone()
                        .unwrap_or_else(|| tr("No picture yet").to_string());
                    ui.label(RichText::new(name).color(theme::TEXT_WEAK));
                });
                let projection = &mut self.projection;
                egui::Grid::new("attributes_projection")
                    .num_columns(2)
                    .spacing(egui::vec2(8.0, 6.0))
                    .show(ui, |ui| {
                        label(ui, tr("Scale"), tr("How big the picture is over the views"));
                        stepper::percent(
                            ui,
                            &mut projection.scale,
                            (0.05, 20.0),
                            tr("Picture scale"),
                        );
                        ui.end_row();
                        label(
                            ui,
                            tr("Move"),
                            tr(
                                "Where the picture lies: across and down from the view's middle, \
                                in points",
                            ),
                        );
                        ui.horizontal(|ui| {
                            for (i, name) in [tr("Picture across"), tr("Picture down")]
                                .into_iter()
                                .enumerate()
                            {
                                let response = ui.add(
                                    egui::DragValue::new(&mut projection.offset[i])
                                        .range(-5000.0..=5000.0)
                                        .speed(1.0),
                                );
                                response.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::DragValue,
                                        true,
                                        name,
                                    )
                                });
                            }
                            if ui.small_button(tr("Centre")).clicked() {
                                projection.offset = [0.0, 0.0];
                            }
                        });
                        ui.end_row();
                        label(
                            ui,
                            tr("Shown"),
                            tr("How strongly the picture shows over the views while you paint"),
                        );
                        stepper::percent(
                            ui,
                            &mut projection.shown,
                            (0.0, 1.0),
                            tr("Picture shown"),
                        );
                        ui.end_row();
                    });
            }
            BrushKind::Clone => {
                ui.label(
                    RichText::new(if self.clone_source_set {
                        tr("Copying from the place you set. Hold V and click to set another.")
                    } else {
                        tr("Hold V and click where to copy from, then paint.")
                    })
                    .small()
                    .color(theme::TEXT_WEAK),
                );
            }
            BrushKind::Smudge | BrushKind::Blend | BrushKind::Blur => {
                ui.label(
                    RichText::new(tr("Flow is how strong the effect is."))
                        .small()
                        .color(theme::TEXT_WEAK),
                );
            }
            BrushKind::Paint | BrushKind::Erase => {}
        }
    }

    /// The pen's settings (decision 0184): pressure on size and flow,
    /// smoothing, and the pen's pressure now.
    fn pen_rows(&mut self, ui: &mut Ui) {
        let settings = &mut self.brush;
        let mut on = Vec::new();
        if settings.size_pressure.on {
            on.push(tr("pressure → size"));
        }
        if settings.flow_pressure.on {
            on.push(tr("pressure → flow"));
        }
        if settings.smoothing_on {
            on.push(tr("smoothing"));
        }
        let aside = if on.is_empty() {
            tr("off").to_string()
        } else {
            on.join(", ")
        };
        let pen = self.pen;
        fold(
            ui,
            "pen",
            crate::text!("Pen and smoothing"),
            &aside,
            false,
            |ui| {
                egui::Grid::new("attributes_pen")
                    .num_columns(2)
                    .spacing(egui::vec2(8.0, 6.0))
                    .show(ui, |ui| {
                        for (curve, name, help, least_label) in [
                            (
                                &mut settings.size_pressure,
                                tr("Pressure → size"),
                                tr("Press harder for a bigger brush. Least: the size at the \
                                    lightest touch, as a share of the full size"),
                                tr("Least size with pressure"),
                            ),
                            (
                                &mut settings.flow_pressure,
                                tr("Pressure → flow"),
                                tr("Press harder for more paint. Least: the flow at the \
                                    lightest touch, as a share of the full flow"),
                                tr("Least flow with pressure"),
                            ),
                        ] {
                            label(ui, name, help);
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut curve.on, "");
                                ui.add_enabled_ui(curve.on, |ui| {
                                    stepper::percent(ui, &mut curve.least, (0.0, 1.0), least_label);
                                    egui::ComboBox::from_id_salt(name)
                                        .selected_text(curve_name(curve.curve))
                                        .width(64.0)
                                        .show_ui(ui, |ui| {
                                            for c in crate::pen::Curve::ALL {
                                                ui.selectable_value(
                                                    &mut curve.curve,
                                                    c,
                                                    curve_name(c),
                                                )
                                                .on_hover_text(curve_help(c));
                                            }
                                        });
                                });
                            });
                            ui.end_row();
                        }
                        label(
                            ui,
                            tr("Smoothing"),
                            tr(
                                "The brush follows the pointer on a string, so lines come out \
                                steadier and wobbles are left out. D turns it on and off; the \
                                number is how long the string is",
                            ),
                        );
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut settings.smoothing_on, "");
                            ui.add_enabled_ui(settings.smoothing_on, |ui| {
                                stepper::percent(
                                    ui,
                                    &mut settings.smoothing,
                                    (0.0, 1.0),
                                    tr("Smoothing"),
                                );
                            });
                        });
                        ui.end_row();
                    });
                let text = match pen {
                    Some(pen) if pen.eraser => {
                        crate::trf!("Pen: eraser end, pressure {:.0}%", pen.pressure * 100.0)
                    }
                    Some(pen) => crate::trf!("Pen: pressure {:.0}%", pen.pressure * 100.0),
                    None => {
                        tr("No pen pressure: a mouse paints at full size and flow.").to_string()
                    }
                };
                ui.label(RichText::new(text).small().color(theme::TEXT_WEAK));
            },
        );
    }

    /// The polygon fill tool's section (decision 0182), in folding
    /// sections like the brush's (decision 0228).
    fn fill_section(&mut self, ui: &mut Ui) {
        let realism = self.realism && self.brush.paint_on == PaintOn::Texture;
        // Fill the layer (8) is the fill tool always filling the whole
        // material: its own name, and no choice of what a click fills (a
        // review found it showed the polygon fill's).
        let bucket = self.brush.bucket;
        section(
            ui,
            if bucket {
                tr("Fill the layer")
            } else {
                tr("Polygon fill")
            },
        );
        let by = match self.brush.fill_area() {
            _ if bucket => tr("Whole layer"),
            paint::FillBy::Polygon => tr("Polygon"),
            paint::FillBy::Piece => tr("UV piece"),
            paint::FillBy::Material => tr("Material"),
        };
        {
            let from_texture = &mut self.vertex_from_texture;
            let settings = &mut self.brush;
            fold(ui, "fill", crate::text!("Fills"), by, true, |ui| {
                egui::Grid::new("attributes_fill")
                    .num_columns(2)
                    .spacing(egui::vec2(8.0, 6.0))
                    .show(ui, |ui| {
                        paint_on_row(ui, settings, from_texture);
                        if bucket {
                            label(
                                ui,
                                tr("Fill"),
                                tr("A click fills the chosen layer everywhere on the material clicked"),
                            );
                            ui.label(
                                egui::RichText::new(tr("The whole layer"))
                                    .color(crate::theme::TEXT_WEAK),
                            );
                            ui.end_row();
                        } else {
                            label(
                                ui,
                                tr("Fill"),
                                tr(
                                    "What a click fills: the polygon under the mouse, its whole UV \
                                     piece, or every polygon with its material. Drag to fill several",
                                ),
                            );
                            ui.horizontal(|ui| {
                                for (by, name) in [
                                    (paint::FillBy::Polygon, tr("Polygon")),
                                    (paint::FillBy::Piece, tr("UV piece")),
                                    (paint::FillBy::Material, tr("Material")),
                                ] {
                                    ui.selectable_value(&mut settings.fill_by, by, name);
                                }
                            });
                            ui.end_row();
                        }
                        label(
                            ui,
                            tr("Erase"),
                            tr("Take paint away from what is clicked instead of filling it"),
                        );
                        let mut erase = settings.brush.kind == BrushKind::Erase;
                        if ui.checkbox(&mut erase, "").changed() {
                            settings.brush.kind = if erase {
                                BrushKind::Erase
                            } else {
                                BrushKind::Paint
                            };
                        }
                        ui.end_row();
                    });
            });
        }
        if realism {
            self.lays_down(ui);
        }
        self.symmetry_rows(ui);
        key_rows(ui, FILL_KEYS);
        self.target_lines(ui);
    }

    /// What the brush paints on, and a warning when it's too small.
    fn target_lines(&self, ui: &mut Ui) {
        let target = &self.brush_target;
        if !target.material.is_empty() {
            ui.label(
                RichText::new(crate::trf!(
                    "Painting on {} → {}",
                    target.material,
                    target.layer
                ))
                .small()
                .color(theme::TEXT_WEAK),
            );
        }
        if let (Some(texel), Some(radius)) = (target.texel, target.radius)
            && radius < texel
        {
            ui.label(
                RichText::new(tr(
                    "The brush is smaller than a texture pixel here: its detail won't show. \
                     Make the brush bigger, or the texture size (UV → Texture) larger.",
                ))
                .color(theme::WARNING),
            );
        }
    }
}

/// A pressure curve's name.
/// Flow, Hardness and Spacing, for when the brush bar isn't showing.
fn number_rows(ui: &mut Ui, settings: &mut BrushSettings) {
    label(
        ui,
        tr("Flow"),
        tr(
            "How much paint one stroke lays on. Within a stroke paint doesn't pile \
            up; stroke again to build it up. Ctrl + left drag (sideways) changes it",
        ),
    );
    stepper::percent(ui, &mut settings.brush.flow, (0.0, 1.0), tr("Brush flow"));
    ui.end_row();
    label(
        ui,
        tr("Hardness"),
        tr(
            "How much of the brush paints at full strength: 100% has a crisp edge, \
            0% fades from the middle. Ctrl + right drag (up and down) changes it",
        ),
    );
    stepper::percent(
        ui,
        &mut settings.brush.hardness,
        (0.0, 1.0),
        tr("Brush hardness"),
    );
    ui.end_row();
    label(
        ui,
        tr("Spacing"),
        tr(
            "The gap between the brush's dabs along a stroke, as a share of its \
            width: small for smooth lines",
        ),
    );
    stepper::percent(
        ui,
        &mut settings.brush.spacing,
        (0.01, 4.0),
        tr("Brush spacing"),
    );
    ui.end_row();
}

/// The brush's keys, one per line, for the Keys section (decision 0228:
/// one line each, so none is cut off at the panel's edge).
const BRUSH_KEYS: &[(&str, &str)] = &[
    ("1", crate::text!("Paint brush")),
    ("2", crate::text!("Eraser")),
    ("3", crate::text!("Projection")),
    ("4", crate::text!("Polygon fill")),
    ("5", crate::text!("Smudge")),
    ("6", crate::text!("Clone")),
    ("7", crate::text!("Blur")),
    ("8", crate::text!("Fill the layer")),
    ("P", crate::text!("Pick a colour")),
    ("Q W R S", crate::text!("Select, move, rotate, scale")),
    ("[ ]", crate::text!("Smaller, bigger")),
    ("Ctrl + drag", crate::text!("Size, hardness and flow")),
    (
        "Shift + click",
        crate::text!("A straight line from the last stroke"),
    ),
    ("D", crate::text!("Smoothing on or off")),
    ("L", crate::text!("Mirror on or off")),
    ("Esc", crate::text!("Leave Paint mode")),
];

/// The polygon fill tool's keys.
const FILL_KEYS: &[(&str, &str)] = &[
    ("Click", crate::text!("Fill")),
    ("Drag", crate::text!("Fill several")),
    ("1", crate::text!("Paint brush")),
    ("2", crate::text!("Eraser")),
    ("L", crate::text!("Mirror on or off")),
    ("Esc", crate::text!("Leave Paint mode")),
];

/// The Keys section, closed to start with: each key and what it does.
fn key_rows(ui: &mut Ui, keys: &[(&str, &'static str)]) {
    fold(
        ui,
        "keys",
        crate::text!("Keys for painting"),
        "",
        false,
        |ui| {
            egui::Grid::new("paint_keys")
                .num_columns(2)
                .spacing(egui::vec2(12.0, 3.0))
                .show(ui, |ui| {
                    for (key, what) in keys {
                        ui.label(RichText::new(*key).small().strong());
                        ui.label(RichText::new(tr(what)).small().color(theme::TEXT_WEAK));
                        ui.end_row();
                    }
                });
        },
    );
}

fn curve_name(curve: crate::pen::Curve) -> &'static str {
    match curve {
        crate::pen::Curve::Soft => tr("Soft"),
        crate::pen::Curve::Even => tr("Even"),
        crate::pen::Curve::Firm => tr("Firm"),
    }
}

/// What a pressure curve does.
fn curve_help(curve: crate::pen::Curve) -> &'static str {
    match curve {
        crate::pen::Curve::Soft => tr("Light touches already paint well"),
        crate::pen::Curve::Even => tr("In step with how hard you press"),
        crate::pen::Curve::Firm => tr("A firm press is needed for the full brush"),
    }
}

/// A brush tip's name.
pub(crate) fn tip_name(tip: paint::Tip) -> &'static str {
    match tip {
        paint::Tip::Round => tr("Round"),
        paint::Tip::Chalk => tr("Chalk"),
        paint::Tip::Bristle => tr("Bristle"),
        paint::Tip::DryBrush => tr("Dry brush"),
        paint::Tip::Spatter => tr("Spatter"),
    }
}

/// What a brush kind is called, as its section's title.
pub(crate) fn kind_name(kind: BrushKind) -> &'static str {
    match kind {
        BrushKind::Paint => tr("Paint brush"),
        BrushKind::Erase => tr("Eraser"),
        BrushKind::Smudge => tr("Smudge"),
        BrushKind::Blend => tr("Blend"),
        BrushKind::Blur => tr("Blur"),
        BrushKind::HueShift => tr("Hue shift"),
        BrushKind::Projection => tr("Projection"),
        BrushKind::Clone => tr("Clone"),
    }
}
