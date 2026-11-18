//! Sculpt mode's plain page (decision 0311): the brush bar over the view's
//! top and the tool row after the view's controls, placed as Paint mode
//! places its own (decisions 0213 and 0224).

use super::Workbench;
use crate::transforming::Tool;

impl Workbench {
    /// What the sculpt tool row shows: the tool, the select arrow and the
    /// brush.
    pub(super) fn sculpt_hands(&self) -> crate::sculpt_tools::Hands {
        crate::sculpt_tools::Hands {
            tool: self.tool,
            selecting: self.sculpting.selecting,
            brush: self.attributes.sculpt.brush,
        }
    }

    /// Takes what the tool row chose: the workbench's tool (through
    /// `set_tool`, so a session under way ends properly), the select arrow
    /// and the brush.
    pub(super) fn set_sculpt_hands(&mut self, hands: crate::sculpt_tools::Hands) {
        if hands == self.sculpt_hands() {
            return;
        }
        if hands.brush != self.attributes.sculpt.brush
            || !hands.selecting && hands.tool == Tool::Select
        {
            self.choose_sculpt_brush(hands.brush);
        }
        if hands.tool != self.tool {
            self.set_tool(hands.tool);
        }
        self.sculpting.selecting = hands.selecting;
    }

    /// The brush bar over the view's top and the tool row along its
    /// bottom (unless Settings puts the tools in the left bar), over the
    /// view `pane`.
    pub(super) fn show_sculpt_page(&mut self, ctx: &egui::Context, pane: &crate::paint_area::Pane) {
        crate::sculpt_bar::bar(ctx, pane.rect, pane.bar_top, &mut self.attributes.sculpt);
        if !self.viewport.options().paint_tools_left {
            let bottom: Vec<egui::Rect> = pane.bottom.iter().flatten().copied().collect();
            let mut hands = self.sculpt_hands();
            crate::sculpt_tools::show(
                ctx,
                pane.rect,
                &bottom,
                self.viewport.paint_controls(),
                &mut hands,
            );
            self.set_sculpt_hands(hands);
        }
    }
}
