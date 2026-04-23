//! Declarative ribbon buttons — pass a slice of [`RibbonButton`]
//! descriptors to [`draw_ribbon_buttons`] and frost handles the
//! layout, drag routing, stale-state invalidation, and per-side
//! exclusive toggle dispatch in one call.
//!
//! Replaces the "snapshot open state / call `layout.button(...)` /
//! collect click flags / apply `toggle_menu`" boilerplate every
//! rail-drawing system otherwise has to repeat.

use bevy_egui::egui;

use super::{RibbonConstraint, RibbonKind, RibbonLayout, SideActive};

/// One menu-toggle button on a rail. Fields match the arguments of
/// [`RibbonLayout::button`]; supply them as plain data and let
/// [`draw_ribbon_buttons`] plumb the calls.
#[derive(Clone, Copy, Debug)]
pub struct RibbonButton {
    /// Stable menu id — also the id `SideActive::toggle_menu` will
    /// flip. Must be unique across the whole app.
    pub id: &'static str,
    /// Where this button is allowed to live / be dragged.
    pub constraint: RibbonConstraint,
    /// Default rail.
    pub side: RibbonKind,
    /// Default slot on that rail (0 = top / leftmost).
    pub slot: u32,
    /// Single-glyph label painted in the button.
    pub glyph: &'static str,
    /// Hover tooltip.
    pub tooltip: &'static str,
}

/// Draw a whole rail's worth of buttons in one call. Runs
/// [`SideActive::invalidate_stale`] first so drag-across-sides
/// doesn't leave a dangling highlight, snapshots each button's open
/// state, calls [`RibbonLayout::button`] for every descriptor, and
/// routes clicks to [`SideActive::toggle_menu`] after the ribbons
/// have finished painting.
pub fn draw_ribbon_buttons(
    ctx: &egui::Context,
    layout: &mut RibbonLayout,
    side_active: &mut SideActive,
    accent: egui::Color32,
    buttons: &[RibbonButton],
) {
    side_active.invalidate_stale(layout);

    // Snapshot open state *before* iterating buttons so the mutable
    // borrow of `layout` below doesn't collide with `side_active`.
    let opens: Vec<bool> = buttons
        .iter()
        .map(|b| side_active.is_menu_open(layout, b.id))
        .collect();
    let mut clicks = vec![false; buttons.len()];

    for ((b, open), flag) in buttons.iter().zip(&opens).zip(clicks.iter_mut()) {
        layout.button(
            ctx,
            b.id,
            b.constraint,
            b.side,
            b.slot,
            b.glyph,
            b.tooltip,
            *open,
            accent,
            || {
                *flag = true;
            },
        );
    }

    for (b, clicked) in buttons.iter().zip(&clicks) {
        if *clicked {
            side_active.toggle_menu(layout, b.id);
        }
    }
}
