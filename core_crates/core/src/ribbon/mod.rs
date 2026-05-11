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

pub mod assembly;
mod paint;

// Layout constants — re-exported so `pane::layout` and other
// modules can compute insets without duplicating values.
pub use paint::{EDGE_GAP, SIDE_BTN_GAP, SIDE_BTN_SIZE};

pub use assembly::{
    RibbonClick, RibbonCluster, RibbonDef, RibbonDrag, RibbonEdge, RibbonGlyph, RibbonItem,
    RibbonMode, RibbonOpen, RibbonPlacement, RibbonRole, RibbonWidth, draw_assembly, find_item,
    find_ribbon, panel_anchor, panel_anchor_for_item,
};
