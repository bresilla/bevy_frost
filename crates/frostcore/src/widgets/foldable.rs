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

use egui;

use crate::style::{
    glass_alpha_card, glass_fill, radius, section_caps, thin_divider, widget_border, BG_2_RAISED,
};

use super::shared::flush_pending_separator;

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
    let _ = section_tracked(ui, id_salt, title, accent, default_open, body);
}

/// What [`section_tracked`] reports back to the pane: the egui id
/// under which the section's `CollapsingState` lives, the full outer
/// rect (frame included), and the header's drag-aware response (a
/// `click_and_drag` Sense — short clicks toggle the chevron, drags
/// drive the pane's reorder gesture).
pub(crate) struct SectionTrack {
    pub state_id: egui::Id,
    pub outer_rect: egui::Rect,
    pub header_response: egui::Response,
}

/// Same visual recipe as [`section`] but with a custom-painted
/// header (chevron + UPPERCASE title) backed by a single
/// `click_and_drag` interaction zone. That's what lets the pane
/// host both fold-toggle (short click) and drag-reorder (sustained
/// motion past egui's drag threshold) on the same header strip
/// without the two senses fighting each other — the long-standing
/// "I can't click to fold once drag is wired up" problem.
///
/// Returns the section's outer rect, the underlying
/// `CollapsingState` id (so the pane's auto-fold pass can force the
/// section closed from outside), and the header's response. Body
/// rendering and frame styling are unchanged from `section`.
pub(crate) fn section_tracked(
    ui: &mut egui::Ui,
    id_salt: &str,
    title: &str,
    accent: egui::Color32,
    default_open: bool,
    body: impl FnOnce(&mut egui::Ui),
) -> SectionTrack {
    flush_pending_separator(ui);
    let full_w = ui.available_width();
    let inner_w = (full_w - OUTER_INSET).max(0.0);
    let outer_top = ui.cursor().min;

    // Use a frost-managed state id so the pane can read / write the
    // open flag from outside without having to mirror egui's internal
    // `make_persistent_id` chain.
    let state_id = ui.make_persistent_id(("frost_section", id_salt));
    let mut state =
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), state_id, default_open);

    let mut captured_header_response: Option<egui::Response> = None;

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
                    let clip = ui.clip_rect().intersect(egui::Rect::from_min_size(
                        ui.min_rect().min,
                        egui::vec2(inner_w, f32::INFINITY),
                    ));
                    ui.set_clip_rect(clip);

                    // Custom header — full-width rect with a single
                    // `click_and_drag` sense so short clicks toggle
                    // the chevron and drags drive reorder. Chevron +
                    // UPPERCASE title are painted on top of the
                    // interaction rect so the rect spans the entire
                    // strip (no thin chevron-only hit zone).
                    const HEADER_H: f32 = 22.0;
                    const CHEVRON_W: f32 = 16.0;
                    let header_w = ui.available_width();
                    let (header_rect, resp) = ui.allocate_exact_size(
                        egui::vec2(header_w, HEADER_H),
                        egui::Sense::click_and_drag(),
                    );

                    let openness = state.openness(ui.ctx());
                    let chevron_rect = egui::Rect::from_min_size(
                        header_rect.min,
                        egui::vec2(CHEVRON_W, HEADER_H),
                    );
                    paint_chevron(ui, chevron_rect, openness, accent);

                    // Render the header text via the `section_caps`
                    // RichText recipe (uppercase + strong + accent) so
                    // it matches what `egui::CollapsingHeader` produced
                    // here previously. RichText -> WidgetText -> Galley.
                    let title_widget = egui::WidgetText::from(section_caps(title, accent));
                    let title_galley = title_widget.into_galley(
                        ui,
                        Some(egui::TextWrapMode::Extend),
                        f32::INFINITY,
                        egui::TextStyle::Body,
                    );
                    let title_pos = egui::pos2(
                        chevron_rect.right() + 2.0,
                        header_rect.center().y - title_galley.size().y * 0.5,
                    );
                    ui.painter().galley(title_pos, title_galley, accent);

                    if resp.clicked() {
                        state.toggle(ui);
                    }

                    captured_header_response = Some(resp);

                    state.show_body_unindented(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        thin_divider(ui);
                        ui.add_space(6.0);
                        body(ui);
                    });
                },
            );
        });

    let outer_bottom = ui.cursor().min.y;
    let outer_rect = egui::Rect::from_min_max(
        outer_top,
        egui::pos2(outer_top.x + full_w, outer_bottom),
    );
    SectionTrack {
        state_id,
        outer_rect,
        header_response: captured_header_response.expect("header always allocated"),
    }
}

/// Paint a chevron triangle at `rect`, rotated by `openness`
/// (0 = closed/▶, 1 = open/▼). Mirrors egui's
/// `paint_default_icon` shape recipe but takes an explicit rect +
/// tint so the pane can place it inside our custom header strip.
fn paint_chevron(ui: &mut egui::Ui, rect: egui::Rect, openness: f32, tint: egui::Color32) {
    let inner = egui::Rect::from_center_size(
        rect.center(),
        egui::vec2(rect.width(), rect.height()) * 0.5,
    );
    let mut points = [inner.left_top(), inner.right_top(), inner.center_bottom()];
    use std::f32::consts::TAU;
    let rotation = egui::emath::Rot2::from_angle(egui::lerp(-TAU / 4.0..=0.0, openness));
    for p in &mut points {
        *p = inner.center() + rotation * (*p - inner.center());
    }
    ui.painter().add(egui::Shape::convex_polygon(
        points.to_vec(),
        tint,
        egui::Stroke::NONE,
    ));
}
