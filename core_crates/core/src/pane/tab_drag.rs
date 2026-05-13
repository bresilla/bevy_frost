//! Per-pane state for drag-reordering tabs within a tab strip
//! AND transferring tabs between containers in the same pane.
//!
//! Scope is explicitly per-pane — a tab can only land in a strip
//! that lives inside the same pane its source strip does. Cross-
//! pane transfers are not supported (and the data plumbing
//! doesn't expose them: a tab's pod payload only flows through
//! one pane's `PaneBody::render` call per frame).

use std::collections::HashMap;

use egui::{Color32, Context, Id, Pos2, Rect};

// ─── State types ───────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub struct TabDragState {
    pub tab_id: Id,
    pub source_container: Id,
    pub cursor: Option<Pos2>,
}

/// One tab button's painted rect, keyed by `(container_id, tab_id)`.
/// Populated by `paint_folder_tabs` / `paint_top_tabs` each frame so
/// `find_drop_target` can resolve the cursor's slot.
#[derive(Clone, Copy, Debug)]
pub struct TabButtonEntry {
    pub container_id: Id,
    pub tab_id: Id,
    pub rect: Rect,
}

/// One container's painted tab strip rect — the hit zone the
/// dragger has to land in for a drop to count. The `axis_horizontal`
/// flag tells `find_drop_target` whether tabs in this strip are
/// laid out left-to-right (`true`) or top-to-bottom (`false`).
#[derive(Clone, Copy, Debug)]
pub struct TabStripEntry {
    pub container_id: Id,
    pub rect: Rect,
    pub axis_horizontal: bool,
}

// ─── ctx-data keys ─────────────────────────────────────────────────

fn drag_key(pane_id: Id) -> Id {
    pane_id.with("frost_tab_drag")
}
fn strip_cache_key(pane_id: Id) -> Id {
    pane_id.with("frost_tab_strip_cache")
}
fn button_cache_key(pane_id: Id) -> Id {
    pane_id.with("frost_tab_button_cache")
}
/// Per-pane: which container each tab id currently belongs to.
fn owner_key(pane_id: Id) -> Id {
    pane_id.with("frost_tab_owner")
}
/// Per-pane: per-container ordered tab id list.
fn order_key(pane_id: Id) -> Id {
    pane_id.with("frost_tab_order")
}

// ─── Drag state accessors ──────────────────────────────────────────

pub fn drag_state(ctx: &Context, pane_id: Id) -> Option<TabDragState> {
    ctx.data(|d| d.get_temp::<TabDragState>(drag_key(pane_id)))
}

pub fn set_drag(ctx: &Context, pane_id: Id, state: TabDragState) {
    ctx.data_mut(|d| d.insert_temp(drag_key(pane_id), state));
}

pub fn clear_drag(ctx: &Context, pane_id: Id) {
    ctx.data_mut(|d| d.remove::<TabDragState>(drag_key(pane_id)));
}

// ─── Strip + button rect cache ─────────────────────────────────────

/// No-op per-frame hook reserved for future cache lifecycle.
/// `push_button` and `push_strip` already replace any stale entry
/// keyed by `(container_id, tab_id)` / `container_id`, so the
/// caches stay coherent across frames without an explicit clear.
pub fn begin_frame(_ctx: &Context, _pane_id: Id) {}

/// Drop every cached button entry for `container_id` before that
/// container's tab strip paints its current-frame buttons. Without
/// this, a tab that moved out of `container_id` last frame would
/// leave a stale entry in the cache and skew `find_drop_target`'s
/// slot computation by one.
pub fn reset_container_buttons(ctx: &Context, pane_id: Id, container_id: Id) {
    ctx.data_mut(|d| {
        let mut cache: Vec<TabButtonEntry> =
            d.get_temp(button_cache_key(pane_id)).unwrap_or_default();
        cache.retain(|e| e.container_id != container_id);
        d.insert_temp(button_cache_key(pane_id), cache);
    });
}

pub fn push_strip(ctx: &Context, pane_id: Id, entry: TabStripEntry) {
    ctx.data_mut(|d| {
        let mut cache: Vec<TabStripEntry> =
            d.get_temp(strip_cache_key(pane_id)).unwrap_or_default();
        cache.retain(|e| e.container_id != entry.container_id);
        cache.push(entry);
        d.insert_temp(strip_cache_key(pane_id), cache);
    });
}

pub fn push_button(ctx: &Context, pane_id: Id, entry: TabButtonEntry) {
    ctx.data_mut(|d| {
        let mut cache: Vec<TabButtonEntry> =
            d.get_temp(button_cache_key(pane_id)).unwrap_or_default();
        cache.retain(|e| !(e.container_id == entry.container_id && e.tab_id == entry.tab_id));
        cache.push(entry);
        d.insert_temp(button_cache_key(pane_id), cache);
    });
}

pub fn strip_cache(ctx: &Context, pane_id: Id) -> Vec<TabStripEntry> {
    ctx.data(|d| d.get_temp(strip_cache_key(pane_id)))
        .unwrap_or_default()
}

pub fn button_cache(ctx: &Context, pane_id: Id) -> Vec<TabButtonEntry> {
    ctx.data(|d| d.get_temp(button_cache_key(pane_id)))
        .unwrap_or_default()
}

// ─── Routing persistence ───────────────────────────────────────────

fn read_owner(ctx: &Context, pane_id: Id) -> HashMap<Id, Id> {
    ctx.data_mut(|d| d.get_persisted(owner_key(pane_id)))
        .unwrap_or_default()
}

fn write_owner(ctx: &Context, pane_id: Id, map: HashMap<Id, Id>) {
    ctx.data_mut(|d| d.insert_persisted(owner_key(pane_id), map));
}

fn read_order(ctx: &Context, pane_id: Id) -> HashMap<Id, Vec<Id>> {
    ctx.data_mut(|d| d.get_persisted(order_key(pane_id)))
        .unwrap_or_default()
}

fn write_order(ctx: &Context, pane_id: Id, map: HashMap<Id, Vec<Id>>) {
    ctx.data_mut(|d| d.insert_persisted(order_key(pane_id), map));
}

/// For one container: the ordered tab ids belonging to it, derived
/// from persisted owner map + persisted per-container order, falling
/// back to declared `default_tabs` for any tab whose owner isn't yet
/// persisted. Tabs the persisted owner map assigns AWAY from this
/// container are filtered out; tabs the persisted owner map assigns
/// TO this container from elsewhere are pulled in.
pub fn route(
    ctx: &Context,
    pane_id: Id,
    container_id: Id,
    default_tabs_here: &[Id],
    all_tabs_in_pane: &[(Id, Id)], // (tab_id, declared_container)
) -> Vec<Id> {
    let owner = read_owner(ctx, pane_id);
    let order = read_order(ctx, pane_id);

    // Tabs currently owned by this container = persisted owner ==
    // container_id, OR declared in this container AND not persisted
    // elsewhere.
    let owned: Vec<Id> = all_tabs_in_pane
        .iter()
        .filter_map(|(tid, declared)| {
            let actual_owner = owner.get(tid).copied().unwrap_or(*declared);
            (actual_owner == container_id).then_some(*tid)
        })
        .collect();

    // Order: persisted order, filtered to owned + appended with any
    // owned tabs missing from the persisted list (newcomers).
    let persisted = order.get(&container_id).cloned().unwrap_or_default();
    let mut out: Vec<Id> = persisted
        .into_iter()
        .filter(|id| owned.contains(id))
        .collect();
    // Append declared-but-unpersisted tabs in their declared order.
    for tid in default_tabs_here {
        if owned.contains(tid) && !out.contains(tid) {
            out.push(*tid);
        }
    }
    // Append any owned tabs that weren't in the declared list either
    // (= transferred in from another container) in tab-id discovery
    // order.
    for tid in &owned {
        if !out.contains(tid) {
            out.push(*tid);
        }
    }
    out
}

/// Commit a drop: move `tab_id` (currently at `source_container`) to
/// `target_container` at slot `target_slot` (0 = first). Updates
/// both the owner map and the per-container order.
pub fn commit_drop(
    ctx: &Context,
    pane_id: Id,
    tab_id: Id,
    source_container: Id,
    target_container: Id,
    target_slot: usize,
) {
    let mut owner = read_owner(ctx, pane_id);
    owner.insert(tab_id, target_container);
    write_owner(ctx, pane_id, owner);

    let mut order = read_order(ctx, pane_id);
    // Remove from source.
    if let Some(src) = order.get_mut(&source_container) {
        src.retain(|id| *id != tab_id);
    }
    // Insert into target at slot, dedup.
    let tgt = order.entry(target_container).or_default();
    tgt.retain(|id| *id != tab_id);
    let slot = target_slot.min(tgt.len());
    tgt.insert(slot, tab_id);
    write_order(ctx, pane_id, order);
}

// ─── Drop target detection ─────────────────────────────────────────

/// Given the cursor, locate the (container, insertion-slot) that
/// would receive the drop. Returns `None` if the cursor isn't over
/// any registered tab strip in this pane.
pub fn find_drop_target(ctx: &Context, pane_id: Id, cursor: Pos2) -> Option<(Id, usize)> {
    let strips = strip_cache(ctx, pane_id);
    let target_strip = strips.iter().find(|s| s.rect.contains(cursor)).copied()?;
    let buttons = button_cache(ctx, pane_id);
    let mut tabs_in_strip: Vec<TabButtonEntry> = buttons
        .into_iter()
        .filter(|b| b.container_id == target_strip.container_id)
        .collect();
    // Sort by axis position so slot indexing is stable.
    tabs_in_strip.sort_by(|a, b| {
        let ax = if target_strip.axis_horizontal {
            a.rect.center().x
        } else {
            a.rect.center().y
        };
        let bx = if target_strip.axis_horizontal {
            b.rect.center().x
        } else {
            b.rect.center().y
        };
        ax.partial_cmp(&bx).unwrap_or(std::cmp::Ordering::Equal)
    });
    let cursor_axis = if target_strip.axis_horizontal {
        cursor.x
    } else {
        cursor.y
    };
    let mut slot = 0usize;
    for entry in &tabs_in_strip {
        let c = if target_strip.axis_horizontal {
            entry.rect.center().x
        } else {
            entry.rect.center().y
        };
        if cursor_axis < c {
            return Some((target_strip.container_id, slot));
        }
        slot += 1;
    }
    Some((target_strip.container_id, slot))
}

// ─── Paint helpers ─────────────────────────────────────────────────

/// Paint the dragged tab's preview at the cursor on
/// `Order::Tooltip` — floats above every pane / container layer.
pub fn paint_drag_preview(
    ctx: &Context,
    pane_id: Id,
    button_size: egui::Vec2,
    cursor: Pos2,
    accent: Color32,
    label: &str,
    icon: Option<&str>,
) {
    let pos = egui::pos2(
        cursor.x - button_size.x * 0.5,
        cursor.y - button_size.y * 0.5,
    );
    let area_id = pane_id.with("frost_tab_drag_preview");
    egui::Area::new(area_id)
        .order(egui::Order::Tooltip)
        .fixed_pos(pos)
        .interactable(false)
        .show(ctx, |ui| {
            let rect = Rect::from_min_size(pos, button_size);
            let theme = crate::style::theme();
            ui.painter().rect(
                rect,
                egui::CornerRadius::same(theme.radius_compact),
                Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 72),
                egui::Stroke::new(1.5, accent),
                egui::StrokeKind::Inside,
            );
            // Glyph + label, centred. Best-effort; icon may be empty.
            if let Some(name) = icon {
                if !name.is_empty() {
                    let icon_size = button_size.y * 0.55;
                    crate::icons::paint_icon(
                        &ui.painter(),
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        name,
                        icon_size,
                        crate::style::on_panel(),
                    );
                }
            }
            let _ = label;
        });
}
