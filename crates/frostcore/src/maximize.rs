//! Shared "maximise this widget to full window" wrapper.
//!
//! Graph canvases, code editors, and similar "canvas-shaped"
//! widgets benefit from a one-click lift to full window that
//! leaves their surrounding pane and container untouched. This
//! module provides exactly that, in a widget-agnostic form:
//!
//! ```ignore
//! maximizable(ui, "my_widget", accent, egui::vec2(w, 300.0), |ui| {
//!     // Render your widget into this inner `ui` — it's either
//!     // the inline rect the caller wanted, or a full-window
//!     // overlay depending on the maximise state.
//! });
//! ```
//!
//! The wrapper:
//!
//! * Stores a `bool` per `id_salt` in `egui::Context::data`.
//! * Not maximised: allocates a rect of `min_size` in the current
//!   `Ui` and renders the body inside a child `Ui` pinned to
//!   that rect.
//! * Maximised: paints a placeholder in the current `Ui` so the
//!   surrounding layout keeps its footprint, then renders the
//!   body inside an `egui::Area` at the highest order covering
//!   the full `ctx.content_rect()` with a frost glass frame.
//! * Paints a 24 px chip in the top-left of whichever rect holds
//!   the body — ribbon-button styling (accent fill on active,
//!   accent border), glyph = two diagonal arrows joined by a line
//!   (outward for maximise, inward for restore).
//!
//! The chip's Area runs at `Order::Tooltip` so it always paints
//! and intercepts clicks above the wrapped widget's shapes, even
//! when the widget (like `egui-snarl`) draws interactive content
//! across its entire rect.

use std::hash::Hash;

use egui;

use crate::style::{
    glass_alpha_window, glass_fill, BG_1_PANEL, BG_2_RAISED, BORDER_SUBTLE, TEXT_PRIMARY,
};

/// The egui data key that [`maximizable`] uses to store the
/// maximise-flag for a given `id_salt`. Exposed so callers can do
/// context-sensitive routing without poking inside the widget.
pub fn maximize_state_key(id_salt: impl std::hash::Hash) -> egui::Id {
    egui::Id::new(("frost_maximize", id_salt))
}

/// Returns `true` if the maximizable widget identified by
/// `id_salt` is currently in full-window mode. Use this from a
/// host's key handler to route Ctrl+K to a widget-specific
/// command palette when the widget is maximised, or to the
/// general palette otherwise.
pub fn is_maximized(ctx: &egui::Context, id_salt: impl std::hash::Hash) -> bool {
    ctx.data(|d| d.get_temp::<bool>(maximize_state_key(id_salt)))
        .unwrap_or(false)
}

/// Returns `true` if ANY maximizable widget is currently in
/// full-window mode. Useful when you only care about "should the
/// general palette behave differently" and don't need to know
/// which specific widget owns the screen.
pub fn is_any_maximized(ctx: &egui::Context) -> bool {
    let global_key = egui::Id::new("frost_maximize_global");
    let pass_nr = ctx.cumulative_pass_nr();
    match ctx.data(|d| d.get_temp::<(u64, egui::Id)>(global_key)) {
        Some((f, _)) => f == pass_nr || f + 1 == pass_nr,
        None => false,
    }
}

/// Wrap a widget so it gains a maximise / restore toggle.
///
/// Call once per frame with the same `id_salt`. `min_size` is the
/// rect the body renders into while inline; when maximised the
/// body fills `ctx.content_rect()` instead.
pub fn maximizable(
    ui: &mut egui::Ui,
    id_salt: impl Hash + Copy,
    accent: egui::Color32,
    min_size: egui::Vec2,
    body: impl FnOnce(&mut egui::Ui),
) {
    // Maximise state keyed purely on the caller's `id_salt` — no
    // `ui.id()` mixed in — so the host can reconstruct the same
    // key from the outside via [`is_maximized`] and route Ctrl+K
    // / context-sensitive logic based on "is THIS widget
    // currently full-window?".
    let max_id = maximize_state_key(id_salt);
    let maximized: bool = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(max_id))
        .unwrap_or(false);
    let mut toggle = false;

    // Global "is any maximizable widget currently full-window?"
    // tracker. Stored as `(pass_nr, owner_id)` so stale values
    // (widget toggled off and never re-rendered) are automatically
    // ignored on the next pass. Every other `maximizable` call
    // checks this to suppress its own button when SOMEONE ELSE is
    // full-window — otherwise `Order::Tooltip` button areas from
    // inline-in-a-pane widgets would still paint on top of the
    // overlay.
    let global_key = egui::Id::new("frost_maximize_global");
    let pass_nr = ui.ctx().cumulative_pass_nr();
    let stored_global: Option<(u64, egui::Id)> =
        ui.ctx().data(|d| d.get_temp(global_key));
    let some_other_maximized = match stored_global {
        Some((f, id)) => (f == pass_nr || f + 1 == pass_nr) && id != max_id,
        None => false,
    };
    if maximized {
        ui.ctx()
            .data_mut(|d| d.insert_temp(global_key, (pass_nr, max_id)));
    }

    if maximized {
        // Placeholder in the caller's layout so the surrounding
        // section / pane keep their footprint while the widget is
        // detached into the overlay.
        let (rect, _) = ui.allocate_exact_size(min_size, egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "(maximised)",
                egui::FontId::proportional(12.0),
                egui::Color32::from_gray(150),
            );
        }

        // Full-window overlay at `Order::Foreground`. Frame has
        // NO corner radius / stroke / inner margin — the whole
        // point is to cover the screen edge-to-edge, so any
        // rounding or inset reads as "not actually full" at the
        // corners.
        let ctx = ui.ctx().clone();
        let screen = ctx.content_rect();
        egui::Area::new(ui.id().with(("frost_maximize_overlay", id_salt)))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(&ctx, |ui| {
                ui.set_min_size(screen.size());
                ui.set_max_size(screen.size());
                let frame = egui::Frame::new()
                    .fill(glass_fill(BG_1_PANEL, accent, glass_alpha_window()))
                    .corner_radius(egui::CornerRadius::ZERO)
                    .inner_margin(egui::Margin::ZERO);
                frame.show(ui, |ui| {
                    body(ui);
                });
                if max_button_overlay(
                    &ctx,
                    screen.min + egui::vec2(8.0, 8.0),
                    true,
                    accent,
                    id_salt,
                )
                .clicked()
                {
                    toggle = true;
                }
            });
    } else {
        // Inline — allocate a rect of `min_size`, render the body
        // into a child `Ui` pinned to that rect, then overlay the
        // maximise chip at the top-left.
        let desired = egui::vec2(ui.available_width().max(min_size.x), min_size.y);
        let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        body(&mut child);
        // Skip the maximise chip when a DIFFERENT widget is
        // currently full-window — its overlay covers the whole
        // screen, and the button area (`Order::Tooltip`) would
        // otherwise paint on top of that overlay in the middle of
        // nowhere.
        if !some_other_maximized
            && max_button_overlay(
                ui.ctx(),
                rect.min + egui::vec2(6.0, 6.0),
                false,
                accent,
                id_salt,
            )
            .clicked()
        {
            toggle = true;
        }
    }

    if toggle {
        ui.ctx()
            .data_mut(|d| d.insert_temp::<bool>(max_id, !maximized));
    }
}

/// The 24 px maximise / restore chip. Lives in its own
/// `Order::Tooltip` Area so it paints (and intercepts clicks)
/// above the wrapped widget's own shapes — Areas at the same
/// `Foreground` order would get shadowed by canvas widgets like
/// the snarl graph that register their own foreground sub-layers.
fn max_button_overlay(
    ctx: &egui::Context,
    pos: egui::Pos2,
    maximized: bool,
    accent: egui::Color32,
    id_salt: impl Hash + Copy,
) -> egui::Response {
    const BTN: f32 = 24.0;
    let area_id = egui::Id::new("frost_maximize_btn").with(id_salt);
    let inner = egui::Area::new(area_id)
        .order(egui::Order::Tooltip)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            let (rect, resp) =
                ui.allocate_exact_size(egui::vec2(BTN, BTN), egui::Sense::click());
            let resp = resp
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(if maximized { "Restore" } else { "Maximize" });
            if ui.is_rect_visible(rect) {
                paint_ribbon_style_chip(
                    &ui.painter(),
                    rect,
                    accent,
                    /* active */ maximized,
                    /* hovered */ resp.hovered(),
                );
                paint_fullscreen_arrows(
                    &ui.painter(),
                    rect,
                    accent,
                    /* inward */ maximized,
                    /* hovered */ resp.hovered(),
                );
            }
            resp
        });
    inner.inner
}

/// Mirror of `ribbon::paint::paint_ribbon_button` (which is
/// `pub(crate)` so we can't call it directly). Same glass tiers
/// and active / hover transitions — keeps the chip in the ribbon
/// button family.
fn paint_ribbon_style_chip(
    painter: &egui::Painter,
    rect: egui::Rect,
    accent: egui::Color32,
    active: bool,
    hovered: bool,
) {
    let bg = if active {
        let blend = |a: u8, b: u8| ((a as f32) * 0.75 + (b as f32) * 0.25).round() as u8;
        let tinted = egui::Color32::from_rgb(
            blend(BG_2_RAISED.r(), accent.r()),
            blend(BG_2_RAISED.g(), accent.g()),
            blend(BG_2_RAISED.b(), accent.b()),
        );
        glass_fill(tinted, accent, glass_alpha_window())
    } else if hovered {
        glass_fill(BG_2_RAISED, accent, glass_alpha_window())
    } else {
        glass_fill(BG_1_PANEL, accent, glass_alpha_window())
    };
    let stroke = if active { accent } else { BORDER_SUBTLE };
    painter.rect(
        rect,
        egui::CornerRadius::same(6),
        bg,
        egui::Stroke::new(1.0, stroke),
        egui::StrokeKind::Inside,
    );
}

/// Paint the fullscreen glyph — single diagonal line through the
/// chip's centre, arrowheads at each end. `inward = false` heads
/// point OUT (maximise); `inward = true` heads point IN (restore).
fn paint_fullscreen_arrows(
    painter: &egui::Painter,
    rect: egui::Rect,
    accent: egui::Color32,
    inward: bool,
    hovered: bool,
) {
    let color = if inward {
        TEXT_PRIMARY
    } else if hovered {
        accent
    } else {
        egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 220)
    };
    let stroke_w = 1.4;
    let shrunk = rect.shrink(5.0);
    let center = rect.center();
    let ne_corner = egui::pos2(shrunk.max.x, shrunk.min.y);
    let sw_corner = egui::pos2(shrunk.min.x, shrunk.max.y);
    let lerp = |a: egui::Pos2, b: egui::Pos2, t: f32| -> egui::Pos2 {
        egui::pos2(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
    };
    let t = 0.65;
    let ne_tip = lerp(center, ne_corner, t);
    let sw_tip = lerp(center, sw_corner, t);
    painter.line_segment(
        [sw_tip, ne_tip],
        egui::Stroke::new(stroke_w, color),
    );
    let (from_ne, from_sw) = if inward {
        (ne_corner, sw_corner)
    } else {
        (center, center)
    };
    paint_arrowhead(painter, from_ne, ne_tip, color);
    paint_arrowhead(painter, from_sw, sw_tip, color);
}

fn paint_arrowhead(
    painter: &egui::Painter,
    from: egui::Pos2,
    tip: egui::Pos2,
    color: egui::Color32,
) {
    let dir = tip - from;
    let len = dir.length().max(1e-3);
    let dir = dir / len;
    let perp = egui::vec2(-dir.y, dir.x);
    let head_len = 4.0;
    let head_half_w = 2.6;
    let back = tip - dir * head_len;
    let p1 = back + perp * head_half_w;
    let p2 = back - perp * head_half_w;
    painter.add(egui::Shape::convex_polygon(
        vec![tip, p1, p2],
        color,
        egui::Stroke::NONE,
    ));
}
