//! Full-width chunky slider with the value readout painted inside
//! the bar. **Self-contained row module** — takes a label,
//! handles its own `stacked_pane` layout (label on top, bar on the
//! full row below), paints a trailing separator.

use egui;

use super::layout::stacked_pane_labelled;
use super::shared::{flush_pending_separator, paint_value_bar, widget_separator};
use crate::style::contrast_text_for;

/// Bar height. Tall enough to drag comfortably and to fit the
/// value text on one line.
const BAR_H: f32 = 18.0;
/// Font size of the value readout painted inside the bar.
const VALUE_FONT: f32 = 11.0;

/// Labelled slider row. Caption on top, full-width slider bar
/// below, trailing separator. Click or drag anywhere on the bar.
pub fn pretty_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    decimals: usize,
    suffix: &str,
    accent: egui::Color32,
) -> egui::Response {
    flush_pending_separator(ui);
    let resp = stacked_pane_labelled(ui, label, |ui| {
        slider_control(ui, value, range, decimals, suffix, accent)
    });
    widget_separator(ui);
    resp
}

/// Standalone slider bar — no label, no layout, no separator.
/// For custom compositions.
pub fn slider_control(
    ui: &mut egui::Ui,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    decimals: usize,
    suffix: &str,
    accent: egui::Color32,
) -> egui::Response {
    let w = ui.available_width().max(1.0);
    let (rect, mut resp) = ui.allocate_exact_size(
        egui::vec2(w, BAR_H),
        egui::Sense::click_and_drag(),
    );

    let (lo, hi) = (*range.start(), *range.end());
    let denom = (hi - lo).max(f64::EPSILON);

    if let Some(pos) = resp.interact_pointer_pos() {
        if resp.dragged() || resp.clicked() {
            let new_t = ((pos.x - rect.min.x) as f64 / rect.width() as f64).clamp(0.0, 1.0);
            let new_val = lo + new_t * denom;
            if (new_val - *value).abs() > f64::EPSILON {
                *value = new_val.clamp(lo, hi);
                resp.mark_changed();
            }
        }
    }

    let resp = resp.on_hover_cursor(egui::CursorIcon::ResizeHorizontal);

    if ui.is_rect_visible(rect) {
        let fraction = ((*value - lo) / denom).clamp(0.0, 1.0) as f32;
        let text = format!("{:.*}{}", decimals, *value, suffix);
        paint_value_bar(
            ui,
            rect,
            fraction,
            &text,
            egui::FontId::new(VALUE_FONT, egui::FontFamily::Monospace),
            accent,
            crate::style::on_track(),
            contrast_text_for(accent),
            crate::style::theme().radius_widget,
        );
    }

    resp
}
