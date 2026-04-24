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
use super::shared::{flush_pending_separator, paint_value_bar, widget_separator};
use crate::style::{contrast_text_for, radius, TEXT_PRIMARY};

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
        paint_value_bar(
            ui,
            rect,
            fraction.clamp(0.0, 1.0),
            inner_text,
            egui::FontId::new(VALUE_FONT, egui::FontFamily::Monospace),
            accent,
            TEXT_PRIMARY,
            contrast_text_for(accent),
            radius::WIDGET,
        );
    }

    resp
}
