//! Frost-styled read-only progress bar. Two-row stacked layout:
//! caption on top, full-width filled bar below. Total height =
//! 2 × [`crate::style::UNIT`] (= 2U) by default.
//!
//! Mirrors `frostcore::widgets::progressbar::pretty_progressbar` /
//! `pretty_progressbar_text`. Drops the digit-tumble + smoothed-
//! fraction animations for v1; can be added back as polish.

use crate::style::{
    contrast_text_for, on_panel_dim, on_track, theme, track_fill, widget_border,
    BODY_FONT_SIZE,
};

/// Bar row height — matches `frostcore::widgets::progressbar::BAR_H`.
pub const PROGRESSBAR_ROW_H: f32 = 18.0;
/// Inline readout font size — matches `frostcore`'s
/// `progressbar::VALUE_FONT = 11` (monospace).
pub const PROGRESSBAR_VALUE_FONT: f32 = 11.0;

/// Default progress bar (2 × [`PROGRESSBAR_ROW_H`] = 36 px total —
/// caption row on top, bar row below). `fraction` clamps to
/// `[0, 1]`. `text` is the centred inline readout — callers
/// typically format something like `"42%"` or `"3/10 used"`.
pub fn progressbar(
    ui: &mut egui::Ui,
    label: &str,
    fraction: f32,
    text: &str,
    accent: egui::Color32,
) -> egui::Response {
    progressbar_h(ui, label, fraction, text, accent, PROGRESSBAR_ROW_H)
}

/// Variable-height variant — `row_height` is the height of EACH
/// row (caption + bar), so total widget height is `2 × row_height`.
/// Used by resizable pods so the whole 2-row block scales together.
pub fn progressbar_h(
    ui: &mut egui::Ui,
    label: &str,
    fraction: f32,
    text: &str,
    accent: egui::Color32,
    row_height: f32,
) -> egui::Response {
    let total_w = ui.available_width().max(1.0);
    let total_h = row_height * 2.0;
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(total_w, total_h),
        egui::Sense::hover(),
    );
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let scale = row_height / PROGRESSBAR_ROW_H;
    // Caption row — left-aligned dim text. Body font size at the
    // default row height; scales linearly when the pod is resized.
    if !label.is_empty() {
        ui.painter().text(
            egui::pos2(rect.left(), rect.top() + row_height * 0.5),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional((BODY_FONT_SIZE * scale).round()),
            on_panel_dim(),
        );
    }
    // Bar row.
    let bar_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left(), rect.top() + row_height),
        egui::vec2(total_w, row_height),
    );
    paint_bar(ui, bar_rect, fraction, text, accent, scale);
    resp
}

fn paint_bar(
    ui: &egui::Ui,
    rect: egui::Rect,
    fraction: f32,
    text: &str,
    accent: egui::Color32,
    scale: f32,
) {
    let f = fraction.clamp(0.0, 1.0);
    let th = theme();
    let painter = ui.painter_at(rect);
    let corner = egui::CornerRadius::same(th.radius_widget);
    // Track background.
    painter.rect(
        rect,
        corner,
        track_fill(accent),
        egui::Stroke::new(th.border_width, widget_border(accent)),
        egui::epaint::StrokeKind::Inside,
    );
    // Filled portion — accent-coloured rect from left edge.
    if f > 0.0 {
        let fill_w = rect.width() * f;
        let fill_rect = egui::Rect::from_min_size(rect.min, egui::vec2(fill_w, rect.height()));
        painter.rect_filled(fill_rect, corner, accent);
    }
    // Inline readout — paint twice with different colours so the
    // text reads against both halves of the bar (filled and
    // unfilled) without colour-bombing one side. Clip-rect each
    // half so the wrong-colour half doesn't bleed.
    if !text.is_empty() {
        let font = egui::FontId::new(
            (PROGRESSBAR_VALUE_FONT * scale).round(),
            egui::FontFamily::Monospace,
        );
        let centre = rect.center();
        let split_x = rect.left() + rect.width() * f;
        let left_half = egui::Rect::from_min_max(rect.min, egui::pos2(split_x, rect.max.y));
        let right_half = egui::Rect::from_min_max(egui::pos2(split_x, rect.min.y), rect.max);
        // Over the filled portion: contrast against accent.
        let left_painter = ui.painter().clone().with_clip_rect(left_half);
        left_painter.text(
            centre,
            egui::Align2::CENTER_CENTER,
            text,
            font.clone(),
            contrast_text_for(accent),
        );
        // Over the unfilled portion: contrast against track.
        let right_painter = ui.painter().clone().with_clip_rect(right_half);
        right_painter.text(
            centre,
            egui::Align2::CENTER_CENTER,
            text,
            font,
            on_track(),
        );
    }
}
