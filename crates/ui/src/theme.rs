//! The calm, dark look from DESIGN.md §5.2: dark neutral greys with low
//! contrast between panels, one accent colour for the active mode or tool
//! (red unless the user picks another), and the Inter font.
//!
//! Colours follow the approved mockups. Text colours are checked for
//! readability by the tests at the bottom (WCAG contrast 4.5:1 for normal
//! text).

use egui::{
    Color32, Context, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Frame, Id,
    Margin, Shadow, Stroke, Style, TextStyle, Visuals, vec2,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Window background behind everything, seen in the thin gaps between
/// panels.
pub const BACKGROUND: Color32 = Color32::from_rgb(0x1b, 0x1b, 0x1b);
/// Panel bodies (Attributes, Objects, Coordinates).
pub const PANEL: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
/// Tab strips above the panels.
pub const HEADER: Color32 = Color32::from_rgb(0x22, 0x22, 0x22);
/// The menu bar and status bar.
pub const BAR: Color32 = Color32::from_rgb(0x22, 0x22, 0x22);
/// The command and mode palettes.
pub const PALETTE: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2a);
/// The viewport's own menu line, along its top edge.
pub const VIEWPORT_HEADER: Color32 = Color32::from_rgb(0x2e, 0x2e, 0x2e);
/// Text and number fields.
pub const FIELD: Color32 = Color32::from_rgb(0x1c, 0x1c, 0x1c);
/// The edge of a text or number field.
pub const FIELD_BORDER: Color32 = Color32::from_rgb(0x16, 0x16, 0x16);
/// Hairline borders between areas.
pub const BORDER: Color32 = Color32::from_rgb(0x16, 0x16, 0x16);
/// A hovered or pressed button.
pub const HOVER: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3a);
/// The tab in front, in a tab strip: the same as the panel below it, so
/// the two read as one sheet.
pub const ACTIVE_TAB: Color32 = PANEL;
/// Normal text.
pub const TEXT: Color32 = Color32::from_rgb(0xc8, 0xc8, 0xc8);
/// Bright text: headings and the active tab.
pub const TEXT_STRONG: Color32 = Color32::from_rgb(0xf0, 0xf0, 0xf0);
/// Quieter text: labels, hints, the status bar.
pub const TEXT_WEAK: Color32 = Color32::from_rgb(0xa5, 0xa5, 0xa5);
/// Text that warns: something works, but not quite as asked (a soft
/// amber, readable on the panels).
pub const WARNING: Color32 = Color32::from_rgb(0xe8, 0xb3, 0x4a);

/// The viewport's backdrop: a studio sky over a slightly darker ground.
pub mod backdrop {
    use egui::Color32;

    /// The sky straight up.
    pub const SKY_TOP: Color32 = Color32::from_rgb(0x4a, 0x4a, 0x4a);
    /// The sky at the horizon.
    pub const HORIZON: Color32 = Color32::from_rgb(0x3e, 0x3e, 0x3e);
    /// The ground just below the horizon.
    pub const GROUND: Color32 = Color32::from_rgb(0x38, 0x38, 0x38);
}

/// The accent colour: marks the active mode or tool, the chosen option in
/// a list, and selected text. Red unless the user picks another
/// (Window → Accent colour).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Accent {
    /// A soft, warm red (the standard).
    #[default]
    Red,
    /// A clear, saturated blue.
    Blue,
    /// A deep blue-green.
    MarineGreen,
    /// A dusky purple-pink.
    Plum,
    /// A natural, slightly yellow green.
    Olive,
    /// A dark blue leaning towards green.
    PetrolBlue,
}

/// The shades of one accent colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentColors {
    /// Behind the active button, under a white icon.
    pub fill: Color32,
    /// Lines and marks on the dark panels: rings, underlines, edges.
    pub light: Color32,
    /// A quiet tint behind a chosen row or selected text.
    pub soft: Color32,
    /// Text in the accent colour, such as a selected object's name: the
    /// light shade lifted a little towards white, so it stays readable on
    /// the soft tint behind it.
    pub text: Color32,
}

impl Accent {
    /// Every accent, in the order the menu lists them.
    pub const ALL: [Accent; 6] = [
        Self::Red,
        Self::Blue,
        Self::MarineGreen,
        Self::Plum,
        Self::Olive,
        Self::PetrolBlue,
    ];

    /// The accent's name, as the menu shows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Red => "Red",
            Self::Blue => "Blue",
            Self::MarineGreen => "Marine green",
            Self::Plum => "Plum",
            Self::Olive => "Olive",
            Self::PetrolBlue => "Petrol blue",
        }
    }

    /// The accent's shades.
    pub fn colors(self) -> AccentColors {
        let (fill, light, soft) = match self {
            Self::Red => ([0xc2, 0x43, 0x3d], [0xe5, 0x73, 0x6b], [0x4a, 0x2b, 0x2a]),
            Self::Blue => ([0x24, 0x68, 0xd8], [0x5a, 0x95, 0xf2], [0x22, 0x38, 0x5a]),
            Self::MarineGreen => ([0x16, 0x6a, 0x62], [0x3e, 0x9d, 0x93], [0x1a, 0x35, 0x32]),
            Self::Plum => ([0x9b, 0x3f, 0x7a], [0xcf, 0x72, 0xab], [0x40, 0x28, 0x3a]),
            Self::Olive => ([0x5d, 0x85, 0x30], [0x90, 0xbd, 0x5a], [0x2e, 0x3a, 0x23]),
            Self::PetrolBlue => ([0x1a, 0x6b, 0x82], [0x4c, 0xa3, 0xbb], [0x1d, 0x35, 0x40]),
        };
        let rgb = |[r, g, b]: [u8; 3]| Color32::from_rgb(r, g, b);
        let light = rgb(light);
        AccentColors {
            fill: rgb(fill),
            light,
            soft: rgb(soft),
            text: light.lerp_to_gamma(Color32::WHITE, 0.25),
        }
    }
}

/// Where the current accent is kept in egui's memory.
fn accent_id() -> Id {
    Id::new("theme_accent")
}

/// The accent colour in use.
pub fn accent(ctx: &Context) -> Accent {
    ctx.data(|d| d.get_temp::<Accent>(accent_id()))
        .unwrap_or_default()
}

/// Switches the accent colour for the whole app.
pub fn set_accent(ctx: &Context, accent: Accent) {
    ctx.data_mut(|d| d.insert_temp(accent_id(), accent));
    ctx.set_style_of(egui::Theme::Dark, style_with(accent));
    ctx.set_style_of(egui::Theme::Light, style_with(accent));
}

/// The numbered checker's two colours (decision 0107) for `accent`: the
/// accent itself at the top left, and a colour a third of the way back
/// round the colour wheel at the bottom right (red runs through purple to
/// blue, blue through teal to green), so the cells between are clear,
/// different colours. sRGB.
pub fn grid_colors(accent: Accent) -> [[u8; 3]; 2] {
    let from = accent.colors().fill;
    let mut hsva = egui::ecolor::Hsva::from(from);
    hsva.h = (hsva.h - 120.0 / 360.0).rem_euclid(1.0);
    let to = Color32::from(hsva);
    [[from.r(), from.g(), from.b()], [to.r(), to.g(), to.b()]]
}

#[cfg(test)]
mod grid_color_tests {
    use super::*;

    #[test]
    fn each_accent_gives_its_own_two_clear_colours() {
        let all = [
            Accent::Red,
            Accent::Blue,
            Accent::MarineGreen,
            Accent::Plum,
            Accent::Olive,
            Accent::PetrolBlue,
        ];
        let made: Vec<[[u8; 3]; 2]> = all.iter().map(|a| grid_colors(*a)).collect();
        for (accent, [from, to]) in all.iter().zip(&made) {
            let fill = accent.colors().fill;
            assert_eq!(*from, [fill.r(), fill.g(), fill.b()]);
            assert_ne!(from, to);
        }
        for (i, a) in made.iter().enumerate() {
            assert!(made[i + 1..].iter().all(|b| b != a));
        }
    }
}

/// Icon and axis colours, following the approved mockups: icons are a
/// soft white, except the shapes you can create, which are blue; the axes
/// are X red, Y blue and Z green, the project lead's choice (most 3D
/// programs give Y green and Z blue; see decision 0012).
pub mod tint {
    use egui::Color32;

    /// Palette icons.
    pub const ICON: Color32 = Color32::from_rgb(0xd4, 0xd4, 0xd4);
    /// Shapes you can create (cube, cylinder, plane, sphere).
    pub const SHAPE: Color32 = Color32::from_rgb(0x62, 0xb8, 0xf2);
    /// The X axis.
    pub const AXIS_X: Color32 = Color32::from_rgb(0xef, 0x6b, 0x63);
    /// The Y axis.
    pub const AXIS_Y: Color32 = Color32::from_rgb(0x6a, 0xa2, 0xff);
    /// The Z axis (up).
    pub const AXIS_Z: Color32 = Color32::from_rgb(0x7f, 0xd0, 0x5a);

    /// Every icon and axis colour, for tests.
    pub const ALL: [Color32; 5] = [ICON, SHAPE, AXIS_X, AXIS_Y, AXIS_Z];
}

/// How strongly an unavailable command's icon is drawn: a little dimmer
/// than an available one, so the palettes keep the mockups' look while
/// still hinting that the command isn't there yet.
pub const UNAVAILABLE_STRENGTH: f32 = 0.85;

/// Name of the bolder font family, for headings.
pub const SEMIBOLD: &str = "semibold";

/// The semibold interface font at `size`, or the regular one where it
/// isn't loaded (tests run without the app's fonts).
pub fn semibold(ctx: &Context, size: f32) -> FontId {
    let family = FontFamily::Name(SEMIBOLD.into());
    if ctx.fonts(|f| f.definitions().families.contains_key(&family)) {
        FontId::new(size, family)
    } else {
        FontId::proportional(size)
    }
}

/// Inter, the interface font (SIL Open Font License 1.1; see licenses/fonts).
const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");

/// Applies the theme to the whole app, with the standard accent. Call
/// once at startup; [`set_accent`] changes the accent later.
///
/// The app is always dark, whatever the system prefers: the same style is
/// set for both of egui's themes, and dark is chosen.
pub fn apply(ctx: &Context) {
    ctx.set_fonts(fonts());
    set_accent(ctx, Accent::default());
    ctx.set_theme(egui::ThemePreference::Dark);
}

/// Inter for interface text, in front of egui's own fonts (which stay as
/// fallbacks for symbols and other scripts).
pub fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "inter".into(),
        Arc::new(FontData::from_static(INTER_REGULAR)),
    );
    fonts.font_data.insert(
        "inter-semibold".into(),
        Arc::new(FontData::from_static(INTER_SEMIBOLD)),
    );
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, "inter".into());
    let mut semibold = vec!["inter-semibold".to_string()];
    semibold.extend(proportional.iter().skip(1).cloned());
    fonts
        .families
        .insert(FontFamily::Name(SEMIBOLD.into()), semibold);
    fonts
}

/// Colours, sizes and spacing, with the standard accent.
pub fn style() -> Style {
    style_with(Accent::default())
}

/// Colours, sizes and spacing, with the given accent.
pub fn style_with(accent: Accent) -> Style {
    let mut style = Style {
        visuals: visuals(accent),
        ..Style::default()
    };
    style.text_styles = [
        (TextStyle::Small, FontId::proportional(11.0)),
        (TextStyle::Body, FontId::proportional(12.5)),
        (TextStyle::Button, FontId::proportional(12.5)),
        (
            TextStyle::Heading,
            FontId::new(16.0, FontFamily::Name(SEMIBOLD.into())),
        ),
        (TextStyle::Monospace, FontId::monospace(12.0)),
    ]
    .into();
    let spacing = &mut style.spacing;
    spacing.item_spacing = vec2(6.0, 4.0);
    spacing.button_padding = vec2(8.0, 3.0);
    spacing.interact_size.y = 22.0;
    spacing.menu_margin = Margin::same(6);
    spacing.window_margin = Margin::same(12);
    style
}

/// The colour scheme.
pub fn visuals(accent: Accent) -> Visuals {
    let accent = accent.colors();
    let mut v = Visuals::dark();
    v.override_text_color = None;
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.window_stroke = Stroke::new(1.0, BORDER);
    v.window_corner_radius = CornerRadius::same(6);
    v.menu_corner_radius = CornerRadius::same(4);
    v.window_shadow = Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(90),
    };
    v.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: Color32::from_black_alpha(80),
    };
    v.extreme_bg_color = FIELD;
    v.text_edit_bg_color = Some(FIELD);
    v.faint_bg_color = Color32::from_rgb(0x2c, 0x2c, 0x2c);
    v.code_bg_color = FIELD;
    v.hyperlink_color = accent.light;
    // Chosen rows, selected text and selected items: the same fill as the
    // active mode button, so the accent is one colour everywhere.
    v.selection.bg_fill = accent.fill;
    v.selection.stroke = Stroke::new(1.0, TEXT_STRONG);
    v.weak_text_color = Some(TEXT_WEAK);

    let radius = CornerRadius::same(4);
    let w = &mut v.widgets;
    // Labels, separators and frames that can't be interacted with.
    w.noninteractive.bg_fill = PANEL;
    w.noninteractive.weak_bg_fill = PANEL;
    w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    w.noninteractive.corner_radius = radius;
    // Buttons and fields at rest.
    w.inactive.bg_fill = FIELD;
    w.inactive.weak_bg_fill = Color32::from_rgb(0x30, 0x30, 0x30);
    w.inactive.bg_stroke = Stroke::new(1.0, FIELD_BORDER);
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    w.inactive.corner_radius = radius;
    // Under the mouse.
    w.hovered.bg_fill = HOVER;
    w.hovered.weak_bg_fill = HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x4a, 0x4a, 0x4a));
    w.hovered.fg_stroke = Stroke::new(1.5, TEXT_STRONG);
    w.hovered.corner_radius = radius;
    w.hovered.expansion = 0.0;
    // Being pressed or dragged.
    w.active.bg_fill = Color32::from_rgb(0x44, 0x44, 0x44);
    w.active.weak_bg_fill = Color32::from_rgb(0x44, 0x44, 0x44);
    w.active.bg_stroke = Stroke::new(1.0, accent.light);
    w.active.fg_stroke = Stroke::new(2.0, TEXT_STRONG);
    w.active.corner_radius = radius;
    w.active.expansion = 0.0;
    // An open menu or combo box.
    w.open.bg_fill = ACTIVE_TAB;
    w.open.weak_bg_fill = ACTIVE_TAB;
    w.open.bg_stroke = Stroke::new(1.0, BORDER);
    w.open.fg_stroke = Stroke::new(1.0, TEXT_STRONG);
    w.open.corner_radius = radius;
    v
}

/// The frame of the menu bar.
pub fn menu_bar_frame() -> Frame {
    Frame::new()
        .fill(BAR)
        .inner_margin(Margin::symmetric(8, 3))
        .stroke(Stroke::new(1.0, BORDER))
}

/// The frame of the command and mode palettes.
pub fn palette_frame() -> Frame {
    Frame::new()
        .fill(PALETTE)
        .inner_margin(Margin::symmetric(6, 2))
        .stroke(Stroke::new(1.0, BORDER))
}

/// The frame of the status bar.
pub fn status_bar_frame() -> Frame {
    Frame::new()
        .fill(BAR)
        .inner_margin(Margin::symmetric(10, 2))
        .stroke(Stroke::new(1.0, BORDER))
}

/// The frame of the area the panels are docked in.
pub fn dock_frame() -> Frame {
    Frame::new().fill(BACKGROUND).inner_margin(Margin::same(2))
}

/// Padding inside the panels other than the viewport.
pub fn panel_margin() -> Margin {
    Margin::symmetric(10, 8)
}

/// The docked panels' look: plain text tabs on a dark strip, the front
/// tab joined to its panel, thin gaps, and the drop overlay.
pub fn dock_style(base: &Style, accent: Accent) -> egui_dock::Style {
    let accent = accent.colors();
    let mut s = egui_dock::Style::from_egui(base);
    s.dock_area_padding = None;
    s.main_surface_border_stroke = Stroke::NONE;
    s.separator.width = 2.0;
    // How small a panel can be dragged (egui_dock's default is 175 px,
    // which left empty space under the Coordinates rows).
    s.separator.extra = crate::layout::MIN_PANEL_SIZE;
    s.separator.color_idle = BACKGROUND;
    s.separator.color_hovered = HOVER;
    s.separator.color_dragged = accent.light;
    s.tab_bar.bg_fill = HEADER;
    s.tab_bar.height = 28.0;
    s.tab_bar.hline_color = HEADER;
    s.tab_bar.corner_radius = CornerRadius::ZERO;
    s.tab.tab_body.bg_fill = PANEL;
    // No padding around tab bodies, so the viewport fills its panel edge
    // to edge; the other panels add their own (see `panel_margin`).
    s.tab.tab_body.inner_margin = Margin::ZERO;
    s.tab.tab_body.stroke = Stroke::NONE;
    s.tab.tab_body.corner_radius = CornerRadius::ZERO;
    s.tab.hline_below_active_tab_name = false;
    for tab in [
        &mut s.tab.active,
        &mut s.tab.focused,
        &mut s.tab.active_with_kb_focus,
        &mut s.tab.focused_with_kb_focus,
    ] {
        tab.bg_fill = ACTIVE_TAB;
        tab.text_color = TEXT_STRONG;
        tab.outline_color = Color32::TRANSPARENT;
        tab.corner_radius = CornerRadius {
            nw: 4,
            ne: 4,
            sw: 0,
            se: 0,
        };
    }
    for tab in [&mut s.tab.inactive, &mut s.tab.inactive_with_kb_focus] {
        tab.bg_fill = HEADER;
        tab.text_color = TEXT_WEAK;
        tab.outline_color = Color32::TRANSPARENT;
        tab.corner_radius = CornerRadius::same(4);
    }
    s.tab.hovered.bg_fill = HEADER;
    s.tab.hovered.text_color = TEXT_STRONG;
    s.tab.hovered.outline_color = Color32::TRANSPARENT;
    // The viewport has no tab (its own menu line takes that place): hide
    // the little button egui_dock shows to bring a hidden tab bar back,
    // and don't let the panel be dragged by its top edge.
    s.buttons.show_tab_bar_size = 0.0;
    s.buttons.show_tab_bar_color = Color32::TRANSPARENT;
    s.buttons.show_tab_bar_active_color = Color32::TRANSPARENT;
    s.tab.tab_body.hidden_tab_bar_drag_height = Some(0.0);
    s.overlay.selection_color = accent.light.gamma_multiply(0.35);
    s.overlay.button_color = TEXT_WEAK;
    s
}

/// WCAG contrast ratio between two opaque colours (1 to 21).
pub fn contrast(a: Color32, b: Color32) -> f32 {
    fn luminance(c: Color32) -> f32 {
        let channel = |v: u8| {
            let s = v as f32 / 255.0;
            if s <= 0.040_45 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
    }
    let (la, lb) = (luminance(a), luminance(b));
    let (light, dark) = if la > lb { (la, lb) } else { (lb, la) };
    (light + 0.05) / (dark + 0.05)
}

// ------------------------------------------------------------ UV Editor ---

/// The UV Editor around the texture square (decision 0096).
pub const UV_BACKGROUND: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
/// The texture square itself: a little lighter, so it reads as the page.
pub const UV_SQUARE: Color32 = Color32::from_rgb(0x31, 0x31, 0x31);
/// The texture square's edge.
pub const UV_SQUARE_EDGE: Color32 = Color32::from_rgb(0x6a, 0x6a, 0x6a);
/// The grid inside the square, every tenth.
pub const UV_GRID: Color32 = Color32::from_rgba_premultiplied(0x18, 0x18, 0x18, 0x18);
/// A UV edge inside an island.
pub const UV_EDGE: Color32 = Color32::from_rgba_premultiplied(0x9a, 0x9a, 0x9a, 0xb4);
/// An island's border (a seam or the shape's open edge): brighter.
pub const UV_BORDER: Color32 = Color32::from_rgb(0xe6, 0xe6, 0xe6);
/// A trim sheet row's band in the UV Editor (decision 0106): a faint
/// blue, every other row a little stronger so neighbours read apart.
pub const TRIM_ROW: Color32 = Color32::from_rgba_premultiplied(0x0b, 0x14, 0x1e, 0x1e);
/// The edges of a trim sheet row, and its name.
pub const TRIM_ROW_EDGE: Color32 = Color32::from_rgb(0x5a, 0x86, 0xb0);
/// The numbered checker behind the UV Editor's UVs is drawn this much
/// dimmer, so the UVs' outlines stand out over it (decision 0107).
pub const UV_GRID_IMAGE_TINT: Color32 = Color32::from_gray(150);
/// Seams in the UV Editor: the viewport's seam orange (decision 0098).
pub const UV_SEAM: Color32 = Color32::from_rgb(0xff, 0x5c, 0x0d);
/// The stretch view's colours (decision 0102), as the viewport's: a
/// polygon lying evenly on the texture.
pub const STRETCH_EVEN: Color32 = Color32::from_rgb(0x7c, 0xc2, 0x9b);
/// One 1.5 times off.
pub const STRETCH_SOME: Color32 = Color32::from_rgb(0xee, 0xd8, 0x75);
/// One twice off or more.
pub const STRETCH_FAR: Color32 = Color32::from_rgb(0xe8, 0x76, 0x6e);
/// One with no room on the texture.
pub const STRETCH_NO_ROOM: Color32 = Color32::from_rgb(0xc2, 0x8a, 0xd0);

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG AA for normal-size text.
    const READABLE: f32 = 4.5;

    #[test]
    fn text_is_readable_on_every_background() {
        for background in [
            BACKGROUND,
            PANEL,
            HEADER,
            BAR,
            PALETTE,
            FIELD,
            ACTIVE_TAB,
            VIEWPORT_HEADER,
            HOVER,
        ] {
            assert!(
                contrast(TEXT, background) >= READABLE,
                "text on {background:?}"
            );
            assert!(contrast(TEXT_STRONG, background) >= READABLE);
        }
    }

    #[test]
    fn quiet_text_is_still_readable() {
        for background in [PANEL, HEADER, BAR, PALETTE, VIEWPORT_HEADER, HOVER] {
            assert!(
                contrast(TEXT_WEAK, background) >= READABLE,
                "weak text on {background:?}: {}",
                contrast(TEXT_WEAK, background)
            );
        }
    }

    #[test]
    fn every_accent_is_readable_and_visible() {
        for accent in Accent::ALL {
            let c = accent.colors();
            // A selected object's name, on its selected row.
            assert!(contrast(c.text, c.soft) >= READABLE, "{accent:?} names");
        }
        for accent in Accent::ALL {
            let c = accent.colors();
            // A white icon on the active button (graphics need 3:1).
            assert!(contrast(Color32::WHITE, c.fill) >= 3.0, "{accent:?}");
            // Rings and underlines on the panels.
            assert!(contrast(c.light, PANEL) >= 3.0, "{accent:?}");
            assert!(contrast(c.light, PALETTE) >= 3.0, "{accent:?}");
            // Text on a chosen row or selected text.
            assert!(contrast(TEXT_STRONG, c.soft) >= READABLE, "{accent:?}");
            assert!(contrast(TEXT, c.soft) >= READABLE, "{accent:?}");
        }
    }

    #[test]
    fn accents_are_distinct_and_red_is_the_standard() {
        assert_eq!(Accent::default(), Accent::Red);
        for (i, a) in Accent::ALL.iter().enumerate() {
            for b in &Accent::ALL[i + 1..] {
                assert_ne!(a.colors(), b.colors());
                assert_ne!(a.name(), b.name());
            }
        }
    }

    #[test]
    fn the_accent_survives_saving_and_switching() {
        let text = ron::to_string(&Accent::PetrolBlue).unwrap();
        assert_eq!(ron::from_str::<Accent>(&text).unwrap(), Accent::PetrolBlue);
        let ctx = Context::default();
        apply(&ctx);
        assert_eq!(accent(&ctx), Accent::Red);
        set_accent(&ctx, Accent::Olive);
        assert_eq!(accent(&ctx), Accent::Olive);
        let style = ctx.style_of(egui::Theme::Dark);
        assert_eq!(style.visuals.selection.bg_fill, Accent::Olive.colors().fill);
    }

    #[test]
    fn the_backdrop_is_a_calm_neutral_grey_with_a_darker_ground() {
        use backdrop::*;
        for c in [SKY_TOP, HORIZON, GROUND] {
            assert!((0x30..=0x60).contains(&c.r()), "{c:?}");
            assert_eq!((c.r(), c.g()), (c.b(), c.b()), "neutral: {c:?}");
        }
        assert!(SKY_TOP.r() > HORIZON.r() && HORIZON.r() > GROUND.r());
    }

    #[test]
    fn icon_colours_stand_out_from_the_palettes() {
        // Non-text graphics need 3:1.
        for colour in tint::ALL {
            assert!(
                contrast(colour, PALETTE) >= 3.0,
                "{colour:?}: {}",
                contrast(colour, PALETTE)
            );
        }
    }

    #[test]
    fn unavailable_icons_are_dimmed_but_still_visible() {
        for colour in tint::ALL {
            let shown = PALETTE.lerp_to_gamma(colour, UNAVAILABLE_STRENGTH);
            let ratio = contrast(shown, PALETTE);
            assert!(ratio > 1.6, "{colour:?} dimmed is invisible: {ratio}");
            assert!(ratio < contrast(colour, PALETTE));
        }
    }

    #[test]
    fn axis_colours_differ_in_brightness_too() {
        // Red and green (X and Z) are hard to tell apart by hue alone for
        // many people, so they must differ in brightness as well; the
        // letters next to them carry the meaning too. Red and blue (X and
        // Y) are told apart by hue even with common colour blindness.
        assert!(contrast(tint::AXIS_X, tint::AXIS_Z) > 1.2);
    }

    #[test]
    fn icon_colours_are_not_an_accent() {
        for colour in tint::ALL {
            for accent in Accent::ALL {
                assert_ne!(colour, accent.colors().fill);
            }
        }
    }

    #[test]
    fn panels_differ_only_gently() {
        // The calm look: neighbouring areas are close in brightness.
        for (a, b) in [
            (PANEL, HEADER),
            (PANEL, BACKGROUND),
            (PALETTE, BAR),
            (VIEWPORT_HEADER, PANEL),
        ] {
            assert!(contrast(a, b) < 1.4, "{a:?} vs {b:?} is too stark");
        }
    }

    #[test]
    fn contrast_matches_known_values() {
        assert!((contrast(Color32::WHITE, Color32::BLACK) - 21.0).abs() < 0.01);
        assert!((contrast(TEXT, TEXT) - 1.0).abs() < 0.001);
    }

    #[test]
    fn inter_comes_first_for_interface_text() {
        let fonts = fonts();
        assert_eq!(fonts.families[&FontFamily::Proportional][0], "inter");
        let semibold = &fonts.families[&FontFamily::Name(SEMIBOLD.into())];
        assert_eq!(semibold[0], "inter-semibold");
        // egui's own fonts stay behind Inter as fallbacks for symbols.
        assert!(fonts.families[&FontFamily::Proportional].len() > 1);
    }

    #[test]
    fn the_font_files_are_real_fonts() {
        // TrueType files start with the version number 1.0.
        for bytes in [INTER_REGULAR, INTER_SEMIBOLD] {
            assert_eq!(&bytes[..4], &[0, 1, 0, 0]);
            assert!(bytes.len() > 100_000);
        }
    }

    #[test]
    fn applying_the_theme_sets_the_panel_colour() {
        let ctx = Context::default();
        apply(&ctx);
        let style = ctx.style_of(egui::Theme::Dark);
        assert_eq!(style.visuals.panel_fill, PANEL);
        assert_eq!(ctx.style_of(egui::Theme::Light).visuals.panel_fill, PANEL);
        assert!(style.visuals.dark_mode);
    }

    #[test]
    fn dock_tabs_use_the_theme() {
        let s = dock_style(&style(), Accent::default());
        assert_eq!(s.tab_bar.bg_fill, HEADER);
        assert_eq!(s.separator.color_dragged, Accent::Red.colors().light);
        assert_eq!(s.tab.active.bg_fill, ACTIVE_TAB);
        assert_eq!(s.tab.tab_body.bg_fill, PANEL);
        assert_eq!(s.tab.tab_body.inner_margin, Margin::ZERO);
    }
}
