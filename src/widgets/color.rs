//! Colour-picker module — label on 70 %, egui colour-swatch button
//! on 30 %, trailing separator. Same layout language as every other
//! widget module.

use bevy_egui::egui;

use super::layout::dual_pane_labelled;
use super::shared::widget_separator;

/// Labelled sRGB colour swatch.
pub fn color_rgb(ui: &mut egui::Ui, label: &str, rgb: &mut [f32; 3]) -> egui::Response {
    let resp = dual_pane_labelled(ui, label, |ui| ui.color_edit_button_rgb(rgb));
    widget_separator(ui);
    resp
}

/// Labelled sRGBA colour swatch.
pub fn color_rgba(ui: &mut egui::Ui, label: &str, rgba: &mut [f32; 4]) -> egui::Response {
    let resp = dual_pane_labelled(ui, label, |ui| ui.color_edit_button_rgba_unmultiplied(rgba));
    widget_separator(ui);
    resp
}
