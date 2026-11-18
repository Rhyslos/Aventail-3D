//! Sculpt mode's settings and where they are shown ([decision 0304](../../../docs/decisions/0304-sculpt-mode-and-layers.md),
//! [decision 0306](../../../docs/decisions/0306-sculpt-brushes-for-forms.md)):
//! the brushes' settings (each brush keeps its own size and strength), their
//! keys, the
//! sculpt's look, the Sculpt tab beside Attributes, the shelf of brushes
//! under the view, and the brush bar floating over the view's top (in the
//! paint brush bar's look, sharing its number bars). Decision 0308 adds
//! the pen (pressure to size and strength, smoothing), the kinds of
//! stroke (Freehand, Line, Drag a stamp) and the alphas: the shelf's
//! Alphas tab, the alpha's angle, and your own alphas. Decision 0311 puts
//! the Sculpt tab's settings in folding sections, as the Paint tab's, and
//! the brush bar in [`crate::sculpt_bar`].

use crate::attributes::folds::fold;
use crate::paint_bar::{Shown, fill_field, weak};
use crate::pen::{Curve, PressureCurve};
use crate::text::tr;
use crate::theme;
use egui::{Pos2, Rect, RichText, Ui, WidgetInfo, WidgetType};
use sculpt::alpha::{Alpha, Made};
use sculpt::{BrushKind, BrushSettings, Falloff};
use std::collections::HashMap;
use std::sync::Arc;

/// Accessible name of the floating sculpt brush bar.
pub(crate) const BAR_LABEL: &str = crate::text!("Sculpt brush bar");
/// Accessible names of its numbers.
pub(crate) const SIZE_LABEL: &str = crate::text!("Sculpt brush size");
pub(crate) const STRENGTH_LABEL: &str = crate::text!("Sculpt brush strength");
pub(crate) const SPACING_LABEL: &str = crate::text!("Sculpt brush spacing");
/// Accessible names of the shelf's tab buttons in Sculpt mode.
pub(crate) const BRUSHES_TAB: &str = crate::text!("Show the sculpt brushes");
pub(crate) const ALPHAS_TAB: &str = crate::text!("Show the alphas");
/// Accessible name of the card that adds an alpha of your own.
pub(crate) const ADD_ALPHA: &str = crate::text!("Add an alpha of your own");
/// Accessible name of the card for no alpha.
pub(crate) const NO_ALPHA: &str = crate::text!("No alpha");

/// How a stroke is laid down (decision 0308, the plan's choice 16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum StrokeKind {
    /// Dabs along wherever the mouse goes: the standard.
    #[default]
    Freehand,
    /// Press, drag and let go: a straight stroke from the press to the
    /// mouse.
    Line,
    /// Press where it goes, drag to size and turn one dab: how alphas are
    /// stamped.
    Stamp,
}

impl StrokeKind {
    /// Every kind, in menu order.
    pub(crate) const ALL: [Self; 3] = [Self::Freehand, Self::Line, Self::Stamp];

    /// Its name.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Freehand => tr("Freehand"),
            Self::Line => tr("Line"),
            Self::Stamp => tr("Drag a stamp"),
        }
    }

    /// What it does, in plain words.
    pub(crate) fn explanation(self) -> &'static str {
        match self {
            Self::Freehand => tr("The brush follows the mouse wherever it goes"),
            Self::Line => tr(
                "Press, drag and let go: a straight stroke from where you pressed to where you \
                 let go, shown as you drag",
            ),
            Self::Stamp => tr(
                "Press where it goes, then drag: how far sets its size and which way turns it. \
                 One dab, for stamping an alpha",
            ),
        }
    }
}

/// Which tab of the shelf shows in Sculpt mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ShelfTab {
    /// The brushes.
    #[default]
    Brushes,
    /// The alphas.
    Alphas,
}

/// How the sculpt is shown in Sculpt mode (decision 0304): a matcap of
/// the app's own, or the app's usual lit look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SculptLook {
    /// Warm clay: the standard.
    #[default]
    Clay,
    /// Red wax, glossy: shows small detail.
    RedWax,
    /// Pale stone, matte.
    PaleStone,
    /// The app's usual lit look, with the objects' colours.
    Lit,
}

impl SculptLook {
    /// Every look, in menu order.
    pub(crate) const ALL: [Self; 4] = [Self::Clay, Self::RedWax, Self::PaleStone, Self::Lit];

    /// Its name.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Clay => tr("Clay"),
            Self::RedWax => tr("Red wax"),
            Self::PaleStone => tr("Pale stone"),
            Self::Lit => tr("Lit"),
        }
    }

    /// The matcap the renderer draws (`render::SurfaceDraw::Matcap`), or
    /// `None` for the usual look.
    pub(crate) fn matcap(self) -> Option<u8> {
        match self {
            Self::Clay => Some(0),
            Self::RedWax => Some(1),
            Self::PaleStone => Some(2),
            Self::Lit => None,
        }
    }
}

/// Sculpt mode's settings.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SculptSettings {
    /// The brush chosen (Smooth is also held with Shift).
    pub(crate) brush: BrushKind,
    /// Each brush's own settings, in [`BrushKind::ALL`]'s order.
    pub(crate) brushes: [BrushSettings; BrushKind::ALL.len()],
    /// How the sculpt is shown.
    pub(crate) look: SculptLook,
    /// Whether creases are darkened and ridges lit (cavity).
    pub(crate) cavity: bool,
    /// What is being sculpted, in plain words, set by the workbench each
    /// frame for the Sculpt tab.
    pub(crate) status: String,
    /// How strokes are laid down.
    pub(crate) stroke: StrokeKind,
    /// The alpha the brushes stamp or drag, if any.
    pub(crate) alpha: Option<Arc<Alpha>>,
    /// Your own alphas, from the alpha library (`None` until first read).
    pub(crate) own_alphas: Option<Vec<Arc<Alpha>>>,
    /// Whether the alpha's top follows the stroke's way (otherwise it
    /// points up the view).
    pub(crate) alpha_follows: bool,
    /// The alpha turned this far, in degrees, anticlockwise.
    pub(crate) alpha_angle: f32,
    /// How the pen's pressure changes the size.
    pub(crate) size_pressure: PressureCurve,
    /// How the pen's pressure changes the strength.
    pub(crate) strength_pressure: PressureCurve,
    /// How long the lazy mouse's string is, 0 to 1 of the longest.
    pub(crate) smoothing: f32,
    /// Whether smoothing is on (D).
    pub(crate) smoothing_on: bool,
    /// The pen's pressure now, if a pen is near, for the Sculpt tab.
    pub(crate) pen: Option<f32>,
    /// The shelf's tab.
    pub(crate) shelf_tab: ShelfTab,
    /// Asked to add an alpha of your own (the shelf's + card).
    pub(crate) alpha_asked: bool,
    /// Asked to take an alpha of your own out of the library, by name.
    pub(crate) alpha_forget: Option<String>,
    /// Symmetry (decision 0310).
    pub(crate) symmetry: crate::sculpt_masks::SymmetrySettings,
    /// What the Mask brush draws with.
    pub(crate) mask_tool: crate::sculpt_masks::MaskTool,
    /// How thick a shape made from the mask is, in centimetres.
    pub(crate) shape_thickness: f32,
    /// An action asked for in the Sculpt tab, for the workbench.
    pub(crate) asked: Option<crate::sculpt_masks::SculptAction>,
    /// Whether the floating brush bar shows (in Sculpt mode), so the
    /// Sculpt tab leaves out its numbers, as the Paint tab does.
    pub(crate) bar_shown: bool,
}

impl Default for SculptSettings {
    fn default() -> Self {
        Self {
            brush: BrushKind::Draw,
            brushes: BrushKind::ALL.map(BrushSettings::standard),
            look: SculptLook::Clay,
            cavity: true,
            status: String::new(),
            stroke: StrokeKind::Freehand,
            alpha: None,
            own_alphas: None,
            alpha_follows: false,
            alpha_angle: 0.0,
            // Pressure changes the size, as in Paint mode, and the
            // strength too (the plan's choice 22).
            size_pressure: PressureCurve {
                on: true,
                least: 0.1,
                curve: Curve::Even,
            },
            strength_pressure: PressureCurve {
                on: true,
                least: 0.1,
                curve: Curve::Even,
            },
            smoothing: 0.3,
            smoothing_on: false,
            pen: None,
            shelf_tab: ShelfTab::Brushes,
            alpha_asked: false,
            alpha_forget: None,
            symmetry: Default::default(),
            mask_tool: Default::default(),
            shape_thickness: 1.0,
            asked: None,
            bar_shown: false,
        }
    }
}

/// The key that picks brush `kind` in Sculpt mode (the plan's choice 8):
/// 1 Draw, 2 Clay strips, 3 Inflate, 4 Crease, 5 Flatten, 6 Grab, 7 Snake
/// hook, 9 Mask (decision 0310; pressed again, the box and lasso
/// masks). The rest are on the shelf and in the tool menu (E); Smooth is
/// held with Shift. 8 is kept for Trim.
pub(crate) fn brush_key(kind: BrushKind) -> Option<egui::Key> {
    use egui::Key;
    match kind {
        BrushKind::Draw => Some(Key::Num1),
        BrushKind::ClayStrips => Some(Key::Num2),
        BrushKind::Inflate => Some(Key::Num3),
        BrushKind::Crease => Some(Key::Num4),
        BrushKind::Flatten => Some(Key::Num5),
        BrushKind::Grab => Some(Key::Num6),
        BrushKind::SnakeHook => Some(Key::Num7),
        BrushKind::Mask => Some(Key::Num9),
        _ => None,
    }
}

/// How brush `kind` is picked with the keyboard, in words.
pub(crate) fn brush_key_text(kind: BrushKind) -> &'static str {
    match brush_key(kind) {
        Some(key) => key.symbol_or_name(),
        None if kind == BrushKind::Smooth => tr("hold Shift"),
        None => tr("shelf or E"),
    }
}

impl SculptSettings {
    /// Brush `kind`'s settings.
    pub(crate) fn of(&self, kind: BrushKind) -> BrushSettings {
        self.brushes[kind.index()]
    }

    /// Brush `kind`'s settings, to change.
    pub(crate) fn of_mut(&mut self, kind: BrushKind) -> &mut BrushSettings {
        &mut self.brushes[kind.index()]
    }

    /// Scales the chosen brush's size by `factor` ([ and ], Ctrl + right
    /// drag).
    pub(crate) fn scale_size(&mut self, factor: f32) {
        let brush = self.of_mut(self.brush);
        brush.radius *= factor;
        *brush = brush.clamped();
    }

    /// Brush `kind`'s settings with the pen pressing `pressure` (none: a
    /// mouse, full size and strength), by the pressure curves.
    pub(crate) fn pressed(&self, kind: BrushKind, pressure: Option<f32>) -> BrushSettings {
        let mut brush = self.of(kind);
        brush.radius *= self.size_pressure.share(pressure);
        brush.strength *= self.strength_pressure.share(pressure);
        brush
    }

    /// How far the lazy mouse's string reaches, in points: nothing when
    /// smoothing is off.
    pub(crate) fn smoothing_reach(&self) -> f32 {
        if self.smoothing_on {
            self.smoothing.clamp(0.0, 1.0) * crate::pen::SMOOTHING_REACH
        } else {
            0.0
        }
    }

    /// The alpha's angle in radians.
    pub(crate) fn alpha_turn(&self) -> f32 {
        if self.alpha_angle.is_finite() {
            self.alpha_angle.to_radians()
        } else {
            0.0
        }
    }

    /// Adds `amount` to the chosen brush's strength (Ctrl + right drag up
    /// and down).
    pub(crate) fn add_strength(&mut self, amount: f32) {
        let brush = self.of_mut(self.brush);
        brush.strength += amount;
        *brush = brush.clamped();
    }
}

/// Size, Strength and Spacing of `settings`, as bars filled to their
/// value.
pub(crate) fn numbers(ui: &mut Ui, settings: &mut BrushSettings) {
    weak(ui, tr("Size"));
    let (low, high) = sculpt::brush::RADIUS_RANGE;
    fill_field(
        ui,
        tr(SIZE_LABEL),
        &mut settings.radius,
        (low, high),
        Shown {
            scale: 1.0,
            suffix: " px",
            decimals: 0,
            eased: true,
            size: true,
        },
    )
    .on_hover_text(tr(
        "The brush's radius on screen: drag sideways or click to type. [ and ] change it, as \
         does Ctrl + right drag (sideways)",
    ));
    let percent = Shown {
        scale: 100.0,
        suffix: "%",
        decimals: 0,
        eased: false,
        size: false,
    };
    weak(ui, tr("Strength"));
    fill_field(
        ui,
        tr(STRENGTH_LABEL),
        &mut settings.strength,
        (0.0, 1.0),
        percent,
    )
    .on_hover_text(tr(
        "How much each dab changes the surface. Ctrl + right drag (up and down) changes it",
    ));
    weak(ui, tr("Spacing"));
    let (low, high) = sculpt::brush::SPACING_RANGE;
    fill_field(
        ui,
        tr(SPACING_LABEL),
        &mut settings.spacing,
        (low, high),
        percent,
    )
    .on_hover_text(tr(
        "The gap between the brush's dabs along a stroke, as a share of its radius: small for \
         smooth strokes",
    ));
    *settings = settings.clamped();
}

/// The brushes as buttons, the chosen one lit.
fn brush_buttons(ui: &mut Ui, settings: &mut SculptSettings) {
    for kind in BrushKind::ALL {
        let on = settings.brush == kind;
        if ui
            .selectable_label(on, tr(kind.name()))
            .on_hover_text(crate::trf!(
                "{} ({})",
                tr(kind.explanation()),
                brush_key_text(kind)
            ))
            .clicked()
        {
            settings.brush = kind;
        }
    }
}

/// The falloff as a drop-down.
pub(crate) fn falloff_choice(ui: &mut Ui, brush: &mut BrushSettings) {
    weak(ui, tr("Falloff"));
    egui::ComboBox::from_id_salt("sculpt_falloff")
        .selected_text(tr(brush.falloff.name()))
        .show_ui(ui, |ui| {
            for f in Falloff::ALL {
                ui.selectable_value(&mut brush.falloff, f, tr(f.name()));
            }
        })
        .response
        .on_hover_text(tr(
            "How the brush's strength fades from its middle to its rim: Smooth for soft forms, \
             Sharp and Spike for creases and points, Flat for an even push",
        ));
}

/// How strokes are laid down, as a drop-down.
pub(crate) fn stroke_choice(ui: &mut Ui, settings: &mut SculptSettings) {
    weak(ui, tr("Stroke"));
    egui::ComboBox::from_id_salt("sculpt_stroke")
        .selected_text(settings.stroke.name())
        .show_ui(ui, |ui| {
            for kind in StrokeKind::ALL {
                ui.selectable_value(&mut settings.stroke, kind, kind.name())
                    .on_hover_text(kind.explanation());
            }
        })
        .response
        .on_hover_text(tr(
            "How a stroke is laid down: Freehand follows the mouse, Line goes straight from the \
             press to where you let go, and Drag a stamp lays one dab, sized and turned by the \
             drag. Grab, Elastic grab and Snake hook always follow the mouse",
        ));
}

/// Every alpha there is to choose: the ones made in code, then your own.
pub(crate) fn every_alpha(settings: &SculptSettings) -> Vec<Arc<Alpha>> {
    let mut all: Vec<_> = Made::ALL.iter().map(|k| sculpt::alpha::made(*k)).collect();
    all.extend(settings.own_alphas.iter().flatten().cloned());
    all
}

/// An alpha's name as shown: a made one's translated.
pub(crate) fn alpha_name(alpha: &Alpha) -> &str {
    match alpha.made() {
        Some(kind) => tr(kind.name()),
        None => alpha.name(),
    }
}

/// What an alpha is good for, in plain words.
fn alpha_help(alpha: &Alpha) -> String {
    match alpha.made() {
        Some(kind) => tr(kind.explanation()).to_string(),
        None => crate::trf!("Your own alpha, {}", alpha.name()),
    }
}

/// The alpha as a drop-down, for the brush bar.
pub(crate) fn alpha_choice(ui: &mut Ui, settings: &mut SculptSettings) {
    weak(ui, tr("Alpha"));
    let shown = settings
        .alpha
        .as_deref()
        .map_or_else(|| tr("None").to_string(), |a| alpha_name(a).to_string());
    let all = every_alpha(settings);
    egui::ComboBox::from_id_salt("sculpt_alpha")
        .selected_text(shown)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(settings.alpha.is_none(), tr("None"))
                .clicked()
            {
                settings.alpha = None;
            }
            for alpha in all {
                let on = settings.alpha.as_deref() == Some(&*alpha);
                if ui
                    .selectable_label(on, alpha_name(&alpha))
                    .on_hover_text(alpha_help(&alpha))
                    .clicked()
                {
                    settings.alpha = Some(alpha);
                }
            }
        })
        .response
        .on_hover_text(tr(
            "A greyscale picture the brush stamps or drags: white works fully, black not at all. \
             More are on the shelf's Alphas tab, where you can add your own",
        ));
}

/// The pen and smoothing rows of the Sculpt tab (decision 0308).
fn pen_rows(ui: &mut Ui, settings: &mut SculptSettings) {
    for (curve, name, help, least) in [
        (
            &mut settings.size_pressure,
            tr("Pressure → size"),
            tr("Press harder for a bigger brush. The number is the size at the lightest touch"),
            tr("Least sculpt size with pressure"),
        ),
        (
            &mut settings.strength_pressure,
            tr("Pressure → strength"),
            tr(
                "Press harder for a stronger brush. The number is the strength at the lightest \
                 touch",
            ),
            tr("Least sculpt strength with pressure"),
        ),
    ] {
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut curve.on, name).on_hover_text(help);
            ui.add_enabled_ui(curve.on, |ui| {
                crate::attributes::stepper::percent(ui, &mut curve.least, (0.0, 1.0), least);
                crate::attributes::brush_rows::curve_list(ui, least, &mut curve.curve);
            });
        });
    }
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut settings.smoothing_on, tr("Smoothing"))
            .on_hover_text(tr(
                "The brush follows the pointer on a string, so strokes come out steadier and \
                 wobbles are left out. D turns it on and off; the number is how long the string \
                 is",
            ));
        ui.add_enabled_ui(settings.smoothing_on, |ui| {
            crate::attributes::stepper::percent(
                ui,
                &mut settings.smoothing,
                (0.0, 1.0),
                tr("Sculpt smoothing"),
            );
        });
    });
    let text = match settings.pen {
        Some(p) => crate::trf!("Pen: pressure {:.0}%", p * 100.0),
        None => tr("No pen pressure: a mouse sculpts at full size and strength.").to_string(),
    };
    ui.label(RichText::new(text).small().color(theme::TEXT_WEAK));
}

/// The stroke and alpha rows of the Sculpt tab (decision 0308).
fn stroke_rows(ui: &mut Ui, settings: &mut SculptSettings) {
    ui.horizontal_wrapped(|ui| {
        stroke_choice(ui, settings);
        alpha_choice(ui, settings);
    });
    ui.label(RichText::new(settings.stroke.explanation()).color(theme::TEXT_WEAK));
    ui.add_enabled_ui(settings.alpha.is_some(), |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut settings.alpha_follows, tr("Follow the stroke"))
                .on_hover_text(tr(
                    "The alpha's top points the way the stroke goes, as a stamp drags along. \
                     Off: it points up the view",
                ));
            weak(ui, tr("Angle"));
            ui.add(
                egui::DragValue::new(&mut settings.alpha_angle)
                    .range(-180.0..=180.0)
                    .speed(1.0)
                    .suffix("°"),
            )
            .on_hover_text(tr(
                "Turns the alpha this far (anticlockwise). Drag a stamp turns it by the drag \
                 instead",
            ));
        });
    });
}

/// The Sculpt tab beside Attributes (decision 0304): what is sculpted,
/// then the brush's settings in folding sections as the Paint tab's
/// (decision 0311), each saying on the right what's set inside: Brush,
/// Stroke and alpha, Symmetry, Mask, Pen and smoothing, Look, and the
/// keys.
pub(crate) fn panel(ui: &mut Ui, settings: &mut SculptSettings) {
    crate::fitting::side_scroll("sculpt_tab").show(ui, |ui| {
        ui.label(RichText::new(settings.status.as_str()).color(theme::TEXT_WEAK));
        crate::attributes::section(ui, tr(settings.brush.name()));
        let kind = settings.brush;
        let falloff = tr(settings.of(kind).falloff.name()).to_string();
        let bar = settings.bar_shown;
        fold(
            ui,
            "sculpt_brush",
            crate::text!("Brush"),
            &falloff,
            true,
            |ui| {
                ui.horizontal_wrapped(|ui| brush_buttons(ui, settings));
                ui.label(RichText::new(tr(settings.brush.explanation())).color(theme::TEXT_WEAK));
                // Size, strength and spacing are on the floating brush bar
                // when it shows, as in the Paint tab (decision 0228).
                if !bar {
                    ui.horizontal_wrapped(|ui| numbers(ui, settings.of_mut(kind)));
                }
                ui.horizontal(|ui| falloff_choice(ui, settings.of_mut(kind)));
            },
        );
        let aside = stroke_aside(settings);
        fold(
            ui,
            "sculpt_stroke",
            crate::text!("Stroke and alpha"),
            &aside,
            false,
            |ui| {
                stroke_rows(ui, settings);
            },
        );
        let aside = symmetry_aside(&settings.symmetry);
        fold(
            ui,
            "sculpt_symmetry",
            crate::text!("Symmetry"),
            &aside,
            false,
            |ui| {
                crate::sculpt_masks::symmetry_rows(ui, &mut settings.symmetry, &mut settings.asked);
            },
        );
        let aside = settings.mask_tool.name().to_lowercase();
        fold(
            ui,
            "sculpt_mask",
            crate::text!("Mask"),
            &aside,
            false,
            |ui| {
                crate::sculpt_masks::mask_rows(
                    ui,
                    &mut settings.mask_tool,
                    &mut settings.shape_thickness,
                    &mut settings.asked,
                );
            },
        );
        let aside = pen_aside(settings);
        fold(
            ui,
            "sculpt_pen",
            crate::text!("Pen and smoothing"),
            &aside,
            false,
            |ui| {
                pen_rows(ui, settings);
            },
        );
        let aside = look_aside(settings);
        fold(
            ui,
            "sculpt_look",
            crate::text!("Look"),
            &aside,
            false,
            |ui| {
                look_rows(ui, settings);
            },
        );
        fold(
            ui,
            "sculpt_keys",
            crate::text!("Keys for sculpting"),
            "",
            false,
            |ui| {
                crate::attributes::brush_rows::key_grid(ui, "sculpt_keys", SCULPT_KEYS);
            },
        );
    });
}

/// The Look section: the matcaps and cavity.
fn look_rows(ui: &mut Ui, settings: &mut SculptSettings) {
    ui.horizontal_wrapped(|ui| {
        for look in SculptLook::ALL {
            ui.selectable_value(&mut settings.look, look, look.name())
                .on_hover_text(tr(
                    "How the sculpt is shown while you sculpt: a matcap (a lit material picture \
                     wrapped on the shape) makes forms read clearly. Only the view changes",
                ));
        }
    });
    ui.checkbox(&mut settings.cavity, tr("Cavity"))
        .on_hover_text(tr(
            "Darkens creases and lights ridges, so detail reads clearly",
        ));
}

/// What the Stroke and alpha section says when closed: the stroke, and
/// the alpha if any.
fn stroke_aside(settings: &SculptSettings) -> String {
    let stroke = settings.stroke.name().to_lowercase();
    match settings.alpha.as_deref() {
        Some(alpha) => format!("{stroke}, {}", alpha_name(alpha).to_lowercase()),
        None => stroke,
    }
}

/// What the Symmetry section says when closed: "Mirror X", with radial
/// copies, or "off".
pub(crate) fn symmetry_aside(symmetry: &crate::sculpt_masks::SymmetrySettings) -> String {
    if !symmetry.on {
        return tr("off").to_string();
    }
    let axes = crate::sculpt_bar::axes_text(symmetry.axes);
    let mut parts = Vec::new();
    if !axes.is_empty() {
        parts.push(crate::trf!("Mirror {}", axes));
    }
    if symmetry.radial {
        parts.push(crate::trf!("radial {}", symmetry.radial_count));
    }
    if parts.is_empty() {
        tr("on").to_string()
    } else {
        parts.join(", ")
    }
}

/// What the Pen and smoothing section says when closed, as the Paint
/// tab's: "pressure → size, pressure → strength", "smoothing", or "off".
fn pen_aside(settings: &SculptSettings) -> String {
    let mut on = Vec::new();
    if settings.size_pressure.on {
        on.push(tr("pressure → size"));
    }
    if settings.strength_pressure.on {
        on.push(tr("pressure → strength"));
    }
    if settings.smoothing_on {
        on.push(tr("smoothing"));
    }
    if on.is_empty() {
        tr("off").to_string()
    } else {
        on.join(", ")
    }
}

/// What the Look section says when closed: the look, and cavity if on.
fn look_aside(settings: &SculptSettings) -> String {
    let look = settings.look.name().to_lowercase();
    if settings.cavity {
        crate::trf!("{}, cavity", look)
    } else {
        look
    }
}

/// Sculpt mode's keys, one per line, for the Keys for sculpting section.
const SCULPT_KEYS: &[(&str, &str)] = &[
    (
        "1 – 7",
        crate::text!("Draw, Clay strips, Inflate, Crease, Flatten, Grab, Snake hook"),
    ),
    (
        "9",
        crate::text!("Mask brush; again for the box and lasso masks"),
    ),
    ("Shift", crate::text!("Hold to smooth with any brush")),
    (
        "Ctrl",
        crate::text!("Hold while stroking to dig in instead"),
    ),
    ("[  ]", crate::text!("Brush smaller, bigger")),
    (
        "Ctrl + right drag",
        crate::text!("Size (sideways) and strength (up and down)"),
    ),
    ("D", crate::text!("Smoothing on or off")),
    ("L", crate::text!("Symmetry on or off")),
    ("Ctrl + I", crate::text!("Invert the mask")),
    ("Alt + M", crate::text!("Clear the mask")),
    ("Ctrl + wheel", crate::text!("Grow or shrink the mask")),
    ("Q", crate::text!("Select: clicks pick the shape to sculpt")),
    (
        "W  R  S",
        crate::text!("Move, rotate or scale (the unmasked part, when masked)"),
    ),
    ("E", crate::text!("Every brush, in a ring at the mouse")),
    (
        "Esc",
        crate::text!("Take back the stroke under way, else leave Sculpt mode"),
    ),
];

/// The shelf's tab buttons in Sculpt mode: Brushes and Alphas.
fn shelf_tabs(ui: &mut Ui, tab: &mut ShelfTab) {
    for (which, label, name) in [
        (ShelfTab::Brushes, tr("Brushes"), tr(BRUSHES_TAB)),
        (ShelfTab::Alphas, tr("Alphas"), tr(ALPHAS_TAB)),
    ] {
        let on = *tab == which;
        let button = ui.add(egui::Button::selectable(on, label));
        button.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, name));
        if button.clicked() {
            *tab = which;
        }
    }
    ui.separator();
}

/// The shelf under the view in Sculpt mode: its Brushes and Alphas tabs.
pub(crate) fn shelf(ui: &mut Ui, settings: &mut SculptSettings) {
    match settings.shelf_tab {
        ShelfTab::Brushes => brush_shelf(ui, settings),
        ShelfTab::Alphas => alpha_shelf(ui, settings),
    }
}

/// The Brushes tab (decision 0306): what the chosen brush does in plain
/// words, then a card per brush with its picture (the surface from the
/// side, before and after), in a row that scrolls like the paint shelf's.
fn brush_shelf(ui: &mut Ui, settings: &mut SculptSettings) {
    let chosen = settings.brush;
    ui.horizontal_wrapped(|ui| {
        shelf_tabs(ui, &mut settings.shelf_tab);
        ui.label(
            RichText::new(crate::trf!(
                "{}: {} ({})",
                tr(chosen.name()),
                tr(chosen.explanation()),
                brush_key_text(chosen)
            ))
            .small()
            .color(theme::TEXT_WEAK),
        );
    });
    crate::shelf_cards::row(ui, "sculpt_brushes", |ui| {
        for kind in BrushKind::ALL {
            let response = crate::shelf_cards::card(
                ui,
                tr(kind.name()),
                tr(kind.name()).to_string(),
                settings.brush == kind,
                |painter, rect| {
                    crate::icons::paint(
                        painter,
                        rect.shrink(rect.width() * 0.12),
                        crate::icons::Icon::SculptBrush(kind),
                        theme::TEXT,
                    );
                },
            )
            .on_hover_text(crate::trf!(
                "{} ({})",
                tr(kind.explanation()),
                brush_key_text(kind)
            ));
            if response.clicked() {
                settings.brush = kind;
            }
        }
    });
}

/// The pictures of the alphas shown on the shelf, by the alpha's key,
/// kept in egui's memory between frames.
#[derive(Clone, Default)]
struct AlphaPictures(HashMap<u64, egui::TextureHandle>);

/// The side of an alpha's picture on the shelf, in pixels.
const PICTURE_SIDE: usize = 92;

/// The picture of `alpha` for the shelf: made once, then kept.
fn alpha_picture(ctx: &egui::Context, alpha: &Alpha) -> egui::TextureId {
    let id = egui::Id::new("sculpt_alpha_pictures");
    let kept = ctx.data(|d| {
        d.get_temp::<AlphaPictures>(id)
            .and_then(|p| p.0.get(&alpha.key()).map(egui::TextureHandle::id))
    });
    if let Some(texture) = kept {
        return texture;
    }
    let side = PICTURE_SIDE;
    let mut pixels = Vec::with_capacity(side * side);
    for y in 0..side {
        for x in 0..side {
            let at = |i: usize| (i as f32 + 0.5) / side as f32 * 2.0 - 1.0;
            let v = alpha.at(at(x), -at(y));
            pixels.push(egui::Color32::from_gray((v * 255.0).round() as u8));
        }
    }
    let handle = ctx.load_texture(
        format!("sculpt alpha {}", alpha.key()),
        egui::ColorImage::new([side, side], pixels),
        egui::TextureOptions::LINEAR,
    );
    let texture = handle.id();
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<AlphaPictures>(id)
            .0
            .insert(alpha.key(), handle);
    });
    texture
}

/// The Alphas tab (decision 0308): no alpha, the alphas made in code,
/// your own (right-click to take one out), and a card to add one.
fn alpha_shelf(ui: &mut Ui, settings: &mut SculptSettings) {
    ui.horizontal_wrapped(|ui| {
        shelf_tabs(ui, &mut settings.shelf_tab);
        let text = match settings.alpha.as_deref() {
            Some(alpha) => crate::trf!("{}: {}", alpha_name(alpha), alpha_help(alpha)),
            None => tr(
                "An alpha is a greyscale picture the brush stamps or drags: white works fully, \
                 black not at all. Pick one, then stroke, or choose Drag a stamp in the brush bar",
            )
            .to_string(),
        };
        ui.label(RichText::new(text).small().color(theme::TEXT_WEAK));
    });
    let all = every_alpha(settings);
    crate::shelf_cards::row(ui, "sculpt_alphas", |ui| {
        let none = crate::shelf_cards::card(
            ui,
            tr("None"),
            tr(NO_ALPHA).to_string(),
            settings.alpha.is_none(),
            |painter, rect| {
                painter.circle_filled(rect.center(), rect.width() * 0.3, theme::TEXT_WEAK);
            },
        )
        .on_hover_text(tr("No alpha: the brush works evenly, by its falloff"));
        if none.clicked() {
            settings.alpha = None;
        }
        for alpha in all {
            let texture = alpha_picture(ui.ctx(), &alpha);
            let chosen = settings.alpha.as_deref() == Some(&*alpha);
            let name = alpha_name(&alpha).to_string();
            let response = crate::shelf_cards::card(
                ui,
                &name,
                crate::trf!("{} {}", tr("Alpha"), name),
                chosen,
                |painter, rect| {
                    painter.image(
                        texture,
                        rect.shrink(2.0),
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                },
            )
            .on_hover_text(alpha_help(&alpha));
            if response.clicked() {
                settings.alpha = Some(Arc::clone(&alpha));
            }
            if alpha.made().is_none() {
                response.context_menu(|ui| {
                    if ui.button(tr("Take out of your alphas")).clicked() {
                        settings.alpha_forget = Some(alpha.name().to_string());
                        ui.close();
                    }
                });
            }
        }
        let add = crate::shelf_cards::card(
            ui,
            tr("Add…"),
            tr(ADD_ALPHA).to_string(),
            false,
            |p, r| {
                p.text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    "+",
                    egui::FontId::proportional(24.0),
                    theme::TEXT_WEAK,
                );
            },
        )
        .on_hover_text(tr(
            "Add a picture of your own (PNG, JPEG, TGA or BMP) as an alpha: its light parts \
             work, its dark and see-through parts don't. It is kept for every project",
        ));
        if add.clicked() {
            settings.alpha_asked = true;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_brush_keeps_its_own_size_and_strength() {
        let mut s = SculptSettings::default();
        s.scale_size(2.0);
        s.add_strength(0.3);
        assert_eq!(s.of(BrushKind::Draw).radius, 100.0);
        assert!((s.of(BrushKind::Draw).strength - 0.8).abs() < 1e-6);
        s.brush = BrushKind::Smooth;
        assert_eq!(s.of(BrushKind::Smooth).radius, 50.0);
        s.scale_size(1e9);
        assert_eq!(
            s.of(BrushKind::Smooth).radius,
            sculpt::brush::RADIUS_RANGE.1
        );
        s.add_strength(-5.0);
        assert_eq!(s.of(BrushKind::Smooth).strength, 0.0);
        assert_eq!(s.of(BrushKind::Draw).radius, 100.0);
        // The grabbing brushes start at full strength.
        assert_eq!(s.of(BrushKind::Grab).strength, 1.0);
        assert_eq!(s.of(BrushKind::Crease).falloff, Falloff::Sharp);
    }

    #[test]
    fn the_main_brushes_have_the_plans_keys() {
        use egui::Key;
        let keys: Vec<_> = BrushKind::ALL
            .iter()
            .filter_map(|k| brush_key(*k))
            .collect();
        assert_eq!(
            keys,
            vec![
                Key::Num1,
                Key::Num2,
                Key::Num3,
                Key::Num4,
                Key::Num5,
                Key::Num6,
                Key::Num7,
                Key::Num9
            ]
        );
        assert_eq!(brush_key(BrushKind::SnakeHook), Some(Key::Num7));
        assert_eq!(brush_key(BrushKind::Smooth), None);
    }

    #[test]
    fn pressure_scales_size_and_strength_by_their_curves() {
        let mut s = SculptSettings::default();
        let full = s.of(BrushKind::Draw);
        assert_eq!(s.pressed(BrushKind::Draw, None), full, "a mouse: full");
        assert_eq!(s.pressed(BrushKind::Draw, Some(1.0)), full);
        let light = s.pressed(BrushKind::Draw, Some(0.0));
        assert!((light.radius - full.radius * 0.1).abs() < 1e-4);
        assert!((light.strength - full.strength * 0.1).abs() < 1e-6);
        let half = s.pressed(BrushKind::Draw, Some(0.5));
        assert!((half.radius - full.radius * 0.55).abs() < 1e-4);
        // A curve switched off leaves its setting alone.
        s.size_pressure.on = false;
        assert_eq!(s.pressed(BrushKind::Draw, Some(0.0)).radius, full.radius);
        // A pressure that isn't a number counts as a full press.
        s.strength_pressure.curve = Curve::Firm;
        assert_eq!(
            s.pressed(BrushKind::Draw, Some(f32::NAN)).strength,
            full.strength
        );
        assert!(s.pressed(BrushKind::Draw, Some(0.5)).strength < half.strength);
    }

    #[test]
    fn smoothing_reaches_only_when_on() {
        let mut s = SculptSettings::default();
        assert_eq!(s.smoothing_reach(), 0.0);
        s.smoothing_on = true;
        s.smoothing = 1.0;
        assert_eq!(s.smoothing_reach(), crate::pen::SMOOTHING_REACH);
        s.smoothing = 7.0;
        assert_eq!(s.smoothing_reach(), crate::pen::SMOOTHING_REACH);
        s.alpha_angle = f32::NAN;
        assert_eq!(s.alpha_turn(), 0.0);
    }

    #[test]
    fn every_alpha_lists_the_made_ones_then_yours() {
        let mut s = SculptSettings::default();
        assert_eq!(every_alpha(&s).len(), Made::ALL.len());
        let own = Arc::new(Alpha::new(1, 1, vec![200], "mine").unwrap());
        s.own_alphas = Some(vec![Arc::clone(&own)]);
        let all = every_alpha(&s);
        assert_eq!(all.last(), Some(&own));
        assert_eq!(alpha_name(&all[0]), "Stones");
        assert_eq!(alpha_name(&own), "mine");
    }

    #[test]
    fn every_look_but_lit_is_a_matcap() {
        let matcaps: Vec<_> = SculptLook::ALL.iter().map(|l| l.matcap()).collect();
        assert_eq!(matcaps, vec![Some(0), Some(1), Some(2), None]);
    }
}
