//! Read-only sibling of `pretty_slider`. **Self-contained row
//! module** — takes a label, stacks it above a full-width progress
//! bar, paints a trailing separator.
//!
//! Two entry points:
//! * [`pretty_progressbar`] — numeric value + suffix (formats the
//!   bar's inline readout as `"{value}{suffix}"`).
//! * [`pretty_progressbar_text`] — caller supplies a pre-formatted
//!   string for the bar's inline readout. Use for "current /
//!   capacity" style displays where there isn't one clean number.
//!
//! Both are stacked (`caption` above, bar below) and append a
//! separator.

use egui;

use super::layout::stacked_pane_labelled;
use super::shared::{
    flush_pending_separator, paint_value_bar, smoothed_fraction, tumble_text, widget_separator,
};
use crate::style::contrast_text_for;

const BAR_H: f32 = 18.0;
const VALUE_FONT: f32 = 11.0;

/// Labelled numeric progress bar: label on top, full-width bar
/// below with `"{value}{suffix}"` inline.
pub fn pretty_progressbar(
    ui: &mut egui::Ui,
    label: &str,
    value: f64,
    range: std::ops::RangeInclusive<f64>,
    decimals: usize,
    suffix: &str,
    accent: egui::Color32,
) -> egui::Response {
    flush_pending_separator(ui);
    let resp = stacked_pane_labelled(ui, label, |ui| {
        let (lo, hi) = (*range.start(), *range.end());
        let denom = (hi - lo).max(f64::EPSILON);
        let fraction = ((value - lo) / denom).clamp(0.0, 1.0) as f32;
        let text = format!("{:.*}{}", decimals, value, suffix);
        progressbar_control(ui, fraction, &text, accent)
    });
    widget_separator(ui);
    resp
}

/// Labelled progress bar with caller-supplied inline text. For
/// "current / capacity" style readouts where the display isn't a
/// single formatted value.
pub fn pretty_progressbar_text(
    ui: &mut egui::Ui,
    label: &str,
    fraction: f32,
    inner_text: &str,
    accent: egui::Color32,
) -> egui::Response {
    flush_pending_separator(ui);
    let resp =
        stacked_pane_labelled(ui, label, |ui| progressbar_control(ui, fraction, inner_text, accent));
    widget_separator(ui);
    resp
}

/// Standalone bar — no label, no layout, no separator.
/// For custom compositions.
pub fn progressbar_control(
    ui: &mut egui::Ui,
    fraction: f32,
    inner_text: &str,
    accent: egui::Color32,
) -> egui::Response {
    let w = ui.available_width().max(1.0);
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(w, BAR_H),
        egui::Sense::hover(),
    );

    if ui.is_rect_visible(rect) {
        // GAME motion #13 — bar fraction is smoothed toward the
        // target so the fill chases the value over ~0.45 s.
        let smoothed = smoothed_fraction(ui.ctx(), resp.id, fraction, 0.45);
        // GAME motion #14 — digits in the inline readout tumble
        // through 0–9 for ~280 ms when they change. Non-digit
        // chars (% . space) pass through untouched.
        let display = tumble_text(ui.ctx(), resp.id, inner_text);
        paint_value_bar(
            ui,
            rect,
            smoothed,
            &display,
            egui::FontId::new(VALUE_FONT, egui::FontFamily::Monospace),
            accent,
            crate::style::on_track(),
            contrast_text_for(accent),
            crate::style::theme().radius_widget,
        );
    }

    resp
}
