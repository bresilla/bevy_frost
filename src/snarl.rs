//! Node-graph integration — thin glue around [`egui_snarl`] so graph
//! widgets inherit the frost palette and border language without
//! every consumer having to hand-tune a `SnarlStyle`.
//!
//! Two pieces of surface:
//!
//! * [`frost_snarl_style`] — builds a [`SnarlStyle`] configured with
//!   frost's `BG_*` / `widget_border` / accent colours, the same
//!   corner radius as [`section`](crate::widgets::foldable::section),
//!   and a pin/wire width that matches the border stroke. Pass the
//!   returned style straight into
//!   [`SnarlWidget::style`](egui_snarl::ui::SnarlWidget::style).
//! * `pub use egui_snarl` re-export — callers don't need a second
//!   direct dep. `use bevy_frost::snarl::{Snarl, SnarlViewer,
//!   SnarlWidget, NodeId, InPin, OutPin, ...};` lands the full
//!   upstream surface.
//!
//! Drop the whole thing into any section body:
//!
//! ```ignore
//! section(ui, "graph", "Graph", accent, true, |ui| {
//!     SnarlWidget::new()
//!         .id_salt("my_graph")
//!         .style(frost_snarl_style(accent))
//!         .min_size(egui::vec2(320.0, 260.0))
//!         .show(&mut state.graph, &mut state.viewer, ui);
//! });
//! ```

use std::hash::Hash;

use bevy_egui::egui;

pub use egui_snarl::{
    ui::{
        AnyPins, BackgroundPattern, NodeLayout, PinInfo, PinPlacement, PinShape,
        SnarlPin, SnarlStyle, SnarlViewer, SnarlWidget,
    },
    InPin, InPinId, NodeId, OutPin, OutPinId, Snarl,
};

use crate::style::{
    glass_alpha_card, glass_alpha_window, glass_fill, radius, widget_border, BG_1_PANEL,
    BG_2_RAISED, BORDER_SUBTLE, TEXT_PRIMARY,
};

/// Build a [`SnarlStyle`] that inherits the frost palette + border
/// language. Call per-frame with the current accent so the graph
/// re-tints when the user swaps accent colour (the same way every
/// other frost surface does).
///
/// What the returned style pins down:
///
/// * **Node frame** — `BG_2_RAISED` glass fill + `widget_border`
///   stroke + `radius::MD` corner, matching
///   [`section`](crate::widgets::foldable::section) so nodes look
///   like first-class frost surfaces.
/// * **Background** — `BG_1_PANEL` glass fill behind everything,
///   the same colour a floating window uses, so the graph canvas
///   sits cleanly in an editor panel.
/// * **Pins / wires** — `widget_border(accent)` + stroke width 1 px,
///   identical to every other widget's edge.
///
/// Everything else stays at the library default so scroll / zoom /
/// selection interactions remain familiar to upstream users.
pub fn frost_snarl_style(accent: egui::Color32) -> SnarlStyle {
    let node_frame = egui::Frame::new()
        .fill(glass_fill(BG_2_RAISED, accent, glass_alpha_card()))
        .stroke(egui::Stroke::new(1.0, widget_border(accent)))
        .corner_radius(egui::CornerRadius::same(radius::MD))
        .inner_margin(egui::Margin::symmetric(8, 4));

    // Header frame — same corners + border but transparent fill so
    // it layers on top of the node's own fill without doubling the
    // opacity. Matches how foldable sections show their title band.
    let header_frame = egui::Frame::new()
        .fill(egui::Color32::TRANSPARENT)
        .stroke(egui::Stroke::NONE)
        .corner_radius(egui::CornerRadius::same(radius::MD))
        .inner_margin(egui::Margin::symmetric(6, 3));

    let bg_fill = glass_fill(BG_1_PANEL, accent, glass_alpha_window());

    SnarlStyle {
        node_frame: Some(node_frame),
        header_frame: Some(header_frame),
        bg_frame: Some(
            egui::Frame::new()
                .fill(bg_fill)
                .stroke(egui::Stroke::new(1.0, widget_border(accent)))
                .corner_radius(egui::CornerRadius::same(radius::LG))
                .inner_margin(egui::Margin::same(2)),
        ),
        pin_fill: Some(accent),
        pin_stroke: Some(egui::Stroke::new(1.0, widget_border(accent))),
        wire_width: Some(1.5),
        wire_style: None,
        downscale_wire_frame: Some(true),
        upscale_wire_frame: Some(true),
        ..SnarlStyle::new()
    }
}

/// Render the graph widget with a built-in **maximise / restore**
/// toggle in its top-left corner.
///
/// The maximise state is scoped to THIS graph — clicking the icon
/// lifts only the graph into a full-window overlay, leaving the
/// floating panel and any outer container the caller placed it in
/// completely untouched. Click again to restore.
///
/// When maximised the caller-supplied `min_size` still allocates
/// in-place so the section / panel layout doesn't collapse while
/// the graph is "gone" to the overlay — the hole is filled with a
/// small "(maximised)" caption.
///
/// Use this instead of calling [`SnarlWidget::new().show`] directly
/// whenever you want the fullscreen affordance. Otherwise, the
/// plain `SnarlWidget` route keeps working.
pub fn frost_snarl<T, V: SnarlViewer<T>>(
    ui: &mut egui::Ui,
    id_salt: impl Hash + Copy,
    snarl: &mut Snarl<T>,
    viewer: &mut V,
    accent: egui::Color32,
    min_size: egui::Vec2,
) {
    let max_id = ui.id().with(("frost_snarl_max", id_salt));
    let maximized: bool = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(max_id))
        .unwrap_or(false);
    let mut toggle = false;

    if maximized {
        // Placeholder in the caller's layout so the surrounding
        // section / panel keeps its footprint while the graph is
        // detached into the overlay.
        let (rect, _) = ui.allocate_exact_size(min_size, egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "(graph maximised)",
                egui::FontId::proportional(12.0),
                egui::Color32::from_gray(150),
            );
        }

        // Full-window overlay — an `Area` at the highest order so
        // it covers any floating panel / ribbon underneath. The
        // overlay's frame uses the same BG / border recipe as a
        // floating pane so it reads as "same family".
        let ctx = ui.ctx().clone();
        let screen = ctx.content_rect();
        egui::Area::new(ui.id().with(("frost_snarl_overlay", id_salt)))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(&ctx, |ui| {
                ui.set_min_size(screen.size());
                ui.set_max_size(screen.size());
                let frame = egui::Frame::new()
                    .fill(glass_fill(BG_1_PANEL, accent, glass_alpha_window()))
                    .stroke(egui::Stroke::new(1.0, BORDER_SUBTLE))
                    .corner_radius(egui::CornerRadius::same(radius::LG))
                    .inner_margin(egui::Margin::same(4));
                frame.show(ui, |ui| {
                    SnarlWidget::new()
                        .id_salt(id_salt)
                        .style(frost_snarl_style(accent))
                        .min_size(screen.size())
                        .show(snarl, viewer, ui);
                });

                // Restore button — foreground-Area, pinned to the
                // overlay's top-left, so it paints above every snarl
                // shape (which otherwise eat clicks).
                if max_button_overlay(&ctx, screen.min + egui::vec2(8.0, 8.0), true, accent, id_salt)
                    .clicked()
                {
                    toggle = true;
                }
            });
    } else {
        // Inline — allocate a rect of `min_size`, show the snarl in
        // a child ui confined to that rect. We grab the rect BEFORE
        // rendering the snarl so we can overlay the maximise button
        // at its top-left after the fact.
        let desired = egui::vec2(ui.available_width().max(min_size.x), min_size.y);
        let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        SnarlWidget::new()
            .id_salt(id_salt)
            .style(frost_snarl_style(accent))
            .min_size(desired)
            .show(snarl, viewer, &mut child);
        if max_button_overlay(ui.ctx(), rect.min + egui::vec2(6.0, 6.0), false, accent, id_salt)
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

/// Paint and interact a small maximise / restore button at `pos`.
/// The button lives in its own foreground `Area` so it paints above
/// the snarl widget's shapes (the snarl claims the whole inline
/// rect for its own pan/zoom interactions — a regular `ui.interact`
/// at the same spot would get shadowed).
fn max_button_overlay(
    ctx: &egui::Context,
    pos: egui::Pos2,
    maximized: bool,
    accent: egui::Color32,
    id_salt: impl Hash + Copy,
) -> egui::Response {
    // Size matches the icon density of a ribbon button but scaled
    // down to read as a corner affordance rather than a primary
    // nav target. 24 px lands between "too chunky" and "too small
    // to hit".
    const BTN: f32 = 24.0;
    let area_id = egui::Id::new("frost_snarl_max_btn").with(id_salt);
    // `Order::Tooltip` — one tier above the snarl widget's
    // foreground layers (the whole graph overlay uses
    // `Order::Foreground`). Anything below that gets shadowed by
    // the snarl's own internal shapes, which is what caused the
    // button to "disappear" in full-window mode.
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

/// Replicates the ribbon button's background / border recipe from
/// `ribbon::paint::paint_ribbon_button` — same glass fill tiers,
/// same active + hover transitions, so the maximise chip reads as
/// part of the same button family.
fn paint_ribbon_style_chip(
    painter: &egui::Painter,
    rect: egui::Rect,
    accent: egui::Color32,
    active: bool,
    hovered: bool,
) {
    let bg = if active {
        // 25 % accent blended into `BG_2_RAISED`, then glass-fill
        // alpha — matches the active ribbon button exactly.
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

/// Paint the fullscreen / exit-fullscreen glyph — a single
/// diagonal line through the chip's centre with an arrowhead at
/// each end. `inward = false` heads point OUT from the centre
/// (expand to full window); `inward = true` heads point IN toward
/// the centre (collapse back).
///
/// When the button is active (maximised) the icon takes
/// `TEXT_PRIMARY` so it pops against the accent-tinted fill, same
/// language as an active ribbon button's glyph.
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
    // Arrow tips sit at 65 % of the half-diagonal — pulls the two
    // arrowheads closer to centre so they read as a single compact
    // glyph rather than stretched across the chip's corners.
    let lerp = |a: egui::Pos2, b: egui::Pos2, t: f32| -> egui::Pos2 {
        egui::pos2(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
    };
    let t = 0.65;
    let ne_tip = lerp(center, ne_corner, t);
    let sw_tip = lerp(center, sw_corner, t);

    // The connecting line — the same continuous diagonal in both
    // states. Only the arrowhead orientation at each tip changes.
    painter.line_segment(
        [sw_tip, ne_tip],
        egui::Stroke::new(stroke_w, color),
    );

    // Arrowhead "direction of travel" reference points. For
    // maximise the heads point OUT (reference = centre, tip is the
    // outer end). For restore the heads point IN (reference = the
    // outside corner, tip is still the same — the triangle faces
    // toward centre).
    let (from_ne, from_sw) = if inward {
        (ne_corner, sw_corner)
    } else {
        (center, center)
    };
    paint_arrowhead(painter, from_ne, ne_tip, color);
    paint_arrowhead(painter, from_sw, sw_tip, color);
}

/// Draw a small filled triangle arrowhead at `tip`, pointing in
/// the direction `from → tip`. The triangle overlays whatever shaft
/// geometry the caller already drew so it reads as part of the
/// line.
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
