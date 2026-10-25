//! The shape lists: press `B` to open a small list of the basic shapes
//! by the mouse, press `B` again to move down it (`Shift + B` to move up,
//! or the arrow keys), and `Enter` or `Space` to create the highlighted
//! shape. `Alt + B` opens the list of the other shapes the same way
//! (`Shift + Alt + B` to move up), so no shape is in both (decision 0130);
//! once a list is open, `B` and `Shift + B` move through that list.
//! `Esc` or a click elsewhere closes a list; clicking a shape creates it.
//!
//! The curves (decision 0138) close the More list: Draw curve and the
//! ready-made Line, Circle, Arc and Spiral. They are there, not in the
//! basic list, because `B` is for the everyday solids and stays short;
//! curves are a different kind of thing (lines, which become solids
//! through a sweep or a lathe), used less often.

use crate::icons;
use crate::icons::Icon;
use crate::objects::shape_icon;
use crate::text::tr;
use crate::theme::{self, tint};
use egui::{Context, Key, Modifiers, Pos2, Sense, Ui, WidgetInfo, WidgetType, vec2};
use scene::{PrimitiveKind, ReadyCurveKind};

/// The key that opens the list and moves down it; with Alt, the list of
/// more shapes.
pub const SHAPE_LIST_KEY: Key = Key::B;

/// Something the Create menu, the shape lists and command search make.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Shape {
    /// A ready-made solid.
    Primitive(PrimitiveKind),
    /// Draw curve: the tool that places a curve's points (decision 0138).
    DrawCurve,
    /// A ready-made curve.
    Curve(ReadyCurveKind),
}

impl Shape {
    /// The curves, in the order the menus list them.
    pub(crate) const CURVES: [Self; 5] = [
        Self::DrawCurve,
        Self::Curve(ReadyCurveKind::Line),
        Self::Curve(ReadyCurveKind::Circle),
        Self::Curve(ReadyCurveKind::Arc),
        Self::Curve(ReadyCurveKind::Spiral),
    ];

    /// Every shape, in the Create menu's order.
    pub(crate) fn all() -> Vec<Self> {
        PrimitiveKind::ALL
            .into_iter()
            .map(Self::Primitive)
            .chain(Self::CURVES)
            .collect()
    }

    /// Its name.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Primitive(kind) => kind.name(),
            Self::DrawCurve => tr("Draw curve"),
            Self::Curve(kind) => kind.name(),
        }
    }

    /// What it is, in a sentence.
    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::Primitive(kind) => kind.description(),
            Self::DrawCurve => tr(
                "Draw a curve point by point: click for a sharp corner, press and drag for a \
                 smooth bend; Enter or a right click finishes",
            ),
            Self::Curve(kind) => kind.description(),
        }
    }

    /// Its icon.
    pub(crate) fn icon(self) -> Icon {
        match self {
            Self::Primitive(kind) => shape_icon(kind),
            Self::DrawCurve => Icon::DrawCurve,
            Self::Curve(kind) => curve_icon(kind),
        }
    }
}

/// A ready-made curve's icon.
pub(crate) fn curve_icon(kind: ReadyCurveKind) -> Icon {
    match kind {
        ReadyCurveKind::Line => Icon::CurveLine,
        ReadyCurveKind::Circle => Icon::CurveCircle,
        ReadyCurveKind::Arc => Icon::CurveArc,
        ReadyCurveKind::Spiral => Icon::CurveSpiral,
    }
}

/// Which of the two shape lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShapeList {
    /// The seven basic shapes (`B`).
    Basic,
    /// The others (`Alt + B`).
    More,
}

impl ShapeList {
    /// The shapes in this list, in the Create menu's order: the basic
    /// solids, or the others with the curves after them.
    pub(crate) fn kinds(self) -> Vec<Shape> {
        Shape::all()
            .into_iter()
            .filter(|shape| match shape {
                Shape::Primitive(k) => k.in_first_files() == (self == Self::Basic),
                Shape::DrawCurve | Shape::Curve(_) => self == Self::More,
            })
            .collect()
    }

    /// The list's heading.
    fn title(self) -> &'static str {
        match self {
            Self::Basic => tr("Create a shape"),
            Self::More => tr("More shapes"),
        }
    }

    /// The keys, at the foot of the list.
    fn hint(self) -> &'static str {
        match self {
            Self::Basic => {
                tr("B: next · Shift + B: back · Enter or Space: create · Alt + B: more shapes")
            }
            Self::More => tr("B or Alt + B: next · Shift + B: back · Enter or Space: create"),
        }
    }
}

/// Accessible name of the list's entries start with this.
pub(crate) const ENTRY_PREFIX: &str = crate::text!("Shape list: ");

/// The list while it is open.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Open {
    /// Which list.
    list: ShapeList,
    /// Which shape is highlighted (an index into the list's kinds).
    highlight: usize,
    /// Where the list's top-left corner is.
    at: Pos2,
}

/// The shape list's state.
#[derive(Debug, Clone, Default)]
pub(crate) struct ShapePicker {
    open: Option<Open>,
}

impl ShapePicker {
    /// Whether the list is showing.
    pub(crate) fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// Opens the list at `at` (from the tool menu), the first shape
    /// highlighted.
    pub(crate) fn open_at(&mut self, at: Pos2) {
        self.open = Some(Open {
            list: ShapeList::Basic,
            highlight: 0,
            at,
        });
    }

    /// Closes the list.
    pub(crate) fn close(&mut self) {
        self.open = None;
    }

    /// The highlighted shape, while the list is open.
    #[cfg(test)]
    pub(crate) fn highlighted(&self) -> Option<Shape> {
        self.open.map(|o| o.list.kinds()[o.highlight])
    }

    /// Which list is open, if one is.
    #[cfg(test)]
    pub(crate) fn list(&self) -> Option<ShapeList> {
        self.open.map(|o| o.list)
    }

    /// Handles the list's keys (not while typing). Returns the shape to
    /// create, if `Enter` or `Space` chose one.
    pub(crate) fn keys(&mut self, ui: &Ui, typing: bool) -> Option<Shape> {
        if typing {
            return None;
        }
        // Exactly these modifiers: Shift + Alt + B is the More list's
        // back, not the Basic list's.
        let pressed = |ui: &Ui, modifiers: Modifiers, key| {
            ui.input_mut(|i| {
                let exact = i.modifiers.matches_exact(modifiers);
                exact && i.consume_key(modifiers, key)
            })
        };
        let more_back = pressed(ui, Modifiers::SHIFT | Modifiers::ALT, SHAPE_LIST_KEY);
        let more = !more_back && pressed(ui, Modifiers::ALT, SHAPE_LIST_KEY);
        let back = pressed(ui, Modifiers::SHIFT, SHAPE_LIST_KEY);
        let forward = !back && pressed(ui, Modifiers::NONE, SHAPE_LIST_KEY);
        // Alt + B means the More list; plain B means whichever list is
        // open (the project lead's choice: Alt + B, then B, B, B goes down
        // the More list), or opens the basic one.
        let (list, forward, back) = if more || more_back {
            (ShapeList::More, more, more_back)
        } else {
            let open = self.open.map_or(ShapeList::Basic, |o| o.list);
            (open, forward, back)
        };
        let count = list.kinds().len();
        // Opening, or switching to the More list: from its first shape (or
        // its last, going back).
        if (forward || back) && self.open.is_none_or(|o| o.list != list) {
            let at = self.open.map(|o| o.at).unwrap_or_else(|| {
                ui.input(|i| i.pointer.latest_pos())
                    .unwrap_or_else(|| ui.ctx().content_rect().center())
                    + vec2(12.0, 12.0)
            });
            self.open = Some(Open {
                list,
                highlight: if back { count - 1 } else { 0 },
                at,
            });
            return None;
        }
        let open = self.open.as_mut()?;
        let count = open.list.kinds().len();
        if forward || pressed(ui, Modifiers::NONE, Key::ArrowDown) {
            open.highlight = (open.highlight + 1) % count;
        }
        if back || pressed(ui, Modifiers::NONE, Key::ArrowUp) {
            open.highlight = (open.highlight + count - 1) % count;
        }
        if pressed(ui, Modifiers::NONE, Key::Escape) {
            self.open = None;
            return None;
        }
        if pressed(ui, Modifiers::NONE, Key::Enter) || pressed(ui, Modifiers::NONE, Key::Space) {
            let chosen = open.list.kinds()[open.highlight];
            self.open = None;
            return Some(chosen);
        }
        None
    }

    /// Draws the list when open. Returns the shape clicked, if any.
    pub(crate) fn show(&mut self, ctx: &Context) -> Option<Shape> {
        let open = self.open?;
        let mut chosen = None;
        let mut hovered = None;
        let area = egui::Area::new(egui::Id::new("shape_list"))
            .order(egui::Order::Foreground)
            .fixed_pos(open.at)
            .constrain(true)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(170.0);
                    ui.label(egui::RichText::new(open.list.title()).strong());
                    ui.add_space(2.0);
                    let accent = theme::accent(ui.ctx()).colors();
                    for (index, kind) in open.list.kinds().into_iter().enumerate() {
                        let (rect, response) = ui
                            .allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::click());
                        response.widget_info(|| {
                            WidgetInfo::selected(
                                WidgetType::SelectableLabel,
                                true,
                                index == open.highlight,
                                format!("{}{}", tr(ENTRY_PREFIX), kind.name()),
                            )
                        });
                        let lit = index == open.highlight;
                        if lit {
                            ui.painter().rect_filled(rect, 4.0, accent.fill);
                        } else if response.hovered() {
                            ui.painter().rect_filled(rect, 4.0, theme::HOVER);
                        }
                        let icon = egui::Rect::from_center_size(
                            rect.left_center() + vec2(14.0, 0.0),
                            vec2(16.0, 16.0),
                        );
                        icons::paint(ui.painter(), icon, kind.icon(), tint::SHAPE);
                        ui.painter().text(
                            rect.left_center() + vec2(30.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            kind.name(),
                            egui::FontId::proportional(13.0),
                            if lit { theme::TEXT_STRONG } else { theme::TEXT },
                        );
                        if response.hovered() {
                            hovered = Some(index);
                        }
                        if response.on_hover_text(kind.description()).clicked() {
                            chosen = Some(kind);
                        }
                    }
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(open.list.hint())
                            .small()
                            .color(theme::TEXT_WEAK),
                    );
                });
            });
        let moved = ctx.input(|i| i.pointer.delta() != egui::Vec2::ZERO);
        if let (Some(index), Some(open), true) = (hovered, self.open.as_mut(), moved) {
            // Moving the mouse over the list moves the highlight too.
            open.highlight = index;
        }
        let clicked_elsewhere = ctx.input(|i| i.pointer.any_pressed())
            && !area.response.contains_pointer()
            && chosen.is_none();
        if chosen.is_some() || clicked_elsewhere {
            self.open = None;
        }
        chosen
    }
}
