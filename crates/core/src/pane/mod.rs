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

pub use anchor::{PaneAnchor, RailZone};

use egui::{Color32, Id, Stroke, Vec2, vec2};

use crate::flex::{item, Flex, FlexAlign};
use crate::style;

// ─── Sizing constants ──────────────────────────────────────────────

/// Pane size for vertical rails (LeftRail / RightRail).
pub const PANE_VERTICAL_SIZE: Vec2 = vec2(280.0, 320.0);
/// Pane size for horizontal rails (TopRail / BottomRail).
pub const PANE_HORIZONTAL_SIZE: Vec2 = vec2(560.0, 220.0);

/// Thickness of the title strip on its main axis (perpendicular to
/// the strip's reading direction).
pub const TITLE_STRIP_THICKNESS: f32 = 25.0;

/// Inset from each screen edge: `EDGE_GAP + SIDE_BTN_SIZE +
/// RAIL_PANEL_GAP`. The pane sits 4 px past the rail's button
/// strip on top/left edges; bottom/right add a wider `far` inset
/// for anchors whose far edge meets a perpendicular rail's button
/// (see [`anchor::far_flags`]).
pub const RAIL_INSET: f32 = crate::ribbon::EDGE_GAP
    + crate::ribbon::SIDE_BTN_SIZE
    + RAIL_PANEL_GAP;

/// Visual gap between the ribbon's button strip and the pane edge.
const RAIL_PANEL_GAP: f32 = 4.0;

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
    pub fn show(self, ctx: &egui::Context, body: impl FnOnce(&mut egui::Ui)) {
        let pane_size = if self.anchor.is_vertical_pane() {
            PANE_VERTICAL_SIZE
        } else {
            PANE_HORIZONTAL_SIZE
        };
        // `content_rect` excludes any reserved areas (statusbar /
        // menubar / docked panels). Matches what
        // `floating::floating_window_scoped` reads, so panes land
        // exactly where ribbon buttons expect.
        let screen = ctx.content_rect();
        let pos = layout::compute_pane_pos(self.anchor, screen, pane_size);

        let area_id = self.id.with("pane2_area");
        egui::Area::new(area_id)
            // `Order::Middle` (not Foreground) — same layer egui's
            // ribbon buttons live on, so the pane and buttons share
            // a stacking context and don't visually pop above each
            // other when adjacent.
            .order(egui::Order::Middle)
            .fixed_pos(pos)
            .show(ctx, |ui| {
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
                    ui.set_min_size(pane_size - vec2(4.0, 4.0));
                    ui.set_max_size(pane_size - vec2(4.0, 4.0));
                    self.lay_out_flex(ui, body);
                });
            });
    }

    /// Inner flex layout: split the pane Ui into a fixed-size title
    /// strip + a `grow(1.0)` body. Direction comes from `title_side`
    /// (per-anchor) — horizontal strips need a vertical flex,
    /// vertical strips need a horizontal flex.
    fn lay_out_flex(self, ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
        let Pane2 {
            id,
            title,
            anchor,
            accent,
        } = self;
        let title_side = anchor.title_side();

        let pane_size = if anchor.is_vertical_pane() {
            PANE_VERTICAL_SIZE
        } else {
            PANE_HORIZONTAL_SIZE
        };
        let inner = pane_size - vec2(4.0, 4.0);

        let (flex, title_size, body_size) = if title_side.is_horizontal_strip() {
            // Title bar runs HORIZONTALLY. Flex direction = column.
            // Title is `inner.x × 25`; body claims the remaining
            // height.
            (
                Flex::vertical(),
                vec2(inner.x, TITLE_STRIP_THICKNESS),
                vec2(inner.x, inner.y - TITLE_STRIP_THICKNESS),
            )
        } else {
            // Title strip runs VERTICALLY. Flex direction = row.
            (
                Flex::horizontal(),
                vec2(TITLE_STRIP_THICKNESS, inner.y),
                vec2(inner.x - TITLE_STRIP_THICKNESS, inner.y),
            )
        };
        let title_at_end = title_side.is_at_end();

        flex.gap(Vec2::ZERO)
            .align_items(FlexAlign::Stretch)
            .size(inner)
            .id_salt(id.with("pane2_flex"))
            .show(ui, |flex| {
                let title_text = title.clone();
                let title_paint = move |ui: &mut egui::Ui| {
                    // Allocate the EXACT expected strip size — not
                    // `ui.available_size_before_wrap()`. Using
                    // available_size is wrong because flex makes
                    // multiple measurement passes (intrinsic-size
                    // first, then final layout). On the intrinsic
                    // pass `available_size` is the whole pane
                    // interior, so we'd paint stripes across the
                    // entire pane before the final pass overpaints
                    // the correct strip — visible as collapsed
                    // 25×25 paints elsewhere or a giant stripe
                    // bleed in the user's screenshots.
                    let (alloc_rect, _) =
                        ui.allocate_exact_size(title_size, egui::Sense::hover());
                    title::paint_pane_title(
                        ui,
                        alloc_rect,
                        id,
                        &title_text,
                        anchor,
                        accent,
                    );
                };
                let body_paint = move |ui: &mut egui::Ui| {
                    // Same recipe — allocate the precise body size
                    // computed from inner minus the strip.
                    let (_alloc_rect, _) =
                        ui.allocate_exact_size(body_size, egui::Sense::hover());
                    body(ui);
                };

                if title_at_end {
                    flex.add_ui(item().grow(1.0).min_size(body_size), body_paint);
                    flex.add_ui(
                        item().basis(TITLE_STRIP_THICKNESS).min_size(title_size),
                        title_paint,
                    );
                } else {
                    flex.add_ui(
                        item().basis(TITLE_STRIP_THICKNESS).min_size(title_size),
                        title_paint,
                    );
                    flex.add_ui(item().grow(1.0).min_size(body_size), body_paint);
                }
            });
    }
}
