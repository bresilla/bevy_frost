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


use egui;

pub use crate::extras::snarl::{
    ui::{
        AnyPins, BackgroundPattern, Dots, Grid, NodeHalo, NodeLayout, PinInfo, PinPlacement,
        PinShape, SnarlPin, SnarlStyle, SnarlViewer, SnarlWidget, WireColorMode,
    },
    InPin, InPinId, NodeId, OutPin, OutPinId, Snarl,
};

// `frost_snarl` / `frost_snarl_with_opts` route through
// `maximizable_with_opts` (fully qualified) for the fullscreen
// chip + overlay swap. `OverlayOpts` is re-exported so callers
// pick up the chip-placement type from the same module.
pub use crate::extras::maximize::OverlayOpts;
pub use crate::extras::node_view::{NodeViewBackend, NodeViewState};
use crate::style::{
    glass_alpha_card, glass_alpha_window, glass_fill, widget_border,
};

/// Build a [`SnarlStyle`] that inherits the frost palette + border
/// language. Call per-frame with the current accent so the graph
/// re-tints when the user swaps accent colour (the same way every
/// other frost surface does).
///
/// What the returned style pins down:
///
/// * **Node frame** — `BG_2_RAISED` glass fill + `widget_border`
///   stroke + `crate::style::theme().radius_md` corner, matching
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
    // ── Blender-style geometry ──
    // Blender (4.x) measures all node geometry off `widget_unit = 20 px`:
    //   * NODE_DY (header height, row height) = widget_unit = 20 px
    //   * BASIS_RAD (corner radius)           = 0.2 × widget_unit = 4 px
    //   * NODE_MARGIN_X (header text indent)  = 1.2 × widget_unit = 24 px
    //   * NODE_DYS (half-row, gutter)         = widget_unit / 2  = 10 px
    //   * NODE_SOCKSIZE (pin radius)          = 0.25 × widget_unit = 5 px
    // We mirror those constants so the node geometry feels
    // proportionally identical, with frost's glass-fill background.
    const NODE_RADIUS: u8 = 4;
    // Horizontal padding shared by body AND header so the header
    // band lines up with the body edges (snarl sizes each frame as
    // content + 2 × inner_margin, so any divergence here makes the
    // header poke out like a hat).
    const NODE_PAD_X: i8 = 8;
    const NODE_PAD_Y: i8 = 4;

    // Body — Blender uses a near-black `#303030` solid fill with no
    // border, but we layer frost's glass tint on top so the
    // accent-coloured panel + dark-on-light theme variant both flow
    // through. Outline: 1 px hairline at black with 25% alpha — the
    // dark seam between the node and its drop shadow that gives
    // each node a crisp silhouette in Blender.
    let body_fill = glass_fill(
        crate::style::theme().bg_raised,
        accent,
        glass_alpha_card(),
    );
    let body_stroke = egui::Stroke::new(1.0, egui::Color32::from_black_alpha(64));
    let node_frame = egui::Frame::new()
        .fill(body_fill)
        .stroke(body_stroke)
        .corner_radius(egui::CornerRadius::same(NODE_RADIUS))
        .inner_margin(egui::Margin::symmetric(NODE_PAD_X, NODE_PAD_Y));

    // Header — TRANSPARENT here. The category-coloured band is
    // painted PER-NODE inside `SnarlViewer::show_header` (see the
    // demo's `show_header` impl) by reading the node's category +
    // smearing a Unreal-style left-anchored gradient across the
    // header rect. That keeps `frost_snarl_style` host-agnostic
    // (no fixed colour palette baked in) and lets each app's
    // viewer decide which colour to spill.
    let header_frame = egui::Frame::new()
        .fill(egui::Color32::TRANSPARENT)
        .stroke(egui::Stroke::NONE)
        .corner_radius(egui::CornerRadius {
            nw: NODE_RADIUS, ne: NODE_RADIUS, sw: 0, se: 0,
        })
        .inner_margin(egui::Margin {
            left: NODE_PAD_X, right: NODE_PAD_X,
            top: NODE_PAD_Y, bottom: NODE_PAD_Y,
        });

    // Background mirrors the code-editor recipe — `pane_fill(accent)`
    // routes through the theme so GAME's accent-tinted dark and
    // PRO's neutral `bg_panel` both flow in here automatically. The
    // node graph and the code editor now visually share the same
    // canvas surface.
    let canvas_base = crate::style::pane_fill(accent);
    let bg_fill = glass_fill(canvas_base, accent, glass_alpha_window());

    // Grid stroke — `contrast_text_for(canvas_base)` at low alpha so
    // the pattern is automatically lighter than the bg on a dark
    // canvas, darker on a light one. Alpha 28 keeps it firmly in
    // the "there but quiet" tier: visible enough to read as a grid,
    // not loud enough to compete with the nodes.
    let grid_base = crate::style::contrast_text_for(canvas_base);
    let grid_stroke = egui::Stroke::new(
        1.0,
        egui::Color32::from_rgba_unmultiplied(grid_base.r(), grid_base.g(), grid_base.b(), 28),
    );

    SnarlStyle {
        node_frame: Some(node_frame),
        header_frame: Some(header_frame),
        bg_frame: Some(
            egui::Frame::new()
                .fill(bg_fill)
                .stroke(egui::Stroke::new(crate::style::theme().border_width, widget_border(accent)))
                .corner_radius(egui::CornerRadius::same(crate::style::theme().radius_lg))
                .inner_margin(egui::Margin::same(2)),
        ),
        // Blender-style dot grid on the canvas. 30-px spacing, 1-px
        // dot radius — large enough to read as a grid when zoomed
        // out, quiet enough to disappear behind nodes.
        bg_pattern: Some(BackgroundPattern::Dots(Dots::new(
            egui::vec2(30.0, 30.0),
            1.0,
        ))),
        bg_pattern_stroke: Some(grid_stroke),
        // Pin defaults — overridden per-node-type by the demo's
        // `PinType::pin()` builder. Blender uses a 1-px black
        // outline on every socket; mirrored here.
        pin_fill: Some(crate::style::on_section()),
        pin_stroke: Some(egui::Stroke::new(
            1.0, egui::Color32::from_black_alpha(160),
        )),
        // Wires — Blender uses 2.5 px width with a 1-px dark
        // outline pass underneath; egui-snarl draws a single
        // stroke, so we settle on 2.0 px (UE Blueprints' default
        // 1.5 px felt too thin against the dot grid).
        wire_width: Some(2.0),
        wire_style: None,
        // Unreal-Blueprints wire colour rule — the wire takes the
        // OUTPUT (source) pin's colour uniformly along its length,
        // not a gradient between source and target. The "wire shows
        // the type that's flowing out" idiom — much easier to
        // read than Blender's interpolated mix when you're
        // following a wire from its origin.
        wire_color_mode: Some(WireColorMode::FromSource),
        // Faux-bloom — wires and pins shed a soft halo in their
        // type colour. Layered alpha-reduced strokes under the
        // crisp wire give a "post-process bloom" feel without an
        // actual GPU pass. 0.6 reads as "vibrant but not hazy".
        wire_glow: Some(0.6),
        pin_glow: Some(0.5),
        // Pin glyph centre sits ON the body's border line — the
        // pin bisects the outline, half inside / half outside.
        // Reads as "above" / sitting on the border the way
        // Blender + Unreal node editors do, with the wire
        // arriving at the body's edge rather than past it.
        pin_placement: Some(PinPlacement::Edge),
        pin_inset: None,
        // Accent halo close to the body, painted UNDER pin
        // glyphs (snarl reserves the painter slot before pins
        // submit, so pins always render on top of the halo
        // line). 3 px gap, 1.5 px stroke.
        node_halo: Some(NodeHalo {
            color: accent,
            gap: 3.0,
            width: 1.5,
            radius: 6,
        }),
        downscale_wire_frame: Some(true),
        upscale_wire_frame: Some(true),
        // ── Outside-in zoom ──
        // Lock snarl's internal `TSTransform.scaling` to 1.0 so it
        // never stretches the rasterised glyphs (a bitmap atlas
        // scaled past 1.0 is the source of the bilinear blur on
        // zoom). Zoom is instead driven from the outside in
        // `node_view::show`, which grows the secondary egui
        // context's `pixels_per_point` AND shrinks its
        // `screen_rect` proportionally — the atlas re-rasterises
        // glyphs at the new pixel resolution AND the layout area
        // shrinks so nodes appear bigger. End result: text stays
        // sharp at any zoom level.
        // Snarl's pan (TSTransform.translation) is still managed
        // internally by drag-pan inside the widget; only the zoom
        // axis is hijacked.
        min_scale: Some(1.0),
        max_scale: Some(1.0),
        // No collapse arrow on the header. Folding is rarely
        // needed and the right-click context menu already handles
        // it; freeing the right edge keeps the title bar clean.
        collapsible: Some(false),
        // Zero the leading drag-space padding — by default snarl
        // allocates an `icon_width × icon_width` (~16×16 px) hover
        // strip before `show_header`. That pushes our icon away
        // from the left edge of the header band and looks broken
        // next to a per-category coloured fill.
        header_drag_space: Some(egui::vec2(0.0, 0.0)),
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
/// Render an `egui-snarl` node graph through frost's sharp-zoom
/// pipeline: a SECONDARY `egui::Context` with `pixels_per_point`
/// compensated for zoom, painted into a wgpu texture by the
/// [`NodeViewBackend`] and composited back into the parent UI. The
/// graph stays sharp at any zoom level (text + shape edges
/// rasterise at the zoomed size, never up-scaled) and stays
/// host-agnostic — the backend trait has impls for `bevy_egui`
/// (`bevy_frost::node_view_backend::BevyNodeViewBackend`) and
/// `eframe` (`egui_frost::EframeNodeViewBackend`).
///
/// `state` carries the per-graph camera (`pan`, `zoom`) plus the
/// secondary egui context and wgpu texture across frames; pass the
/// SAME `NodeViewState` each frame for the same graph instance.
///
/// Use this in place of [`SnarlWidget::new().show`] whenever you
/// want the frost styling + the fullscreen affordance. The
/// fullscreen chip lands top-right by default;
/// [`frost_snarl_with_opts`] takes an [`OverlayOpts`] for custom
/// chip placement.
pub fn frost_snarl<T, V: SnarlViewer<T>>(
    ui: &mut egui::Ui,
    state: &mut NodeViewState,
    backend: &mut dyn NodeViewBackend,
    snarl: &mut Snarl<T>,
    viewer: &mut V,
    accent: egui::Color32,
    desired_size: egui::Vec2,
) {
    frost_snarl_with_opts(
        ui, state, backend, snarl, viewer, accent, desired_size,
        OverlayOpts::default(),
    )
}

/// Like [`frost_snarl`] but accepts [`OverlayOpts`] so the caller
/// picks where the fullscreen / minimize chip lands on the overlay
/// (which edge + which cluster along that edge).
pub fn frost_snarl_with_opts<T, V: SnarlViewer<T>>(
    ui: &mut egui::Ui,
    state: &mut NodeViewState,
    backend: &mut dyn NodeViewBackend,
    snarl: &mut Snarl<T>,
    viewer: &mut V,
    accent: egui::Color32,
    desired_size: egui::Vec2,
    fs_opts: OverlayOpts,
) {
    let id_for_snarl_base = egui::Id::new("frost_snarl_widget");
    // Auto-recentre bookkeeping. The `version` is folded into the
    // SnarlWidget's id below; bumping it invalidates egui-snarl's
    // saved transform so `SnarlState::initial` runs again and
    // refits the bb to the live viewport. We bump on first paint
    // (no `last_sz` yet) and whenever the viewport size drifts
    // more than `RESIZE_THRESHOLD` from the last fit — pane drags,
    // maximise / restore, and fullscreen toggles all easily cross
    // that threshold; per-pixel render jitter does not. We also
    // keep bumping for `SETTLE_FRAMES` extra frames after a
    // natural trigger so the eventual layout (often resolved a
    // frame or two AFTER the size change) gets fit instead of the
    // mid-resolve rect.
    const RESIZE_THRESHOLD: f32 = 8.0;
    const SETTLE_FRAMES: u32 = 2;
    let version_id = ui.id().with(("frost_snarl_version", id_for_snarl_base));
    let last_sz_id = ui.id().with(("frost_snarl_last_sz", id_for_snarl_base));
    let settle_id = ui.id().with(("frost_snarl_settle", id_for_snarl_base));

    // `maximizable_with_opts` paints the maximize chip and, when
    // active, swaps to a fullscreen body — its body callback gets
    // a `&mut Ui` whose `available_size()` is either the inline
    // pod rect or the full window. Use that as the sharp-zoom
    // target size so the secondary egui context renders at the
    // exact pixel dimensions of whichever surface owns the pane
    // this frame.
    crate::extras::maximize::maximizable_with_opts(
        ui, id_for_snarl_base, accent, desired_size, fs_opts,
        |inner_ui| {
            let size = inner_ui.available_size();

            let parent_ctx = inner_ui.ctx().clone();
            let mut version: u32 =
                parent_ctx.data(|d| d.get_temp(version_id)).unwrap_or(0);
            let last_sz: Option<egui::Vec2> =
                parent_ctx.data(|d| d.get_temp::<egui::Vec2>(last_sz_id));
            let settle_left: u32 = parent_ctx
                .data(|d| d.get_temp::<u32>(settle_id))
                .unwrap_or(0);
            let size_usable = size.x >= 10.0 && size.y >= 10.0;
            let natural_bump = size_usable
                && match last_sz {
                    None => true,
                    Some(prev) => {
                        let dx = (size.x - prev.x).abs();
                        let dy = (size.y - prev.y).abs();
                        dx > RESIZE_THRESHOLD || dy > RESIZE_THRESHOLD
                    }
                };
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
                parent_ctx.data_mut(|d| {
                    d.insert_temp::<u32>(version_id, version);
                    d.insert_temp::<egui::Vec2>(last_sz_id, size);
                    d.insert_temp::<u32>(settle_id, new_settle);
                });
                if should_bump || new_settle > 0 {
                    parent_ctx.request_repaint();
                }
            }
            // On a real resize (maximise / restore / pane drag),
            // reset our outside-in `state.zoom` to 1.0 so the
            // re-fit pass below renders at native scale and the
            // whole graph fits the new viewport. Without this the
            // user's previous zoom level (e.g. zoomed-in 3× while
            // maximised) carries over to the smaller inline rect
            // and the graph stays cropped.
            if natural_bump {
                state.set_zoom(1.0);
            }
            // Versioned snarl id — bumping version forces a fresh
            // fit because the saved SnarlStateData lookup misses.
            let id_for_snarl = id_for_snarl_base.with(version);

            crate::extras::node_view::show_with_anchor(
                inner_ui,
                state,
                backend,
                size,
                // Cursor-anchor the wheel zoom by nudging snarl's
                // saved `TSTransform.translation` by the same
                // sub-points delta `node_view::show_with_anchor`
                // computes — applied here BEFORE the snarl widget
                // runs in the body callback below, so snarl's
                // first `SnarlState::load` of this frame picks up
                // the updated translation and the scene point
                // under the cursor stays under the cursor.
                |sub_ctx, delta| {
                    crate::extras::snarl::ui::SnarlState::nudge_saved_translation(
                        sub_ctx, id_for_snarl, delta,
                    );
                },
                |sub_ui| {
                    SnarlWidget::new()
                        .id(id_for_snarl)
                        .style(frost_snarl_style(accent))
                        .min_size(size)
                        .show(snarl, viewer, sub_ui);
                },
            );
        },
    );
}
