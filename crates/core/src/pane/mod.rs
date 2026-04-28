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
mod layout;
mod title;

pub use anchor::{PaneAnchor, RailZone, TitleSide};

use egui::{vec2, Align, Color32, Id, Layout, Sense, Stroke};

use crate::style;

// ─── Sizing constants ──────────────────────────────────────────────

/// Cross-axis OUTER size for the pane = `CONTAINER_DEFAULT_*` +
/// `PANE_FRAME_CHROME`. The pane's frame inner_margin steals 2 px on
/// each side (4 total per axis), so pane inner cross = 280, which
/// matches the container's outer cross — container fits exactly.
pub const VERTICAL_PANE_X: f32 = 284.0;
pub const VERTICAL_PANE_Y: f32 = 284.0;
pub const HORIZONTAL_PANE_X: f32 = 284.0;
pub const HORIZONTAL_PANE_Y: f32 = 284.0;

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
const CONTAINER_OUTER_MARGIN_TOTAL: f32 = 6.0; // 3 px each side
/// Pane frame's `inner_margin` (2 px each side, total 4 per axis).
const PANE_FRAME_CHROME: f32 = 4.0;

/// Compute the pane's animated openness 0..=1 for `pane_id`. Both
/// `Pane2` and `Normal` call this with the same id so they lerp in
/// lockstep and the pane size is known in-frame (no anchor drift).
pub fn body_openness(ctx: &egui::Context, pane_id: Id) -> f32 {
    let open: bool = ctx
        .data_mut(|d| *d.get_persisted_mut_or_insert_with(pane_id.with("body_open"), || true));
    ctx.animate_bool_with_time(pane_id.with("body_open").with("anim"), open, BODY_ANIMATION_TIME)
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
        let cross_outer = if horizontal_strip {
            if self.anchor.is_vertical_pane() { VERTICAL_PANE_X } else { HORIZONTAL_PANE_X }
        } else {
            if self.anchor.is_vertical_pane() { VERTICAL_PANE_Y } else { HORIZONTAL_PANE_Y }
        };

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
        // uses section_padding 4×3, GAME uses 6×8). Compute from
        // the active theme so the pane main lerp matches the
        // container's actual rendered size on both axes.
        let pad = style::section_padding();
        let container_pad_main = if horizontal_strip {
            (pad.top as f32) + (pad.bottom as f32)
        } else {
            (pad.left as f32) + (pad.right as f32)
        };
        let body_main_collapsed = CONTAINER_TITLE_THICKNESS
            + container_pad_main
            + CONTAINER_OUTER_MARGIN_TOTAL;
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
                let layout = if horizontal_strip {
                    egui::Layout::top_down(egui::Align::Min)
                } else {
                    egui::Layout::left_to_right(egui::Align::Min)
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
                    inner_margin: egui::Margin::same(2),
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
        let cross = if horizontal_strip {
            // Title runs along X → cross axis is X (width).
            if anchor.is_vertical_pane() {
                VERTICAL_PANE_X
            } else {
                HORIZONTAL_PANE_X
            }
        } else {
            // Title runs along Y → cross axis is Y (height).
            if anchor.is_vertical_pane() {
                VERTICAL_PANE_Y
            } else {
                HORIZONTAL_PANE_Y
            }
        };
        // The pane's outer Frame has a 2px inner_margin on every side
        // (4 total per axis) — subtract so the flex's locked dimension
        // matches the desired outer pane size.
        let cross_inner = cross - 4.0;

        let title_size = if horizontal_strip {
            vec2(cross_inner, TITLE_STRIP_THICKNESS)
        } else {
            vec2(TITLE_STRIP_THICKNESS, cross_inner)
        };
        let title_at_end = title_side.is_at_end();

        // Plain-egui layout (no flex). Cross axis is locked via
        // `set_max_*` so `ui.available_*` is stable for child
        // widgets; main axis is content-driven by `body(ui)`. Title
        // strip and body are placed in the natural reading order
        // dictated by `title_at_end`.
        let title_text = title.clone();
        let paint_title_strip = |ui: &mut egui::Ui| {
            let (alloc_rect, _) =
                ui.allocate_exact_size(title_size, Sense::hover());
            title::paint_pane_title(ui, alloc_rect, id, &title_text, anchor, accent);
        };

        if horizontal_strip {
            ui.set_max_width(cross_inner);
            ui.vertical(|ui| {
                // Zero `item_spacing` — egui defaults to ~3 px
                // vertical / ~8 px horizontal between widgets, which
                // would push our title strip + container past
                // `pane_rect` (the pane gets visibly clipped). The
                // pane chrome is layout-tight by design.
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                if title_at_end {
                    body(ui);
                    paint_title_strip(ui);
                } else {
                    paint_title_strip(ui);
                    body(ui);
                }
            });
        } else {
            ui.set_max_height(cross_inner);
            // `ui.horizontal` initialises height = `interact_size.y`
            // (~20 px) — too small for a vertical title strip. Use
            // `with_layout(Layout::left_to_right(Align::Min))` which
            // takes the full `available_size_before_wrap` instead.
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                if title_at_end {
                    body(ui);
                    paint_title_strip(ui);
                } else {
                    paint_title_strip(ui);
                    body(ui);
                }
            });
        }
    }
}
