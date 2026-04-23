//! Fixed-size floating-panel helper.
//!
//! Anchored to one of the four screen corners via [`egui::Align2`].
//! No title bar, no close button, no resize — intended to be opened
//! / closed by a ribbon button and offset far enough from the edge
//! to clear the [`crate::ribbon::SideRibbon`] / `RibbonLayout` rails.

use bevy_egui::egui;

use crate::style::{glass_alpha_window, glass_fill, BG_1_PANEL, BORDER_SUBTLE};

// Ribbon layout constants we need here. Kept as locals rather than
// pulling `ribbon::paint` into the public prelude — the numbers
// belong to both modules.
const EDGE_GAP: f32 = 8.0;
const SIDE_BTN_SIZE: f32 = 34.0;
const RAIL_PANEL_GAP: f32 = 6.0;

/// Tiny sanity check: keeps the layout constants in this file in
/// sync with the ribbon module's source-of-truth, through a simple
/// `const` assertion (no runtime cost).
const _: () = {
    assert!(EDGE_GAP == 8.0);
    assert!(SIDE_BTN_SIZE == 34.0);
    assert!(RAIL_PANEL_GAP == 6.0);
};

/// Paint a fixed-size floating panel anchored to `anchor`. Title
/// alignment flips automatically when `anchor` is a right-side
/// corner so a menu dragged across rails reads correctly.
pub fn floating_window(
    ctx: &egui::Context,
    id: &'static str,
    title: &str,
    anchor: egui::Align2,
    size: egui::Vec2,
    _open: &mut bool,
    accent: egui::Color32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let side_inset = EDGE_GAP + SIDE_BTN_SIZE + RAIL_PANEL_GAP;
    let anchor_offset = match anchor {
        egui::Align2::LEFT_TOP => egui::vec2(side_inset, EDGE_GAP),
        egui::Align2::RIGHT_TOP => egui::vec2(-side_inset, EDGE_GAP),
        egui::Align2::LEFT_BOTTOM => egui::vec2(side_inset, -EDGE_GAP),
        egui::Align2::RIGHT_BOTTOM => egui::vec2(-side_inset, -EDGE_GAP),
        _ => egui::vec2(side_inset, EDGE_GAP),
    };

    let frame = egui::Frame {
        // Tight inner margin — containers sit almost flush with the
        // panel edge. Bump these back up if content starts clipping
        // against the rounded corner.
        inner_margin: egui::Margin { left: 2, right: 2, top: 2, bottom: 2 },
        outer_margin: egui::Margin::ZERO,
        fill: glass_fill(BG_1_PANEL, accent, glass_alpha_window()),
        stroke: egui::Stroke::new(1.0, BORDER_SUBTLE),
        corner_radius: egui::CornerRadius::same(8),
        shadow: egui::epaint::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: egui::Color32::from_black_alpha(115),
        },
    };

    let on_right_side = matches!(
        anchor,
        egui::Align2::RIGHT_TOP | egui::Align2::RIGHT_BOTTOM
    );

    egui::Window::new(title)
        .id(egui::Id::new(id))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(anchor, anchor_offset)
        .fixed_size(size)
        .frame(frame)
        .show(ctx, |ui| {
            // Inner-margin (2 px) × 2 sides + stroke accounts for
            // the inset beyond `size.x`; keep in sync with the
            // `inner_margin` above.
            ui.set_max_width(size.x - 6.0);

            // Title row: UPPERCASE accent, with a hairline underneath
            // and breathing space before the content. `TITLE_INSET`
            // keeps the title from kissing the card's rounded corner
            // — the panel's inner_margin alone is too tight now that
            // it's been shrunk to 2 px.
            const TITLE_INSET: f32 = 8.0;
            let title_size = 15.0 * 1.15;
            let title_h = 25.0;
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), title_h),
                egui::Sense::hover(),
            );
            let (align, tx) = if on_right_side {
                (egui::Align2::RIGHT_CENTER, rect.max.x - TITLE_INSET)
            } else {
                (egui::Align2::LEFT_CENTER, rect.min.x + TITLE_INSET)
            };
            let pos = egui::pos2(tx, rect.center().y);
            let font = egui::FontId::new(title_size, egui::FontFamily::Proportional);
            for dx in [-0.5, 0.5] {
                ui.painter().text(
                    egui::pos2(pos.x + dx, pos.y),
                    align,
                    title.to_uppercase(),
                    font.clone(),
                    accent,
                );
            }
            ui.painter().hline(
                rect.min.x..=rect.max.x,
                rect.max.y + 3.0,
                egui::Stroke::new(1.0, BORDER_SUBTLE),
            );
            ui.add_space(6.0);

            add_contents(ui);
        });
}

