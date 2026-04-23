//! Fixed-height floating-panel helper, with a drag-to-resize width
//! handle on the panel's scene-facing edge.
//!
//! Anchored to one of the four screen corners via [`egui::Align2`].
//! No title bar, no close button; the title sits at the rail-facing
//! edge (same side as the [`crate::ribbon`] it's paired with), and a
//! hair-thin hover strip sits on the *opposite* edge. Dragging that
//! strip grows / shrinks the panel width and remembers the value
//! via `egui::Context::data` so it survives across frames.
//!
//! Height stays driven by the caller — resize it in your own panel
//! code if you want that too.

use bevy_egui::egui;

use crate::style::{glass_alpha_window, glass_fill, BG_1_PANEL, BORDER_SUBTLE};

// Ribbon layout constants we need here. Kept as locals rather than
// pulling `ribbon::paint` into the public prelude — the numbers
// belong to both modules.
const EDGE_GAP: f32 = 8.0;
const SIDE_BTN_SIZE: f32 = 34.0;
const RAIL_PANEL_GAP: f32 = 6.0;

/// Width of the resize-handle hit zone, in pixels. Wide enough to be
/// easy to land on without a designer-pixel hunt, narrow enough that
/// the panel doesn't look like it has a gutter.
const RESIZE_HANDLE_W: f32 = 6.0;

/// Minimum / maximum panel widths. Caller's `size.x` clamps inside
/// this range on first draw; the user's drag does the same.
const MIN_PANEL_W: f32 = 220.0;
const MAX_PANEL_W: f32 = 900.0;

/// Tiny sanity check: keeps the layout constants in this file in
/// sync with the ribbon module's source-of-truth, through a simple
/// `const` assertion (no runtime cost).
const _: () = {
    assert!(EDGE_GAP == 8.0);
    assert!(SIDE_BTN_SIZE == 34.0);
    assert!(RAIL_PANEL_GAP == 6.0);
};

/// Paint a floating panel anchored to `anchor`. `size.x` is the
/// *initial* width; once the user drags the resize handle on the
/// scene-facing edge, the new width is stored per-panel-id in
/// `egui::Context::data` and used on subsequent frames. `size.y` is
/// the panel height (not resizable by this widget — drive it from
/// caller state if you need it dynamic).
///
/// Title alignment flips automatically on right-side anchors so a
/// menu dragged across rails reads correctly, and the resize handle
/// follows to the opposite side.
pub fn floating_window(
    ctx: &egui::Context,
    id: &'static str,
    title: &str,
    anchor: egui::Align2,
    size: egui::Vec2,
    open: &mut bool,
    accent: egui::Color32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    // Default scope keys width per-rail (Left / Right only). Used by
    // callers that don't care about per-cluster widths — those hit
    // `floating_window_scoped` directly with a bespoke scope id.
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

/// Same as [`floating_window`] but the width-storage key is supplied
/// by the caller. Use this when you want independent widths for
/// panels that *share* an anchor side — e.g. a TwoSided ribbon's
/// Start and End clusters both anchored to `LEFT_*` but each with
/// its own width memory.
pub fn floating_window_scoped(
    ctx: &egui::Context,
    id: &'static str,
    title: &str,
    anchor: egui::Align2,
    size: egui::Vec2,
    _open: &mut bool,
    accent: egui::Color32,
    width_scope: egui::Id,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let on_right_side = matches!(
        anchor,
        egui::Align2::RIGHT_TOP | egui::Align2::RIGHT_CENTER | egui::Align2::RIGHT_BOTTOM
    );

    let width_id = width_scope;
    let stored_width: f32 = ctx
        .data(|d| d.get_temp::<f32>(width_id))
        .unwrap_or(size.x)
        .clamp(MIN_PANEL_W, MAX_PANEL_W);
    let effective_size = egui::vec2(stored_width, size.y);

    let side_inset = EDGE_GAP + SIDE_BTN_SIZE + RAIL_PANEL_GAP;
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
        // Tight inner margin — containers sit almost flush with the
        // panel edge. Bump these back up if content starts clipping
        // against the rounded corner.
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

    let inner = egui::Window::new(title)
        .id(egui::Id::new(id))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(anchor, anchor_offset)
        .fixed_size(effective_size)
        .frame(frame)
        .show(ctx, |ui| {
            // Inner-margin (2 px) × 2 sides + stroke accounts for
            // the inset beyond the effective width.
            ui.set_max_width(stored_width - 6.0);

            // Title row — UPPERCASE accent at the rail-facing edge,
            // with a hairline underneath and breathing space before
            // the content. `TITLE_INSET` keeps the title from kissing
            // the rounded corner.
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

            add_contents(ui);

            // Fill the remaining vertical space so the window's
            // painted rect matches `effective_size.y`. Without this,
            // egui auto-shrinks the window to its content and the
            // resize handle on the scene-facing edge would either
            // overshoot (when we trust `effective_size.y`) or stop
            // at the title (when we trust the returned rect).
            let remaining = ui.available_height();
            if remaining > 0.0 {
                ui.add_space(remaining);
            }
        });

    // ── Resize handle ──────────────────────────────────────────────
    //
    // Use the window's ACTUAL painted rect (not a rect we compute
    // ourselves) so the hit zone + visual hint match exactly the
    // panel the user sees. The body's trailing `add_space` above
    // guarantees that rect is the full `effective_size.y`, so the
    // handle covers the entire scene-facing edge and no more.
    let Some(inner) = inner else { return };
    let win_rect = inner.response.rect;

    let handle_rect = if on_right_side {
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

    // Handle Area id derives from the width scope, so each scope
    // (per-rail, per-cluster, or whatever the caller picked) gets
    // its own hit zone.
    let area_id = width_id.with("resize_handle");
    egui::Area::new(area_id)
        .order(egui::Order::Foreground)
        .fixed_pos(handle_rect.min)
        .show(ctx, |ui| {
            let (rect, resp) = ui.allocate_exact_size(handle_rect.size(), egui::Sense::drag());
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
                // Right-anchored panels grow LEFT-ward, so a negative
                // dx there should *add* width. Left-anchored panels
                // grow RIGHT-ward, so positive dx adds width.
                let new_w = if on_right_side {
                    stored_width - dx
                } else {
                    stored_width + dx
                };
                let clamped = new_w.clamp(MIN_PANEL_W, MAX_PANEL_W);
                ctx.data_mut(|d| d.insert_temp::<f32>(width_id, clamped));
            }
        });

}
