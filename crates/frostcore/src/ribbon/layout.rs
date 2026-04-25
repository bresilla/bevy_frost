//! Drag-aware layout for ribbon buttons.
//!
//! Where [`super::static_ribbon`] paints fixed rows, this module
//! layers a stateful layout on top: each button is registered by a
//! stable string id, placement lives in a [`RibbonLayout`]
//! `Resource`, and the user can drag buttons between ribbons.
//!
//! The geometry helpers here (`rect_for`, `ribbon_under_cursor`,
//! `insertion_slot_under_cursor`, `effective_visual`) are all
//! `pub(super)` so the sibling [`super::ghost`] module can reuse
//! them without every consumer seeing them.

use std::collections::HashMap;

#[cfg(feature = "bevy")] use bevy::prelude::*;
use egui;


use super::kinds::{RibbonConstraint, RibbonKind};
use super::paint::{paint_ribbon_button, EDGE_GAP, SIDE_BTN_GAP, SIDE_BTN_SIZE};

// ─── Internal state ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub(super) struct Placement {
    pub kind: RibbonKind,
    pub slot: u32,
}

#[derive(Debug, Clone)]
pub(super) struct DragState {
    pub id: String,
    pub origin_kind: RibbonKind,
    pub origin_slot: u32,
    pub cursor: egui::Pos2,
    /// Which ribbons the dragged button is allowed to land on.
    pub constraint: RibbonConstraint,
}

// ─── Public resource ────────────────────────────────────────────────

/// Per-button placement registry plus live drag state. Initialised
/// empty; populated on first call to [`RibbonLayout::button`] for
/// each id.
#[cfg_attr(feature = "bevy", derive(bevy::prelude::Resource))]
#[derive(Default)]
pub struct RibbonLayout {
    pub(super) placements: HashMap<String, Placement>,
    pub(super) drag: Option<DragState>,
}

impl RibbonLayout {
    /// Which ribbon does `id`'s button currently sit on? `None` if
    /// the button hasn't been registered yet (i.e. its owning
    /// system hasn't run this frame).
    pub fn ribbon_of(&self, id: &str) -> Option<RibbonKind> {
        self.placements.get(id).map(|p| p.kind)
    }

    /// Convenience: convert [`Self::ribbon_of`] into the egui
    /// [`egui::Align2`] anchor best suited for a panel that opens
    /// from that button's rail. Defaults to `LEFT_TOP` when the
    /// button can't be found.
    pub fn panel_anchor(&self, id: &str) -> egui::Align2 {
        match self.ribbon_of(id) {
            Some(RibbonKind::Left) => egui::Align2::LEFT_TOP,
            Some(RibbonKind::Right) => egui::Align2::RIGHT_TOP,
            Some(RibbonKind::Top) => egui::Align2::CENTER_TOP,
            Some(RibbonKind::Bottom) => egui::Align2::CENTER_BOTTOM,
            None => egui::Align2::LEFT_TOP,
        }
    }

    /// Paint one ribbon button. First call for a given `id`
    /// registers it at the supplied default; subsequent calls paint
    /// wherever the user has dragged it.
    ///
    /// `on_click` fires only on genuine clicks, never at the end of
    /// a drag — so you can't accidentally toggle a panel while
    /// rearranging.
    #[allow(clippy::too_many_arguments)]
    pub fn button(
        &mut self,
        ctx: &egui::Context,
        id: &'static str,
        constraint: RibbonConstraint,
        default_kind: RibbonKind,
        default_slot: u32,
        glyph: &str,
        tooltip: &str,
        is_active: bool,
        accent: egui::Color32,
        on_click: impl FnOnce(),
    ) {
        debug_assert!(
            constraint.allows(default_kind),
            "default_kind {default_kind:?} violates constraint {constraint:?}",
        );

        let placement = *self.placements.entry(id.to_string()).or_insert(Placement {
            kind: default_kind,
            slot: default_slot,
        });

        let is_dragging_this = self
            .drag
            .as_ref()
            .map(|d| d.id == id)
            .unwrap_or(false);

        let screen = ctx.content_rect();

        // Effective position accounts for an in-flight drag: every
        // non-dragged button on the source or target ribbons shifts
        // to make room so you see the gap open up live.
        let (vis_kind, vis_slot, vis_total) = if is_dragging_this {
            (
                placement.kind,
                placement.slot,
                self.placements
                    .values()
                    .filter(|p| p.kind == placement.kind)
                    .count() as u32,
            )
        } else {
            effective_visual(placement, &self.placements, self.drag.as_ref(), screen)
        };
        let resting_rect = rect_for(vis_kind, vis_slot, vis_total, screen);

        // While being dragged, the button paints at the cursor — and
        // in an area with `Order::Tooltip` so it sits above every
        // other ribbon / panel.
        let (paint_rect, area_order) = if is_dragging_this {
            let cursor = self
                .drag
                .as_ref()
                .map(|d| d.cursor)
                .unwrap_or_else(|| resting_rect.center());
            let r = egui::Rect::from_center_size(
                cursor,
                egui::vec2(SIDE_BTN_SIZE, SIDE_BTN_SIZE),
            );
            (r, egui::Order::Tooltip)
        } else {
            (resting_rect, egui::Order::Middle)
        };

        // Build the area manually so we can (a) set `Order` per frame
        // and (b) attach `click_and_drag` sense.
        let area_id = egui::Id::new(("ribbon_btn", id));
        let resp = egui::Area::new(area_id)
            .order(area_order)
            .fixed_pos(paint_rect.min)
            .interactable(true)
            .show(ctx, |ui| {
                let (rect, r) = ui.allocate_exact_size(
                    egui::vec2(SIDE_BTN_SIZE, SIDE_BTN_SIZE),
                    egui::Sense::click_and_drag(),
                );
                paint_ribbon_button(
                    ui.painter(),
                    rect,
                    accent,
                    is_active,
                    r.hovered() || is_dragging_this,
                );
                let fg = if is_active || is_dragging_this {
                    crate::style::contrast_text_for(accent)
                } else {
                    crate::style::on_panel_dim()
                };
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    glyph,
                    egui::FontId::new(14.0, egui::FontFamily::Monospace),
                    fg,
                );
                r.on_hover_text(tooltip)
            })
            .inner;

        // ── Drag lifecycle ──
        if resp.drag_started() {
            self.drag = Some(DragState {
                id: id.to_string(),
                origin_kind: placement.kind,
                origin_slot: placement.slot,
                cursor: ctx
                    .pointer_interact_pos()
                    .unwrap_or_else(|| resting_rect.center()),
                constraint,
            });
        } else if is_dragging_this && resp.dragged() {
            if let (Some(drag), Some(pos)) = (self.drag.as_mut(), ctx.pointer_interact_pos()) {
                drag.cursor = pos;
            }
        } else if is_dragging_this && resp.drag_stopped() {
            if let Some(drag) = self.drag.take() {
                self.resolve_drop(id, drag, constraint, screen);
            }
        }

        // ── Click ──
        // Skip during any drag on this button — `drag_stopped()` and
        // `clicked()` both fire on the same release, and we don't
        // want a rearrange to also toggle the panel.
        if resp.clicked() && self.drag.is_none() && !is_dragging_this {
            on_click();
        }
    }

    fn resolve_drop(
        &mut self,
        id: &str,
        drag: DragState,
        constraint: RibbonConstraint,
        screen: egui::Rect,
    ) {
        let Some(target_kind) = ribbon_under_cursor(drag.cursor, screen) else {
            return; // Dropped outside any ribbon — snap back.
        };
        if !constraint.allows(target_kind) {
            return; // Disallowed cross — snap back.
        }

        let insert_slot = insertion_slot_under_cursor(
            target_kind,
            drag.cursor,
            &self.placements,
            id,
            screen,
        );

        let source_kind = drag.origin_kind;
        let source_slot = drag.origin_slot;

        // Compact the source ribbon (close the gap left by the
        // dragged button) when crossing ribbons.
        if source_kind != target_kind {
            for p in self.placements.values_mut() {
                if p.kind == source_kind && p.slot > source_slot {
                    p.slot -= 1;
                }
            }
        }

        // Make room on the target ribbon.
        for (other_id, p) in self.placements.iter_mut() {
            if other_id == id {
                continue;
            }
            if p.kind == target_kind && p.slot >= insert_slot {
                p.slot += 1;
            }
        }

        if let Some(p) = self.placements.get_mut(id) {
            p.kind = target_kind;
            p.slot = insert_slot;
        }

        // Re-pack every ribbon so slots stay contiguous 0..N.
        for kind in [
            RibbonKind::Left,
            RibbonKind::Right,
            RibbonKind::Top,
            RibbonKind::Bottom,
        ] {
            let mut ids: Vec<(String, u32)> = self
                .placements
                .iter()
                .filter(|(_, p)| p.kind == kind)
                .map(|(k, p)| (k.clone(), p.slot))
                .collect();
            ids.sort_by_key(|(_, s)| *s);
            for (i, (bid, _)) in ids.into_iter().enumerate() {
                if let Some(p) = self.placements.get_mut(&bid) {
                    p.slot = i as u32;
                }
            }
        }
    }
}

// ─── Geometry helpers (pub(super) for ghost + tests) ────────────────

pub(super) fn rect_for(kind: RibbonKind, slot: u32, total: u32, screen: egui::Rect) -> egui::Rect {
    let slot_s = slot as f32 * (SIDE_BTN_SIZE + SIDE_BTN_GAP);
    match kind {
        RibbonKind::Left => egui::Rect::from_min_size(
            egui::pos2(
                screen.min.x + EDGE_GAP,
                screen.min.y + EDGE_GAP + slot_s,
            ),
            egui::vec2(SIDE_BTN_SIZE, SIDE_BTN_SIZE),
        ),
        RibbonKind::Right => egui::Rect::from_min_size(
            egui::pos2(
                screen.max.x - EDGE_GAP - SIDE_BTN_SIZE,
                screen.min.y + EDGE_GAP + slot_s,
            ),
            egui::vec2(SIDE_BTN_SIZE, SIDE_BTN_SIZE),
        ),
        RibbonKind::Top => {
            let (bar_left, _) = bar_extent(total, screen);
            egui::Rect::from_min_size(
                egui::pos2(
                    bar_left + slot as f32 * (SIDE_BTN_SIZE + SIDE_BTN_GAP),
                    screen.min.y + EDGE_GAP,
                ),
                egui::vec2(SIDE_BTN_SIZE, SIDE_BTN_SIZE),
            )
        }
        RibbonKind::Bottom => {
            let (bar_left, _) = bar_extent(total, screen);
            egui::Rect::from_min_size(
                egui::pos2(
                    bar_left + slot as f32 * (SIDE_BTN_SIZE + SIDE_BTN_GAP),
                    screen.max.y - EDGE_GAP - SIDE_BTN_SIZE,
                ),
                egui::vec2(SIDE_BTN_SIZE, SIDE_BTN_SIZE),
            )
        }
    }
}

fn bar_extent(total: u32, screen: egui::Rect) -> (f32, f32) {
    let n = total.max(1) as f32;
    let row_w = n * SIDE_BTN_SIZE + (n - 1.0).max(0.0) * SIDE_BTN_GAP;
    let center_x = screen.center().x;
    let left = center_x - row_w * 0.5;
    (left, left + row_w)
}

/// Which ribbon (if any) does `cursor` fall inside?
pub(super) fn ribbon_under_cursor(cursor: egui::Pos2, screen: egui::Rect) -> Option<RibbonKind> {
    let zone = SIDE_BTN_SIZE + EDGE_GAP * 2.0;
    if cursor.x >= screen.min.x && cursor.x <= screen.min.x + zone {
        return Some(RibbonKind::Left);
    }
    if cursor.x >= screen.max.x - zone && cursor.x <= screen.max.x {
        return Some(RibbonKind::Right);
    }
    if cursor.y >= screen.min.y && cursor.y <= screen.min.y + zone {
        return Some(RibbonKind::Top);
    }
    if cursor.y >= screen.max.y - zone && cursor.y <= screen.max.y {
        return Some(RibbonKind::Bottom);
    }
    None
}

/// Slot index to insert the dragged button at on `kind`'s ribbon.
pub(super) fn insertion_slot_under_cursor(
    kind: RibbonKind,
    cursor: egui::Pos2,
    placements: &HashMap<String, Placement>,
    dragged_id: &str,
    screen: egui::Rect,
) -> u32 {
    let siblings = placements
        .iter()
        .filter(|(id, p)| p.kind == kind && id.as_str() != dragged_id)
        .count() as u32;
    let total_visible = siblings + 1;

    if kind.is_vertical() {
        let rel = cursor.y - screen.min.y - EDGE_GAP;
        let step = SIDE_BTN_SIZE + SIDE_BTN_GAP;
        let raw = (rel / step).round();
        raw.clamp(0.0, total_visible as f32 - 1.0) as u32
    } else {
        let (bar_left, _) = bar_extent(total_visible, screen);
        let rel = cursor.x - bar_left;
        let step = SIDE_BTN_SIZE + SIDE_BTN_GAP;
        let raw = (rel / step).round();
        raw.clamp(0.0, total_visible as f32 - 1.0) as u32
    }
}

// ─── Live reflow ────────────────────────────────────────────────────

/// Compute the effective `(kind, slot, total)` a non-dragged button
/// should render at, given the current drag. Buttons on the source
/// ribbon close the gap; buttons on the target ribbon make space;
/// same-ribbon reorders shift in either direction.
pub(super) fn effective_visual(
    placement: Placement,
    placements: &HashMap<String, Placement>,
    drag: Option<&DragState>,
    screen: egui::Rect,
) -> (RibbonKind, u32, u32) {
    let raw_total = |kind: RibbonKind| -> u32 {
        placements.values().filter(|p| p.kind == kind).count() as u32
    };

    let Some(drag) = drag else {
        return (placement.kind, placement.slot, raw_total(placement.kind));
    };
    let Some(target) = ribbon_under_cursor(drag.cursor, screen) else {
        return (placement.kind, placement.slot, raw_total(placement.kind));
    };
    if !drag.constraint.allows(target) {
        return (placement.kind, placement.slot, raw_total(placement.kind));
    }

    let source = drag.origin_kind;
    let src_slot = drag.origin_slot;
    let insert = insertion_slot_under_cursor(
        target,
        drag.cursor,
        placements,
        &drag.id,
        screen,
    );

    let kind = placement.kind;
    let mut slot = placement.slot;
    if kind == source && kind == target {
        if src_slot < insert && slot > src_slot && slot <= insert {
            slot -= 1;
        } else if src_slot > insert && slot >= insert && slot < src_slot {
            slot += 1;
        }
    } else {
        if kind == source && slot > src_slot {
            slot -= 1;
        }
        if kind == target && slot >= insert {
            slot += 1;
        }
    }

    let total = if source != target {
        if kind == source {
            raw_total(kind).saturating_sub(1)
        } else if kind == target {
            raw_total(kind) + 1
        } else {
            raw_total(kind)
        }
    } else {
        raw_total(kind)
    };

    (kind, slot, total)
}
