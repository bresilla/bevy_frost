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

use bevy_egui::egui;

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

/// Builder handed to every [`floating_window`] / [`floating_window_scoped`]
/// body closure. Only exposes container-creating methods — callers
/// cannot reach the underlying `egui::Ui`, so it's impossible to
/// drop bare widgets directly on the pane. Every control in a pane
/// lives inside a [`section`](PaneBuilder::section) (or a nested
/// subsection inside that section's body).
///
/// Construction is crate-private; consumers only ever receive a
/// `&mut PaneBuilder` inside the pane's body closure.
pub struct PaneBuilder<'a> {
    ui: &'a mut egui::Ui,
    accent: egui::Color32,
}

impl<'a> PaneBuilder<'a> {
    /// Add a foldable container section to the pane. `id_salt`
    /// disambiguates the section's collapsed-state storage,
    /// `title` is the UPPERCASE accent header, `default_open`
    /// controls the initial expansion, and `body` receives a
    /// regular `&mut egui::Ui` — inside the section, any widget
    /// works as normal.
    ///
    /// This is the ONLY way to put content into a pane. See the
    /// module-level docs for the rationale.
    pub fn section(
        &mut self,
        id_salt: &str,
        title: &str,
        default_open: bool,
        body: impl FnOnce(&mut egui::Ui),
    ) {
        crate::widgets::section(self.ui, id_salt, title, self.accent, default_open, body);
    }

    /// Accent colour in use for this pane — handed to widgets that
    /// accept an `accent: egui::Color32` parameter. Exposed because
    /// almost every frost widget takes it and the caller normally
    /// has it in scope already, but this avoids re-threading it
    /// through every intermediate function.
    pub fn accent(&self) -> egui::Color32 {
        self.accent
    }

    /// Access to the underlying [`egui::Context`] for read-only
    /// queries (pointer state, keyboard input, …). Read-only by
    /// convention — painting at the pane level is still gated
    /// behind [`PaneBuilder::section`].
    pub fn ctx(&self) -> &egui::Context {
        self.ui.ctx()
    }
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
            // can only be added via `.section(...)`. See the type's
            // doc-comment for the rationale.
            let mut pane = PaneBuilder { ui, accent };
            add_contents(&mut pane);
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
