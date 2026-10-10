//! The calm, dark look from DESIGN.md §5.2: neutral greys with low
//! contrast between panels, one warm accent for the active mode or tool,
//! and the Inter font.
//!
//! Colours follow the approved mockups. Text colours are checked for
//! readability by the tests at the bottom (WCAG contrast 4.5:1 for normal
//! text).

use egui::{
    Color32, Context, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Frame, Margin,
    Shadow, Stroke, Style, TextStyle, Visuals, vec2,
};
use std::sync::Arc;

/// Window background behind everything.
pub const BACKGROUND: Color32 = Color32::from_rgb(0x2b, 0x2b, 0x2b);
/// Panel bodies (Attributes, Objects, Coordinates).
pub const PANEL: Color32 = Color32::from_rgb(0x33, 0x33, 0x33);
/// Tab bars and panel headers.
pub const HEADER: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2a);
/// The menu bar and status bar, a step darker than the panels.
pub const BAR: Color32 = Color32::from_rgb(0x23, 0x23, 0x23);
/// The command and mode palettes.
pub const PALETTE: Color32 = Color32::from_rgb(0x2f, 0x2f, 0x2f);
/// Text and number fields.
pub const FIELD: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
/// Hairline borders between areas.
pub const BORDER: Color32 = Color32::from_rgb(0x19, 0x19, 0x19);
/// A hovered or pressed button.
pub const HOVER: Color32 = Color32::from_rgb(0x46, 0x49, 0x4e);
/// The tab in front, in a tab bar.
pub const ACTIVE_TAB: Color32 = Color32::from_rgb(0x3b, 0x3b, 0x3b);
/// Normal text.
pub const TEXT: Color32 = Color32::from_rgb(0xd6, 0xd6, 0xd6);
/// Bright text: headings and the active tab.
pub const TEXT_STRONG: Color32 = Color32::from_rgb(0xf0, 0xf0, 0xf0);
/// Quieter text: labels, hints, the status bar.
pub const TEXT_WEAK: Color32 = Color32::from_rgb(0xa5, 0xa5, 0xa5);
/// Selected rows and selected text (a cool blue-grey, as in the mockups).
pub const SELECTION: Color32 = Color32::from_rgb(0x3d, 0x4a, 0x5c);
/// The warm accent for the active mode or tool, and focused fields.
pub const ACCENT: Color32 = Color32::from_rgb(0xf2, 0xa3, 0x3a);

/// Name of the bolder font family, for headings.
pub const SEMIBOLD: &str = "semibold";

/// Inter, the interface font (SIL Open Font License 1.1; see licenses/fonts).
const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");

/// Applies the theme to the whole app. Call once at startup.
///
/// The app is always dark, whatever the system prefers: the same style is
/// set for both of egui's themes, and dark is chosen.
pub fn apply(ctx: &Context) {
    ctx.set_fonts(fonts());
    ctx.set_style_of(egui::Theme::Dark, style());
    ctx.set_style_of(egui::Theme::Light, style());
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

/// Colours, sizes and spacing.
pub fn style() -> Style {
    let mut style = Style {
        visuals: visuals(),
        ..Style::default()
    };
    style.text_styles = [
        (TextStyle::Small, FontId::proportional(11.0)),
        (TextStyle::Body, FontId::proportional(13.0)),
        (TextStyle::Button, FontId::proportional(13.0)),
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
pub fn visuals() -> Visuals {
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
    v.faint_bg_color = Color32::from_rgb(0x30, 0x30, 0x30);
    v.code_bg_color = FIELD;
    v.hyperlink_color = ACCENT;
    v.selection.bg_fill = SELECTION;
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
    w.inactive.weak_bg_fill = Color32::from_rgb(0x3a, 0x3a, 0x3a);
    w.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x1c, 0x1c, 0x1c));
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    w.inactive.corner_radius = radius;
    // Under the mouse.
    w.hovered.bg_fill = HOVER;
    w.hovered.weak_bg_fill = HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x55, 0x58, 0x5e));
    w.hovered.fg_stroke = Stroke::new(1.5, TEXT_STRONG);
    w.hovered.corner_radius = radius;
    w.hovered.expansion = 0.0;
    // Being pressed or dragged.
    w.active.bg_fill = Color32::from_rgb(0x52, 0x55, 0x5b);
    w.active.weak_bg_fill = Color32::from_rgb(0x52, 0x55, 0x5b);
    w.active.bg_stroke = Stroke::new(1.0, ACCENT);
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

/// The docked panels' look: tab bars, tabs, gaps and the drop overlay.
pub fn dock_style(base: &Style) -> egui_dock::Style {
    let mut s = egui_dock::Style::from_egui(base);
    s.dock_area_padding = None;
    s.main_surface_border_stroke = Stroke::NONE;
    s.separator.width = 2.0;
    s.separator.color_idle = BACKGROUND;
    s.separator.color_hovered = HOVER;
    s.separator.color_dragged = ACCENT;
    s.tab_bar.bg_fill = HEADER;
    s.tab_bar.height = 26.0;
    s.tab_bar.hline_color = BORDER;
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
        tab.corner_radius = CornerRadius::same(4);
    }
    for tab in [&mut s.tab.inactive, &mut s.tab.inactive_with_kb_focus] {
        tab.bg_fill = HEADER;
        tab.text_color = TEXT_WEAK;
        tab.outline_color = Color32::TRANSPARENT;
        tab.corner_radius = CornerRadius::same(4);
    }
    s.tab.hovered.bg_fill = HOVER;
    s.tab.hovered.text_color = TEXT_STRONG;
    s.tab.hovered.outline_color = Color32::TRANSPARENT;
    s.overlay.selection_color = ACCENT.gamma_multiply(0.35);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG AA for normal-size text.
    const READABLE: f32 = 4.5;

    #[test]
    fn text_is_readable_on_every_background() {
        for background in [BACKGROUND, PANEL, HEADER, BAR, PALETTE, FIELD, ACTIVE_TAB] {
            assert!(
                contrast(TEXT, background) >= READABLE,
                "text on {background:?}"
            );
            assert!(contrast(TEXT_STRONG, background) >= READABLE);
        }
    }

    #[test]
    fn quiet_text_is_still_readable() {
        for background in [PANEL, HEADER, BAR, PALETTE] {
            assert!(
                contrast(TEXT_WEAK, background) >= READABLE,
                "weak text on {background:?}: {}",
                contrast(TEXT_WEAK, background)
            );
        }
    }

    #[test]
    fn text_is_readable_on_selection_and_hover() {
        assert!(contrast(TEXT_STRONG, SELECTION) >= READABLE);
        assert!(contrast(TEXT_STRONG, HOVER) >= READABLE);
    }

    #[test]
    fn the_accent_stands_out_from_the_palettes() {
        // Non-text graphics (icons) need 3:1.
        assert!(contrast(ACCENT, PALETTE) >= 3.0);
        assert!(contrast(ACCENT, PANEL) >= 3.0);
    }

    #[test]
    fn panels_differ_only_gently() {
        // The calm look: neighbouring areas are close in brightness.
        for (a, b) in [(PANEL, HEADER), (PANEL, BACKGROUND), (PALETTE, BAR)] {
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
        let s = dock_style(&style());
        assert_eq!(s.tab_bar.bg_fill, HEADER);
        assert_eq!(s.tab.active.bg_fill, ACTIVE_TAB);
        assert_eq!(s.tab.tab_body.bg_fill, PANEL);
        assert_eq!(s.tab.tab_body.inner_margin, Margin::ZERO);
    }
}
