//! Labelled rows — the label + control pair primitive that makes
//! every panel column-align on one vertical line.
//!
//! The left cell is **fixed width** and truncates overlong text
//! with `…` so the control column stays aligned across rows
//! regardless of which row has the longest label.

use egui;

use super::layout::{dual_pane, dual_pane_labelled};
use super::shared::{flush_pending_separator, widget_separator};
use crate::style::body_label;

/// Width of the label column. Picked to fit every typical label at
/// 11 pt body size; anything longer truncates with an ellipsis.
pub const LABEL_COL_WIDTH: f32 = 140.0;

/// Label on the left, control(s) right-aligned on the right.
pub fn labelled_row(
    ui: &mut egui::Ui,
    label: &str,
    right: impl FnOnce(&mut egui::Ui),
) {
    labelled_row_custom_left(
        ui,
        |ui| {
            ui.add(egui::Label::new(body_label(label)).truncate());
        },
        right,
    );
}

/// Same row skeleton as [`labelled_row`] — fixed-width left cell,
/// right-aligned right cell with a strict max width — but the left
/// cell is rendered by a caller-supplied closure. Used by rows that
/// want a coloured glyph in the label slot (e.g. axis rows), a chip,
/// or any composite label.
pub fn labelled_row_custom_left(
    ui: &mut egui::Ui,
    left: impl FnOnce(&mut egui::Ui),
    right: impl FnOnce(&mut egui::Ui),
) {
    flush_pending_separator(ui);
    ui.horizontal(|ui| {
        let row_h = ui.spacing().interact_size.y;
        ui.allocate_ui_with_layout(
            egui::vec2(LABEL_COL_WIDTH, row_h),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                left(ui);
            },
        );
        let remaining = ui.available_width().max(0.0);
        ui.allocate_ui_with_layout(
            egui::vec2(remaining, row_h),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.set_max_width(remaining);
                right(ui);
            },
        );
    });
}

/// Read-only numeric/text module: label in the 70 % left pane,
/// monospaced value in the 30 % right pane, trailing separator
/// under the row. Same layout language as [`super::toggle`].
pub fn readout_row(ui: &mut egui::Ui, label: &str, value: &str) {
    flush_pending_separator(ui);
    dual_pane_labelled(ui, label, |ui| {
        ui.label(
            egui::RichText::new(value)
                .monospace()
                .small()
                .color(crate::style::on_section()),
        );
    });
    widget_separator(ui);
}

/// Label on the left, a horizontal stack of chips on the right.
/// Use when a row needs multiple short categorical tags rather
/// than a single control — "animated: 12 (rotX, rotY, translate)"
/// reads far better rendered as `animated  |  [rotX] [rotY]
/// [translate]`.
///
/// Each entry of `badges` becomes a [`chip`](super::chip::chip),
/// rendered right-to-left so they hug the right gutter in the
/// same order as the slice.
pub fn badge_row(
    ui: &mut egui::Ui,
    label: &str,
    badges: &[&str],
    accent: egui::Color32,
) {
    flush_pending_separator(ui);
    labelled_row(ui, label, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            // Right-to-left layout (provided by the outer cell)
            // paints elements from the right edge inward. Iterate
            // in reverse so the caller-facing order matches
            // left-to-right reading.
            for b in badges.iter().rev() {
                super::chip::chip(ui, b, accent);
            }
        });
    });
    widget_separator(ui);
}

/// Coloured-glyph + value module (e.g. `X  +1.234 m` in `AXIS_X`).
/// 70 / 30 split + trailing separator — same language as every other
/// widget module.
pub fn axis_readout_row(
    ui: &mut egui::Ui,
    glyph: &str,
    glyph_color: egui::Color32,
    value: &str,
) {
    flush_pending_separator(ui);
    dual_pane(
        ui,
        |ui| {
            ui.label(
                egui::RichText::new(glyph)
                    .strong()
                    .monospace()
                    .small()
                    .color(glyph_color),
            );
        },
        |ui| {
            ui.label(
                egui::RichText::new(value)
                    .monospace()
                    .small()
                    .color(crate::style::on_section()),
            );
        },
    );
    widget_separator(ui);
}
