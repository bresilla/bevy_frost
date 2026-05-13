//! Drag-reorder for [`super::Pane`] containers.
//!
//! Direct port of `frostcore::floating::SectionDragState`. The
//! pattern is:
//!
//! * **The dragged container `return`s early** in `Normal::show` —
//!   it doesn't allocate a layout slot, so the OTHER containers
//!   visibly collapse upward to fill its place.
//! * **An inline ghost gap** is allocated via `allocate_exact_size`
//!   at the cursor's target slot during the iteration — pushing
//!   subsequent containers DOWN to make room. The gap is painted
//!   with an accent rect so the user sees where the drop will land.
//! * **The persistent order is stable** during the drag — only the
//!   gap moves around as the cursor moves. On release, the
//!   dragged id is spliced into the persisted order at the target
//!   slot.
//! * **A floating preview** of the dragged container's last-known
//!   rect renders at the cursor (paint-only, separate Area).

use egui::{Color32, Context, Id, Pos2, Rect, Sense, Ui, Vec2};

use crate::pane::active_pane_key;
use crate::style;

// ─── State ─────────────────────────────────────────────────────────

/// Per-pane drag bookkeeping. `item` latches the dragged
/// container's id; `cursor` is the latest pointer position used to
/// compute the target slot for the ghost gap.
#[derive(Clone, Copy, Debug, Default)]
pub struct DragState {
    pub item: Option<Id>,
    pub cursor: Option<Pos2>,
}

#[derive(Clone, Copy, Debug)]
pub struct RectEntry {
    pub id: Id,
    pub rect: Rect,
    pub frame: Option<Rect>,
}

// ─── ctx-data accessors ────────────────────────────────────────────

fn drag_key(pane_id: Id) -> Id {
    pane_id.with("frost_pane_drag")
}
/// Cache of containers RENDERED THIS FRAME, populated by
/// `Normal::show` as it paints. Cleared at body start.
fn current_key(pane_id: Id) -> Id {
    pane_id.with("frost_pane_drag_current")
}
/// Snapshot of the PREVIOUS frame's full cache — including the
/// dragged container's last-known rect (carried forward from
/// before the drag started). Read paths (compute_target, ghost gap
/// sizing, preview) consult this so the dragged dimension stays
/// available even though `Normal::show` skips the dragged item.
fn snapshot_key(pane_id: Id) -> Id {
    pane_id.with("frost_pane_drag_snapshot")
}
fn order_key(pane_id: Id) -> Id {
    pane_id.with("frost_pane_section_order")
}
pub fn state(ctx: &Context, pane_id: Id) -> DragState {
    ctx.data(|d| d.get_temp(drag_key(pane_id)))
        .unwrap_or_default()
}

pub fn set_drag(ctx: &Context, pane_id: Id, state: DragState) {
    ctx.data_mut(|d| d.insert_temp(drag_key(pane_id), state));
}

pub fn clear_drag(ctx: &Context, pane_id: Id) {
    ctx.data_mut(|d| d.remove::<DragState>(drag_key(pane_id)));
}

/// Clear the per-frame current cache at body start. Snapshot from
/// the prev frame is preserved so reads still see the dragged
/// container's size.
pub fn begin_frame(ctx: &Context, pane_id: Id) {
    ctx.data_mut(|d| {
        d.remove::<Vec<RectEntry>>(current_key(pane_id));
    });
}

pub fn push_rect(ctx: &Context, pane_id: Id, id: Id, rect: Rect) {
    push_rect_with_frame(ctx, pane_id, id, rect, None);
}

pub fn push_rect_with_frame(ctx: &Context, pane_id: Id, id: Id, rect: Rect, frame: Option<Rect>) {
    ctx.data_mut(|d| {
        let mut cache: Vec<RectEntry> = d.get_temp(current_key(pane_id)).unwrap_or_default();
        if let Some(slot) = cache.iter_mut().find(|e| e.id == id) {
            slot.rect = rect;
            slot.frame = frame;
        } else {
            cache.push(RectEntry { id, rect, frame });
        }
        d.insert_temp(current_key(pane_id), cache);
    });
}

pub fn current_cache(ctx: &Context, pane_id: Id) -> Vec<RectEntry> {
    ctx.data(|d| d.get_temp(current_key(pane_id)))
        .unwrap_or_default()
}

pub fn snapshot(ctx: &Context, pane_id: Id) -> Vec<RectEntry> {
    ctx.data(|d| d.get_temp(snapshot_key(pane_id)))
        .unwrap_or_default()
}

/// Build this frame's snapshot from `current_cache` + the dragged
/// container's previous-frame rect (so its size stays available
/// for ghost gap / preview during the drag).
pub fn finalize_snapshot(ctx: &Context, pane_id: Id) {
    let drag = state(ctx, pane_id);
    let mut cache = current_cache(ctx, pane_id);
    if let Some(dragged_id) = drag.item {
        if !cache.iter().any(|e| e.id == dragged_id) {
            let prev = snapshot(ctx, pane_id);
            if let Some(entry) = prev.iter().find(|e| e.id == dragged_id).copied() {
                cache.push(entry);
            }
        }
    }
    ctx.data_mut(|d| d.insert_temp(snapshot_key(pane_id), cache));
}

// ─── Order persistence ─────────────────────────────────────────────

/// Read the persisted section order for `pane_id`, merged with
/// `defaults`. Stored ids that are still in `defaults` keep their
/// stored order; new ids in `defaults` (= containers added after
/// the last drag) are appended in their default position.
///
/// **Stable during drag**: the order is NOT visually shuffled while
/// a drag is in flight — the dragged container vanishes from
/// layout and a ghost gap travels with the cursor instead. On
/// release, the persistent order is updated.
pub fn section_order_for(ctx: &Context, pane_id: Id, defaults: &[Id]) -> Vec<Id> {
    let stored: Vec<Id> = ctx
        .data_mut(|d| d.get_persisted(order_key(pane_id)))
        .unwrap_or_default();
    let mut order: Vec<Id> = stored
        .iter()
        .copied()
        .filter(|id| defaults.contains(id))
        .collect();
    for id in defaults {
        if !order.contains(id) {
            order.push(*id);
        }
    }
    order
}

/// Persist a new section order for `pane_id`. Survives across
/// runs (`insert_persisted`).
pub fn set_section_order(ctx: &Context, pane_id: Id, order: Vec<Id>) {
    ctx.data_mut(|d| d.insert_persisted(order_key(pane_id), order));
}

// ─── Convenience for Normal ────────────────────────────────────────

/// Look up the **active pane**'s drag state. Used by `Normal` —
/// which doesn't directly know its parent `Pane`'s id — via the
/// `active_pane_key` pointer that `Pane::show` writes at the top
/// of every frame.
pub fn active_drag(ctx: &Context) -> Option<(Id, DragState)> {
    let pane_id: Id = ctx.data(|d| d.get_temp(active_pane_key()))?;
    let s = state(ctx, pane_id);
    Some((pane_id, s))
}

// ─── Geometry ──────────────────────────────────────────────────────

/// Pick the gap-index where the cursor would drop the dragged
/// container. Walks the snapshot in display order, skipping the
/// dragged entry. Auto-detects whether the layout direction is
/// reversed (`bottom_up` / `right_to_left`) by comparing the first
/// two non-dragged entries' main-axis centres — if entry[1]'s
/// centre is BEFORE entry[0]'s on the stack-axis, the layout is
/// reversed and the cursor-vs-centre comparison is flipped.
///
/// Indices are in the non-dragged-only space (0 = before all
/// others in iteration order, N = after all others).
pub fn compute_target(
    cache: &[RectEntry],
    dragged: Id,
    cursor: f32,
    horizontal_stack: bool,
) -> usize {
    let centre = |e: &RectEntry| -> f32 {
        if horizontal_stack {
            e.rect.center().x
        } else {
            e.rect.center().y
        }
    };
    let others: Vec<&RectEntry> = cache.iter().filter(|e| e.id != dragged).collect();
    let reversed = if others.len() >= 2 {
        centre(others[1]) < centre(others[0])
    } else {
        false
    };
    let mut idx = 0;
    for entry in others {
        let c = centre(entry);
        let before = if reversed { cursor > c } else { cursor < c };
        if before {
            return idx;
        }
        idx += 1;
    }
    idx
}

pub fn dragged_size(snapshot: &[RectEntry], dragged: Id) -> Option<Vec2> {
    snapshot
        .iter()
        .find(|e| e.id == dragged)
        .map(|e| e.rect.size())
}

pub fn dragged_entry(snapshot: &[RectEntry], dragged: Id) -> Option<RectEntry> {
    snapshot.iter().find(|e| e.id == dragged).copied()
}

// ─── Paint helpers ─────────────────────────────────────────────────

/// Allocate a same-sized slot inline in the parent layout and paint
/// a translucent accent rect. Pushes subsequent containers along
/// the stack axis exactly like the dragged container would.
pub fn paint_ghost_gap_inline(
    ui: &mut Ui,
    dragged_size: Vec2,
    accent: Color32,
    _horizontal_stack: bool,
) {
    let (rect, _) = ui.allocate_exact_size(dragged_size, Sense::hover());
    let theme = style::theme();
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(theme.radius_md),
        Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 36),
        egui::Stroke::new(1.5, accent),
        egui::StrokeKind::Inside,
    );
}

/// Allocate a same-main-axis slot but keep the ghost's cross-axis
/// position from the dragged entry's previous real rect. This is
/// important for tabbed containers: their full footprint may be
/// inset by the folder-tab strip, so painting at the raw layout
/// cursor makes the ghost appear shifted left/up compared to where
/// the container will land.
pub fn paint_ghost_gap_entry_inline(
    ui: &mut Ui,
    entry: RectEntry,
    accent: Color32,
    horizontal_stack: bool,
) {
    let size = entry.rect.size();
    let (slot_rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let rect = if horizontal_stack {
        Rect::from_min_size(egui::pos2(slot_rect.left(), entry.rect.top()), size)
    } else {
        Rect::from_min_size(egui::pos2(entry.rect.left(), slot_rect.top()), size)
    };
    let theme = style::theme();
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(theme.radius_md),
        Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 36),
        egui::Stroke::new(1.5, accent),
        egui::StrokeKind::Inside,
    );
}

/// Paint the dragged container's preview at the cursor on
/// `Order::Tooltip` so it floats above every other UI element.
pub fn paint_drag_preview(
    ctx: &Context,
    pane_id: Id,
    snapshot: &[RectEntry],
    dragged: Id,
    cursor: Pos2,
    accent: Color32,
) {
    let Some(entry) = snapshot.iter().find(|e| e.id == dragged) else {
        return;
    };
    let size = entry.rect.size();
    let pos = egui::pos2(cursor.x - size.x * 0.5, cursor.y - size.y * 0.5);
    let area_id = pane_id.with("frost_pane_drag_preview");
    egui::Area::new(area_id)
        .order(egui::Order::Tooltip)
        .fixed_pos(pos)
        .interactable(false)
        .show(ctx, |ui| {
            let rect = egui::Rect::from_min_size(pos, size);
            let theme = style::theme();
            ui.painter().rect(
                rect,
                egui::CornerRadius::same(theme.radius_md),
                Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 72),
                egui::Stroke::new(1.5, accent),
                egui::StrokeKind::Inside,
            );
        });
}
