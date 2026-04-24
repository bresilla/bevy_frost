//! Static, non-draggable ribbons.
//!
//! For cases where you just want a row of buttons on an edge with
//! no drag, no state, no persistence. If you want buttons the user
//! can rearrange, use [`super::layout::RibbonLayout`] instead.

use egui;

use super::kinds::{Bar, Side};
use super::paint::{
    ribbon_button_area, EDGE_GAP, SIDE_BTN_GAP, SIDE_BTN_SIZE,
};

// ─── SideRibbon ─────────────────────────────────────────────────────

/// Vertical rail of menu-toggle buttons on the Left or Right edge.
/// Construct once per frame with the total count, then call
/// `.button(...)` per slot.
#[derive(Debug, Clone, Copy)]
pub struct SideRibbon {
    pub side: Side,
    /// Total number of buttons on this rail. Kept as a field so the
    /// layout can evolve (e.g. push the last N buttons to the
    /// bottom of the rail) without an API break.
    pub total: u32,
}

impl SideRibbon {
    #[must_use]
    pub fn new(side: Side, total: u32) -> Self {
        Self { side, total }
    }

    pub fn button(
        &self,
        id: &'static str,
        ctx: &egui::Context,
        slot: u32,
        glyph: &str,
        tooltip: &str,
        is_active: bool,
        accent: egui::Color32,
        on_click: impl FnOnce(),
    ) {
        let slot_y = slot as f32 * (SIDE_BTN_SIZE + SIDE_BTN_GAP);
        let (anchor, offset) = match self.side {
            Side::Left => (
                egui::Align2::LEFT_TOP,
                egui::vec2(EDGE_GAP, EDGE_GAP + slot_y),
            ),
            Side::Right => (
                egui::Align2::RIGHT_TOP,
                egui::vec2(-EDGE_GAP, EDGE_GAP + slot_y),
            ),
        };
        ribbon_button_area(id, ctx, anchor, offset, glyph, tooltip, is_active, accent, on_click);
    }
}

// ─── BarRibbon ──────────────────────────────────────────────────────

/// Horizontal row of action-only buttons on the Top or Bottom edge.
/// Buttons are centred as a group. `total` is the button count used
/// to compute each slot's centred X offset.
#[derive(Debug, Clone, Copy)]
pub struct BarRibbon {
    pub bar: Bar,
    pub total: u32,
}

impl BarRibbon {
    #[must_use]
    pub fn new(bar: Bar, total: u32) -> Self {
        Self { bar, total }
    }

    pub fn button(
        &self,
        id: &'static str,
        ctx: &egui::Context,
        slot: u32,
        glyph: &str,
        tooltip: &str,
        is_active: bool,
        accent: egui::Color32,
        on_click: impl FnOnce(),
    ) {
        let n = self.total.max(1) as f32;
        let step = SIDE_BTN_SIZE + SIDE_BTN_GAP;
        let row_w = n * SIDE_BTN_SIZE + (n - 1.0).max(0.0) * SIDE_BTN_GAP;
        let offset_x = -(row_w - SIDE_BTN_SIZE) * 0.5 + slot as f32 * step;
        let (anchor, offset) = match self.bar {
            Bar::Top => (
                egui::Align2::CENTER_TOP,
                egui::vec2(offset_x, EDGE_GAP),
            ),
            Bar::Bottom => (
                egui::Align2::CENTER_BOTTOM,
                egui::vec2(offset_x, -EDGE_GAP),
            ),
        };
        ribbon_button_area(id, ctx, anchor, offset, glyph, tooltip, is_active, accent, on_click);
    }
}
