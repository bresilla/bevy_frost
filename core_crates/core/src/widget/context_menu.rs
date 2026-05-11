//! Frost-styled context menu — thin wrapper around egui's
//! `Popup::context_menu` with the frost glass frame + accent
//! border. Attach to any `egui::Response` (tree row body, button,
//! inspector cell) and the menu opens on right-click / long-press.

use crate::style::{glass_alpha_card, glass_alpha_window, glass_fill, popup_fill, theme, widget_border};

/// Attach a frost-styled context menu to `resp`. Opens on
/// secondary-click, closes on outside click. `accent` drives the
/// border colour and glass-tint of the popup.
pub fn context_menu_frost(
    resp: &egui::Response,
    accent: egui::Color32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let frame = egui::Frame::new()
        .fill(glass_fill(popup_fill(accent), accent, glass_alpha_window()))
        .stroke(egui::Stroke::new(theme().border_width, widget_border(accent)))
        .corner_radius(egui::CornerRadius::same(theme().radius_md))
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
            ui.spacing_mut().item_spacing.y = 0.0;
            let _ = glass_alpha_card();
            add_contents(ui);
        });
}
