//! Compatibility shim — minimal `floating_window_for_item` +
//! `PaneBuilder` API that maps old-frostcore-style panels onto the
//! new [`crate::pane::Pane2`] system.
//!
//! Apps written against `frostcore::floating::floating_window_for_item`
//! call into this shim; new code should target [`crate::pane::Pane2`]
//! directly. The shim swaps in `Pane2` underneath, so the visual
//! ends up identical but the caller doesn't have to migrate every
//! `pane.section(...)` call site.
//!
//! What's supported:
//!
//! * [`floating_window_for_item`] — single per-item floating panel,
//!   anchored to the item's ribbon edge + cluster. Picks a
//!   `PaneAnchor::*Rail(*)` from the [`RibbonDef`] / [`RibbonCluster`].
//! * [`PaneBuilder::section`] — collapsible section inside the
//!   panel body. Routes to [`crate::widget::section`] (a frosted
//!   `CollapsingState`-backed frame).
//!
//! What's NOT supported (ad-hoc additions kept in old frostcore):
//!
//! * `section_with` (icon-prefixed sections) → use [`PaneBuilder::section`]
//!   and add the icon manually inside the body.
//! * `section_order` (drag-reorder of sections) → not relevant
//!   without the pane's container plumbing; use the new
//!   [`crate::pane::Pane2`] + [`crate::container::Normal`] path
//!   for that capability.

use egui::Vec2;

use crate::pane::{Pane2, PaneAnchor, PaneResize, RailZone};
use crate::ribbon::{
    find_item, find_ribbon, RibbonCluster, RibbonDef, RibbonEdge, RibbonItem,
    RibbonPlacement,
};

/// Caller-facing handle inside a `floating_window_for_item` body.
/// Forwards `section(...)` calls to [`crate::widget::section`].
pub struct PaneBuilder<'a> {
    ui: &'a mut egui::Ui,
    accent: egui::Color32,
}

impl<'a> PaneBuilder<'a> {
    fn new(ui: &'a mut egui::Ui, accent: egui::Color32) -> Self {
        Self { ui, accent }
    }

    /// Render a collapsible section. Same signature as the old
    /// `frostcore::floating::PaneBuilder::section`.
    pub fn section(
        &mut self,
        id_salt: &str,
        title: &str,
        default_open: bool,
        body: impl FnOnce(&mut egui::Ui),
    ) {
        crate::widget::section(self.ui, id_salt, title, self.accent, default_open, body);
    }
}

/// Map an item's `(ribbon edge, cluster)` to the corresponding
/// [`PaneAnchor`].
fn pane_anchor_for(def: &RibbonDef, cluster: RibbonCluster) -> PaneAnchor {
    let zone = match cluster {
        RibbonCluster::Start => RailZone::Start,
        RibbonCluster::Middle => RailZone::Middle,
        RibbonCluster::End => RailZone::End,
    };
    match def.edge {
        RibbonEdge::Left => PaneAnchor::LeftRail(zone),
        RibbonEdge::Right => PaneAnchor::RightRail(zone),
        RibbonEdge::Top => PaneAnchor::TopRail(zone),
        RibbonEdge::Bottom => PaneAnchor::BottomRail(zone),
    }
}

/// Show a floating panel for a declared ribbon item. Anchor +
/// rail-zone are derived from the item's resolved
/// `(ribbon, cluster)`. The panel is only painted when `*open` is
/// `true`; the caller is responsible for toggling it (typically
/// via the matching ribbon button or a keyboard shortcut).
///
/// `size` is the (width, height) the body should aim for. The new
/// `Pane2` system auto-sizes around its actual content, so this
/// argument is treated as a hint only — the visible pane may end
/// up smaller (auto-fold) or larger (when the body declares
/// minimum widths via the container system).
pub fn floating_window_for_item(
    ctx: &egui::Context,
    ribbons: &[RibbonDef],
    items: &[RibbonItem],
    placement: &RibbonPlacement,
    item_id: &'static str,
    title: &str,
    _size: Vec2,
    open: &mut bool,
    accent: egui::Color32,
    add_contents: impl FnOnce(&mut PaneBuilder),
) {
    if !*open {
        return;
    }
    let Some(item) = find_item(items, item_id) else {
        return;
    };
    let (rid, cluster, _) = placement.resolve(item);
    let Some(def) = find_ribbon(ribbons, rid) else {
        return;
    };
    let anchor = pane_anchor_for(def, cluster);

    Pane2::new(item_id, title, anchor, accent)
        .resize(PaneResize::SPAN)
        .show(ctx, |body_ui| {
            let mut builder = PaneBuilder::new(body_ui, accent);
            add_contents(&mut builder);
        });
}
