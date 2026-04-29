//! Flex-based floating pane (Phase 1 of `PLAN_NEWUI.md`).
//!
//! A pane that paints a theme-aware **title strip** in any of 12
//! anchor positions (4 screen rails × 3 zones each) and reserves
//! the remainder for a body closure. Layout is delegated to
//! [`crate::flex`] (vendored `egui_flex`) so children always fit
//! and the pane never overflows.
//!
//! ## Submodule layout
//!
//! * [`anchor`] — `PaneAnchor`, `RailZone`, the per-anchor
//!   `title_side` / `title_reversed` / `is_middle` decisions, and
//!   the `far_flags` table that drives bottom/right inset choice.
//! * [`layout`] — `compute_pane_pos` (anchor → screen position).
//! * [`title`] — `paint_pane_title` (theme-aware strip painter).
//! * `mod.rs` (this file) — `Pane2` builder + render entry point.

mod anchor;
mod drag;
mod layout;
mod title;

pub use anchor::{PaneAnchor, RailZone, TitleSide};
pub use drag::{
    active_drag, begin_frame as begin_drag_frame, clear_drag, compute_target,
    current_cache, dragged_size, finalize_snapshot, paint_drag_preview,
    paint_ghost_gap_inline, push_rect, section_order_for, set_drag,
    set_section_order, snapshot, state as drag_state, DragState, RectEntry,
};

use egui::{vec2, Color32, Id, Sense, Stroke};

use crate::style;

// ─── Sizing constants ──────────────────────────────────────────────

/// Pane Frame's `inner_margin` per side. Used both literally (in the
/// `Frame { inner_margin: … }` builder) and to compute the inner
/// cross-axis available to the body via `cross - 2 * PANE_INNER_MARGIN`.
/// Keep these in sync — if you change the Frame margin, recompute the
/// available space.
const PANE_INNER_MARGIN: f32 = 2.0;
/// Total chrome (both sides) the pane Frame steals from the inner ui
/// — used in the body main-axis size lerp so the pane's outer height
/// includes the chrome both above and below the body.
const PANE_FRAME_CHROME: f32 = PANE_INNER_MARGIN * 2.0;

/// Pane outer cross-axis size. The pane is square in cross-axis
/// regardless of which rail it lives on, and the container inside
/// clamps its own cross to `outer_avail` so it always fits — no
/// per-orientation tuning needed.
pub const PANE_OUTER_CROSS: f32 = 280.0;

/// Thickness of the title strip on its main axis (perpendicular to
/// the strip's reading direction).
pub const TITLE_STRIP_THICKNESS: f32 = 25.0;

/// Animation duration for the body's open/close transition. Shared
/// between [`Pane2`] (for size animation) and
/// [`crate::container::Normal`] (for body content animation), so
/// both lerp at the same rate.
pub const BODY_ANIMATION_TIME: f32 = 0.18;

/// Container outer main-axis size when the body is fully expanded.
/// Equal to `crate::container::Normal::CONTAINER_DEFAULT_*`, so the
/// pane and container agree on the fully-open size.
pub const DEFAULT_BODY_MAIN_OPEN: f32 = 280.0;
/// Container's title-strip thickness and outer-margin reservation
/// — used to compute the collapsed body main size from the active
/// theme each frame (see `body_main_collapsed`). Themes differ in
/// `section_padding` (PRO 4×3, GAME 6×8) so a hardcoded constant
/// can't get this right for both.
const CONTAINER_TITLE_THICKNESS: f32 = 22.0;

/// Compute the pane's animated openness 0..=1 for `pane_id`. Both
/// `Pane2` and `Normal` call this with the same id so they lerp in
/// lockstep and the pane size is known in-frame (no anchor drift).
pub fn body_openness(ctx: &egui::Context, pane_id: Id) -> f32 {
    let open: bool = ctx
        .data_mut(|d| *d.get_persisted_mut_or_insert_with(pane_id.with("body_open"), || true));
    ctx.animate_bool_with_time(pane_id.with("body_open").with("anim"), open, BODY_ANIMATION_TIME)
}

/// Shared ctx-data key that points to the **currently active**
/// `Pane2`'s id. Pane2 writes this at the top of `show` so children
/// (e.g. `Normal`) can look up their parent pane's stagger state
/// without needing the pane id wired through their constructors.
/// Multiple panes' bodies run sequentially within a frame so the
/// pointer is well-defined while any one body callback runs.
pub fn active_pane_key() -> Id {
    Id::new("frost_active_pane_id")
}

/// Toggle the pane's body open state. Called from the container's
/// title-strip click handler.
pub fn toggle_body(ctx: &egui::Context, pane_id: Id) {
    let key = pane_id.with("body_open");
    ctx.data_mut(|d| {
        let cur: bool = d.get_persisted(key).unwrap_or(true);
        d.insert_persisted(key, !cur);
    });
}

/// Inset from each screen edge: `EDGE_GAP + SIDE_BTN_SIZE +
/// RAIL_PANEL_GAP`. The pane sits 4 px past the rail's button
/// strip on top/left edges; bottom/right add a wider `far` inset
/// for anchors whose far edge meets a perpendicular rail's button
/// (see [`anchor::far_flags`]).
pub const RAIL_INSET: f32 = crate::ribbon::EDGE_GAP
    + crate::ribbon::SIDE_BTN_SIZE
    + RAIL_PANEL_GAP;

/// Visual gap between the ribbon's button strip and the pane edge.
const RAIL_PANEL_GAP: f32 = 8.0;

// ─── Builder ───────────────────────────────────────────────────────

/// A single floating window keyed by `id` and pinned to one of 12
/// screen positions. Build with [`Pane2::new`], then call
/// [`Pane2::show`] each frame the pane should be visible.
pub struct Pane2 {
    id: Id,
    title: String,
    anchor: PaneAnchor,
    accent: Color32,
}

impl Pane2 {
    /// Construct a pane builder. `id` is used to scope the
    /// `egui::Area` and any title-strip animations.
    pub fn new(
        id: impl Into<Id>,
        title: impl Into<String>,
        anchor: PaneAnchor,
        accent: Color32,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            anchor,
            accent,
        }
    }

    /// Render the pane this frame. `body` runs after the title strip
    /// is laid out; its `Ui` covers the rest of the pane.
    ///
    /// Pane sizing is content-driven: an empty `body` collapses the
    /// pane to JUST the title strip thickness, and adding content
    /// extends the pane along the title's perpendicular axis (a
    /// horizontal title bar grows down with stacked containers; a
    /// vertical title strip grows right). The cross axis (the one
    /// the title spans) is fixed per anchor.
    pub fn show(self, ctx: &egui::Context, body: impl FnOnce(&mut egui::Ui)) {
        let (align, offset) = layout::anchor_align(self.anchor);
        let area_id = self.id.with("pane2_area");

        let title_side = self.anchor.title_side();
        let horizontal_strip = title_side.is_horizontal_strip();
        let cross_outer = PANE_OUTER_CROSS;

        // ── Per-pane staggered fade-in clock (port of frostcore's
        //    `PaneBuilder::pane_open_elapsed`) ──
        //
        // Tracks elapsed seconds since this pane became visible.
        // The `cumulative_pass_nr + 1 < frame_now` check detects
        // a paint gap (= the pane was hidden last frame, e.g.
        // user just clicked its ribbon button) and resets the
        // clock to 0. Stored under `self.id.with(...)` so each
        // pane has its own independent timer.
        let pane_open_elapsed: f32 = {
            let frame_key = self.id.with("frost_pane_anim_frame");
            let state_key = self.id.with("frost_pane_anim_elapsed");
            let frame_now = ctx.cumulative_pass_nr();
            let last_frame: u64 = ctx.data(|d| d.get_temp(frame_key)).unwrap_or(0);
            let mut elapsed: f32 = ctx.data(|d| d.get_temp(state_key)).unwrap_or(99.0);
            if last_frame + 1 < frame_now {
                elapsed = 0.0;
            }
            let dt = ctx.input(|i| i.unstable_dt).max(0.0);
            elapsed += dt;
            ctx.data_mut(|d| {
                d.insert_temp(state_key, elapsed);
                d.insert_temp(frame_key, frame_now);
            });
            // Repaint while any reasonably-staged section is still
            // animating in (~12 sections × 0.18 stagger + 0.45 fade
            // ≈ 2.6 s — keep some headroom).
            if elapsed < 3.0 {
                ctx.request_repaint();
            }
            elapsed
        };
        // Publish the active pane's id PLUS its current elapsed
        // and a fresh `section_idx = 0` counter under that id.
        // The active-pane pointer lives at a single global key so
        // `Normal::show` (whose own `pane_id` field is the
        // CONTAINER's id, not Pane2's) can find its parent pane.
        ctx.data_mut(|d| {
            d.insert_temp(active_pane_key(), self.id);
            d.insert_temp(self.id.with("frost_pane_open_elapsed"), pane_open_elapsed);
            d.insert_temp(self.id.with("frost_pane_section_idx"), 0u32);
        });

        // Compute pane main from the body's animation state in
        // THIS frame. Both `Pane2` and `Normal` call
        // `body_openness(ctx, pane_id)` with the same `pane_id`, so
        // egui returns the same value to both — meaning the pane's
        // size and the container's content are in lockstep, with
        // ZERO 1-frame lag. egui::Area's anchor math then uses this
        // `state.size` (we lock it via set_min/max_size) and the
        // anchored corner stays pixel-pinned during the animation.
        let openness = body_openness(ctx, self.id);
        // Container's collapsed outer size differs per theme (PRO
        // uses section_padding 4×3 + outer_margin 3, GAME uses 6×8 +
        // outer_margin 9 main / 1 cross). Compute from the active
        // theme so the pane main lerp matches the container's
        // actual rendered size on both axes.
        let theme_now = style::theme();
        let pad = style::section_padding();
        let container_pad_main = if horizontal_strip {
            (pad.top as f32) + (pad.bottom as f32)
        } else {
            (pad.left as f32) + (pad.right as f32)
        };
        let container_outer_main_total = (theme_now.section_outer_margin_main_title as f32)
            + (theme_now.section_outer_margin_main_body as f32);
        let body_main_collapsed = CONTAINER_TITLE_THICKNESS
            + container_pad_main
            + container_outer_main_total;
        let collapsed_main =
            TITLE_STRIP_THICKNESS + body_main_collapsed + PANE_FRAME_CHROME;
        let expanded_main =
            TITLE_STRIP_THICKNESS + DEFAULT_BODY_MAIN_OPEN + PANE_FRAME_CHROME;
        let pane_main =
            collapsed_main + (expanded_main - collapsed_main) * openness;

        let outer_size = if horizontal_strip {
            vec2(cross_outer, pane_main)
        } else {
            vec2(pane_main, cross_outer)
        };

        // Compute position MANUALLY from `outer_size` using the
        // anchor + offset + screen rect. egui's `Area::anchor()`
        // would use `state.size` from the previous frame, which
        // lags during animation by the per-frame size delta — that
        // was the visible drift on right/bottom-anchored panes.
        // With `fixed_pos`, position is computed in-frame from
        // our just-computed size, so the anchored corner is
        // pinned with ZERO lag.
        let screen = ctx.content_rect();
        let pane_pos = layout::compute_pane_pos(align, offset, screen, outer_size);


        egui::Area::new(area_id)
            // `Order::Background` keeps the pane's drop shadow
            // BELOW the ribbon buttons — buttons paint over any
            // shadow bleed. Removes the need for a tight clip_rect
            // (which was slicing the title strip on the rail-side
            // edge by a couple of pixels).
            .order(egui::Order::Background)
            .fixed_pos(pane_pos)
            .movable(false)
            .interactable(true)
            .fade_in(false)
            .default_size(outer_size)
            .show(ctx, |outer_ui| {
                // egui's Area constrains its content_ui.max_rect to
                // `state.size` from the PREVIOUS frame. During
                // animation that prev value is smaller than this
                // frame's `outer_size`, so anything that uses
                // `available_size` (e.g. `allocate_ui_with_layout`)
                // would clamp content too small and clip it.
                // Workaround: bypass `outer_ui` and create a child
                // with EXPLICIT `max_rect = pane_rect`, then
                // `allocate_rect` on the parent so its `min_rect`
                // reaches `pane_rect` and `state.size` (next frame)
                // matches our computed value.
                let pane_rect = egui::Rect::from_min_size(
                    outer_ui.cursor().min,
                    outer_size,
                );
                // Use the title-at-end layout DIRECTLY on the outer
                // child_ui — not via a `with_layout(bottom_up)` inside
                // a top_down parent. egui tracks `min_rect` by union
                // with the parent's initial cursor (top-left for
                // top_down). When the title strip lands at the far
                // edge (Bottom/Right rails) and the body folds to 0,
                // the allocated strip sits at the bottom/right of
                // `pane_rect`. Union-ed with the parent's top-left
                // cursor, the resulting min_rect spans the FULL pane
                // height/width — and the Frame paints across the
                // whole pane instead of shrinking. Pushing the
                // bottom_up/right_to_left layout one level up so the
                // child_ui's cursor starts at the anchor edge keeps
                // min_rect tight to the strip.
                let title_at_end = title_side.is_at_end();
                let layout = if horizontal_strip {
                    if title_at_end {
                        egui::Layout::bottom_up(egui::Align::Min)
                    } else {
                        egui::Layout::top_down(egui::Align::Min)
                    }
                } else {
                    if title_at_end {
                        egui::Layout::right_to_left(egui::Align::Min)
                    } else {
                        egui::Layout::left_to_right(egui::Align::Min)
                    }
                };
                let mut child_ui = outer_ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(pane_rect)
                        .layout(layout),
                );
                // No clip_rect — the pane is on `Order::Background`
                // (below the ribbon buttons), so any shadow bleed
                // is painted over by the buttons. Content is laid
                // out tight to `pane_rect` (item_spacing = 0), so
                // there's nothing to clip anyway.
                {
                let ui = &mut child_ui;
                let theme = style::theme();
                let fill = if theme.pane_fill_visible {
                    style::glass_fill(
                        style::pane_fill(self.accent),
                        self.accent,
                        style::glass_alpha_window(),
                    )
                } else {
                    Color32::TRANSPARENT
                };
                let shadow = egui::epaint::Shadow {
                    offset: [0, theme.pane_shadow_y],
                    blur: theme.pane_shadow_blur,
                    spread: 0,
                    color: Color32::from_black_alpha(115),
                };
                egui::Frame {
                    inner_margin: egui::Margin::same(PANE_INNER_MARGIN as i8),
                    outer_margin: egui::Margin::ZERO,
                    fill,
                    stroke: Stroke::new(
                        theme.border_width,
                        style::widget_border(self.accent),
                    ),
                    corner_radius: egui::CornerRadius::same(theme.radius_lg),
                    shadow,
                }
                .show(ui, |ui| {
                    self.lay_out_flex(ui, body);
                });
                }
                let _ = outer_ui.allocate_rect(pane_rect, egui::Sense::hover());
            });
    }

    /// Inner flex layout: split the pane Ui into a fixed-thickness
    /// title strip + a content-sized body. Direction comes from
    /// `title_side` (per-anchor) — horizontal strips need a vertical
    /// flex, vertical strips need a horizontal flex.
    ///
    /// Sizing is content-driven: the cross axis (the dimension the
    /// title spans) is locked per anchor, while the main axis (the
    /// dimension perpendicular to the title) is left free so the
    /// pane is exactly tall/wide enough to fit the title strip plus
    /// whatever the body closure allocates. Empty body → pane is
    /// just the strip.
    fn lay_out_flex(self, ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
        let Pane2 {
            id,
            title,
            anchor,
            accent,
        } = self;
        let title_side = anchor.title_side();
        let horizontal_strip = title_side.is_horizontal_strip();

        // Cross axis = the dimension the title strip spans. Locked.
        // Main axis = perpendicular; grows with body content.
        // Subtract the pane Frame's inner_margin (both sides) so the
        // child ui's max_width/height matches the area inside the
        // frame chrome — what's actually available to the body.
        let cross_inner = PANE_OUTER_CROSS - PANE_FRAME_CHROME;

        let title_size = if horizontal_strip {
            vec2(cross_inner, TITLE_STRIP_THICKNESS)
        } else {
            vec2(TITLE_STRIP_THICKNESS, cross_inner)
        };

        // Plain-egui layout (no flex). Cross axis is locked via
        // `set_max_*` so `ui.available_*` is stable for child
        // widgets; main axis is content-driven by `body(ui)`. Title
        // strip and body are placed in the natural reading order
        // dictated by `title_at_end` (decided in `Pane2::show` when
        // building the outer child_ui's layout).
        let title_text = title.clone();
        let paint_title_strip = |ui: &mut egui::Ui| {
            let (alloc_rect, _) =
                ui.allocate_exact_size(title_size, Sense::hover());
            title::paint_pane_title(ui, alloc_rect, id, &title_text, anchor, accent);
        };

        // The outer child_ui already carries the correct layout
        // (top_down / bottom_up / left_to_right / right_to_left)
        // chosen by `Pane2::show` so the cursor starts at the anchor
        // edge — see the comment there for why we *don't* rewrap in
        // a `with_layout(bottom_up)` here. We just clamp the cross
        // axis and zero the item-spacing.
        if horizontal_strip {
            ui.set_max_width(cross_inner);
        } else {
            ui.set_max_height(cross_inner);
        }
        // Zero `item_spacing` — egui defaults to ~3 px vertical / ~8
        // px horizontal between widgets, which would push our title
        // strip + container past `pane_rect`.
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        // SAME order in both directions: title FIRST (lands at the
        // anchor edge thanks to the layout direction), body SECOND
        // (fills outward). Reversed layouts handle visual placement
        // automatically — `bottom_up` puts first-allocated at the
        // BOTTOM, `right_to_left` at the RIGHT, etc.
        paint_title_strip(ui);
        // Reset per-frame drag bookkeeping (current cache + section
        // idx counter). Snapshot from prev frame stays available
        // for size lookups.
        drag::begin_frame(ui.ctx(), id);

        // Update cursor BEFORE body runs so `Normal::show`'s
        // target_idx computation sees this frame's cursor.
        let pre_body_drag = drag::state(ui.ctx(), id);
        if let (Some(item), Some(pos)) =
            (pre_body_drag.item, ui.ctx().pointer_interact_pos())
        {
            drag::set_drag(
                ui.ctx(),
                id,
                drag::DragState {
                    item: Some(item),
                    cursor: Some(pos),
                },
            );
        }

        body(ui);

        // Stack axis: matches `body` layout direction — BottomRail /
        // TopRail panes stack vertically (Y), LeftRail / RightRail
        // stack horizontally (X).
        let horizontal_stack = !title_side.is_horizontal_strip();

        // ── Trailing ghost gap ──
        //
        // If the cursor's target slot is AFTER the last rendered
        // container (target == total non-dragged), paint the ghost
        // gap inline at the end of the body layout. The inline gaps
        // inside `Normal::show` handle every other position.
        let drag_state = drag::state(ui.ctx(), id);
        if let Some(dragged_id) = drag_state.item {
            let snap = drag::snapshot(ui.ctx(), id);
            let total = drag::current_cache(ui.ctx(), id).len();
            let cursor = ui
                .ctx()
                .pointer_interact_pos()
                .or(drag_state.cursor);
            if let Some(c) = cursor {
                let cursor_axis = if horizontal_stack { c.x } else { c.y };
                let target_idx = drag::compute_target(
                    &snap,
                    dragged_id,
                    cursor_axis,
                    horizontal_stack,
                );
                if target_idx >= total {
                    if let Some(size) =
                        drag::dragged_size(&snap, dragged_id)
                    {
                        drag::paint_ghost_gap_inline(
                            ui,
                            size,
                            accent,
                            horizontal_stack,
                        );
                    }
                }
            }
        }

        // ── Build snapshot for next frame ──
        //
        // current cache (this frame's renders) + dragged entry
        // carried forward from prev snapshot.
        drag::finalize_snapshot(ui.ctx(), id);

        // ── Floating preview + cursor + release commit ──
        if let Some(dragged_id) = drag_state.item {
            let snap = drag::snapshot(ui.ctx(), id);
            let cursor = ui
                .ctx()
                .pointer_interact_pos()
                .or(drag_state.cursor);
            if let Some(c) = cursor {
                drag::paint_drag_preview(
                    ui.ctx(),
                    id,
                    &snap,
                    dragged_id,
                    c,
                    accent,
                );
                ui.ctx()
                    .set_cursor_icon(egui::CursorIcon::Grabbing);
            }

            if ui.ctx().input(|i| i.pointer.any_released()) {
                if let Some(c) = cursor {
                    let cursor_axis =
                        if horizontal_stack { c.x } else { c.y };
                    let target_idx = drag::compute_target(
                        &snap,
                        dragged_id,
                        cursor_axis,
                        horizontal_stack,
                    );
                    let defaults: Vec<Id> =
                        snap.iter().map(|e| e.id).collect();
                    let mut order = drag::section_order_for(
                        ui.ctx(),
                        id,
                        &defaults,
                    );
                    order.retain(|cid| *cid != dragged_id);
                    let clamped = target_idx.min(order.len());
                    order.insert(clamped, dragged_id);
                    drag::set_section_order(ui.ctx(), id, order);
                }
                drag::clear_drag(ui.ctx(), id);
            }
        }
    }
}
