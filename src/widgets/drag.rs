//! Numeric drag-value modules — `drag_value` (label on 70 %, value on
//! 30 %) and `axis_drag` (coloured X/Y/Z glyph on the left, value on
//! the right). Both paint a trailing separator, matching every other
//! widget module in this crate.

use std::ops::RangeInclusive;

use bevy_egui::egui;

use super::layout::{dual_pane, dual_pane_labelled};
use super::shared::widget_separator;

/// Fixed width of the DragValue box, in px. Every numeric input in a
/// panel allocates this same width, so rows with short values
/// (`2.00`) align with rows with long values (`023234.8`) instead of
/// egui auto-sizing each box to its current content.
pub const INPUT_WIDTH: f32 = 72.0;

fn input_size(ui: &egui::Ui) -> egui::Vec2 {
    egui::vec2(INPUT_WIDTH, ui.spacing().interact_size.y)
}

/// Labelled DragValue row. 70 / 30 split, trailing separator. The
/// DragValue is allocated at a fixed [`INPUT_WIDTH`] so boxes align
/// across rows.
pub fn drag_value(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f64,
    speed: f64,
    range: RangeInclusive<f64>,
    decimals: usize,
    suffix: &str,
) -> egui::Response {
    let resp = dual_pane_labelled(ui, label, |ui| {
        let size = input_size(ui);
        ui.add_sized(
            size,
            egui::DragValue::new(value)
                .speed(speed)
                .range(range)
                .fixed_decimals(decimals)
                .suffix(suffix),
        )
    });
    widget_separator(ui);
    resp
}

/// Coloured-axis DragValue row — X/Y/Z glyph tinted `glyph_color` in
/// the left pane, fixed-width DragValue in the right pane. 70 / 30
/// split, trailing separator.
pub fn axis_drag(
    ui: &mut egui::Ui,
    glyph: &str,
    glyph_color: egui::Color32,
    value: &mut f64,
    speed: f64,
    suffix: &str,
    decimals: usize,
) -> egui::Response {
    let resp = dual_pane(
        ui,
        |ui| {
            ui.label(
                egui::RichText::new(glyph)
                    .strong()
                    .monospace()
                    .color(glyph_color),
            );
        },
        |ui| {
            let size = input_size(ui);
            ui.add_sized(
                size,
                egui::DragValue::new(value)
                    .speed(speed)
                    .fixed_decimals(decimals)
                    .suffix(suffix),
            )
        },
    );
    widget_separator(ui);
    resp
}
