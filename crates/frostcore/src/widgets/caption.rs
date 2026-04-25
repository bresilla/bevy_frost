//! Small-text bits: sub-captions inside sections, plus the
//! key-chip row used in help tables.

use egui;

use crate::style::{caption, space};

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
/// The chip's fill + text both flow through theme helpers — the
/// chip sits at the same brightness tier as the search field and
/// dropdown trigger (`track_fill`), with text picked by
/// `on_track()` so it's readable on either theme + accent combo.
pub fn keybinding_row(ui: &mut egui::Ui, keys: &str, action: &str) {
    flush_pending_separator(ui);
    let accent = ui.visuals().selection.stroke.color;
    ui.horizontal(|ui| {
        let chip = egui::RichText::new(keys)
            .monospace()
            .small()
            .color(crate::style::on_track());
        let frame = egui::Frame::new()
            .fill(crate::style::track_fill(accent))
            .inner_margin(egui::Margin::symmetric(5, 1))
            .corner_radius(egui::CornerRadius::same(crate::style::theme().radius_widget));
        frame.show(ui, |ui| ui.label(chip));
        ui.add_space(space::TIGHT);
        ui.add(
            egui::Label::new(
                egui::RichText::new(action)
                    .small()
                    .color(crate::style::on_section_dim()),
            )
            .truncate(),
        );
    });
}
