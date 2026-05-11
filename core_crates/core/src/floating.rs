//! Compatibility shim — `floating_window_for_item` + `PaneBuilder`
//! API mapped onto the new [`crate::pane::Pane2`] +
//! [`crate::container::Normal`] system. Apps written against
//! `frostcore::floating::floating_window_for_item` keep their call
//! sites; the visuals are the new frost_core's pane chrome + container
//! chrome wrapping the legacy widget calls inside.
//!
//! What's supported:
//!
//! * [`floating_window_for_item`] — single per-item floating panel,
//!   anchored to the item's ribbon edge + cluster. Picks a
//!   `PaneAnchor::*Rail(*)` from the [`RibbonDef`] / [`RibbonCluster`].
//! * [`PaneBuilder::section`] — each section becomes a separate
//!   [`crate::container::Normal`] inside the pane, with the caller's
//!   body painted via [`crate::pod::Pod::with_custom`]. So the
//!   visual is the same as `demo.rs` (container header + accent
//!   banner + fold chevron) and the body is whatever raw egui +
//!   legacy-widget code the caller passes in.

use egui::Vec2;

use crate::container::Normal;
use crate::pane::{Pane2, PaneAnchor, PaneResize, RailZone};
use crate::ribbon::{
    find_item, find_ribbon, RibbonCluster, RibbonDef, RibbonEdge, RibbonItem,
    RibbonPlacement,
};

/// Caller-facing handle inside a `floating_window_for_item` body.
/// Each `section(...)` call paints one [`Normal`] container.
pub struct PaneBuilder<'a> {
    ui: &'a mut egui::Ui,
    pane_id: egui::Id,
    anchor: PaneAnchor,
    accent: egui::Color32,
}

impl<'a> PaneBuilder<'a> {
    fn new(
        ui: &'a mut egui::Ui,
        pane_id: egui::Id,
        anchor: PaneAnchor,
        accent: egui::Color32,
    ) -> Self {
        Self { ui, pane_id, anchor, accent }
    }

    /// Render one container in the pane. The caller's `body` runs
    /// inside a [`Pod::with_custom`] so any combination of legacy
    /// frostcore widgets (`wide_button`, `readout_row`, `chip`, …)
    /// or raw egui calls works inside.
    ///
    /// `default_open` is honoured by the container's persisted
    /// fold state — first-frame paints respect it; subsequent
    /// frames use the user's last toggle. Same semantics the old
    /// `frostcore::floating::PaneBuilder::section` shipped.
    pub fn section(
        &mut self,
        id_salt: &str,
        title: &str,
        default_open: bool,
        body: impl FnOnce(&mut egui::Ui),
    ) {
        let cid = self.pane_id.with(("frost_compat_section", id_salt));
        seed_default_open(self.ui.ctx(), cid, default_open);
        Normal::new(title, self.anchor, self.accent, cid).show_raw(self.ui, body);
    }

    /// Same as [`PaneBuilder::section`] but lets the caller provide
    /// an icon glyph for the container title strip — mirrors the
    /// legacy `section_with` shape.
    pub fn section_with(
        &mut self,
        id_salt: &str,
        title: &str,
        icon: &'static str,
        default_open: bool,
        body: impl FnOnce(&mut egui::Ui),
    ) {
        let cid = self.pane_id.with(("frost_compat_section", id_salt));
        seed_default_open(self.ui.ctx(), cid, default_open);
        Normal::new(title, self.anchor, self.accent, cid)
            .icon(icon)
            .show_raw(self.ui, body);
    }
}

/// First-frame seed for a container's persisted `body_open`. Once
/// the user toggles the fold chevron, the persisted value takes
/// over — this only fires while the slot is unset.
fn seed_default_open(ctx: &egui::Context, cid: egui::Id, default_open: bool) {
    let key = cid.with("body_open");
    let already_set: bool =
        ctx.data_mut(|d| d.get_persisted::<bool>(key)).is_some();
    if !already_set {
        ctx.data_mut(|d| d.insert_persisted(key, default_open));
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
/// `true`.
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
    let pane_id = egui::Id::new(item_id);

    Pane2::new(item_id, title, anchor, accent)
        .resize(PaneResize::SPAN)
        .show(ctx, |body_ui| {
            let mut builder = PaneBuilder::new(body_ui, pane_id, anchor, accent);
            add_contents(&mut builder);
        });
}
