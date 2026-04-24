//! Group frame — a subtle rounded rectangle around a small cluster
//! of widgets. Distinct from the foldable container in
//! [`super::foldable`]: `group_frame` is a row-level clustering
//! hint for things like a radio group or a button + hint pair
//! *inside* a container, not the container itself.

use egui;

use crate::style::{glass_alpha_group, glass_fill, radius, BG_3_HOVER, BORDER_SUBTLE};

pub fn group_frame(
    ui: &mut egui::Ui,
    accent: egui::Color32,
    body: impl FnOnce(&mut egui::Ui),
) {
    // Uses `BG_3_HOVER` as the base so groups sit a touch brighter
    // than cards, reinforcing the stacked-pane feel.
    egui::Frame::new()
        .fill(glass_fill(BG_3_HOVER, accent, glass_alpha_group()))
        .corner_radius(egui::CornerRadius::same(radius::WIDGET))
        .stroke(egui::Stroke::new(1.0, BORDER_SUBTLE))
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, body);
}
