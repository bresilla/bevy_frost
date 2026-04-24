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

use std::hash::Hash;

use egui;

pub use egui_snarl::{
    ui::{
        AnyPins, BackgroundPattern, Grid, NodeLayout, PinInfo, PinPlacement, PinShape,
        SnarlPin, SnarlStyle, SnarlViewer, SnarlWidget,
    },
    InPin, InPinId, NodeId, OutPin, OutPinId, Snarl,
};

use crate::maximize::maximizable;
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

    // Grid stroke — accent-tinted and semi-transparent so the
    // canvas pattern stays clearly below the graph content without
    // competing. Alpha 60 is similar to the hairline divider under
    // container titles; the grid reads as "there but quiet".
    let grid_stroke = egui::Stroke::new(
        1.0,
        egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 60),
    );

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
        // Accent-tinted semi-transparent grid across the canvas —
        // `BackgroundPattern::Grid` is egui-snarl's default, but we
        // explicitly set it so the stroke below (which we also set)
        // always applies.
        bg_pattern: Some(BackgroundPattern::Grid(Grid::default())),
        bg_pattern_stroke: Some(grid_stroke),
        // Pins + wires: flat white for now. Each node will override
        // its own pin colour via `PinInfo::with_fill` later, and
        // wires follow the pin's fill by default. Accent stays
        // reserved for panel chrome / borders so the graph itself
        // doesn't change colour every time the user re-tints the
        // theme.
        pin_fill: Some(egui::Color32::WHITE),
        pin_stroke: Some(egui::Stroke::new(1.0, egui::Color32::from_gray(60))),
        wire_width: Some(1.5),
        wire_style: None,
        downscale_wire_frame: Some(true),
        upscale_wire_frame: Some(true),
        ..SnarlStyle::new()
    }
}

/// Render the graph widget with a built-in **maximise / restore**
/// toggle in its top-left corner.
///
/// The maximise state is scoped to THIS graph — clicking the icon
/// lifts only the graph into a full-window overlay, leaving the
/// floating panel and any outer container the caller placed it in
/// completely untouched. Click again to restore.
///
/// When maximised the caller-supplied `min_size` still allocates
/// in-place so the section / panel layout doesn't collapse while
/// the graph is "gone" to the overlay — the hole is filled with a
/// small "(maximised)" caption.
///
/// Use this instead of calling [`SnarlWidget::new().show`] directly
/// whenever you want the fullscreen affordance. Otherwise, the
/// plain `SnarlWidget` route keeps working.
pub fn frost_snarl<T, V: SnarlViewer<T>>(
    ui: &mut egui::Ui,
    id_salt: impl Hash + Copy,
    snarl: &mut Snarl<T>,
    viewer: &mut V,
    accent: egui::Color32,
    min_size: egui::Vec2,
) {
    // Two persistent bits of state per-graph:
    //
    //  * `version`  — folded into the snarl's Id. Bumping it
    //                 invalidates egui-snarl's stored pan/zoom
    //                 transform and forces its `SnarlState::initial`
    //                 path to recompute, which fits the node
    //                 bounding-box to the current viewport. That's
    //                 our "auto recentre".
    //  * `last_sz` — the viewport size the snarl was laid out at
    //                last frame. When the current size drifts more
    //                than `RESIZE_THRESHOLD` px in either axis, bump
    //                version. Maximise / restore and pane drags both
    //                cross the threshold easily; a pixel of render
    //                jitter doesn't.
    const RESIZE_THRESHOLD: f32 = 8.0;
    // How many extra frames to keep bumping after a "natural" trigger
    // (first open, resize, maximise toggle). Gives egui-snarl's
    // state-save round-trip time to settle — some triggers are
    // followed by a frame where the layout is still resolving, so
    // one bump alone can fit the graph to a stale rect.
    const SETTLE_FRAMES: u32 = 2;
    let version_id = ui.id().with(("frost_snarl_version", id_salt));
    let last_sz_id = ui.id().with(("frost_snarl_last_sz", id_salt));
    let settle_id = ui.id().with(("frost_snarl_settle", id_salt));

    maximizable(ui, id_salt, accent, min_size, |ui| {
        let size = ui.available_size();
        let ctx = ui.ctx().clone();
        let mut version: u32 = ctx.data(|d| d.get_temp(version_id)).unwrap_or(0);
        let last_sz_opt: Option<egui::Vec2> =
            ctx.data(|d| d.get_temp::<egui::Vec2>(last_sz_id));
        let settle_left: u32 = ctx
            .data(|d| d.get_temp::<u32>(settle_id))
            .unwrap_or(0);

        // A usable viewport is any rect with both dims >= 10 px —
        // below that, egui-snarl's `initial` computes a degenerate
        // transform (common on frames where layout hasn't resolved
        // yet). Defer all bump bookkeeping until we have real
        // dimensions.
        let size_usable = size.x >= 10.0 && size.y >= 10.0;

        // "Natural" bump trigger — first render, or viewport size
        // crossed the resize threshold.
        let natural_bump = size_usable
            && match last_sz_opt {
                None => true,
                Some(last_sz) => {
                    let dx = (size.x - last_sz.x).abs();
                    let dy = (size.y - last_sz.y).abs();
                    dx > RESIZE_THRESHOLD || dy > RESIZE_THRESHOLD
                }
            };

        // After a natural bump, keep bumping for `SETTLE_FRAMES`
        // more frames. That absorbs the case where the layout
        // wasn't final on the bump frame and `SnarlState::initial`
        // ran against a slightly-wrong rect.
        let settle_bump = size_usable && settle_left > 0;

        let should_bump = natural_bump || settle_bump;
        if should_bump {
            version = version.wrapping_add(1);
        }
        let new_settle = if natural_bump {
            SETTLE_FRAMES
        } else {
            settle_left.saturating_sub(1)
        };

        if size_usable {
            ctx.data_mut(|d| {
                d.insert_temp::<u32>(version_id, version);
                d.insert_temp::<egui::Vec2>(last_sz_id, size);
                d.insert_temp::<u32>(settle_id, new_settle);
            });
            // Force a repaint for the settle frames so we don't
            // have to wait for user interaction to emit another
            // frame — otherwise egui can idle-sleep after the
            // initial render and the "settle bumps" never fire.
            if should_bump || new_settle > 0 {
                ctx.request_repaint();
            }
        }

        // Stable Id (not `id_salt`, which is hashed with the
        // current `ui.id()` and therefore differs between inline
        // and maximised viewports). Version folds in so bumps
        // reset the stored transform.
        let snarl_id = egui::Id::new(("frost_snarl_widget", id_salt, version));
        SnarlWidget::new()
            .id(snarl_id)
            .style(frost_snarl_style(accent))
            .min_size(size)
            .show(snarl, viewer, ui);
    });
}
