//! Sculpt mode's tool row ([decision 0311](../../../docs/decisions/0311-sculpt-mode-looks-like-paint-mode.md)):
//! the same floating row as Paint mode's ([`crate::paint_tools`]), right
//! after the view's controls along the bottom of the view, or down the left
//! bar when Settings puts the tools there. Select, move, rotate and scale,
//! a line, then the brushes with a number key. The keys choose the same:
//! Q, W, R and S, then 1 to 7 and 9.

use crate::icons::Icon;
use crate::paint_tools::RowButton;
use crate::text::tr;
use crate::transforming::Tool;
use sculpt::BrushKind;

/// Accessible name of the row.
pub(crate) const TOOLS_LABEL: &str = crate::text!("Sculpt tools");
/// Accessible name of the left bar's tools.
pub(crate) const COLUMN_LABEL: &str = crate::text!("Sculpt tools in the left bar");

/// What the row needs of the workbench: the tool in use (select, move,
/// rotate, scale), whether the select arrow is on instead of the brush,
/// and the brush.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Hands {
    /// The workbench's tool.
    pub(crate) tool: Tool,
    /// The select arrow is on: clicks select instead of sculpting.
    pub(crate) selecting: bool,
    /// The brush chosen.
    pub(crate) brush: BrushKind,
}

/// One button of the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SculptToolButton {
    /// Selecting objects (Q): clicks pick the shape to sculpt.
    Select,
    /// Moving them, or the unmasked part (W).
    Move,
    /// Rotating them (R).
    Rotate,
    /// Scaling them (S).
    Scale,
    /// A brush with a number key.
    Brush(BrushKind),
}

impl SculptToolButton {
    /// Every button, in the row's order: the brushes with a number key,
    /// in their keys' order.
    pub(crate) const ALL: [SculptToolButton; 12] = [
        Self::Select,
        Self::Move,
        Self::Rotate,
        Self::Scale,
        Self::Brush(BrushKind::Draw),
        Self::Brush(BrushKind::ClayStrips),
        Self::Brush(BrushKind::Inflate),
        Self::Brush(BrushKind::Crease),
        Self::Brush(BrushKind::Flatten),
        Self::Brush(BrushKind::Grab),
        Self::Brush(BrushKind::SnakeHook),
        Self::Brush(BrushKind::Mask),
    ];

    /// Its name, with its key.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Select => tr("Select (Q)"),
            Self::Move => tr("Move (W)"),
            Self::Rotate => tr("Rotate (R)"),
            Self::Scale => tr("Scale (S)"),
            Self::Brush(kind) => brush_label(kind),
        }
    }

    /// What it does, for its tooltip.
    fn explanation(self) -> &'static str {
        match self {
            Self::Select => tr("Click a shape to sculpt it instead; nothing is sculpted"),
            Self::Move | Self::Rotate | Self::Scale => tr(
                "Move, turn or scale the selected objects with the gizmo, or the unmasked part \
                 when part of the shape is masked; nothing is sculpted",
            ),
            Self::Brush(kind) => tr(kind.explanation()),
        }
    }

    fn icon(self) -> Icon {
        match self {
            Self::Select => Icon::BoxSelection,
            Self::Move => Icon::Move,
            Self::Rotate => Icon::Rotate,
            Self::Scale => Icon::Scale,
            Self::Brush(kind) => Icon::SculptBrush(kind),
        }
    }

    /// Whether it is the tool in use.
    pub(crate) fn is_on(self, hands: Hands) -> bool {
        let sculpting = hands.tool == Tool::Select && !hands.selecting;
        match self {
            Self::Select => hands.tool == Tool::Select && hands.selecting,
            Self::Move => hands.tool == Tool::Move,
            Self::Rotate => hands.tool == Tool::Rotate,
            Self::Scale => hands.tool == Tool::Scale,
            Self::Brush(kind) => sculpting && hands.brush == kind,
        }
    }

    /// Makes it the tool in use, as its key does. The workbench's tool and
    /// the select arrow are the caller's to set from `hands` afterwards.
    pub(crate) fn choose(self, hands: &mut Hands) {
        hands.selecting = false;
        match self {
            Self::Select => {
                hands.tool = Tool::Select;
                hands.selecting = true;
            }
            Self::Move => hands.tool = Tool::Move,
            Self::Rotate => hands.tool = Tool::Rotate,
            Self::Scale => hands.tool = Tool::Scale,
            Self::Brush(kind) => {
                hands.tool = Tool::Select;
                hands.brush = kind;
            }
        }
    }

    /// Its button in the row.
    fn row_button(self, hands: Hands) -> RowButton {
        RowButton {
            icon: self.icon(),
            label: self.label(),
            explanation: self.explanation(),
            on: self.is_on(hands),
            line_before: self == Self::Brush(BrushKind::Draw),
        }
    }
}

/// A brush's name with its key, for its button.
fn brush_label(kind: BrushKind) -> &'static str {
    match kind {
        BrushKind::Draw => tr("Draw (1)"),
        BrushKind::ClayStrips => tr("Clay strips (2)"),
        BrushKind::Inflate => tr("Inflate (3)"),
        BrushKind::Crease => tr("Crease (4)"),
        BrushKind::Flatten => tr("Flatten (5)"),
        BrushKind::Grab => tr("Grab (6)"),
        BrushKind::SnakeHook => tr("Snake hook (7)"),
        BrushKind::Mask => tr("Mask (9)"),
        other => tr(other.name()),
    }
}

/// The row's buttons as they stand.
pub(crate) fn buttons(hands: Hands) -> Vec<RowButton> {
    SculptToolButton::ALL
        .iter()
        .map(|b| b.row_button(hands))
        .collect()
}

/// Draws the row over `area`, the sculpting view: right after the view's
/// controls (`after`) when they are drawn, otherwise in the free stretch
/// along its bottom (`bottom`). Switches the tool clicked.
pub(crate) fn show(
    ctx: &egui::Context,
    area: egui::Rect,
    bottom: &[egui::Rect],
    after: Option<egui::Rect>,
    hands: &mut Hands,
) {
    let place = crate::paint_tools::RowPlace {
        area,
        bottom,
        after,
        label: TOOLS_LABEL,
    };
    if let Some(i) = crate::paint_tools::show_row(ctx, place, &buttons(*hands)) {
        SculptToolButton::ALL[i].choose(hands);
    }
}

/// The tools down the left bar, under where the modes are, when Settings
/// puts them there.
pub(crate) fn column(ui: &mut egui::Ui, hands: &mut Hands) {
    if let Some(i) = crate::paint_tools::column_of(ui, COLUMN_LABEL, &buttons(*hands)) {
        SculptToolButton::ALL[i].choose(hands);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hands() -> Hands {
        Hands {
            tool: Tool::Select,
            selecting: false,
            brush: BrushKind::Draw,
        }
    }

    #[test]
    fn each_button_chooses_its_tool_and_shows_it_on() {
        let mut hands = hands();
        for tool in SculptToolButton::ALL {
            tool.choose(&mut hands);
            for other in SculptToolButton::ALL {
                assert_eq!(other.is_on(hands), other == tool, "{tool:?} {other:?}");
            }
        }
    }

    #[test]
    fn a_brush_puts_the_select_arrow_and_transform_tools_down() {
        let mut hands = hands();
        SculptToolButton::Move.choose(&mut hands);
        assert_eq!(hands.tool, Tool::Move);
        SculptToolButton::Brush(BrushKind::Grab).choose(&mut hands);
        assert_eq!(hands.tool, Tool::Select);
        assert_eq!(hands.brush, BrushKind::Grab);
        SculptToolButton::Select.choose(&mut hands);
        assert!(hands.selecting);
        SculptToolButton::Brush(BrushKind::Draw).choose(&mut hands);
        assert!(!hands.selecting);
    }

    #[test]
    fn the_brushes_are_those_with_keys_in_their_keys_order() {
        let keys: Vec<_> = SculptToolButton::ALL
            .iter()
            .filter_map(|b| match b {
                SculptToolButton::Brush(kind) => crate::sculpt_tab::brush_key(*kind),
                _ => None,
            })
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
        assert_eq!(keys.len(), 8);
        // Every brush with a key has a button, and its label names the key.
        for kind in BrushKind::ALL {
            if crate::sculpt_tab::brush_key(kind).is_some() {
                let button = SculptToolButton::Brush(kind);
                assert!(SculptToolButton::ALL.contains(&button), "{kind:?}");
                assert!(
                    button
                        .label()
                        .contains(crate::sculpt_tab::brush_key_text(kind))
                );
            }
        }
    }

    #[test]
    fn a_brush_without_a_button_lights_none() {
        let hands = Hands {
            brush: BrushKind::Smooth,
            ..hands()
        };
        assert!(SculptToolButton::ALL.iter().all(|t| !t.is_on(hands)));
    }
}
