//! Row-level layout primitives for container content.
//!
//! Two arrangements:
//!
//! * **[`dual_pane`]** — horizontal split, **70 % description /
//!   30 % control**. Use for "label ↔ compact widget" pairs
//!   (toggles, colour pickers, combo boxes, radios). The label
//!   hugs the left gutter and the widget hugs the right gutter,
//!   so stacks of rows column-align on both sides.
//!
//! * **[`stacked_pane`]** — vertical stack, caption on top, widget
//!   on the full row below. Use for widgets that **want the full
//!   row width** (sliders, progress bars) — a 30 % right pane
//!   would leave them cramped.
//!
//! Buttons are their own single-row form (see
//! [`super::button::wide_button`] / [`super::button::card_button`])
//! — no label column, they span the row.
//!
//! New arrangements (tri-pane, quad-pane, named-group, …) land
//! next to these two as we need them.

use egui;

use crate::style::body_label;

// ─── dual_pane — 70 / 30 horizontal ────────────────────────────────

/// Fraction of the row the left (description) pane consumes. The
/// remainder goes to the right (control) pane.
pub const DUAL_PANE_LEFT_FRACTION: f32 = 0.70;

/// Split the available row width `70 / 30` — left pane for the
/// description, right pane for a compact widget (toggle, radio,
/// colour picker). Returns whatever the `right` closure returns so
/// widget modules can propagate their `Response`.
///
/// Not the right primitive for sliders / progress bars — those
/// want the full row. Use [`stacked_pane`] for those.
pub fn dual_pane<R>(
    ui: &mut egui::Ui,
    left: impl FnOnce(&mut egui::Ui),
    right: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let mut result: Option<R> = None;
    ui.horizontal(|ui| {
        let full_w = ui.available_width();
        let left_w = (full_w * DUAL_PANE_LEFT_FRACTION).max(0.0);
        let row_h = ui.spacing().interact_size.y;

        ui.allocate_ui_with_layout(
            egui::vec2(left_w, row_h),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_max_width(left_w);
                left(ui);
            },
        );

        let remaining = ui.available_width().max(0.0);
        ui.allocate_ui_with_layout(
            egui::vec2(remaining, row_h),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.set_max_width(remaining);
                result = Some(right(ui));
            },
        );
    });
    // Safe: the horizontal closure always runs synchronously and
    // always invokes `right`.
    result.expect("dual_pane: right closure must run")
}

/// Convenience over [`dual_pane`] — describe the left side with a
/// plain string, rendered in the frost body-label style (small +
/// truncating). Passes through the inner `R` from `right`.
pub fn dual_pane_labelled<R>(
    ui: &mut egui::Ui,
    label: &str,
    right: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    dual_pane(
        ui,
        |ui| {
            ui.add(egui::Label::new(body_label(label)).truncate());
        },
        right,
    )
}

// ─── stacked_pane — caption above, widget on full row ──────────────

/// Two-line arrangement: caption on top, widget on the full row
/// below. Returns `R` from the `body` closure so widget modules
/// can propagate their `Response`.
pub fn stacked_pane<R>(
    ui: &mut egui::Ui,
    caption: impl FnOnce(&mut egui::Ui),
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let mut result: Option<R> = None;
    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
        let w = ui.available_width();
        ui.set_max_width(w);
        caption(ui);
        result = Some(body(ui));
    });
    result.expect("stacked_pane: body closure must run")
}

/// Convenience over [`stacked_pane`] — describe the top with a
/// plain string, rendered in the frost body-label style.
pub fn stacked_pane_labelled<R>(
    ui: &mut egui::Ui,
    label: &str,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    stacked_pane(
        ui,
        |ui| {
            ui.add(egui::Label::new(body_label(label)).truncate());
        },
        body,
    )
}
