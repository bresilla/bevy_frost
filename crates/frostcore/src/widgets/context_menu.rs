//! Frost-styled context menu — a thin wrapper around egui's
//! `Popup::context_menu` that swaps in the frost glass frame
//! + accent border.
//!
//! Attach to any `egui::Response` (a tree row's body, a button,
//! an inspector cell) and the menu opens on right-click /
//! long-press, closes on outside click, same semantics as the
//! built-in. Inside the closure any frost widget works — use
//! [`wide_button`](super::button::wide_button) / other widgets
//! for menu items:
//!
//! ```ignore
//! let resp = tree_row(ui, ...);
//! context_menu_frost(&resp.body, accent, |ui| {
//!     if wide_button(ui, "Fly-to", accent).clicked() {
//!         fly_to(ctx);
//!         ui.close_menu();
//!     }
//!     if wide_button(ui, "Copy path", accent).clicked() { ... }
//! });
//! ```

use egui;

use crate::style::{
    glass_alpha_card, glass_alpha_window, glass_fill, radius, widget_border, BG_1_PANEL,
};

/// Attach a frost-styled context menu to `resp`. Opens on
/// secondary-click (right-click on desktop, long-press on touch),
/// closes on outside click. `accent` drives the border colour.
pub fn context_menu_frost(
    resp: &egui::Response,
    accent: egui::Color32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let frame = egui::Frame::new()
        .fill(glass_fill(BG_1_PANEL, accent, glass_alpha_window()))
        .stroke(egui::Stroke::new(1.0, widget_border(accent)))
        .corner_radius(egui::CornerRadius::same(radius::MD))
        .inner_margin(egui::Margin::symmetric(4, 4))
        .shadow(egui::epaint::Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: egui::Color32::from_black_alpha(120),
        });

    egui::Popup::context_menu(resp)
        .frame(frame)
        .show(|ui| {
            // Keep rows snug so the menu looks like a tidy list,
            // not a floating form — same cadence the foldable
            // section body uses.
            ui.spacing_mut().item_spacing.y = 0.0;
            let _ = glass_alpha_card(); // imported for future use; suppress unused
            add_contents(ui);
        });
}
