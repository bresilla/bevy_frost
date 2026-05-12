//! # Ribbons — edge-anchored draggable button strips.
//!
//! `frost_core`'s ribbon module is the *modern* assembly API distilled
//! down. The older `RibbonLayout` / `SideActive` / `RibbonButton` /
//! `SideRibbon` / `BarRibbon` paths that lived next to it in
//! `frostcore` were retired here — every consumer in `frost_core`
//! (and the `newui` example) drives ribbons through
//! [`draw_assembly`] alone.
//!
//! ## What's here
//!
//! * [`RibbonDef`] / [`RibbonItem`] — declarative shape (one slice
//!   of each describes a whole UI's ribbon layout).
//! * [`RibbonOpen`] / [`RibbonPlacement`] / [`RibbonDrag`] — three
//!   `Resource`-able state types: which panel is currently open per
//!   ribbon, where draggable buttons currently live, and ongoing
//!   drag state.
//! * [`draw_assembly`] — the per-frame entry point. Paints buttons,
//!   handles drag-to-rearrange + cross-ribbon drops, returns
//!   `Icon`-role click events.
//! * [`paint_ribbon_button`] / [`paint_ribbon_glyph`] / etc. —
//!   surfaced from [`paint`] for downstream callers that want to
//!   paint their own button-shaped widgets in the same style.
//! * [`EDGE_GAP`] / [`SIDE_BTN_SIZE`] / [`SIDE_BTN_GAP`] — layout
//!   constants every consumer (incl. [`crate::pane`]) reads to
//!   align with the rail strip.

pub mod action;
pub mod assembly;
pub mod dispatch;
mod paint;
pub mod permanent;
pub mod resolve;
pub mod slot;
pub mod slot_paint;

// Layout constants — re-exported so `pane::layout` and other
// modules can compute insets without duplicating values.
pub use paint::{EDGE_GAP, SIDE_BTN_GAP, SIDE_BTN_SIZE};

pub use action::RibbonAction;
pub use assembly::{
    RibbonClick, RibbonCluster, RibbonDef, RibbonDrag, RibbonEdge, RibbonGlyph, RibbonItem,
    RibbonMode, RibbonOpen, RibbonPlacement, RibbonRole, RibbonWidth, draw_assembly, find_item,
    find_ribbon, main_bar_empty_drag_started, panel_anchor, panel_anchor_for_item,
};
pub use dispatch::{RibbonActionError, RibbonActionResult, dispatch_ribbon_action};
pub use permanent::{
    permanent_system_control_ribbon, permanent_view_switcher_ribbon,
    restore_workspace_slot_override, system_close_or_restore_slot_id,
};
pub use resolve::{resolve_slot_item, resolve_slot_items};
pub use slot::{
    RibbonOverrideLayer, RibbonOverridePolicy, RibbonScope, RibbonSlot, RibbonSlotDef,
    RibbonSlotId, RibbonSlotItem, RibbonSlotOverride,
};
pub use slot_paint::{ResolvedSlotRibbon, RibbonSlotClick, draw_slot_ribbons};
