//! Floating-panel helper, with drag-to-resize handles on the
//! panel's scene-facing edge (horizontal) AND bottom / top edge
//! (vertical).
//!
//! ## Pane vs container — the enforced constraint
//!
//! A floating pane **cannot host widgets directly**. Every control
//! that lives in a pane has to sit inside a container — either a
//! [`crate::widgets::section`] (foldable card) or a subsection. The
//! pane's body closure takes a [`PaneBuilder`] rather than a raw
//! `egui::Ui`, and `PaneBuilder` only exposes `.section(...)` — so
//! dropping a `toggle` / `slider` / bare widget at the pane level
//! is a *compile error*, not a convention.
//!
//! This is deliberate: panes without container structure devolve
//! into ad-hoc layouts that break under resize, drag, and the
//! frost visual language. Forcing one container per block keeps
//! every panel readable and consistent across projects.
//!
//! Anchored to one of the four screen corners via [`egui::Align2`].
//! No title bar, no close button; the title sits at the rail-facing
//! edge (same side as the [`crate::ribbon`] it's paired with). Two
//! hit-thin strips sit on the *opposite* edges:
//!
//! * **Horizontal handle** — scene-facing edge. Drag to grow / shrink
//!   width.
//! * **Vertical handle** — the edge facing away from the panel's
//!   vertical anchor (bottom for `*_TOP` / `*_CENTER`, top for
//!   `*_BOTTOM`). Drag to grow / shrink height.
//!
//! Both values are stored per-panel-id in `egui::Context::data` so
//! the user's drags survive across frames. Width and height are both
//! clamped every frame to the current window size, so shrinking the
//! Bevy window never leaves the panel extending past the visible
//! screen.

use egui;

use crate::style::{glass_alpha_window, glass_fill, BG_1_PANEL, BORDER_SUBTLE};

// Ribbon layout constants we need here. Kept as locals rather than
// pulling `ribbon::paint` into the public prelude — the numbers
// belong to both modules.
const EDGE_GAP: f32 = 8.0;
const SIDE_BTN_SIZE: f32 = 34.0;
const RAIL_PANEL_GAP: f32 = 6.0;

/// Width of the horizontal (scene-facing) resize-handle hit zone.
const RESIZE_HANDLE_W: f32 = 6.0;
/// Height of the vertical (bottom/top) resize-handle hit zone.
const RESIZE_HANDLE_H: f32 = 6.0;

/// Minimum / maximum panel widths. Caller's `size.x` clamps inside
/// this range on first draw; the user's drag does the same.
const MIN_PANEL_W: f32 = 220.0;
const MAX_PANEL_W: f32 = 1600.0;
/// Minimum / maximum panel heights — same intent as the widths.
const MIN_PANEL_H: f32 = 120.0;
const MAX_PANEL_H: f32 = 1600.0;

const _: () = {
    assert!(EDGE_GAP == 8.0);
    assert!(SIDE_BTN_SIZE == 34.0);
    assert!(RAIL_PANEL_GAP == 6.0);
};

/// Per-pane drag-reorder state, persisted across frames in
/// `ctx.data` keyed by the pane id. `item` latches the dragged
/// section's id_salt; `cursor` is the latest pointer position so
/// the finalize pass can compute the target gap.
#[derive(Clone, Default)]
struct SectionDragState {
    item: Option<String>,
    cursor: Option<egui::Pos2>,
}

/// Builder handed to every [`floating_window`] / [`floating_window_scoped`]
/// body closure. Only exposes container-creating methods — callers
/// cannot reach the underlying `egui::Ui`, so it's impossible to
/// drop bare widgets directly on the pane. Every control in a pane
/// lives inside a [`section`](PaneBuilder::section) (or a nested
/// subsection inside that section's body).
///
/// Sections render in the order the caller invokes them. The pane
/// adds two automatic behaviours on top of the plain section list:
///
/// 1. **Drag-to-reorder** — a transparent drag-sense overlay sits
///    on top of every section's header. Press-and-drag on a header
///    starts a reorder gesture; a thin accent line shows the target
///    gap; release commits the new order. To pick the new order up
///    on the next frame, the caller iterates the result of
///    [`section_order`](Self::section_order) and dispatches via
///    `match` — that's what makes the visual reorder stick. Without
///    that loop, the drag still records intent but the user's code
///    keeps drawing in the same order.
/// 2. **Auto-fold on overflow** — if the rendered section stack
///    overshoots the pane body, the topmost open section is
///    force-closed so the next frame fits. One per frame, converges
///    naturally.
pub struct PaneBuilder<'a> {
    ui: &'a mut egui::Ui,
    accent: egui::Color32,
    pane_id: egui::Id,
    /// Body rect (the area below the title strip).
    body_rect: egui::Rect,
    /// Sections rendered this frame, in user call order. The dragged
    /// section is skipped (lifted out), so this only ever contains
    /// the OTHER sections during a drag.
    rendered: Vec<RenderedSection>,
    /// Number of non-dragged sections rendered so far this frame —
    /// used to decide when to insert the ghost gap during the user's
    /// loop.
    non_dragged_count: usize,
    /// Drag state read from ctx at construction.
    drag: SectionDragState,
    /// Latched in `.section()` when a header reports `drag_started`
    /// — promoted into `drag.item` during finalize.
    drag_started_id: Option<String>,
    /// Stored order this frame, resolved by `section_order`.
    base_order_this_frame: Vec<String>,
    /// Previous-frame snapshot — used to derive the cursor's target
    /// slot from its Y plus the dragged section's natural size for
    /// the ghost gap and floating preview.
    cached_rects: RectCache,
}

struct RenderedSection {
    id_salt: String,
    state_id: egui::Id,
    outer_rect: egui::Rect,
    title: String,
}

impl<'a> PaneBuilder<'a> {
    /// Add a foldable container section to the pane. `id_salt`
    /// disambiguates the section's collapsed-state storage,
    /// `title` is the UPPERCASE accent header, `default_open`
    /// controls the initial expansion, and `body` receives a
    /// regular `&mut egui::Ui` — inside the section, any widget
    /// works as normal.
    ///
    /// Sections render in the order they're called. To make
    /// drag-reorder visually take effect, drive the call order from
    /// [`section_order`](Self::section_order); see the type-level
    /// docs.
    pub fn section(
        &mut self,
        id_salt: &str,
        title: &str,
        default_open: bool,
        body: impl FnOnce(&mut egui::Ui),
    ) {
        // If THIS section is the one being dragged, lift it out of
        // the layout entirely — no header, no body, no allocated
        // space. A faded preview will paint at the cursor in the
        // finalize pass; here we just suppress the natural slot.
        if self.drag.item.as_deref() == Some(id_salt) {
            return;
        }

        // If a drag is in progress and we're at the cursor's target
        // slot (computed in the non-dragged-only index space),
        // insert a ribbon-style ghost-rect gap before rendering this
        // section. The ghost is a same-sized accent-fill rectangle
        // marking exactly where the held section will snap on
        // release.
        if let (Some(dragged_id), Some(cursor)) =
            (self.drag.item.as_deref(), self.drag.cursor)
        {
            let target = compute_target_among_others(&self.cached_rects, dragged_id, cursor.y);
            if self.non_dragged_count == target {
                paint_ghost_gap(self.ui, &self.cached_rects, dragged_id, self.accent);
            }
        }

        let track = crate::widgets::foldable::section_tracked(
            self.ui,
            id_salt,
            title,
            self.accent,
            default_open,
            body,
        );

        // Latch drag start (the only response field we still need —
        // everything else is polled from `ctx.input` in finalize so
        // we don't depend on the dragged section's response, which
        // doesn't get rendered after the first frame).
        if track.header_response.drag_started() {
            self.drag_started_id = Some(id_salt.to_string());
        }

        self.rendered.push(RenderedSection {
            id_salt: id_salt.to_string(),
            state_id: track.state_id,
            outer_rect: track.outer_rect,
            title: title.to_string(),
        });
        self.non_dragged_count += 1;
    }

    /// Returns the stored drag-order for this pane's sections. On
    /// the first frame (or when `default_ids` introduces a new
    /// section), the order is initialised from `default_ids`; once
    /// the user drags to reorder, this method returns the new order
    /// on subsequent frames. Callers iterate the result and
    /// dispatch via `match` so the call order tracks the drag
    /// state:
    ///
    /// ```ignore
    /// for id in pane.section_order(["widgets", "scene", "theme"]) {
    ///     match id.as_str() {
    ///         "widgets" => pane.section("widgets", "Widgets", true, |ui| { /* … */ }),
    ///         "scene"   => pane.section("scene",   "Scene",   true, |ui| { /* … */ }),
    ///         "theme"   => pane.section("theme",   "Theme",   true, |ui| { /* … */ }),
    ///         _ => {}
    ///     }
    /// }
    /// ```
    ///
    /// `default_ids` doubles as the canonical id list — any id in
    /// `default_ids` not present in the stored order gets appended
    /// to the end (so adding a new section to the user's code
    /// inserts it at the bottom rather than dropping it). Stored
    /// ids that no longer appear in `default_ids` are pruned.
    pub fn section_order<I, S>(&mut self, default_ids: I) -> Vec<String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let key = order_key(self.pane_id);
        let defaults: Vec<String> = default_ids.into_iter().map(Into::into).collect();
        let stored: Option<Vec<String>> = self.ui.ctx().data(|d| d.get_temp(key));
        // Resolve the stable stored order, merging defaults so
        // newly-added sections appear (at the end) and stale ids
        // drop. Sections render in this base order — the dragged
        // section keeps its slot during the drag; only the gap moves.
        let base_order: Vec<String> = match stored {
            Some(mut order) => {
                let known: std::collections::HashSet<&str> =
                    defaults.iter().map(|s| s.as_str()).collect();
                order.retain(|id| known.contains(id.as_str()));
                for d in &defaults {
                    if !order.iter().any(|id| id == d) {
                        order.push(d.clone());
                    }
                }
                self.ui
                    .ctx()
                    .data_mut(|d| d.insert_temp::<Vec<String>>(key, order.clone()));
                order
            }
            None => {
                self.ui
                    .ctx()
                    .data_mut(|d| d.insert_temp::<Vec<String>>(key, defaults.clone()));
                defaults
            }
        };
        // Cache for `.section()` so it can place the gap at the
        // right slot index without recomputing.
        self.base_order_this_frame = base_order.clone();
        base_order
    }

    /// Accent colour in use for this pane.
    pub fn accent(&self) -> egui::Color32 {
        self.accent
    }

    /// Read-only [`egui::Context`] access for callers that need
    /// pointer / input state while building pane content.
    pub fn ctx(&self) -> &egui::Context {
        self.ui.ctx()
    }

    /// Drive the drag-reorder state machine, paint the ghost line,
    /// commit a drop if released, and run the auto-fold pass when
    /// the stack overshoots the pane body.
    fn finalize(self) {
        let PaneBuilder {
            ui,
            accent,
            pane_id,
            body_rect,
            rendered,
            non_dragged_count,
            mut drag,
            drag_started_id,
            base_order_this_frame: _,
            cached_rects,
        } = self;

        // Promote the drag-started latch into persistent state.
        if let Some(id) = drag_started_id {
            drag.item = Some(id);
            drag.cursor = ui.ctx().pointer_hover_pos();
        }

        // While a drag is active, track the cursor + check for
        // pointer release directly off `ctx.input`. We can't rely on
        // the dragged section's response: that section isn't
        // rendered during the drag (it's been "lifted out"), so
        // there's no widget for egui to fire `dragged()` /
        // `drag_stopped()` on.
        if drag.item.is_some() {
            if let Some(p) = ui.ctx().pointer_hover_pos() {
                drag.cursor = Some(p);
            }
        }
        let drag_stopped = drag.item.is_some()
            && ui.ctx().input(|i| i.pointer.any_released());

        // End-case ghost gap: if the cursor's target slot is past
        // every rendered section, paint the ghost AFTER them. The
        // pre-section path in `.section()` covers all in-between
        // cases; this handles "drop at the bottom".
        if let (Some(dragged_id), Some(cursor)) = (drag.item.as_deref(), drag.cursor) {
            let target = compute_target_among_others(&cached_rects, dragged_id, cursor.y);
            if target == non_dragged_count {
                paint_ghost_gap(ui, &cached_rects, dragged_id, accent);
            }
        }

        // Floating cursor preview — a faded glass card the size of
        // the lifted section, following the cursor at `Order::Tooltip`.
        if let (Some(dragged_id), Some(cursor)) = (drag.item.as_deref(), drag.cursor) {
            paint_drag_preview(ui.ctx(), pane_id, &cached_rects, dragged_id, cursor, accent);
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }

        // Drop committed — reorder the stored sequence into the
        // non-dragged-index space and clear drag state.
        if drag_stopped {
            if let (Some(dragged_id), Some(cursor)) = (drag.item.clone(), drag.cursor) {
                let target_idx = compute_target_among_others(&cached_rects, &dragged_id, cursor.y);
                let key = order_key(pane_id);
                let mut order: Vec<String> =
                    ui.ctx().data(|d| d.get_temp(key)).unwrap_or_default();
                // Build the non-dragged sequence and insert the
                // dragged id at the target slot.
                let mut new_order: Vec<String> =
                    order.iter().filter(|id| **id != dragged_id).cloned().collect();
                let clamped = target_idx.min(new_order.len());
                new_order.insert(clamped, dragged_id);
                order = new_order;
                ui.ctx().data_mut(|d| d.insert_temp::<Vec<String>>(key, order));
            }
            drag = SectionDragState::default();
            ui.ctx().request_repaint();
        }

        ui.ctx()
            .data_mut(|d| d.insert_temp::<SectionDragState>(drag_key(pane_id), drag.clone()));

        // Cache this frame's rendered sections so the next frame's
        // drag pass can read sizes / titles. Merge the lifted
        // section's previous-frame entry back in so we still know
        // how big the floating preview should be next frame.
        let mut cache: RectCache = rendered
            .iter()
            .map(|r| CachedSection {
                id: r.id_salt.clone(),
                rect: r.outer_rect,
                title: r.title.clone(),
            })
            .collect();
        if let Some(dragged_id) = drag.item.as_deref() {
            if !cache.iter().any(|cs| cs.id == dragged_id) {
                if let Some(prev) = cached_rects
                    .iter()
                    .find(|cs| cs.id == dragged_id)
                    .cloned()
                {
                    cache.push(prev);
                }
            }
        }
        ui.ctx()
            .data_mut(|d| d.insert_temp::<RectCache>(rects_key(pane_id), cache));

        // Auto-fold: if the rendered stack overshoots the pane body,
        // close the topmost open section that the user isn't
        // actively dragging. One per frame, converges naturally.
        let stack_bottom = rendered
            .iter()
            .map(|r| r.outer_rect.bottom())
            .fold(body_rect.top(), f32::max);
        if stack_bottom <= body_rect.bottom() + 0.5 {
            return;
        }
        for r in &rendered {
            if drag.item.as_deref() == Some(r.id_salt.as_str()) {
                continue;
            }
            let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                r.state_id,
                false,
            );
            if state.is_open() {
                state.set_open(false);
                state.store(ui.ctx());
                ui.ctx().request_repaint();
                break;
            }
        }
    }
}

fn order_key(pane_id: egui::Id) -> egui::Id {
    pane_id.with("frost_pane_section_order")
}
fn drag_key(pane_id: egui::Id) -> egui::Id {
    pane_id.with("frost_pane_section_drag")
}
fn rects_key(pane_id: egui::Id) -> egui::Id {
    pane_id.with("frost_pane_section_rects")
}

/// Cached `(id, rect, title)` snapshot of last-frame's section
/// layout — drives target-slot computation, the ghost gap's size,
/// and the floating cursor preview's size + label during a drag.
#[derive(Clone)]
struct CachedSection {
    id: String,
    rect: egui::Rect,
    title: String,
}
type RectCache = Vec<CachedSection>;

/// Pick the target gap-index for a drag, walking the cache in
/// display order, SKIPPING the dragged section, and returning the
/// first slot whose centre Y is below the cursor. Indices are in
/// the non-dragged-only space (0 = above all others, N = below all
/// others, where N is the number of non-dragged sections).
fn compute_target_among_others(cache: &RectCache, dragged: &str, cursor_y: f32) -> usize {
    let mut idx = 0;
    for cs in cache {
        if cs.id == dragged {
            continue;
        }
        if cursor_y < cs.rect.center().y {
            return idx;
        }
        idx += 1;
    }
    idx
}

/// Paint a same-sized ghost rect — the visual placeholder the user
/// sees opening up at the drop target. Same recipe as the ribbon's
/// drop-target outline (accent fill at α 28, 1.5 px accent stroke,
/// `radius::MD` corner) so the two drag UIs feel like one family.
/// Allocates the rect so layout flows around it; height comes from
/// the dragged section's cached size.
fn paint_ghost_gap(
    ui: &mut egui::Ui,
    cache: &RectCache,
    dragged: &str,
    accent: egui::Color32,
) {
    let h = cache
        .iter()
        .find(|cs| cs.id == dragged)
        .map(|cs| cs.rect.height())
        .unwrap_or(48.0);
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(crate::style::radius::MD),
        egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 28),
        egui::Stroke::new(1.5, accent),
        egui::StrokeKind::Inside,
    );
}

/// Paint the dragged section's floating preview at the cursor: a
/// faded glass card sized to match the lifted section, with its
/// title centred at the top so the user sees what they're holding.
/// Drawn in an `egui::Area` at `Order::Tooltip` (independent of the
/// pane window's paint layer — no glass blending issues) with
/// `multiply_opacity` to fade the whole thing.
fn paint_drag_preview(
    ctx: &egui::Context,
    pane_id: egui::Id,
    cache: &RectCache,
    dragged: &str,
    cursor: egui::Pos2,
    accent: egui::Color32,
) {
    let Some(cs) = cache.iter().find(|cs| cs.id == dragged) else {
        return;
    };
    let size = cs.rect.size();
    let pos = egui::pos2(cursor.x - size.x * 0.5, cursor.y - size.y * 0.5);
    let area_id = pane_id.with(("frost_pane_drag_preview", dragged));
    egui::Area::new(area_id)
        .order(egui::Order::Tooltip)
        .fixed_pos(pos)
        .interactable(false)
        .show(ctx, |ui| {
            ui.set_max_width(size.x);
            ui.multiply_opacity(0.5);
            egui::Frame::new()
                .fill(crate::style::glass_fill(
                    crate::style::BG_2_RAISED,
                    accent,
                    crate::style::glass_alpha_card(),
                ))
                .corner_radius(egui::CornerRadius::same(crate::style::radius::MD))
                .stroke(egui::Stroke::new(1.0, crate::style::widget_border(accent)))
                .show(ui, |ui| {
                    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                    let title_widget = egui::WidgetText::from(crate::style::section_caps(
                        &cs.title,
                        accent,
                    ));
                    let galley = title_widget.into_galley(
                        ui,
                        Some(egui::TextWrapMode::Extend),
                        size.x,
                        egui::TextStyle::Body,
                    );
                    let pos = egui::pos2(rect.left() + 18.0, rect.top() + 11.0);
                    ui.painter().galley(pos, galley, accent);
                });
        });
}

/// Paint a floating panel anchored to `anchor`. `size.x` / `size.y`
/// are the *initial* dimensions; once the user drags a resize
/// handle the new values are stored per-panel-id in
/// [`egui::Context::data`] and used on subsequent frames.
///
/// Title alignment flips automatically on right-side anchors so a
/// menu dragged across rails reads correctly, and the horizontal
/// resize handle follows to the opposite side. The vertical handle
/// follows the same anchor-opposite rule.
pub fn floating_window(
    ctx: &egui::Context,
    id: &'static str,
    title: &str,
    anchor: egui::Align2,
    size: egui::Vec2,
    open: &mut bool,
    accent: egui::Color32,
    add_contents: impl FnOnce(&mut PaneBuilder),
) {
    let on_right_side = matches!(
        anchor,
        egui::Align2::RIGHT_TOP | egui::Align2::RIGHT_BOTTOM
    );
    let scope = egui::Id::new(if on_right_side {
        "frost_panel_width_right"
    } else {
        "frost_panel_width_left"
    });
    floating_window_scoped(ctx, id, title, anchor, size, open, accent, scope, add_contents)
}

/// Same as [`floating_window`] but the dim-storage key is supplied
/// by the caller. Use this when you want independent widths /
/// heights for panels that *share* an anchor side — e.g. a
/// TwoSided ribbon's Start and End clusters both anchored to
/// `LEFT_*` but each with its own memory.
pub fn floating_window_scoped(
    ctx: &egui::Context,
    id: &'static str,
    title: &str,
    anchor: egui::Align2,
    size: egui::Vec2,
    _open: &mut bool,
    accent: egui::Color32,
    width_scope: egui::Id,
    add_contents: impl FnOnce(&mut PaneBuilder),
) {
    let on_right_side = matches!(
        anchor,
        egui::Align2::RIGHT_TOP | egui::Align2::RIGHT_CENTER | egui::Align2::RIGHT_BOTTOM
    );
    // "Bottom-anchored" — panel grows upward from the bottom edge,
    // so its vertical-resize handle lives on its TOP edge (the edge
    // facing *away* from the anchor, same logic as the horizontal
    // handle).
    let bottom_anchored = matches!(
        anchor,
        egui::Align2::LEFT_BOTTOM
            | egui::Align2::CENTER_BOTTOM
            | egui::Align2::RIGHT_BOTTOM
    );

    let width_id = width_scope;
    let height_id = width_scope.with("_height");

    // Load stored values. Clamp to the current content_rect so
    // shrinking the Bevy window never leaves the panel wider / taller
    // than the visible area.
    let screen = ctx.content_rect();
    let side_inset = EDGE_GAP + SIDE_BTN_SIZE + RAIL_PANEL_GAP;
    let max_allowed_w = (screen.width() - side_inset - EDGE_GAP)
        .clamp(MIN_PANEL_W, MAX_PANEL_W);
    let max_allowed_h = (screen.height() - 2.0 * EDGE_GAP)
        .clamp(MIN_PANEL_H, MAX_PANEL_H);

    let stored_width: f32 = ctx
        .data(|d| d.get_temp::<f32>(width_id))
        .unwrap_or(size.x)
        .clamp(MIN_PANEL_W, max_allowed_w);
    let stored_height: f32 = ctx
        .data(|d| d.get_temp::<f32>(height_id))
        .unwrap_or(size.y)
        .clamp(MIN_PANEL_H, max_allowed_h);

    // Write the clamped values back so a shrunken Bevy window
    // permanently shrinks the stored values (user's drag wasn't
    // wasted, but it no longer exceeds the visible area).
    ctx.data_mut(|d| {
        d.insert_temp::<f32>(width_id, stored_width);
        d.insert_temp::<f32>(height_id, stored_height);
    });

    // Handle every anchor that a ribbon cluster might hand us —
    // corners AND the three `*_CENTER` variants used by `Middle`
    // clusters. Centre anchors keep the non-anchored axis at 0 so
    // egui centres the panel on that axis.
    let anchor_offset = match anchor {
        egui::Align2::LEFT_TOP => egui::vec2(side_inset, EDGE_GAP),
        egui::Align2::LEFT_CENTER => egui::vec2(side_inset, 0.0),
        egui::Align2::LEFT_BOTTOM => egui::vec2(side_inset, -EDGE_GAP),
        egui::Align2::RIGHT_TOP => egui::vec2(-side_inset, EDGE_GAP),
        egui::Align2::RIGHT_CENTER => egui::vec2(-side_inset, 0.0),
        egui::Align2::RIGHT_BOTTOM => egui::vec2(-side_inset, -EDGE_GAP),
        egui::Align2::CENTER_TOP => egui::vec2(0.0, side_inset),
        egui::Align2::CENTER_BOTTOM => egui::vec2(0.0, -side_inset),
        _ => egui::vec2(side_inset, EDGE_GAP),
    };

    let frame = egui::Frame {
        inner_margin: egui::Margin { left: 2, right: 2, top: 2, bottom: 2 },
        outer_margin: egui::Margin::ZERO,
        fill: glass_fill(BG_1_PANEL, accent, glass_alpha_window()),
        stroke: egui::Stroke::new(1.0, BORDER_SUBTLE),
        corner_radius: egui::CornerRadius::same(8),
        shadow: egui::epaint::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: egui::Color32::from_black_alpha(115),
        },
    };

    // Both dimensions are pinned — `min_size == max_size`. That lets
    // the vertical handle actually do something (before this, height
    // was content-driven and nothing a vertical drag could change).
    let pinned_size = egui::vec2(stored_width, stored_height);
    let inner = egui::Window::new(title)
        .id(egui::Id::new(id))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(anchor, anchor_offset)
        .min_size(pinned_size)
        .max_size(pinned_size)
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_max_width(stored_width - 6.0);

            const TITLE_INSET: f32 = 8.0;
            let title_size = 15.0 * 1.15;
            let title_h = 25.0;
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), title_h),
                egui::Sense::hover(),
            );
            let (align, tx) = if on_right_side {
                (egui::Align2::RIGHT_CENTER, rect.max.x - TITLE_INSET)
            } else {
                (egui::Align2::LEFT_CENTER, rect.min.x + TITLE_INSET)
            };
            let pos = egui::pos2(tx, rect.center().y);
            let font = egui::FontId::new(title_size, egui::FontFamily::Proportional);
            for dx in [-0.5, 0.5] {
                ui.painter().text(
                    egui::pos2(pos.x + dx, pos.y),
                    align,
                    title.to_uppercase(),
                    font.clone(),
                    accent,
                );
            }
            ui.painter().hline(
                rect.min.x..=rect.max.x,
                rect.max.y + 3.0,
                egui::Stroke::new(1.0, BORDER_SUBTLE),
            );
            ui.add_space(6.0);

            // Wrap the caller's closure with `PaneBuilder` so widgets
            // can only be added via `.section(...)`. After the
            // closure returns, `finalize` runs the drag-reorder
            // book-keeping (paints the ghost, commits drops) and
            // the auto-fold pass.
            let pane_id = egui::Id::new(id);
            let body_top = ui.cursor().min.y;
            let body_left = ui.cursor().min.x;
            let body_w = ui.available_width();
            let body_h = (ui.max_rect().bottom() - body_top).max(0.0);
            let body_rect = egui::Rect::from_min_size(
                egui::pos2(body_left, body_top),
                egui::vec2(body_w, body_h),
            );
            let drag: SectionDragState = ctx
                .data(|d| d.get_temp::<SectionDragState>(drag_key(pane_id)))
                .unwrap_or_default();
            let cached_rects: RectCache = ctx
                .data(|d| d.get_temp::<RectCache>(rects_key(pane_id)))
                .unwrap_or_default();
            let mut pane = PaneBuilder {
                ui,
                accent,
                pane_id,
                body_rect,
                rendered: Vec::new(),
                non_dragged_count: 0,
                drag,
                drag_started_id: None,
                base_order_this_frame: Vec::new(),
                cached_rects,
            };
            add_contents(&mut pane);
            pane.finalize();
        });

    let Some(inner) = inner else { return };
    let win_rect = inner.response.rect;

    // ── Horizontal resize handle (scene-facing edge) ──────────────
    let h_handle_rect = if on_right_side {
        egui::Rect::from_min_size(
            egui::pos2(win_rect.min.x - RESIZE_HANDLE_W * 0.5, win_rect.min.y),
            egui::vec2(RESIZE_HANDLE_W, win_rect.height()),
        )
    } else {
        egui::Rect::from_min_size(
            egui::pos2(win_rect.max.x - RESIZE_HANDLE_W * 0.5, win_rect.min.y),
            egui::vec2(RESIZE_HANDLE_W, win_rect.height()),
        )
    };
    let h_area_id = width_id.with("resize_handle_w");
    egui::Area::new(h_area_id)
        .order(egui::Order::Foreground)
        .fixed_pos(h_handle_rect.min)
        .show(ctx, |ui| {
            let (rect, resp) = ui.allocate_exact_size(h_handle_rect.size(), egui::Sense::drag());
            let hot = resp.hovered() || resp.dragged();
            if hot {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::same(2),
                    egui::Color32::from_rgba_unmultiplied(
                        accent.r(),
                        accent.g(),
                        accent.b(),
                        if resp.dragged() { 120 } else { 70 },
                    ),
                );
            }
            if resp.dragged() {
                let dx = resp.drag_delta().x;
                // Right-anchored panels grow LEFT-ward, so negative
                // dx ADDS width there.
                let new_w = if on_right_side {
                    stored_width - dx
                } else {
                    stored_width + dx
                };
                let clamped = new_w.clamp(MIN_PANEL_W, max_allowed_w);
                ctx.data_mut(|d| d.insert_temp::<f32>(width_id, clamped));
            }
        });

    // ── Vertical resize handle (anchor-opposite edge) ─────────────
    //
    // Bottom edge for TOP/CENTER-anchored panels; top edge for
    // BOTTOM-anchored ones — the edge that a drag "pulls" along the
    // panel's growth direction.
    let v_handle_rect = if bottom_anchored {
        egui::Rect::from_min_size(
            egui::pos2(win_rect.min.x, win_rect.min.y - RESIZE_HANDLE_H * 0.5),
            egui::vec2(win_rect.width(), RESIZE_HANDLE_H),
        )
    } else {
        egui::Rect::from_min_size(
            egui::pos2(win_rect.min.x, win_rect.max.y - RESIZE_HANDLE_H * 0.5),
            egui::vec2(win_rect.width(), RESIZE_HANDLE_H),
        )
    };
    let v_area_id = width_id.with("resize_handle_h");
    egui::Area::new(v_area_id)
        .order(egui::Order::Foreground)
        .fixed_pos(v_handle_rect.min)
        .show(ctx, |ui| {
            let (rect, resp) = ui.allocate_exact_size(v_handle_rect.size(), egui::Sense::drag());
            let hot = resp.hovered() || resp.dragged();
            if hot {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical);
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::same(2),
                    egui::Color32::from_rgba_unmultiplied(
                        accent.r(),
                        accent.g(),
                        accent.b(),
                        if resp.dragged() { 120 } else { 70 },
                    ),
                );
            }
            if resp.dragged() {
                let dy = resp.drag_delta().y;
                // Bottom-anchored grows UP; drag up (negative dy) ADDS
                // height. Top-anchored grows DOWN; drag down (positive
                // dy) ADDS height.
                let new_h = if bottom_anchored {
                    stored_height - dy
                } else {
                    stored_height + dy
                };
                let clamped = new_h.clamp(MIN_PANEL_H, max_allowed_h);
                ctx.data_mut(|d| d.insert_temp::<f32>(height_id, clamped));
            }
        });
}
