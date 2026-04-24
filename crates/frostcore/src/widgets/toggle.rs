//! Binary on/off toggle. **Self-contained row module** — takes a
//! label, handles its own `dual_pane` layout, paints a trailing
//! separator. Call it and move on; no layout wrapping required at
//! the callsite.
//!
//! Shape: [`pill track + sliding knob`]. Track colour stays
//! constant (modulo a small accent hint on ON to avoid the
//! irradiation illusion that makes ON states look bigger); the
//! knob slides from left to right and shifts from dim grey to
//! accent colour as `animate_bool_responsive` interpolates.

use egui;

use super::layout::dual_pane_labelled;
use super::shared::{lerp_color, widget_separator};
use crate::style::{radius, widget_border, BG_3_HOVER};

/// Overall track width.
const W: f32 = 38.0;
/// Overall track height — same as the slider / progressbar so
/// rows line up.
const H: f32 = 18.0;
/// Padding between the track edge and the knob, on every side.
const KNOB_PAD: f32 = 2.0;

/// Knob colour when OFF. Dim grey, visibly "not activated" against
/// the track.
const KNOB_OFF: egui::Color32 = egui::Color32::from_rgb(0x70, 0x70, 0x76);
/// How much of the accent gets blended into the track on ON — a
/// hint, not a fill. Small enough to stay under the irradiation-
/// illusion threshold.
const TRACK_ACCENT_HINT: f32 = 0.22;

/// Labelled on/off toggle row. Label on the left (70 % of row),
/// toggle hugs the right gutter. Paints a trailing separator.
pub fn toggle(
    ui: &mut egui::Ui,
    label: &str,
    on: &mut bool,
    accent: egui::Color32,
) -> egui::Response {
    let resp = dual_pane_labelled(ui, label, |ui| toggle_control(ui, on, accent));
    widget_separator(ui);
    resp
}

/// Standalone toggle control — no label, no layout, no separator.
/// For custom compositions where `toggle` is too opinionated.
pub fn toggle_control(
    ui: &mut egui::Ui,
    on: &mut bool,
    accent: egui::Color32,
) -> egui::Response {
    let (rect, mut response) =
        ui.allocate_exact_size(egui::vec2(W, H), egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, "")
    });

    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);

        let painter = ui.painter_at(rect);
        let track_corner = egui::CornerRadius::same(radius::COMPACT);

        let track_bg = lerp_color(BG_3_HOVER, accent, how_on * TRACK_ACCENT_HINT);
        painter.rect(
            rect,
            track_corner,
            track_bg,
            egui::Stroke::new(1.0, widget_border(accent)),
            egui::StrokeKind::Inside,
        );

        let knob_size = H - KNOB_PAD * 2.0;
        let x_min = rect.left() + KNOB_PAD;
        let x_max = rect.right() - KNOB_PAD - knob_size;
        let knob_x = egui::lerp(x_min..=x_max, how_on);
        let knob_rect = egui::Rect::from_min_size(
            egui::pos2(knob_x, rect.top() + KNOB_PAD),
            egui::vec2(knob_size, knob_size),
        );
        let knob_color = lerp_color(KNOB_OFF, accent, how_on);
        painter.rect(
            knob_rect,
            egui::CornerRadius::same(radius::COMPACT),
            knob_color,
            egui::Stroke::new(1.0, widget_border(accent)),
            egui::StrokeKind::Inside,
        );
    }
    response
}
