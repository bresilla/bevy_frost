//! Standalone foldable section — a chevron-on-header collapsible
//! frame with a frost-glass background. Used outside of the
//! pane/container system as a tiered group inside ad-hoc panels
//! (e.g. nested variant browsers, debug overlays).
//!
//! The pane/container path uses [`crate::container::Normal`]
//! instead — this helper is a thin shim for callers that just want
//! "an egui collapsible with frost paint".

use egui::collapsing_header::CollapsingState;

use crate::style::{
    glass_alpha_card, glass_fill, on_section, section_caps, section_fill, theme,
    widget_border,
};

const PAD_X: i8 = 8;
const PAD_Y: i8 = 4;

/// Render a frost-styled collapsible section. `id_salt` makes the
/// section's open/closed state distinct from siblings; `title` is
/// the header label; `default_open` is the initial state on first
/// paint.
pub fn section(
    ui: &mut egui::Ui,
    id_salt: &str,
    title: &str,
    accent: egui::Color32,
    default_open: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    let id = ui.id().with(("frost_section", id_salt));
    let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, default_open);

    let theme_now = theme();
    let frame = egui::Frame::new()
        .fill(glass_fill(section_fill(accent), accent, glass_alpha_card()))
        .stroke(egui::Stroke::new(theme_now.border_width, widget_border(accent)))
        .corner_radius(egui::CornerRadius::same(theme_now.radius_md))
        .inner_margin(egui::Margin::symmetric(PAD_X, PAD_Y));

    frame.show(ui, |ui| {
        // Header — chevron + UPPERCASE title, click-toggles open.
        let header_resp = ui
            .horizontal(|ui| {
                let openness = state.openness(ui.ctx());
                let chevron = if openness > 0.5 { "▾" } else { "▸" };
                let chevron_resp = ui.add(
                    egui::Label::new(
                        egui::RichText::new(chevron)
                            .color(on_section())
                            .size(11.0),
                    )
                    .sense(egui::Sense::click()),
                );
                let title_resp = ui.add(
                    egui::Label::new(section_caps(title, accent))
                        .sense(egui::Sense::click()),
                );
                chevron_resp.union(title_resp)
            })
            .inner;
        if header_resp.clicked() {
            state.toggle(ui);
        }
        state.show_body_indented(&header_resp, ui, |ui| {
            body(ui);
        });
    });
}
