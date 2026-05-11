//! Frost-styled dropdown — a single-select trigger with a popup list.
//!
//! Shape:
//!
//! ```text
//!   [  Selected option            ▾  ]
//!   └── trigger ─────────────────────┘
//!                                       ┌───────────────┐
//!                                       │  option a     │  ← popup
//!                                       │  option b ✓   │
//!                                       │  option c     │
//!                                       └───────────────┘
//! ```
//!
//! `selected` is the index into `options`. Clicking an option writes
//! the new index to `*selected` and the returned `Response` reports
//! `.changed() == true` for that frame.
//!
//! Two entry points:
//!
//! * [`dropdown`] — render at the canonical 1U height.
//! * [`dropdown_h`] — same, with caller-supplied height (used by
//!   resizable pods so the trigger grows with its slot).

use std::hash::Hash;

use crate::style::{
    glass_alpha_card, glass_fill, on_section, on_track, on_track_dim, popup_fill,
    surface_lift_target, theme, track_fill, widget_border,
};

/// Default trigger height — the canonical 1U row used elsewhere in
/// the kit.
pub const DROPDOWN_ROW_H: f32 = crate::style::UNIT;
/// Height of each row in the popup body.
const ITEM_H: f32 = 20.0;
/// Width of the right-aligned chevron column inside the trigger.
const CHEVRON_W: f32 = 14.0;
/// Inner padding inside the trigger (text vs. left edge / chevron).
const PAD_X: f32 = 8.0;
/// Trigger label / popup row font size.
const TEXT_FONT: f32 = 12.0;

/// Render a dropdown at the default [`DROPDOWN_ROW_H`] height.
/// `id_salt` disambiguates this dropdown's popup id from siblings in
/// the same `Ui` (a string, an enum value, an index — anything
/// hashable).
pub fn dropdown(
    ui: &mut egui::Ui,
    id_salt: impl Hash,
    selected: &mut usize,
    options: &[&str],
    accent: egui::Color32,
) -> egui::Response {
    dropdown_h(ui, id_salt, selected, options, accent, DROPDOWN_ROW_H)
}

/// Variable-height variant. Used by resizable pods.
pub fn dropdown_h(
    ui: &mut egui::Ui,
    id_salt: impl Hash,
    selected: &mut usize,
    options: &[&str],
    accent: egui::Color32,
    height: f32,
) -> egui::Response {
    let w = ui.available_width();
    let (rect, mut resp) =
        ui.allocate_exact_size(egui::vec2(w, height), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let th = theme();
        let tint = if resp.is_pointer_button_down_on() {
            0.28
        } else if resp.hovered() {
            0.14
        } else {
            0.06
        };
        let solid = lerp_col(track_fill(accent), surface_lift_target(accent), tint);
        let bg = egui::Color32::from_rgba_unmultiplied(
            solid.r(),
            solid.g(),
            solid.b(),
            glass_alpha_card(),
        );
        let border = if resp.hovered() {
            accent
        } else {
            widget_border(accent)
        };
        ui.painter().rect(
            rect,
            egui::CornerRadius::same(th.radius_widget),
            bg,
            egui::Stroke::new(th.border_width, border),
            egui::epaint::StrokeKind::Inside,
        );
        // Selected text — truncated so long option labels don't
        // overflow the trigger.
        let text_rect = egui::Rect::from_min_max(
            egui::pos2(rect.min.x + PAD_X, rect.min.y),
            egui::pos2(rect.max.x - CHEVRON_W - PAD_X, rect.max.y),
        );
        let display = options.get(*selected).copied().unwrap_or("—");
        let text_col = on_track();
        let galley = {
            let mut job = egui::text::LayoutJob::single_section(
                display.to_string(),
                egui::TextFormat::simple(egui::FontId::proportional(TEXT_FONT), text_col),
            );
            job.wrap.max_width = text_rect.width().max(0.0);
            job.wrap.max_rows = 1;
            job.wrap.break_anywhere = true;
            job.halign = egui::Align::LEFT;
            ui.painter().layout_job(job)
        };
        ui.painter().galley(
            egui::pos2(text_rect.min.x, text_rect.center().y - galley.size().y * 0.5),
            galley,
            text_col,
        );
        // Chevron — right-aligned, accent on hover.
        let cx = rect.max.x - PAD_X - CHEVRON_W * 0.5;
        let cy = rect.center().y;
        let chev_color = if resp.hovered() { accent } else { on_track_dim() };
        // `paint_icon` no-ops silently when the iconflow fonts haven't
        // been installed yet (very first frame before
        // `apply_theme_system` has run); a fallback chevron from the
        // proportional font keeps the trigger readable in that one-frame
        // window. After the fonts are ready, both paint — the bundled
        // glyph sits over the fallback, which is fine since they
        // occupy the same cell.
        ui.painter().text(
            egui::pos2(cx, cy),
            egui::Align2::CENTER_CENTER,
            "▾",
            egui::FontId::proportional(12.0),
            chev_color,
        );
        crate::icons::paint_icon(
            &ui.painter(),
            egui::pos2(cx, cy),
            egui::Align2::CENTER_CENTER,
            "chevron_down",
            12.0,
            chev_color,
        );
    }
    resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

    // Stable id for the popup so its open-state survives across
    // frames. `id_salt` disambiguates sibling dropdowns.
    let trigger_id = ui.id().with(("frost_dropdown", &id_salt));
    let resp_with_id = egui::Response { id: trigger_id, ..resp.clone() };

    let popup = egui::Popup::from_toggle_button_response(&resp_with_id)
        .align(egui::RectAlign::BOTTOM_START)
        .gap(2.0)
        .width(rect.width())
        .frame(
            egui::Frame::new()
                .fill(glass_fill(popup_fill(accent), accent, glass_alpha_card()))
                .stroke(egui::Stroke::new(theme().border_width, widget_border(accent)))
                .corner_radius(egui::CornerRadius::same(theme().radius_widget))
                .inner_margin(egui::Margin::same(2)),
        );

    let mut changed = false;
    if let Some(inner) = popup.show(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 1.0);
        for (idx, opt) in options.iter().enumerate() {
            let is_selected = *selected == idx;
            let (row_rect, row_resp) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), ITEM_H),
                egui::Sense::click(),
            );
            if ui.is_rect_visible(row_rect) {
                let bg = if is_selected {
                    Some(crate::style::row_selected_fill(accent))
                } else if row_resp.hovered() {
                    Some(crate::style::row_hover_fill(accent))
                } else {
                    None
                };
                if let Some(c) = bg {
                    ui.painter().rect_filled(
                        row_rect,
                        egui::CornerRadius::same(theme().radius_compact),
                        c,
                    );
                }
                ui.painter().text(
                    egui::pos2(row_rect.min.x + PAD_X, row_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    opt,
                    egui::FontId::proportional(TEXT_FONT),
                    on_section(),
                );
            }
            if row_resp.clicked() && *selected != idx {
                *selected = idx;
                changed = true;
            }
        }
    }) {
        drop(inner);
    }

    if changed {
        resp.mark_changed();
    }
    resp
}

fn lerp_col(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let blend = |x: u8, y: u8| ((x as f32) * (1.0 - t) + (y as f32) * t).round() as u8;
    egui::Color32::from_rgb(blend(a.r(), b.r()), blend(a.g(), b.g()), blend(a.b(), b.b()))
}
