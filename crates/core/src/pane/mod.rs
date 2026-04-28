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

use crate::flex::{item, Flex, FlexAlign, Size};
use crate::style;

// ─── Sizing constants ──────────────────────────────────────────────

/// Cross-axis size for vertical-rail (Left/Right) panes — the
/// dimension that doesn't grow with body content. Width when the
/// title is horizontal (top/bottom of pane); height when the title
/// is vertical (left/right of pane).
pub const VERTICAL_PANE_X: f32 = 280.0;
pub const VERTICAL_PANE_Y: f32 = 320.0;
/// Cross-axis size for horizontal-rail (Top/Bottom) panes.
pub const HORIZONTAL_PANE_X: f32 = 560.0;
pub const HORIZONTAL_PANE_Y: f32 = 220.0;

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
        egui::Area::new(area_id)
            // `Order::Middle` (not Foreground) — same layer egui's
            // ribbon buttons live on, so the pane and buttons share
            // a stacking context.
            .order(egui::Order::Middle)
            // `anchor(align, offset)` pins the corresponding corner /
            // edge centre of the Area to a fixed screen position;
            // egui sizes the Area to its content, so as the body
            // grows the pinned corner stays put and the opposite
            // edge moves.
            .anchor(align, offset)
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
                    self.lay_out_flex(ui, body);
                });
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

        let mut flex = if horizontal_strip {
            Flex::vertical().width(Size::Points(cross_inner))
        } else {
            Flex::horizontal().height(Size::Points(cross_inner))
        };
        flex = flex
            .gap(Vec2::ZERO)
            .align_items(FlexAlign::Stretch)
            .id_salt(id.with("pane2_flex"));

        flex.show(ui, |flex| {
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
                    // No fixed allocation — let the user's body grow
                    // the pane along the main axis. If `body` is a
                    // no-op, the item collapses to 0 on the main
                    // axis and the pane shows JUST the title strip.
                    body(ui);
                };

                if title_at_end {
                    flex.add_ui(item(), body_paint);
                    flex.add_ui(
                        item().basis(TITLE_STRIP_THICKNESS).min_size(title_size),
                        title_paint,
                    );
                } else {
                    flex.add_ui(
                        item().basis(TITLE_STRIP_THICKNESS).min_size(title_size),
                        title_paint,
                    );
                    flex.add_ui(item(), body_paint);
                }
            });
    }
}
