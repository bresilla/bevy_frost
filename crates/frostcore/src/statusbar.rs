//! Edge-anchored status strip — same glass chrome as
//! [`floating_window`](crate::floating::floating_window), but:
//!
//! * No title bar.
//! * No resize handles.
//! * Content-driven height; caller-controlled width.
//! * No [`PaneBuilder`](crate::floating::PaneBuilder) wrapper —
//!   the body gets a plain `&mut egui::Ui`, because a status
//!   bar is usually a single horizontal chain of labels and
//!   wrapping each label in a `.section(...)` would defeat the
//!   point.
//!
//! Typical placement: anchored to `LEFT_BOTTOM` or `RIGHT_BOTTOM`
//! for a classic editor status strip under a 3D viewport. Can also
//! be anchored `*_TOP` for header-style overlays; the glass frame
//! reads the same either way.
//!
//! ```ignore
//! statusbar(ctx, "viewer_status", egui::Align2::LEFT_BOTTOM, accent, |ui| {
//!     ui.label(format!("prims: {count}"));
//!     ui.separator();
//!     ui.label(format!("fps: {fps:.0}"));
//! });
//! ```

use egui;

use crate::style::{glass_alpha_window, glass_fill, radius, widget_border, BG_1_PANEL};

/// Distance from the screen edge to the strip — matches the
/// floating window's own edge gap so the two sit in the same
/// gutter when both are anchored on the same side.
const EDGE_GAP: f32 = 8.0;
/// Ribbon button side length (kept in sync with
/// `ribbon::paint::SIDE_BTN_SIZE`).
const SIDE_BTN_SIZE: f32 = 34.0;
/// Gap between the ribbon rail and anything that sits on the
/// rail-facing side.
const RAIL_PANEL_GAP: f32 = 6.0;
/// Inset from a rail-side edge — clears the ribbon buttons so
/// the status bar doesn't paint ON TOP of them. Matches
/// `floating_window`'s `side_inset`, so a pane + a status bar
/// anchored on the same side line up flush with each other.
const SIDE_INSET: f32 = EDGE_GAP + SIDE_BTN_SIZE + RAIL_PANEL_GAP;

/// Render a status strip anchored to one of the four corners of
/// `ctx.content_rect()`. `add_contents` receives a plain
/// `egui::Ui` already laid out horizontally — add labels,
/// separators, icons, whatever fits on one line.
pub fn statusbar(
    ctx: &egui::Context,
    id: &'static str,
    anchor: egui::Align2,
    accent: egui::Color32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    // Left/right anchors clear the ribbon rail via `SIDE_INSET`;
    // top/bottom centre anchors use a small edge gap. Matches
    // `floating_window`'s offset table exactly so a status bar +
    // a pane anchored to the same corner sit in the same gutter
    // instead of overlapping each other or the rail.
    let offset = match anchor {
        egui::Align2::LEFT_TOP => egui::vec2(SIDE_INSET, EDGE_GAP),
        egui::Align2::LEFT_CENTER => egui::vec2(SIDE_INSET, 0.0),
        egui::Align2::LEFT_BOTTOM => egui::vec2(SIDE_INSET, -EDGE_GAP),
        egui::Align2::RIGHT_TOP => egui::vec2(-SIDE_INSET, EDGE_GAP),
        egui::Align2::RIGHT_CENTER => egui::vec2(-SIDE_INSET, 0.0),
        egui::Align2::RIGHT_BOTTOM => egui::vec2(-SIDE_INSET, -EDGE_GAP),
        egui::Align2::CENTER_TOP => egui::vec2(0.0, EDGE_GAP),
        egui::Align2::CENTER_BOTTOM => egui::vec2(0.0, -EDGE_GAP),
        _ => egui::vec2(SIDE_INSET, EDGE_GAP),
    };

    let frame = egui::Frame::new()
        .fill(glass_fill(BG_1_PANEL, accent, glass_alpha_window()))
        .stroke(egui::Stroke::new(1.0, widget_border(accent)))
        .corner_radius(egui::CornerRadius::same(radius::MD))
        .inner_margin(egui::Margin::symmetric(8, 3));

    egui::Area::new(egui::Id::new(id))
        .anchor(anchor, offset)
        .order(egui::Order::Middle)
        .interactable(true)
        .show(ctx, |ui| {
            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    add_contents(ui);
                });
            });
        });
}
