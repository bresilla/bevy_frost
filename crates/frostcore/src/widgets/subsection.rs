//! Sub-section — a nested collapsible container that sits *inside*
//! a [`super::foldable::section`]. Same visual language (frame +
//! border + chevron) as the outer section, just distinct enough to
//! read as "nested": brighter fill (sits atop the card), tighter
//! corner radius, body indented by the chevron column so text inside
//! never starts further left than the parent's dropdown.
//!
//! Header renders as
//!
//! ```text
//!   ▸  Title                       ← chevron aligns with THIS line
//!      subtitle in tiny italic     ← optional, always visible
//! ```
//!
//! — the chevron is horizontally co-lined with the title only, not
//! centred on the (title + subtitle) block, so it tracks the label
//! a reader's eye naturally lands on.
//!
//! Appends a trailing [`super::row_separator`] so stacks of
//! subsections pick up the same divider-between-modules rhythm as
//! the rest of the panel widgets.

use egui;

use crate::style::{
    font, glass_alpha_group, glass_fill, radius, widget_border, BG_3_HOVER, TEXT_DISABLED,
    TEXT_PRIMARY,
};

use super::foldable::{OUTER_INSET, PAD_X, PAD_Y};
use super::shared::widget_separator;

/// Left indent applied to the body so it steps in from the chevron
/// column — reinforces the "each layer goes narrower" look.
pub const SUBSECTION_BODY_INDENT: f32 = 6.0;

/// Alpha applied on top of the shared [`widget_border`] for nested
/// sub-sections. Lower than 255 so it doesn't compete with the
/// parent section's stroke, but high enough to be clearly visible.
const BORDER_ALPHA: u8 = 160;

/// Nested collapsible container.
pub fn subsection(
    ui: &mut egui::Ui,
    id_salt: &str,
    title: &str,
    subtitle: Option<&str>,
    accent: egui::Color32,
    default_open: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    let full_w = ui.available_width();
    let inner_w = (full_w - OUTER_INSET).max(0.0);

    // Same accent-tinted recipe as the parent section's stroke,
    // re-alpha'd so nested frames read softer.
    let shared = widget_border(accent);
    let border = egui::Color32::from_rgba_unmultiplied(
        shared.r(),
        shared.g(),
        shared.b(),
        BORDER_ALPHA,
    );

    egui::Frame::new()
        .fill(glass_fill(
            BG_3_HOVER,
            egui::Color32::TRANSPARENT,
            glass_alpha_group(),
        ))
        .corner_radius(egui::CornerRadius::same(radius::WIDGET))
        .stroke(egui::Stroke::new(1.0, border))
        .inner_margin(egui::Margin::symmetric(PAD_X, PAD_Y))
        .show(ui, |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(inner_w, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(inner_w);
                    let clip = ui.clip_rect().intersect(egui::Rect::from_min_size(
                        ui.min_rect().min,
                        egui::vec2(inner_w, f32::INFINITY),
                    ));
                    ui.set_clip_rect(clip);

                    // Kill vertical between-widget spacing so title,
                    // subtitle, and body stack hair-tight. Each widget
                    // inside the body still controls its own trailing
                    // separator so rows don't touch.
                    ui.spacing_mut().item_spacing.y = 0.0;

                    let id = ui.make_persistent_id(id_salt);
                    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                        ui.ctx(),
                        id,
                        default_open,
                    );

                    // Header: chevron pinned to the TOP of the row
                    // (via `horizontal_top` — `horizontal` centres, so
                    // the chevron would drift between the two text
                    // lines when a subtitle is present). Title +
                    // subtitle live in a nested vertical block with
                    // `item_spacing.y = 0` so the two labels hug hair-
                    // tight against each other.
                    ui.horizontal_top(|ui| {
                        state.show_toggle_button(
                            ui,
                            egui::collapsing_header::paint_default_icon,
                        );
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            let title_resp = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(title)
                                        .strong()
                                        .size(font::BODY)
                                        .color(TEXT_PRIMARY),
                                )
                                .sense(egui::Sense::click()),
                            );
                            if title_resp.clicked() {
                                state.toggle(ui);
                            }
                            if let Some(sub) = subtitle {
                                ui.label(
                                    egui::RichText::new(sub)
                                        .size(font::CAPTION - 1.0)
                                        .italics()
                                        .color(TEXT_DISABLED),
                                );
                            }
                        });
                    });

                    state.show_body_unindented(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.horizontal(|ui| {
                            ui.add_space(SUBSECTION_BODY_INDENT);
                            ui.vertical(|ui| {
                                body(ui);
                            });
                        });
                    });
                },
            );
        });

    // Trailing divider so stacked subsections breathe and share the
    // "every module ends with a separator" rhythm.
    widget_separator(ui);
}
