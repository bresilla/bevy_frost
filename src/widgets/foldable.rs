//! Foldable container — the main building block of every panel.
//!
//! A collapsible card with an accent-coloured UPPERCASE header:
//!
//!   [HEADER]                (accent UPPERCASE, chevron on the left)
//!   body goes here
//!
//! Uses egui's `CollapsingHeader` under the hood so the ▶ / ▼
//! chevron still animates. The frame pins to the panel's full
//! available width and hard-clips its body so unconstrained child
//! widgets can't push the card wider than the panel.
//!
//! When `unfoldable` siblings land (a container with the same frame
//! but no collapse header), they'll share this file's sizing
//! constants.

use bevy_egui::egui;

use crate::style::{
    glass_alpha_card, glass_fill, radius, section_caps, thin_divider, widget_border, BG_2_RAISED,
};

/// Horizontal inner padding inside the container, in px.
pub const PAD_X: i8 = 4;
/// Vertical inner padding inside the container, in px.
pub const PAD_Y: i8 = 3;
/// Pixels reserved inside the panel for the card's horizontal
/// padding + stroke. Must match `2 * PAD_X + stroke_width`.
pub const OUTER_INSET: f32 = (PAD_X as f32) * 2.0 + 2.0;

pub fn section(
    ui: &mut egui::Ui,
    id_salt: &str,
    title: &str,
    accent: egui::Color32,
    default_open: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    let full_w = ui.available_width();
    let inner_w = (full_w - OUTER_INSET).max(0.0);
    egui::Frame::new()
        .fill(glass_fill(BG_2_RAISED, accent, glass_alpha_card()))
        .corner_radius(egui::CornerRadius::same(radius::MD))
        .stroke(egui::Stroke::new(1.0, widget_border(accent)))
        .inner_margin(egui::Margin::symmetric(PAD_X, PAD_Y))
        .show(ui, |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(inner_w, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(inner_w);
                    // Hard clip: anything trying to draw past
                    // `inner_w` on the right (long text, unconstrained
                    // DragValue, whatever) gets visually cut instead
                    // of bulging the card. Intersected with the
                    // current clip so parent scroll / window clips
                    // still apply.
                    let clip = ui.clip_rect().intersect(egui::Rect::from_min_size(
                        ui.min_rect().min,
                        egui::vec2(inner_w, f32::INFINITY),
                    ));
                    ui.set_clip_rect(clip);
                    egui::CollapsingHeader::new(section_caps(title, accent))
                        .id_salt(id_salt)
                        .default_open(default_open)
                        // `.show_unindented` — the frame already pads
                        // the body, so the default 14 px indent would
                        // double up and misalign rows across panels.
                        .show_unindented(ui, |ui| {
                            // Every widget module here carries its own
                            // trailing separator; egui's default 4 px
                            // item spacing would sit on TOP of that and
                            // make the body feel airy. Zero it out so
                            // the separator's own 1+1+1 px is the only
                            // inter-row gap.
                            ui.spacing_mut().item_spacing.y = 0.0;
                            // Hairline under the section title, then
                            // breathing space before the first body
                            // element so the title block reads as
                            // visually distinct from the content it
                            // heads.
                            thin_divider(ui);
                            ui.add_space(6.0);
                            body(ui);
                        });
                },
            );
        });
}
