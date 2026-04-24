//! Node-graph integration — thin glue around [`egui_snarl`] so graph
//! widgets inherit the frost palette and border language without
//! every consumer having to hand-tune a `SnarlStyle`.
//!
//! Two pieces of surface:
//!
//! * [`frost_snarl_style`] — builds a [`SnarlStyle`] configured with
//!   frost's `BG_*` / `widget_border` / accent colours, the same
//!   corner radius as [`section`](crate::widgets::foldable::section),
//!   and a pin/wire width that matches the border stroke. Pass the
//!   returned style straight into
//!   [`SnarlWidget::style`](egui_snarl::ui::SnarlWidget::style).
//! * `pub use egui_snarl` re-export — callers don't need a second
//!   direct dep. `use bevy_frost::snarl::{Snarl, SnarlViewer,
//!   SnarlWidget, NodeId, InPin, OutPin, ...};` lands the full
//!   upstream surface.
//!
//! Drop the whole thing into any section body:
//!
//! ```ignore
//! section(ui, "graph", "Graph", accent, true, |ui| {
//!     SnarlWidget::new()
//!         .id_salt("my_graph")
//!         .style(frost_snarl_style(accent))
//!         .min_size(egui::vec2(320.0, 260.0))
//!         .show(&mut state.graph, &mut state.viewer, ui);
//! });
//! ```

use bevy_egui::egui;

pub use egui_snarl::{
    ui::{
        AnyPins, BackgroundPattern, NodeLayout, PinInfo, PinPlacement, PinShape,
        SnarlPin, SnarlStyle, SnarlViewer, SnarlWidget,
    },
    InPin, InPinId, NodeId, OutPin, OutPinId, Snarl,
};

use crate::style::{
    glass_alpha_card, glass_alpha_window, glass_fill, radius, widget_border, BG_1_PANEL,
    BG_2_RAISED,
};

/// Build a [`SnarlStyle`] that inherits the frost palette + border
/// language. Call per-frame with the current accent so the graph
/// re-tints when the user swaps accent colour (the same way every
/// other frost surface does).
///
/// What the returned style pins down:
///
/// * **Node frame** — `BG_2_RAISED` glass fill + `widget_border`
///   stroke + `radius::MD` corner, matching
///   [`section`](crate::widgets::foldable::section) so nodes look
///   like first-class frost surfaces.
/// * **Background** — `BG_1_PANEL` glass fill behind everything,
///   the same colour a floating window uses, so the graph canvas
///   sits cleanly in an editor panel.
/// * **Pins / wires** — `widget_border(accent)` + stroke width 1 px,
///   identical to every other widget's edge.
///
/// Everything else stays at the library default so scroll / zoom /
/// selection interactions remain familiar to upstream users.
pub fn frost_snarl_style(accent: egui::Color32) -> SnarlStyle {
    let node_frame = egui::Frame::new()
        .fill(glass_fill(BG_2_RAISED, accent, glass_alpha_card()))
        .stroke(egui::Stroke::new(1.0, widget_border(accent)))
        .corner_radius(egui::CornerRadius::same(radius::MD))
        .inner_margin(egui::Margin::symmetric(8, 4));

    // Header frame — same corners + border but transparent fill so
    // it layers on top of the node's own fill without doubling the
    // opacity. Matches how foldable sections show their title band.
    let header_frame = egui::Frame::new()
        .fill(egui::Color32::TRANSPARENT)
        .stroke(egui::Stroke::NONE)
        .corner_radius(egui::CornerRadius::same(radius::MD))
        .inner_margin(egui::Margin::symmetric(6, 3));

    let bg_fill = glass_fill(BG_1_PANEL, accent, glass_alpha_window());

    SnarlStyle {
        node_frame: Some(node_frame),
        header_frame: Some(header_frame),
        bg_frame: Some(
            egui::Frame::new()
                .fill(bg_fill)
                .stroke(egui::Stroke::new(1.0, widget_border(accent)))
                .corner_radius(egui::CornerRadius::same(radius::LG))
                .inner_margin(egui::Margin::same(2)),
        ),
        pin_fill: Some(accent),
        pin_stroke: Some(egui::Stroke::new(1.0, widget_border(accent))),
        wire_width: Some(1.5),
        wire_style: None,
        downscale_wire_frame: Some(true),
        upscale_wire_frame: Some(true),
        ..SnarlStyle::new()
    }
}
