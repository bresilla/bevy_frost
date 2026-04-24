//! Dropdown — a select-one value picker with a frost-styled trigger
//! button and a popup list below it. Two entry points:
//!
//! * [`dropdown`] — **self-contained row module**. Takes a label,
//!   renders the `dual_pane_labelled` 70 / 30 split, paints its own
//!   trailing separator. Drop it next to toggles / sliders / drag
//!   values without any layout wrapping.
//! * [`dropdown_control`] — standalone trigger (no label, no row,
//!   no separator) for callers that want to embed the dropdown
//!   inside a bespoke layout.
//!
//! The popup list is painted with the frost glass fill + unified
//! border, matching the rest of the widget family. Selection lives
//! with the caller — the widget writes `*selected` to the new index
//! when an option is clicked and returns a `Response` whose
//! `.changed()` reflects that write.
//!
//! ```text
//!   label…………………………         [  Selected value        ▾  ]
//!     └── dual_pane left cell    └── trigger → opens list below
//! ```

use std::hash::Hash;

use egui;

use super::layout::dual_pane_labelled;
use super::shared::{flush_pending_separator, widget_separator};
use crate::style::{
    glass_alpha_card, glass_fill, radius, widget_border, BG_2_RAISED, BG_3_HOVER,
    TEXT_PRIMARY, TEXT_SECONDARY,
};

/// Height of the trigger button. Matches the shared `interact_size.y`
/// (20 px) so dropdown rows align with toggle / slider rows.
const TRIGGER_H: f32 = 20.0;
/// Width of the right-aligned chevron column inside the trigger.
const CHEVRON_W: f32 = 14.0;
/// Left/right inner padding inside the trigger button.
const PAD_X: f32 = 8.0;
/// Row height inside the popup list.
const ITEM_H: f32 = 20.0;

/// Labelled dropdown row — 70 / 30 split, popup list opens below the
/// trigger, trailing separator. `options` is a slice of display
/// strings; `selected` is the index into that slice.
///
/// Clicking an option writes the new index to `*selected` and returns
/// a `Response` with `.changed() == true`; clicking the trigger
/// without changing selection returns a plain hover/click response.
pub fn dropdown(
    ui: &mut egui::Ui,
    label: &str,
    selected: &mut usize,
    options: &[&str],
    accent: egui::Color32,
) -> egui::Response {
    flush_pending_separator(ui);
    let resp = dual_pane_labelled(ui, label, |ui| {
        dropdown_control(ui, label, selected, options, accent)
    });
    widget_separator(ui);
    resp
}

/// Standalone dropdown control — just the trigger + popup, no
/// surrounding row. Use for custom compositions (a dropdown next to
/// a button in a horizontal strip, etc.) where [`dropdown`] is too
/// opinionated. `id_salt` disambiguates this dropdown's popup from
/// any sibling dropdowns in the same Ui.
pub fn dropdown_control(
    ui: &mut egui::Ui,
    id_salt: impl Hash,
    selected: &mut usize,
    options: &[&str],
    accent: egui::Color32,
) -> egui::Response {
    // Reserve the right-cell max width minus a small gutter so the
    // trigger lines up with the DragValue / slider widths when this
    // lives in a `dual_pane` right cell.
    let max_w = ui.available_width().max(60.0).min(200.0);
    let (rect, mut resp) = ui.allocate_exact_size(
        egui::vec2(max_w, TRIGGER_H),
        egui::Sense::click(),
    );
    let resp_hover = resp.clone().on_hover_cursor(egui::CursorIcon::PointingHand);

    // Paint trigger background: accent-tinted glass fill, hover
    // brightens, border switches to accent on hover.
    if ui.is_rect_visible(rect) {
        let tint = if resp.is_pointer_button_down_on() {
            0.28
        } else if resp.hovered() {
            0.14
        } else {
            0.06
        };
        let solid = lerp_color(BG_2_RAISED, accent, tint);
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
            egui::CornerRadius::same(radius::WIDGET),
            bg,
            egui::Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );

        // Selected text — truncated so long option labels don't
        // overflow the trigger.
        let text_rect = egui::Rect::from_min_max(
            egui::pos2(rect.min.x + PAD_X, rect.min.y),
            egui::pos2(rect.max.x - CHEVRON_W - PAD_X, rect.max.y),
        );
        let display = options.get(*selected).copied().unwrap_or("—");
        let galley = {
            let mut job = egui::text::LayoutJob::single_section(
                display.to_string(),
                egui::TextFormat::simple(
                    egui::FontId::proportional(12.0),
                    TEXT_PRIMARY,
                ),
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
            TEXT_PRIMARY,
        );

        // Chevron — small downward triangle in the right column.
        let cx = rect.max.x - PAD_X - CHEVRON_W * 0.5;
        let cy = rect.center().y;
        let r = 3.0_f32;
        let chev_color = if resp.hovered() { accent } else { TEXT_SECONDARY };
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(cx - r, cy - r * 0.5),
                egui::pos2(cx + r, cy - r * 0.5),
                egui::pos2(cx, cy + r * 0.7),
            ],
            chev_color,
            egui::Stroke::NONE,
        ));
    }

    // Attach a stable id so the popup's open-state memory keeps
    // matching this trigger across frames. `id_salt` is hashed into
    // the response id so multiple dropdowns in one Ui don't collide.
    resp = resp.clone().on_hover_cursor(egui::CursorIcon::PointingHand);
    let trigger_id = ui.id().with(("frost_dropdown", &id_salt));
    let resp_with_id = egui::Response {
        id: trigger_id,
        ..resp_hover
    };

    // Popup body — one row per option, selection tint on the current
    // index, accent hover fill on whatever row the pointer is over.
    let popup = egui::Popup::from_toggle_button_response(&resp_with_id)
        .align(egui::RectAlign::BOTTOM_START)
        .gap(2.0)
        .width(rect.width())
        .frame(
            egui::Frame::new()
                .fill(glass_fill(BG_2_RAISED, accent, glass_alpha_card()))
                .stroke(egui::Stroke::new(1.0, widget_border(accent)))
                .corner_radius(egui::CornerRadius::same(radius::WIDGET))
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
                    let blend = |a: u8, b: u8| {
                        ((a as f32) * 0.55 + (b as f32) * 0.45).round() as u8
                    };
                    Some(egui::Color32::from_rgb(
                        blend(BG_2_RAISED.r(), accent.r()),
                        blend(BG_2_RAISED.g(), accent.g()),
                        blend(BG_2_RAISED.b(), accent.b()),
                    ))
                } else if row_resp.hovered() {
                    Some(BG_3_HOVER)
                } else {
                    None
                };
                if let Some(c) = bg {
                    ui.painter()
                        .rect_filled(row_rect, egui::CornerRadius::same(2), c);
                }
                ui.painter().text(
                    egui::pos2(row_rect.min.x + PAD_X, row_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    opt,
                    egui::FontId::proportional(12.0),
                    TEXT_PRIMARY,
                );
            }
            if row_resp.clicked() && *selected != idx {
                *selected = idx;
                changed = true;
            }
        }
    }) {
        // `popup.show` returns `Some(InnerResponse)` only when the
        // popup is actually on-screen — that's the only frame we need
        // to propagate the inner click state outward.
        drop(inner);
    }

    if changed {
        resp.mark_changed();
    }
    resp
}

fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| ((x as f32) * (1.0 - t) + (y as f32) * t).round() as u8;
    egui::Color32::from_rgba_premultiplied(
        mix(a.r(), b.r()),
        mix(a.g(), b.g()),
        mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}
