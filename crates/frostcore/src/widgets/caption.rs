//! Small-text bits: sub-captions inside sections, plus the
//! key-chip row used in help tables.

use egui;

use crate::style::{caption, radius, space, TEXT_SECONDARY};

use super::shared::flush_pending_separator;

/// Subtle caption text (italic, small, tertiary colour). Use
/// between related sub-blocks inside a section to describe what
/// follows.
pub fn sub_caption(ui: &mut egui::Ui, text: &str) {
    flush_pending_separator(ui);
    ui.label(caption(text));
}

/// Key-chip + action-label row, used in "Keys" help sections.
/// Action text truncates with `…` if the full line would overflow.
pub fn keybinding_row(ui: &mut egui::Ui, keys: &str, action: &str) {
    flush_pending_separator(ui);
    ui.horizontal(|ui| {
        let chip = egui::RichText::new(keys)
            .monospace()
            .small()
            .color(ui.visuals().text_color());
        let frame = egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(egui::Margin::symmetric(5, 1))
            .corner_radius(egui::CornerRadius::same(radius::WIDGET));
        frame.show(ui, |ui| ui.label(chip));
        ui.add_space(space::TIGHT);
        ui.add(
            egui::Label::new(
                egui::RichText::new(action).small().color(TEXT_SECONDARY),
            )
            .truncate(),
        );
    });
}
