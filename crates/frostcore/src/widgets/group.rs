//! Group frame — a subtle rounded rectangle around a small cluster
//! of widgets. Distinct from the foldable container in
//! [`super::foldable`]: `group_frame` is a row-level clustering
//! hint for things like a radio group or a button + hint pair
//! *inside* a container, not the container itself.

use egui;

use crate::style::{glass_alpha_group, glass_fill, BORDER_SUBTLE};

use super::shared::flush_pending_separator;

pub fn group_frame(
    ui: &mut egui::Ui,
    accent: egui::Color32,
    body: impl FnOnce(&mut egui::Ui),
) {
    flush_pending_separator(ui);
    // PRO paints the slightly-brighter nested glass; GAME drops the
    // frame so the group becomes a transparent grouper.
    let frame = if crate::style::section_show_frame() {
        egui::Frame::new()
            .fill(glass_fill(crate::style::theme().bg_hover, accent, glass_alpha_group()))
            .corner_radius(egui::CornerRadius::same(crate::style::theme().radius_widget))
            .stroke(egui::Stroke::new(crate::style::theme().border_width, BORDER_SUBTLE))
            .inner_margin(egui::Margin::symmetric(8, 6))
    } else {
        egui::Frame::new().inner_margin(egui::Margin::symmetric(8, 6))
    };
    frame.show(ui, body);
}
