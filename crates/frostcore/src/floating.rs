//! Floating-panel helper, with drag-to-resize handles on the
//! panel's scene-facing edge (horizontal) AND bottom / top edge
//! (vertical).
//!
//! ## Pane vs container — the enforced constraint
//!
//! A floating pane **cannot host widgets directly**. Every control
//! that lives in a pane has to sit inside a container — either a
//! [`crate::widgets::section`] (foldable card) or a subsection. The
//! pane's body closure takes a [`PaneBuilder`] rather than a raw
//! `egui::Ui`, and `PaneBuilder` only exposes `.section(...)` — so
//! dropping a `toggle` / `slider` / bare widget at the pane level
//! is a *compile error*, not a convention.
//!
//! This is deliberate: panes without container structure devolve
//! into ad-hoc layouts that break under resize, drag, and the
//! frost visual language. Forcing one container per block keeps
//! every panel readable and consistent across projects.
//!
//! Anchored to one of the four screen corners via [`egui::Align2`].
//! No title bar, no close button; the title sits at the rail-facing
//! edge (same side as the [`crate::ribbon`] it's paired with). Two
//! hit-thin strips sit on the *opposite* edges:
//!
//! * **Horizontal handle** — scene-facing edge. Drag to grow / shrink
//!   width.
//! * **Vertical handle** — the edge facing away from the panel's
//!   vertical anchor (bottom for `*_TOP` / `*_CENTER`, top for
//!   `*_BOTTOM`). Drag to grow / shrink height.
//!
//! Both values are stored per-panel-id in `egui::Context::data` so
//! the user's drags survive across frames. Width and height are both
//! clamped every frame to the current window size, so shrinking the
//! Bevy window never leaves the panel extending past the visible
//! screen.

use egui;

use crate::style::{glass_alpha_window, glass_fill, BORDER_SUBTLE};

// Ribbon layout constants we need here. Kept as locals rather than
// pulling `ribbon::paint` into the public prelude — the numbers
// belong to both modules.
const EDGE_GAP: f32 = 8.0;
const SIDE_BTN_SIZE: f32 = 34.0;
// Gap between the rail's button edge and the pane edge. Must equal
// the inter-button gap (`SIDE_BTN_GAP`) so panes from a side rail
// and panes from a top/bottom rail meet at exactly the same corner
// pixel — otherwise corner-cluster panes from perpendicular rails
// land 2 px apart and look misaligned.
const RAIL_PANEL_GAP: f32 = 4.0;

/// Read back the anchor-side of the floating pane currently
/// rendering. `Some(true)` means the active pane is anchored to a
/// `RIGHT_*` edge; `Some(false)` means a `LEFT_*` / `CENTER_*`
/// anchor; `None` means we're not inside a `floating_window` body
/// at all. Widgets call this to mirror their internal layout
/// (e.g. the maximise chip flips left/right) so the pane reads
/// symmetrically on either rail.
pub fn current_pane_on_right_side(ctx: &egui::Context) -> Option<bool> {
    let key = egui::Id::new("frost_current_pane_right_anchored");
    ctx.data(|d| d.get_temp::<bool>(key))
}

/// Width of the horizontal (scene-facing) resize-handle hit zone.
const RESIZE_HANDLE_W: f32 = 8.0;
/// Height of the vertical (bottom/top) resize-handle hit zone.
const RESIZE_HANDLE_H: f32 = 10.0;

/// Default per-section width when a pane is in horizontal layout
/// mode (Top / Bottom rail panes), AND the section is open. Folded
/// sections shrink to `H_SECTION_TITLE_STRIP_W` so multiple folded
/// sections take very little horizontal space — same idea as a
/// vertical pane where folded sections shrink vertically to just
/// the header.
const HORIZONTAL_SECTION_W: f32 = 360.0;

/// Inter-card gap inside a horizontal pane's section row. Used by
/// both the pane-width calculation and the actual `ui.add_space`
/// at render time. Zero — flex butts items together; visual
/// separation comes from each card's own border.
const HORIZONTAL_INTER_CARD_GAP: f32 = 0.0;

/// Width of the vertical title strip for horizontal (TOP/BOTTOM rail)
/// panes. Matches the vertical pane's title strip HEIGHT (`title_h`
/// = 25 px) so the title-bar thickness is the same across both pane
/// orientations — a vertical pane's 25-tall horizontal title sits
/// at the same visual weight as a horizontal pane's 25-wide vertical
/// title.
pub(crate) const VERTICAL_TITLE_W: f32 = 25.0;
/// Gap between the pane's vertical title strip and the body / first
/// container. Matches the 6 px `add_space` that the vertical pane
/// inserts between its horizontal title strip and the body — keeps
/// the visual breathing room consistent across orientations.
pub(crate) const VERTICAL_TITLE_BODY_GAP: f32 = 6.0;

/// Where a pane title sits and which way it reads. Threaded into
/// `paint_title` so one painter handles every layout.
///
/// For vertical strips, the *side* (left vs right) tracks the
/// cluster's screen position (`on_right_side`) — panes anchored on
/// the right of the screen carry their title on the right edge.
/// The *rotation* tracks the rail direction (top vs bottom) —
/// TOP-rail panes read bottom-to-top, BOTTOM-rail panes read
/// top-to-bottom (mirror). Side and rotation are independent so all
/// four combinations are valid.
#[derive(Clone, Copy)]
pub(crate) enum TitleOrientation {
    /// Horizontal strip at the top of the pane; divider hairline
    /// (if enabled by the theme) sits BELOW the title.
    HorizontalTop,
    /// Horizontal strip at the bottom of the pane; divider sits ABOVE.
    HorizontalBottom,
    /// Vertical strip on the side of the pane.
    Vertical {
        /// `false` = strip on the LEFT edge, `true` = on the RIGHT.
        on_right: bool,
        /// `false` = text reads bottom-to-top (rotation -π/2),
        /// `true` = top-to-bottom (rotation +π/2).
        top_to_bottom: bool,
    },
}

/// Minimum / maximum panel widths. Caller's `size.x` clamps inside
/// this range on first draw; the user's drag does the same.
const MIN_PANEL_W: f32 = 220.0;
const MAX_PANEL_W: f32 = 1600.0;
/// Minimum / maximum panel heights — same intent as the widths.
/// Lowered so horizontal panes with little content don't ship a
/// ~340 px tall empty card — sections are now content-driven, so
/// a tiny pane is fine.
const MIN_PANEL_H: f32 = 140.0;
const MAX_PANEL_H: f32 = 1600.0;

const _: () = {
    assert!(EDGE_GAP == 8.0);
    assert!(SIDE_BTN_SIZE == 34.0);
    assert!(RAIL_PANEL_GAP == 4.0);
};

/// Per-pane drag-reorder state, persisted across frames in
/// `ctx.data` keyed by the pane id. `item` latches the dragged
/// section's id_salt; `cursor` is the latest pointer position so
/// the finalize pass can compute the target gap.
#[derive(Clone, Default)]
struct SectionDragState {
    item: Option<String>,
    cursor: Option<egui::Pos2>,
}

/// Builder handed to every [`floating_window`] / [`floating_window_scoped`]
/// body closure. Only exposes container-creating methods — callers
/// cannot reach the underlying `egui::Ui`, so it's impossible to
/// drop bare widgets directly on the pane. Every control in a pane
/// lives inside a [`section`](PaneBuilder::section) (or a nested
/// subsection inside that section's body).
///
/// Sections render in the order the caller invokes them. The pane
/// adds two automatic behaviours on top of the plain section list:
///
/// 1. **Drag-to-reorder** — a transparent drag-sense overlay sits
///    on top of every section's header. Press-and-drag on a header
///    starts a reorder gesture; a thin accent line shows the target
///    gap; release commits the new order. To pick the new order up
///    on the next frame, the caller iterates the result of
///    [`section_order`](Self::section_order) and dispatches via
///    `match` — that's what makes the visual reorder stick. Without
///    that loop, the drag still records intent but the user's code
///    keeps drawing in the same order.
/// 2. **Auto-fold on overflow** — if the rendered section stack
///    overshoots the pane body, the topmost open section is
///    force-closed so the next frame fits. One per frame, converges
///    naturally.
/// Render target for a [`PaneBuilder`]. Vertical panes operate on
/// a plain `&mut Ui` (sections stack via `section_tracked`);
/// horizontal panes operate on a [`crate::flex::FlexInstance`], so
/// each section becomes a flex item placed by Flex's own row layout.
/// This is the bridge that lets `section_with` route through Flex
/// without `PaneBuilder` having to refactor every caller.
pub(crate) enum PaneTarget<'a, 'b> {
    Ui(&'a mut egui::Ui),
    Flex(&'a mut crate::flex::FlexInstance<'b>),
}

impl<'a, 'b> PaneTarget<'a, 'b> {
    fn ctx(&self) -> &egui::Context {
        match self {
            PaneTarget::Ui(u) => u.ctx(),
            PaneTarget::Flex(f) => f.ui().ctx(),
        }
    }

    fn is_horizontal(&self) -> bool {
        matches!(self, PaneTarget::Flex(_))
    }
}

pub struct PaneBuilder<'a, 'b> {
    target: PaneTarget<'a, 'b>,
    accent: egui::Color32,
    pane_id: egui::Id,
    /// Body rect (the area below the title strip).
    body_rect: egui::Rect,
    /// Sections rendered this frame, in user call order. The dragged
    /// section is skipped (lifted out), so this only ever contains
    /// the OTHER sections during a drag.
    rendered: Vec<RenderedSection>,
    /// Number of non-dragged sections rendered so far this frame —
    /// used to decide when to insert the ghost gap during the user's
    /// loop.
    non_dragged_count: usize,
    /// Drag state read from ctx at construction.
    drag: SectionDragState,
    /// Latched in `.section()` when a header reports `drag_started`
    /// — promoted into `drag.item` during finalize.
    drag_started_id: Option<String>,
    /// Stored order this frame, resolved by `section_order`.
    base_order_this_frame: Vec<String>,
    /// Previous-frame snapshot — used to derive the cursor's target
    /// slot from its Y plus the dragged section's natural size for
    /// the ghost gap and floating preview.
    cached_rects: RectCache,
    /// Latched whenever a section's header was `clicked()` this
    /// frame. The auto-fold pass excludes it so the section the
    /// user just expanded doesn't immediately get force-closed when
    /// the new content overshoots the body.
    just_toggled_id: Option<String>,
    /// Seconds since the pane became visible this open. Section
    /// order indexes this to compute their staggered fade-in
    /// progress. `99.0` (or any value > the longest stagger window)
    /// means the pane is settled and sections paint at full
    /// opacity; freshly opened panes start at `0.0`.
    pane_open_elapsed: f32,
    /// `true` when the pane is laid out horizontally (TOP/BOTTOM
    /// rail) — sections placed in a row via `ui.horizontal`, each
    /// rendered through `section_tracked_horizontal`.
    horizontal: bool,
    /// In horizontal mode, which side of the section card the
    /// rotated title strip sits on. `false` = LEFT, `true` = RIGHT.
    h_section_title_on_right: bool,
    /// In horizontal mode, which way the rotated section titles
    /// read. `false` = bottom-to-top, `true` = top-to-bottom.
    h_section_top_to_bottom: bool,
}

struct RenderedSection {
    id_salt: String,
    state_id: egui::Id,
    outer_rect: egui::Rect,
    title: String,
    openness: f32,
}

impl<'a, 'b> PaneBuilder<'a, 'b> {
    /// Add a foldable container section to the pane. `id_salt`
    /// disambiguates the section's collapsed-state storage,
    /// `title` is the UPPERCASE accent header, `default_open`
    /// controls the initial expansion, and `body` receives a
    /// regular `&mut egui::Ui` — inside the section, any widget
    /// works as normal.
    ///
    /// Sections render in the order they're called. To make
    /// drag-reorder visually take effect, drive the call order from
    /// [`section_order`](Self::section_order); see the type-level
    /// docs.
    pub fn section(
        &mut self,
        id_salt: &str,
        title: &str,
        default_open: bool,
        body: impl FnOnce(&mut egui::Ui),
    ) {
        self.section_with(
            id_salt,
            title,
            default_open,
            None::<crate::icons::Icon<'_>>,
            body,
        );
    }

    /// Section with an optional icon. The icon argument is anything
    /// convertible into [`crate::icons::Icon`] — pass a Fluent icon
    /// name (`Some("flag")`), an `Icon::Svg(svg_str)` for inline SVG,
    /// or build one via [`crate::icons::Icon::from`]. Header-action
    /// buttons were removed; widgets that need a "lift to
    /// full-window" affordance host their own floating chip via
    /// [`crate::maximize::maximizable`].
    pub fn section_with<'i, I: Into<crate::icons::Icon<'i>>>(
        &mut self,
        id_salt: &str,
        title: &str,
        default_open: bool,
        icon: Option<I>,
        body: impl FnOnce(&mut egui::Ui),
    ) {
        // If THIS section is the one being dragged, lift it out of
        // the layout entirely.
        if self.drag.item.as_deref() == Some(id_salt) {
            return;
        }

        // Per-section staggered fade-in opacity.
        const STAGGER_BASE: f32 = 0.18;
        const FADE_BASE: f32 = 0.45;
        let th = crate::style::theme();
        let opacity = if th.animations_enabled {
            let scale = th.pane_fade_scale.max(0.01);
            let stagger = STAGGER_BASE * scale;
            let fade = FADE_BASE * scale;
            let section_idx = self.non_dragged_count as f32;
            let start = section_idx * stagger;
            let raw = ((self.pane_open_elapsed - start) / fade).clamp(0.0, 1.0);
            raw * raw * (3.0 - 2.0 * raw)
        } else {
            1.0
        };

        let accent = self.accent;
        let pane_id = self.pane_id;
        let scroll_id = pane_id.with("frost_section_vscroll").with(id_salt);
        let title_on_right = self.h_section_title_on_right;
        let top_to_bottom = self.h_section_top_to_bottom;
        let drag_item = self.drag.item.clone();
        let drag_cursor = self.drag.cursor;
        let cached_rects_clone = self.cached_rects.clone();
        let non_dragged_count = self.non_dragged_count;
        let id_salt_owned = id_salt.to_string();

        let track = match &mut self.target {
            PaneTarget::Ui(ui) => {
                if self.horizontal {
                    // Horizontal pane: ui is `body_ui.horizontal`'s
                    // row_ui. Each section is allocated a slot at
                    // the row's full height. The width LERPS with
                    // the section's openness — folded sections
                    // shrink to `H_SECTION_TITLE_STRIP_W` so the
                    // pane can hold many folded sections compactly,
                    // exactly like a vertical pane shrinks folded
                    // sections vertically.
                    let avail_h = ui.available_height().max(0.0);
                    let scroll_id = pane_id.with("frost_section_vscroll").with(id_salt);
                    let title_on_right = self.h_section_title_on_right;
                    let top_to_bottom = self.h_section_top_to_bottom;

                    // Read the openness BEFORE rendering so the
                    // card_size used by section_tracked_horizontal
                    // matches what gets painted.
                    let state_id = ui.make_persistent_id(("frost_section", id_salt));
                    let state =
                        egui::collapsing_header::CollapsingState::load_with_default_open(
                            ui.ctx(),
                            state_id,
                            default_open,
                        );
                    let openness = state.openness(ui.ctx());

                    // Cap unfolded width to `body_avail_w / count` so
                    // many-section panes don't overflow the body's
                    // right edge into the title strip. Read both
                    // values from ctx (set in floating.rs's row_ui
                    // setup and finalize). Falls back to
                    // `HORIZONTAL_SECTION_W` if either is missing.
                    let body_avail_w: f32 = ui
                        .ctx()
                        .data(|d| d.get_temp::<f32>(h_body_avail_key(pane_id)))
                        .unwrap_or(f32::INFINITY);
                    let prev_count: usize = ui
                        .ctx()
                        .data(|d| d.get_temp::<usize>(section_count_key(pane_id)))
                        .unwrap_or(1)
                        .max(1);
                    let max_unfolded = (body_avail_w / prev_count as f32)
                        .clamp(
                            crate::widgets::foldable::H_SECTION_TITLE_STRIP_W,
                            HORIZONTAL_SECTION_W,
                        );
                    let card_w = egui::lerp(
                        crate::widgets::foldable::H_SECTION_TITLE_STRIP_W..=max_unfolded,
                        openness,
                    );
                    let card_size = egui::vec2(card_w, avail_h);

                    if non_dragged_count > 0 {
                        ui.add_space(HORIZONTAL_INTER_CARD_GAP);
                    }
                    let prev_opacity = ui.opacity();
                    if opacity < 1.0 {
                        ui.multiply_opacity(opacity);
                    }
                    let wrapped_body = move |body_ui: &mut egui::Ui| {
                        let h = body_ui.available_height().max(0.0);
                        body_ui.set_min_height(h);
                        egui::ScrollArea::vertical()
                            .id_salt(scroll_id)
                            .auto_shrink([false, false])
                            .max_height(h)
                            .show(body_ui, |scroll_ui| {
                                body(scroll_ui);
                            });
                    };
                    let track = crate::widgets::foldable::section_tracked_horizontal(
                        ui,
                        &id_salt_owned,
                        title,
                        accent,
                        default_open,
                        icon,
                        title_on_right,
                        top_to_bottom,
                        card_size,
                        wrapped_body,
                    );
                    ui.set_opacity(prev_opacity);
                    track
                } else {
                    let gap = crate::style::theme().section_gap;
                    if gap > 0.0 && non_dragged_count > 0 {
                        ui.add_space(gap);
                    }
                    if let (Some(dragged_id), Some(cursor)) =
                        (drag_item.as_deref(), drag_cursor)
                    {
                        let target = compute_target_among_others(
                            &cached_rects_clone,
                            dragged_id,
                            cursor.y,
                            false,
                        );
                        if non_dragged_count == target {
                            paint_ghost_gap(
                                ui,
                                &cached_rects_clone,
                                dragged_id,
                                accent,
                                false,
                            );
                        }
                    }
                    let prev_opacity = ui.opacity();
                    if opacity < 1.0 {
                        ui.multiply_opacity(opacity);
                    }
                    let track = crate::widgets::foldable::section_tracked(
                        ui,
                        &id_salt_owned,
                        title,
                        accent,
                        default_open,
                        icon,
                        body,
                    );
                    ui.set_opacity(prev_opacity);
                    log_layout_change(
                        ui.ctx(),
                        egui::Id::new(("frost_layout_v_section", track.state_id)),
                        format_args!(
                            "CONTAINER_V \"{}\" tl=({:.0},{:.0}) br=({:.0},{:.0}) sz=({:.0}x{:.0})",
                            id_salt_owned,
                            track.outer_rect.min.x, track.outer_rect.min.y,
                            track.outer_rect.max.x, track.outer_rect.max.y,
                            track.outer_rect.width(), track.outer_rect.height(),
                        ),
                    );
                    track
                }
            }
            PaneTarget::Flex(flex) => {
                // Horizontal pane: each section is a flex item.
                // Inter-section gap is handled by `flex.gap` at
                // creation time; drag-reorder is not supported in
                // horizontal layout. Opacity multiplier is applied
                // to the inner ui passed in by Flex.
                let mut track_out: Option<crate::widgets::foldable::SectionTrack> =
                    None;
                flex.add_ui(
                    crate::flex::item().basis(HORIZONTAL_SECTION_W),
                    |inner_ui| {
                        let avail_h = inner_ui.available_height();
                        let card_size =
                            egui::vec2(HORIZONTAL_SECTION_W, avail_h);
                        let prev_opacity = inner_ui.opacity();
                        if opacity < 1.0 {
                            inner_ui.multiply_opacity(opacity);
                        }
                        let wrapped_body = move |body_ui: &mut egui::Ui| {
                            // Body fills its full flex-allocated
                            // height — `set_min_height` ensures the
                            // body claims the slot's height even
                            // when the content is short. ScrollArea
                            // scrolls when content > slot.
                            let h = body_ui.available_height().max(0.0);
                            body_ui.set_min_height(h);
                            egui::ScrollArea::vertical()
                                .id_salt(scroll_id)
                                .auto_shrink([false, false])
                                .max_height(h)
                                .show(body_ui, |scroll_ui| {
                                    body(scroll_ui);
                                });
                        };
                        track_out = Some(
                            crate::widgets::foldable::section_tracked_horizontal(
                                inner_ui,
                                &id_salt_owned,
                                title,
                                accent,
                                default_open,
                                icon,
                                title_on_right,
                                top_to_bottom,
                                card_size,
                                wrapped_body,
                            ),
                        );
                        inner_ui.set_opacity(prev_opacity);
                    },
                );
                track_out.expect("section_tracked_horizontal always populates")
            }
        };

        if track.header_response.drag_started() {
            self.drag_started_id = Some(id_salt.to_string());
        }
        if track.header_response.clicked() {
            self.just_toggled_id = Some(id_salt.to_string());
        }

        self.rendered.push(RenderedSection {
            id_salt: id_salt.to_string(),
            state_id: track.state_id,
            outer_rect: track.outer_rect,
            title: title.to_string(),
            openness: track.openness,
        });
        self.non_dragged_count += 1;
    }

    /// Returns the stored drag-order for this pane's sections. On
    /// the first frame (or when `default_ids` introduces a new
    /// section), the order is initialised from `default_ids`; once
    /// the user drags to reorder, this method returns the new order
    /// on subsequent frames. Callers iterate the result and
    /// dispatch via `match` so the call order tracks the drag
    /// state:
    ///
    /// ```ignore
    /// for id in pane.section_order(["widgets", "scene", "theme"]) {
    ///     match id.as_str() {
    ///         "widgets" => pane.section("widgets", "Widgets", true, |ui| { /* … */ }),
    ///         "scene"   => pane.section("scene",   "Scene",   true, |ui| { /* … */ }),
    ///         "theme"   => pane.section("theme",   "Theme",   true, |ui| { /* … */ }),
    ///         _ => {}
    ///     }
    /// }
    /// ```
    ///
    /// `default_ids` doubles as the canonical id list — any id in
    /// `default_ids` not present in the stored order gets appended
    /// to the end (so adding a new section to the user's code
    /// inserts it at the bottom rather than dropping it). Stored
    /// ids that no longer appear in `default_ids` are pruned.
    pub fn section_order<I, S>(&mut self, default_ids: I) -> Vec<String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let key = order_key(self.pane_id);
        let defaults: Vec<String> = default_ids.into_iter().map(Into::into).collect();
        let ctx = self.target.ctx().clone();
        let stored: Option<Vec<String>> = ctx.data(|d| d.get_temp(key));
        // Resolve the stable stored order, merging defaults so
        // newly-added sections appear (at the end) and stale ids
        // drop. Sections render in this base order — the dragged
        // section keeps its slot during the drag; only the gap moves.
        let base_order: Vec<String> = match stored {
            Some(mut order) => {
                let known: std::collections::HashSet<&str> =
                    defaults.iter().map(|s| s.as_str()).collect();
                order.retain(|id| known.contains(id.as_str()));
                for d in &defaults {
                    if !order.iter().any(|id| id == d) {
                        order.push(d.clone());
                    }
                }
                ctx.data_mut(|d| d.insert_temp::<Vec<String>>(key, order.clone()));
                order
            }
            None => {
                ctx.data_mut(|d| d.insert_temp::<Vec<String>>(key, defaults.clone()));
                defaults
            }
        };
        // Cache for `.section()` so it can place the gap at the
        // right slot index without recomputing.
        self.base_order_this_frame = base_order.clone();
        base_order
    }

    /// Accent colour in use for this pane.
    pub fn accent(&self) -> egui::Color32 {
        self.accent
    }

    /// Read-only [`egui::Context`] access for callers that need
    /// pointer / input state while building pane content.
    pub fn ctx(&self) -> &egui::Context {
        self.target.ctx()
    }

    /// Drive the drag-reorder state machine, paint the ghost line,
    /// commit a drop if released, and run the auto-fold pass when
    /// the stack overshoots the pane body.
    fn finalize(self) {
        let PaneBuilder {
            target,
            accent,
            pane_id,
            body_rect,
            rendered,
            non_dragged_count,
            mut drag,
            drag_started_id,
            base_order_this_frame: _,
            cached_rects,
            just_toggled_id,
            pane_open_elapsed: _,
            horizontal: _,
            h_section_title_on_right: _,
            h_section_top_to_bottom: _,
        } = self;

        let ctx = target.ctx().clone();
        let is_horizontal = target.is_horizontal();

        // Promote the drag-started latch into persistent state.
        if let Some(id) = drag_started_id {
            drag.item = Some(id);
            drag.cursor = ctx.pointer_hover_pos();
        }

        if drag.item.is_some() {
            if let Some(p) = ctx.pointer_hover_pos() {
                drag.cursor = Some(p);
            }
        }
        let drag_stopped = drag.item.is_some()
            && ctx.input(|i| i.pointer.any_released());

        // Drag-reorder painting works on PaneTarget::Ui regardless
        // of orientation. The ghost-gap and target-slot logic are
        // axis-aware now — vertical panes use cursor.y, horizontal
        // use cursor.x.
        if let PaneTarget::Ui(ui) = target {
            if let (Some(dragged_id), Some(cursor)) =
                (drag.item.as_deref(), drag.cursor)
            {
                let cursor_axis = if is_horizontal { cursor.x } else { cursor.y };
                let target_idx = compute_target_among_others(
                    &cached_rects,
                    dragged_id,
                    cursor_axis,
                    is_horizontal,
                );
                if target_idx == non_dragged_count {
                    paint_ghost_gap(ui, &cached_rects, dragged_id, accent, is_horizontal);
                }
            }
            if let (Some(dragged_id), Some(cursor)) =
                (drag.item.as_deref(), drag.cursor)
            {
                paint_drag_preview(
                    ui.ctx(),
                    pane_id,
                    &cached_rects,
                    dragged_id,
                    cursor,
                    accent,
                );
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
        }

        if drag_stopped {
            if let (Some(dragged_id), Some(cursor)) = (drag.item.clone(), drag.cursor) {
                let cursor_axis = if is_horizontal { cursor.x } else { cursor.y };
                let target_idx = compute_target_among_others(
                    &cached_rects,
                    &dragged_id,
                    cursor_axis,
                    is_horizontal,
                );
                let key = order_key(pane_id);
                let mut order: Vec<String> =
                    ctx.data(|d| d.get_temp(key)).unwrap_or_default();
                let mut new_order: Vec<String> =
                    order.iter().filter(|id| **id != dragged_id).cloned().collect();
                let clamped = target_idx.min(new_order.len());
                new_order.insert(clamped, dragged_id);
                order = new_order;
                ctx.data_mut(|d| d.insert_temp::<Vec<String>>(key, order));
            }
            drag = SectionDragState::default();
            ctx.request_repaint();
        }

        ctx.data_mut(|d| d.insert_temp::<SectionDragState>(drag_key(pane_id), drag.clone()));

        // Cache this frame's rendered sections so the next frame's
        // drag pass can read sizes / titles. Merge the lifted
        // section's previous-frame entry back in so we still know
        // how big the floating preview should be next frame.
        let mut cache: RectCache = rendered
            .iter()
            .map(|r| CachedSection {
                id: r.id_salt.clone(),
                rect: r.outer_rect,
                title: r.title.clone(),
            })
            .collect();
        if let Some(dragged_id) = drag.item.as_deref() {
            if !cache.iter().any(|cs| cs.id == dragged_id) {
                if let Some(prev) = cached_rects
                    .iter()
                    .find(|cs| cs.id == dragged_id)
                    .cloned()
                {
                    cache.push(prev);
                }
            }
        }
        ctx.data_mut(|d| d.insert_temp::<RectCache>(rects_key(pane_id), cache));

        // Persist this frame's rendered-section count so a horizontal
        // pane can size its width to `count * SECTION_W` on the next
        // frame.
        ctx.data_mut(|d| d.insert_temp::<usize>(section_count_key(pane_id), non_dragged_count));

        // Per-section widths so the next frame's pane width sums
        // each section's CURRENT width (fold-aware shrink).
        let section_widths: Vec<f32> =
            rendered.iter().map(|r| r.outer_rect.width()).collect();
        ctx.data_mut(|d| {
            d.insert_temp::<Vec<f32>>(section_widths_key(pane_id), section_widths)
        });

        // Per-section openness in render order — read by the NEXT
        // frame's pane width calc to lerp each section's slot
        // width without a one-frame lag.
        let section_states: Vec<f32> =
            rendered.iter().map(|r| r.openness).collect();
        ctx.data_mut(|d| {
            d.insert_temp::<Vec<f32>>(section_states_key(pane_id), section_states)
        });

        // Auto-fold removed: clicking one section was force-closing
        // others as a side-effect, even when the new section's
        // animation produced only transient overshoot. User
        // section-state is now sticky — open what you open, close
        // what you close, layout clips at the body bottom if
        // everything's open at once. The dragged-section skip,
        // ghost-gap, and rect-cache work above is unaffected.
        let _ = (
            &rendered,
            body_rect,
            &drag,
            just_toggled_id,
            pane_id,
        );
    }
}

fn order_key(pane_id: egui::Id) -> egui::Id {
    pane_id.with("frost_pane_section_order")
}
fn drag_key(pane_id: egui::Id) -> egui::Id {
    pane_id.with("frost_pane_section_drag")
}
/// Layout-debug printer. Tracks the previous string for the given
/// id in ctx.data and prints to stdout (with `[LAYOUT]` prefix) only
/// when the string changes — keeps the log readable while still
/// showing every layout movement. Grep `[LAYOUT]` to filter.
pub(crate) fn log_layout_change(
    ctx: &egui::Context,
    key: egui::Id,
    args: std::fmt::Arguments<'_>,
) {
    let line = args.to_string();
    let prev: Option<String> = ctx.data(|d| d.get_temp::<String>(key));
    if prev.as_deref() != Some(line.as_str()) {
        println!("[LAYOUT] {}", line);
        ctx.data_mut(|d| d.insert_temp::<String>(key, line));
    }
}

/// Resolve a pane's top-left position from the anchor + offset that
/// `panel_anchor_offset` produces, given the current size. Mirrors
/// what `egui::Window::anchor()` would have done — but as a manual
/// position so the window doesn't disable its resize machinery in
/// the process.
fn compute_pane_pos(
    anchor: egui::Align2,
    offset: egui::Vec2,
    screen: egui::Rect,
    size: egui::Vec2,
) -> egui::Pos2 {
    let x = match anchor.x() {
        egui::Align::Min => screen.min.x + offset.x,
        egui::Align::Center => screen.center().x - size.x * 0.5 + offset.x,
        egui::Align::Max => screen.max.x - size.x + offset.x,
    };
    let y = match anchor.y() {
        egui::Align::Min => screen.min.y + offset.y,
        egui::Align::Center => screen.center().y - size.y * 0.5 + offset.y,
        egui::Align::Max => screen.max.y - size.y + offset.y,
    };
    egui::pos2(x, y)
}

fn section_count_key(pane_id: egui::Id) -> egui::Id {
    pane_id.with("frost_pane_section_count")
}

/// Per-frame snapshot of each rendered section's actual painted
/// width (post-lerp, fold-aware). Used by the next frame's pane
/// width calc to size the pane to the SUM of current section
/// widths — folded sections shrink the pane horizontally.
fn section_widths_key(pane_id: egui::Id) -> egui::Id {
    pane_id.with("frost_pane_section_widths")
}

/// Per-section openness snapshot — `Vec<f32>` of openness values
/// in render order. Lets the next frame's pane width calc lerp
/// each section's width directly without a one-frame lag.
fn section_states_key(pane_id: egui::Id) -> egui::Id {
    pane_id.with("frost_pane_section_states")
}

/// Width available to the horizontal section row. Stored when the
/// row Ui is allocated; read in `PaneBuilder::section_with` so each
/// section can size itself to fit (`body_avail_w / count`) when the
/// natural unfolded width would overflow the body.
fn h_body_avail_key(pane_id: egui::Id) -> egui::Id {
    pane_id.with("frost_pane_h_body_avail")
}

fn rects_key(pane_id: egui::Id) -> egui::Id {
    pane_id.with("frost_pane_section_rects")
}

/// Cached `(id, rect, title)` snapshot of last-frame's section
/// layout — drives target-slot computation, the ghost gap's size,
/// and the floating cursor preview's size + label during a drag.
#[derive(Clone)]
struct CachedSection {
    id: String,
    rect: egui::Rect,
    title: String,
}
type RectCache = Vec<CachedSection>;

/// Pick the target gap-index for a drag, walking the cache in
/// display order, SKIPPING the dragged section, and returning the
/// first slot whose centre Y is below the cursor. Indices are in
/// the non-dragged-only space (0 = above all others, N = below all
/// others, where N is the number of non-dragged sections).
fn compute_target_among_others(
    cache: &RectCache,
    dragged: &str,
    cursor: f32,
    horizontal: bool,
) -> usize {
    let mut idx = 0;
    for cs in cache {
        if cs.id == dragged {
            continue;
        }
        let center = if horizontal {
            cs.rect.center().x
        } else {
            cs.rect.center().y
        };
        if cursor < center {
            return idx;
        }
        idx += 1;
    }
    idx
}

/// Paint a same-sized ghost rect — the visual placeholder the user
/// sees opening up at the drop target. Same recipe as the ribbon's
/// drop-target outline (accent fill at α 28, 1.5 px accent stroke,
/// `radius::MD` corner) so the two drag UIs feel like one family.
/// Allocates the rect so layout flows around it; height comes from
/// the dragged section's cached size.
fn paint_ghost_gap(
    ui: &mut egui::Ui,
    cache: &RectCache,
    dragged: &str,
    accent: egui::Color32,
    horizontal: bool,
) {
    let dragged_size = cache
        .iter()
        .find(|cs| cs.id == dragged)
        .map(|cs| cs.rect.size())
        .unwrap_or(egui::vec2(48.0, 48.0));
    let alloc_size = if horizontal {
        egui::vec2(dragged_size.x, ui.available_height())
    } else {
        egui::vec2(ui.available_width(), dragged_size.y)
    };
    let (rect, _) = ui.allocate_exact_size(alloc_size, egui::Sense::hover());
    let th = crate::style::theme();
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(th.radius_md),
        egui::Color32::from_rgba_unmultiplied(
            accent.r(),
            accent.g(),
            accent.b(),
            th.ghost_fill_alpha,
        ),
        egui::Stroke::new(th.ghost_stroke_width, accent),
        egui::StrokeKind::Inside,
    );
}

/// Paint the dragged section's floating preview at the cursor: a
/// faded glass card sized to match the lifted section, with its
/// title centred at the top so the user sees what they're holding.
/// Drawn in an `egui::Area` at `Order::Tooltip` (independent of the
/// pane window's paint layer — no glass blending issues) with
/// `multiply_opacity` to fade the whole thing.
fn paint_drag_preview(
    ctx: &egui::Context,
    pane_id: egui::Id,
    cache: &RectCache,
    dragged: &str,
    cursor: egui::Pos2,
    accent: egui::Color32,
) {
    let Some(cs) = cache.iter().find(|cs| cs.id == dragged) else {
        return;
    };
    let size = cs.rect.size();
    let pos = egui::pos2(cursor.x - size.x * 0.5, cursor.y - size.y * 0.5);
    let area_id = pane_id.with(("frost_pane_drag_preview", dragged));
    egui::Area::new(area_id)
        .order(egui::Order::Tooltip)
        .fixed_pos(pos)
        .interactable(false)
        .show(ctx, |ui| {
            ui.set_max_width(size.x);
            ui.multiply_opacity(0.5);
            egui::Frame::new()
                .fill(crate::style::glass_fill(
                    crate::style::theme().bg_raised,
                    accent,
                    crate::style::glass_alpha_card(),
                ))
                .corner_radius(egui::CornerRadius::same(crate::style::theme().radius_md))
                .stroke(egui::Stroke::new(crate::style::theme().border_width, crate::style::widget_border(accent)))
                .show(ui, |ui| {
                    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                    let title_widget = egui::WidgetText::from(crate::style::section_caps(
                        &cs.title,
                        accent,
                    ));
                    let galley = title_widget.into_galley(
                        ui,
                        Some(egui::TextWrapMode::Extend),
                        size.x,
                        egui::TextStyle::Body,
                    );
                    let pos = egui::pos2(rect.left() + 18.0, rect.top() + 11.0);
                    ui.painter().galley(pos, galley, accent);
                });
        });
}

/// Paint a floating panel anchored to `anchor`. `size.x` / `size.y`
/// are the *initial* dimensions; once the user drags a resize
/// handle the new values are stored per-panel-id in
/// [`egui::Context::data`] and used on subsequent frames.
///
/// Title alignment flips automatically on right-side anchors so a
/// menu dragged across rails reads correctly, and the horizontal
/// resize handle follows to the opposite side. The vertical handle
/// follows the same anchor-opposite rule.
pub fn floating_window(
    ctx: &egui::Context,
    id: &'static str,
    title: &str,
    anchor: egui::Align2,
    size: egui::Vec2,
    open: &mut bool,
    accent: egui::Color32,
    add_contents: impl FnOnce(&mut PaneBuilder),
) {
    let on_right_side = matches!(
        anchor,
        egui::Align2::RIGHT_TOP | egui::Align2::RIGHT_BOTTOM
    );
    let scope = egui::Id::new(if on_right_side {
        "frost_panel_width_right"
    } else {
        "frost_panel_width_left"
    });
    // Default offset — assume a Left/Right rail since this
    // convenience entry has no edge info. Top/Bottom-rail callers
    // should use `floating_window_for_item` (which threads the
    // ribbon's edge through `panel_anchor_offset`) for correct
    // pane placement.
    let side_inset = EDGE_GAP + SIDE_BTN_SIZE + RAIL_PANEL_GAP;
    let default_offset = match anchor {
        egui::Align2::LEFT_TOP    => egui::vec2(side_inset, EDGE_GAP),
        egui::Align2::LEFT_CENTER => egui::vec2(side_inset, 0.0),
        egui::Align2::LEFT_BOTTOM => egui::vec2(side_inset, -EDGE_GAP),
        egui::Align2::RIGHT_TOP    => egui::vec2(-side_inset, EDGE_GAP),
        egui::Align2::RIGHT_CENTER => egui::vec2(-side_inset, 0.0),
        egui::Align2::RIGHT_BOTTOM => egui::vec2(-side_inset, -EDGE_GAP),
        egui::Align2::CENTER_TOP    => egui::vec2(0.0, side_inset),
        egui::Align2::CENTER_BOTTOM => egui::vec2(0.0, -side_inset),
        _ => egui::vec2(side_inset, EDGE_GAP),
    };
    floating_window_scoped(
        ctx, id, title, anchor, default_offset, size, open, accent, scope, false, add_contents,
    )
}

/// Same as [`floating_window`] but the dim-storage key is supplied
/// by the caller. Use this when you want independent widths /
/// heights for panels that *share* an anchor side — e.g. a
/// TwoSided ribbon's Start and End clusters both anchored to
/// `LEFT_*` but each with its own memory.
///
/// `horizontal`: when `true`, sections lay out side-by-side with a
/// horizontal scroll on overflow (used by Top / Bottom rail panes
/// where the pane is wider than tall). When `false` (default),
/// sections stack vertically.
pub fn floating_window_scoped(
    ctx: &egui::Context,
    id: &'static str,
    title: &str,
    anchor: egui::Align2,
    anchor_offset: egui::Vec2,
    size: egui::Vec2,
    _open: &mut bool,
    accent: egui::Color32,
    width_scope: egui::Id,
    horizontal: bool,
    add_contents: impl FnOnce(&mut PaneBuilder),
) {
    // Adapt the user's raw accent into the readable lightness band
    // for the active brightness mode BEFORE threading it through
    // every section / widget below — this makes the aggressive
    // light/dark caps actually take effect (otherwise the raw
    // accent would propagate through `PaneBuilder.accent` and the
    // adaptation would only affect helpers that read
    // `active_accent()`).
    let accent = crate::style::adapt_accent_to_mode(accent, crate::style::theme().is_light);
    let on_right_side = matches!(
        anchor,
        egui::Align2::RIGHT_TOP | egui::Align2::RIGHT_CENTER | egui::Align2::RIGHT_BOTTOM
    );
    // "Bottom-anchored" — panel grows upward from the bottom edge,
    // so its vertical-resize handle lives on its TOP edge (the edge
    // facing *away* from the anchor, same logic as the horizontal
    // handle).
    let bottom_anchored = matches!(
        anchor,
        egui::Align2::LEFT_BOTTOM
            | egui::Align2::CENTER_BOTTOM
            | egui::Align2::RIGHT_BOTTOM
    );

    let width_id = width_scope;
    let height_id = width_scope.with("_height");

    // Load stored values. Clamp to the current content_rect so
    // shrinking the Bevy window never leaves the panel wider / taller
    // than the visible area.
    let screen = ctx.content_rect();
    let side_inset = EDGE_GAP + SIDE_BTN_SIZE + RAIL_PANEL_GAP;
    let max_allowed_w = (screen.width() - side_inset - EDGE_GAP)
        .clamp(MIN_PANEL_W, MAX_PANEL_W);
    let max_allowed_h = (screen.height() - 2.0 * EDGE_GAP)
        .clamp(MIN_PANEL_H, MAX_PANEL_H);

    // Vertical (LEFT/RIGHT) panes: width is user-resizable. Horizontal
    // (TOP/BOTTOM) panes: width is content-driven — `N * SECTION_W`
    // plus the gap / padding budget below — so the user's only
    // resize axis is height (the vertical handle on the body's far
    // edge). Section count comes from the previous frame; first
    // open defaults to 1 so the pane is `SECTION_W`-wide until
    // the body has run once.
    // Section count — read from the previous frame's `finalize`
    // pass. Used to compute the horizontal pane's content-driven
    // width below. Defaults to 1 on the very first frame so the
    // pane is one section wide until the body has run once.
    let count_id = section_count_key(egui::Id::new(id));
    let stored_section_count: usize = ctx
        .data(|d| d.get_temp::<usize>(count_id))
        .unwrap_or(1)
        .max(1);
    let stored_width: f32 = if horizontal {
        // Pane width = title strip + body-gap + section row.
        // Section row sums each section's CURRENT lerped width.
        // `finalize` writes each section's `(state_id, openness)`
        // pair into `section_states_key` after the body renders;
        // we read it back here in the SAME frame's "pre-render"
        // calc. The cache races toward the live state in one
        // frame — same-frame reads in egui are fine because
        // `ctx.data` is a single shared map.
        let pane_id = egui::Id::new(id);
        let openness_cache: Vec<f32> = ctx
            .data(|d| d.get_temp::<Vec<f32>>(section_states_key(pane_id)))
            .unwrap_or_default();
        let n = stored_section_count.max(1);
        let mut widths: Vec<f32> = Vec::with_capacity(n);
        for i in 0..n {
            let openness = *openness_cache.get(i).unwrap_or(&1.0);
            widths.push(egui::lerp(
                crate::widgets::foldable::H_SECTION_TITLE_STRIP_W
                    ..=HORIZONTAL_SECTION_W,
                openness,
            ));
        }
        let count = widths.len() as f32;
        let inter_gap = HORIZONTAL_INTER_CARD_GAP;
        let frame_pad = 8.0_f32;
        let section_row =
            widths.iter().sum::<f32>() + (count - 1.0).max(0.0) * inter_gap;
        let raw = frame_pad + VERTICAL_TITLE_W + VERTICAL_TITLE_BODY_GAP + section_row;
        let h_min_w = frame_pad
            + VERTICAL_TITLE_W
            + VERTICAL_TITLE_BODY_GAP
            + crate::widgets::foldable::H_SECTION_TITLE_STRIP_W;
        raw.clamp(h_min_w, max_allowed_w)
    } else {
        ctx
            .data(|d| d.get_temp::<f32>(width_id))
            .unwrap_or(size.x)
            .clamp(MIN_PANEL_W, max_allowed_w)
    };
    let stored_height: f32 = ctx
        .data(|d| d.get_temp::<f32>(height_id))
        .unwrap_or(size.y)
        .clamp(MIN_PANEL_H, max_allowed_h);

    // Write the clamped values back so a shrunken Bevy window
    // permanently shrinks the stored values (user's drag wasn't
    // wasted, but it no longer exceeds the visible area).
    // For horizontal panes the width is content-driven so we don't
    // re-write `width_id` — only the height persists.
    ctx.data_mut(|d| {
        if !horizontal {
            d.insert_temp::<f32>(width_id, stored_width);
        }
        d.insert_temp::<f32>(height_id, stored_height);
    });

    // `anchor_offset` is supplied by the caller now — `panel_anchor_offset`
    // in the ribbon module computes it edge-aware for ribbon-spawned
    // panes (so a Top-rail pane offsets DOWN, not RIGHT). The
    // `floating_window` convenience wrapper still derives a sensible
    // default from the bare anchor for non-ribbon callers.

    // Open-animation: track elapsed seconds since the pane became
    // visible. Sections render with a staggered per-section
    // fade-in (each section pops in fully, one after the other)
    // instead of a body-wide clip curtain. The elapsed time is
    // threaded into `PaneBuilder` and consumed by `section_with`.
    let pane_open_elapsed: f32 = {
        let frame_key = egui::Id::new(("frost_pane_anim_frame", id));
        let state_key = egui::Id::new(("frost_pane_anim_state", id));
        let frame_now = ctx.cumulative_pass_nr();
        let last_frame: u64 = ctx.data(|d| d.get_temp(frame_key)).unwrap_or(0);
        let mut elapsed: f32 = ctx.data(|d| d.get_temp(state_key)).unwrap_or(99.0);
        if last_frame + 1 < frame_now {
            elapsed = 0.0;
        }
        let dt = ctx.input(|i| i.unstable_dt).max(0.0);
        elapsed += dt;
        ctx.data_mut(|d| {
            d.insert_temp(state_key, elapsed);
            d.insert_temp(frame_key, frame_now);
        });
        // Repaint while any reasonably staged section is still
        // animating in (~12 sections × 0.18 stagger + 0.45 fade
        // ≈ 2.6 s — keep some headroom).
        if elapsed < 3.0 {
            ctx.request_repaint();
        }
        elapsed
    };

    // `pane_fill(accent)` resolves the theme's panel-fill mode —
    // PRO returns the dark `bg_panel`; GAME returns an
    // accent-derived dark colour so the pane visually carries the
    // user's accent across its whole surface.
    // Pane fill: themes that want a pane-wide opaque background
    // paint it via the egui Frame's fill (PRO). Themes that want
    // see-through gaps between sections (GAME) flip
    // `pane_fill_visible` off — the egui frame paints transparent,
    // and each section will render its own opaque card so the gap
    // *between* sections shows the scene below.
    let pane_fill_col = if crate::style::theme().pane_fill_visible {
        glass_fill(crate::style::pane_fill(accent), accent, glass_alpha_window())
    } else {
        egui::Color32::TRANSPARENT
    };
    let frame = egui::Frame {
        inner_margin: egui::Margin { left: 2, right: 2, top: 2, bottom: 2 },
        outer_margin: egui::Margin::ZERO,
        fill: pane_fill_col,
        stroke: egui::Stroke::new(crate::style::theme().border_width, crate::style::widget_border(accent)),
        corner_radius: egui::CornerRadius::same(crate::style::theme().radius_lg),
        shadow: egui::epaint::Shadow {
            offset: [0, crate::style::theme().pane_shadow_y],
            blur: crate::style::theme().pane_shadow_blur,
            spread: 0,
            color: egui::Color32::from_black_alpha(115),
        },
    };

    // Render via egui::Area + Frame::show. Two sizing modes:
    //
    // * HORIZONTAL panes (TOP/BOTTOM rails): both axes pinned —
    //   width is content-driven (`N * SECTION_W`), height is the
    //   user-resizable `stored_height`. Container cells inside
    //   stretch to the full pinned height.
    // * VERTICAL panes (LEFT/RIGHT rails): width is the user-resizable
    //   `stored_width`, height is content-driven — the pane shrinks
    //   to exactly fit its sections (folded or open). No empty
    //   space below the last container.
    //
    // For bottom-anchored VERTICAL panes the position depends on
    // the actual content height, so we cache the height from the
    // previous frame and use it for `compute_pane_pos`. The cache
    // converges in one frame after the body's first paint.
    let actual_h_key = egui::Id::new(("frost_pane_actual_h", id));
    let estimated_h = if horizontal {
        stored_height
    } else {
        ctx.data(|d| d.get_temp::<f32>(actual_h_key))
            .unwrap_or(stored_height)
    };
    let estimated_size = egui::vec2(stored_width, estimated_h);
    let pane_pos = compute_pane_pos(anchor, anchor_offset, screen, estimated_size);

    // ── LAYOUT DEBUG: pane corners ────────────────────────────────
    // Print on change (rounded to integers to filter sub-pixel
    // jitter) so the log doesn't spam every frame. Grep for
    // `[LAYOUT]` in stdout to inspect.
    log_layout_change(
        ctx,
        egui::Id::new(("frost_layout_pane", id)),
        format_args!(
            "PANE \"{}\" tl=({:.0},{:.0}) br=({:.0},{:.0}) sz=({:.0}x{:.0}) horizontal={}",
            id,
            pane_pos.x,
            pane_pos.y,
            pane_pos.x + estimated_size.x,
            pane_pos.y + estimated_size.y,
            estimated_size.x,
            estimated_size.y,
            horizontal,
        ),
    );

    let area_id = egui::Id::new(("frost_pane_area", id));
    let inner_resp = egui::Area::new(area_id)
        .order(egui::Order::Middle)
        .fixed_pos(pane_pos)
        .movable(false)
        .interactable(true)
        .show(ctx, |ui| {
            // Width is always pinned; height is pinned for horizontal
            // panes and content-driven for vertical ones.
            let outer_min = if horizontal {
                egui::vec2(stored_width, stored_height)
            } else {
                egui::vec2(stored_width, 0.0)
            };
            let outer_max = if horizontal {
                egui::vec2(stored_width, stored_height)
            } else {
                egui::vec2(stored_width, max_allowed_h)
            };
            // Clamp size args to non-negative — egui panics on
            // negative arguments to set_min_size / set_max_size.
            let outer_min = egui::vec2(outer_min.x.max(0.0), outer_min.y.max(0.0));
            let outer_max = egui::vec2(outer_max.x.max(0.0), outer_max.y.max(0.0));
            ui.set_min_size(outer_min);
            ui.set_max_size(outer_max);
            frame.show(ui, |ui| {
                let inner_min = if horizontal {
                    egui::vec2(stored_width - 4.0, stored_height - 4.0)
                } else {
                    egui::vec2(stored_width - 4.0, 0.0)
                };
                let inner_min = egui::vec2(inner_min.x.max(0.0), inner_min.y.max(0.0));
                ui.set_min_size(inner_min);
            // Vertical panes keep the legacy -6 max_width to leave a
            // sliver for the custom h_handle. Horizontal panes don't
            // have an h_handle (width is content-driven), so they use
            // the full inner width — without this the outer flex's
            // `width(Percent(1.0))` was 6 px short and the section
            // row overflowed by exactly that.
            if !horizontal {
                ui.set_max_width(stored_width - 6.0);
            } else {
                ui.set_max_width(stored_width - 4.0);
            }

            const TITLE_INSET: f32 = 8.0;
            let title_size = 15.0 * 1.15;
            let title_h = 25.0;

            // Title painter — handles all four orientations
            // (horizontal top/bottom, vertical left/right). The
            // vertical variants render the title text rotated ±π/2
            // via `epaint::TextShape::angle`, with the divider
            // hairline placed on the side facing the body so it
            // always reads as "separator between title and content"
            // regardless of where the title sits.
            let paint_title = |ui: &mut egui::Ui,
                               rect: egui::Rect,
                               orientation: TitleOrientation| {
                let stripes_on = crate::style::theme().pane_title_stripes;
                if !crate::style::theme().pane_fill_visible && !stripes_on {
                    ui.painter().rect_filled(
                        rect,
                        egui::CornerRadius::same(crate::style::theme().radius_lg),
                        crate::style::pane_fill(accent),
                    );
                }
                if stripes_on {
                    crate::style::paint_caution_stripes(ui.painter(), rect, accent);
                }
                let title_col = if stripes_on {
                    if crate::style::theme().is_light {
                        egui::Color32::BLACK
                    } else {
                        egui::Color32::WHITE
                    }
                } else {
                    crate::style::section_title_color(accent)
                };
                let font = egui::FontId::new(title_size, crate::style::title_font_family());
                let title_uc = title.to_uppercase();
                let displayed = if crate::style::theme().scramble_titles {
                    let session_id = egui::Id::new(("frost_pane_title_session", id));
                    let session = crate::style::appearance_session(ui.ctx(), session_id);
                    let scramble_id = session_id.with(session);
                    crate::style::scramble_text(ui.ctx(), scramble_id, &title_uc, true)
                } else {
                    title_uc
                };

                match orientation {
                    TitleOrientation::HorizontalTop | TitleOrientation::HorizontalBottom => {
                        let (align, tx) = if on_right_side {
                            (egui::Align2::RIGHT_CENTER, rect.max.x - TITLE_INSET)
                        } else {
                            (egui::Align2::LEFT_CENTER, rect.min.x + TITLE_INSET)
                        };
                        let pos = egui::pos2(tx.round(), rect.center().y.round());
                        ui.painter().text(pos, align, displayed, font, title_col);
                    }
                    TitleOrientation::Vertical { on_right, top_to_bottom } => {
                        log_layout_change(
                            ui.ctx(),
                            egui::Id::new(("frost_layout_pane_title", id)),
                            format_args!(
                                "PANE_TITLE \"{}\" tl=({:.0},{:.0}) br=({:.0},{:.0}) sz=({:.0}x{:.0}) on_right={} top_to_bottom={}",
                                id,
                                rect.min.x, rect.min.y,
                                rect.max.x, rect.max.y,
                                rect.width(), rect.height(),
                                on_right, top_to_bottom,
                            ),
                        );
                        // Mirrors the vertical pane's left-aligned
                        // title: the FIRST letter sits at the
                        // strip's "start" edge (bottom for
                        // bottom-to-top, top for top-to-bottom).
                        // Across the narrow axis the text is aligned
                        // to the OUTER edge of the strip — the side
                        // facing away from the body — same way a
                        // horizontal title hugs the left edge of
                        // its strip on a left-anchored pane.
                        let galley = ui.painter().layout_no_wrap(
                            displayed,
                            font,
                            title_col,
                        );
                        let text_h = galley.size().y;
                        // Outer-edge horizontal position. For LEFT
                        // strip, the rotated text's outer (left)
                        // edge sits at `rect.min.x + 1` (1 px so
                        // the divider hairline can sit cleanly).
                        // For RIGHT strip, mirror.
                        let (pos, angle) = if top_to_bottom {
                            // +π/2: rotated text extends LEFT of pos.
                            // To pin text-LEFT at the outer edge:
                            //   text-left = pos.x - text_h.
                            // For LEFT strip (on_right=false), outer
                            // = strip.min.x. → pos.x = min.x + text_h.
                            // For RIGHT strip, outer = strip.max.x.
                            // → pos.x = max.x.
                            let px = if on_right {
                                rect.max.x - 1.0
                            } else {
                                rect.min.x + text_h + 1.0
                            };
                            let py = (rect.min.y + TITLE_INSET).round();
                            (egui::pos2(px.round(), py), std::f32::consts::FRAC_PI_2)
                        } else {
                            // -π/2: rotated text extends RIGHT of pos.
                            // To pin text-LEFT at outer edge:
                            //   text-left = pos.x.
                            // LEFT strip outer = strip.min.x → pos.x = min.x.
                            // RIGHT strip outer = strip.max.x → pos.x = max.x - text_h.
                            let px = if on_right {
                                rect.max.x - text_h - 1.0
                            } else {
                                rect.min.x + 1.0
                            };
                            let py = (rect.max.y - TITLE_INSET).round();
                            (egui::pos2(px.round(), py), -std::f32::consts::FRAC_PI_2)
                        };
                        let mut shape = egui::epaint::TextShape::new(pos, galley, title_col);
                        shape.angle = angle;
                        ui.painter().add(shape);
                    }
                }

                if crate::style::theme().pane_title_stripes {
                    const PIP_SIZE: f32 = 6.0;
                    const PIP_INSET: f32 = TITLE_INSET;
                    let time = ui.ctx().input(|i| i.time) as f32;
                    let on = time.fract() < 0.08;
                    let alpha = if on { 255 } else { 76 };
                    let pip_color = egui::Color32::from_rgba_unmultiplied(
                        title_col.r(),
                        title_col.g(),
                        title_col.b(),
                        alpha,
                    );
                    let pip_rect = match orientation {
                        TitleOrientation::HorizontalTop
                        | TitleOrientation::HorizontalBottom => {
                            let pip_x = if on_right_side {
                                rect.min.x + PIP_INSET
                            } else {
                                rect.max.x - PIP_INSET - PIP_SIZE
                            };
                            egui::Rect::from_min_size(
                                egui::pos2(
                                    pip_x.round(),
                                    (rect.center().y - PIP_SIZE * 0.5).round(),
                                ),
                                egui::vec2(PIP_SIZE, PIP_SIZE),
                            )
                        }
                        TitleOrientation::Vertical { top_to_bottom, .. } => {
                            // Pip lives at the OPPOSITE end of the
                            // strip from the text — bottom-up text
                            // ⇒ pip at the top, top-down text ⇒ pip
                            // at the bottom.
                            let pip_y = if top_to_bottom {
                                rect.max.y - PIP_INSET - PIP_SIZE
                            } else {
                                rect.min.y + PIP_INSET
                            };
                            egui::Rect::from_min_size(
                                egui::pos2(
                                    (rect.center().x - PIP_SIZE * 0.5).round(),
                                    pip_y.round(),
                                ),
                                egui::vec2(PIP_SIZE, PIP_SIZE),
                            )
                        }
                    };
                    ui.painter().rect_filled(
                        pip_rect,
                        egui::CornerRadius::ZERO,
                        pip_color,
                    );
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
                }
                if crate::style::theme().pane_show_title_divider {
                    let stroke = egui::Stroke::new(
                        crate::style::theme().border_width,
                        crate::style::widget_border(accent),
                    );
                    match orientation {
                        TitleOrientation::HorizontalTop => {
                            ui.painter().hline(
                                rect.min.x..=rect.max.x,
                                rect.max.y + 3.0,
                                stroke,
                            );
                        }
                        TitleOrientation::HorizontalBottom => {
                            ui.painter().hline(
                                rect.min.x..=rect.max.x,
                                rect.min.y - 3.0,
                                stroke,
                            );
                        }
                        TitleOrientation::Vertical { on_right, .. } => {
                            // Divider sits on the body-facing edge
                            // of the strip: right of LEFT strip,
                            // left of RIGHT strip.
                            let x = if on_right {
                                rect.min.x - 3.0
                            } else {
                                rect.max.x + 3.0
                            };
                            ui.painter().vline(x, rect.min.y..=rect.max.y, stroke);
                        }
                    }
                }
            };

            let pane_id = egui::Id::new(id);
            let drag: SectionDragState = ctx
                .data(|d| d.get_temp::<SectionDragState>(drag_key(pane_id)))
                .unwrap_or_default();
            let cached_rects: RectCache = ctx
                .data(|d| d.get_temp::<RectCache>(rects_key(pane_id)))
                .unwrap_or_default();
            let side_key = egui::Id::new("frost_current_pane_right_anchored");

            // Run the user's body with PaneBuilder. Two paths:
            //
            // * VERTICAL mode → PaneBuilder holds `PaneTarget::Ui`.
            //   Sections render via the standard `section_tracked`
            //   stack on the parent ui.
            // * HORIZONTAL mode → wrap the body in a Flex row,
            //   PaneBuilder holds `PaneTarget::Flex(&mut flex)`,
            //   and each section call routes through `flex.add_ui`.
            //
            // Both paths funnel through `dispatch` which builds
            // PaneBuilder, runs add_contents, finalizes.
            let run_body = |body_ui: &mut egui::Ui,
                            drag: SectionDragState,
                            cached_rects: RectCache,
                            add_contents: Box<dyn FnOnce(&mut PaneBuilder) + '_>| {
                let body_top = body_ui.cursor().min.y;
                let body_left = body_ui.cursor().min.x;
                let body_w = body_ui.available_width();
                let body_h = (body_ui.max_rect().bottom() - body_top).max(0.0);
                let body_rect = egui::Rect::from_min_size(
                    egui::pos2(body_left, body_top),
                    egui::vec2(body_w, body_h),
                );

                let dispatch = |target: PaneTarget<'_, '_>,
                                add_contents: Box<dyn FnOnce(&mut PaneBuilder) + '_>| {
                    let mut pane = PaneBuilder {
                        target,
                        accent,
                        pane_id,
                        body_rect,
                        rendered: Vec::new(),
                        non_dragged_count: 0,
                        drag,
                        drag_started_id: None,
                        base_order_this_frame: Vec::new(),
                        cached_rects,
                        just_toggled_id: None,
                        pane_open_elapsed,
                        horizontal,
                        h_section_title_on_right: on_right_side,
                        h_section_top_to_bottom: !bottom_anchored,
                    };
                    let ctx_clone = pane.target.ctx().clone();
                    let prev_side: Option<bool> = ctx_clone.data(|d| d.get_temp(side_key));
                    ctx_clone.data_mut(|d| d.insert_temp::<bool>(side_key, on_right_side));
                    add_contents(&mut pane);
                    ctx_clone.data_mut(|d| match prev_side {
                        Some(v) => d.insert_temp::<bool>(side_key, v),
                        None => {
                            d.remove::<bool>(side_key);
                        }
                    });
                    pane.finalize();
                };

                if horizontal {
                    // Section row: allocate a full-height
                    // left-to-right child ui explicitly. Plain
                    // `body_ui.horizontal` defaults the row height
                    // to a single text-line — sections inside
                    // would inherit that ~20 px tall rect via
                    // `ui.available_height()`. Egui_flex would
                    // cache item sizes between frames, masking
                    // pane resizes. Explicit allocate_ui_with_layout
                    // + set_min_height forces the row to the body's
                    // current full height every frame.
                    let row_size = egui::vec2(
                        body_ui.available_width(),
                        body_ui.available_height(),
                    );
                    // Publish the row's available width so each
                    // `section_with` call can fit its card to
                    // `body_avail_w / count` — prevents the last
                    // container from overflowing past the body's
                    // right edge when many sections are unfolded.
                    body_ui.ctx().data_mut(|d| {
                        d.insert_temp::<f32>(h_body_avail_key(pane_id), row_size.x)
                    });
                    body_ui.allocate_ui_with_layout(
                        row_size,
                        egui::Layout::left_to_right(egui::Align::TOP),
                        |row_ui| {
                            row_ui.set_min_height(row_size.y);
                            row_ui.set_max_height(row_size.y);
                            // Hard clip so any per-section overflow
                            // (transient on first frame, or stale
                            // cache) can't paint past the body rect
                            // into the title strip.
                            row_ui.set_clip_rect(row_ui.clip_rect().intersect(
                                egui::Rect::from_min_size(row_ui.cursor().min, row_size),
                            ));
                            // Zero `item_spacing` so the pane-width
                            // calc (which only sums per-section card
                            // widths + `HORIZONTAL_INTER_CARD_GAP`)
                            // matches what egui actually renders. With
                            // the default 6 px spacing each extra
                            // section drifts further past the body's
                            // right edge.
                            row_ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                            dispatch(PaneTarget::Ui(row_ui), add_contents);
                        },
                    );
                } else {
                    dispatch(PaneTarget::Ui(body_ui), add_contents);
                }
            };

            if horizontal {
                // Manual layout — egui_flex's Stretch align doesn't
                // size paint-only items (title strip) to flex
                // height, so the title strip ended up 0-tall.
                // Direct rect math gives exact, instant sizes.
                // TOP rail panes: text reads top-to-bottom (first
                // letter at the top edge of the strip). BOTTOM
                // rail: bottom-to-top (first letter at bottom).
                // Mirror the convention `text starts at the rail's
                // outer side` so the eye reads the title flowing
                // INTO the screen.
                let title_orientation = TitleOrientation::Vertical {
                    on_right: on_right_side,
                    top_to_bottom: !bottom_anchored,
                };
                let max_rect = ui.max_rect();
                let title_rect = if on_right_side {
                    egui::Rect::from_min_max(
                        egui::pos2(max_rect.max.x - VERTICAL_TITLE_W, max_rect.min.y),
                        max_rect.max,
                    )
                } else {
                    egui::Rect::from_min_max(
                        max_rect.min,
                        egui::pos2(max_rect.min.x + VERTICAL_TITLE_W, max_rect.max.y),
                    )
                };
                let body_rect = if on_right_side {
                    egui::Rect::from_min_max(
                        max_rect.min,
                        egui::pos2(
                            max_rect.max.x - VERTICAL_TITLE_W - VERTICAL_TITLE_BODY_GAP,
                            max_rect.max.y,
                        ),
                    )
                } else {
                    egui::Rect::from_min_max(
                        egui::pos2(
                            max_rect.min.x + VERTICAL_TITLE_W + VERTICAL_TITLE_BODY_GAP,
                            max_rect.min.y,
                        ),
                        max_rect.max,
                    )
                };

                paint_title(ui, title_rect, title_orientation);

                log_layout_change(
                    ui.ctx(),
                    egui::Id::new(("frost_layout_pane_body", id)),
                    format_args!(
                        "PANE_BODY \"{}\" tl=({:.0},{:.0}) br=({:.0},{:.0}) sz=({:.0}x{:.0})",
                        id,
                        body_rect.min.x, body_rect.min.y,
                        body_rect.max.x, body_rect.max.y,
                        body_rect.width(), body_rect.height(),
                    ),
                );

                let mut body_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(body_rect)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                body_ui.set_min_size(body_rect.size());
                body_ui.set_max_size(body_rect.size());
                run_body(&mut body_ui, drag, cached_rects, Box::new(add_contents));

                // Advance ui's cursor across the whole pane area so
                // anything else expecting full-width allocation
                // (resize-handle math, ctx.allocate space) sees the
                // full pane consumed.
                let _ = ui.allocate_exact_size(
                    max_rect.size(),
                    egui::Sense::hover(),
                );
            } else if bottom_anchored {
                // Vertical pane (LEFT/RIGHT) with bottom-anchored
                // cluster: body first (top of pane), title at the
                // pane's bottom. Cursor-based so content drives
                // height — no empty space below the last container.
                run_body(ui, drag, cached_rects, Box::new(add_contents));
                ui.add_space(6.0);
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), title_h),
                    egui::Sense::hover(),
                );
                paint_title(ui, rect, TitleOrientation::HorizontalBottom);
            } else {
                // Vertical pane (LEFT/RIGHT) with top-anchored cluster:
                // title at top, body below. Cursor-based so the pane
                // shrinks to fit the section stack with no empty
                // space below the last container.
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), title_h),
                    egui::Sense::hover(),
                );
                paint_title(ui, rect, TitleOrientation::HorizontalTop);
                ui.add_space(6.0);
                run_body(ui, drag, cached_rects, Box::new(add_contents));
            }
            });
        });

    // Cache the actual rendered height for vertical panes so the
    // next frame's `compute_pane_pos` (and the v_handle math, if
    // anything depends on it) sees the up-to-date size.
    let actual_h = inner_resp.response.rect.height();
    if !horizontal {
        ctx.data_mut(|d| d.insert_temp::<f32>(actual_h_key, actual_h));
    }

    let pane_size_now = if horizontal {
        egui::vec2(stored_width, stored_height)
    } else {
        egui::vec2(stored_width, actual_h)
    };
    let win_rect = egui::Rect::from_min_size(pane_pos, pane_size_now);
    let _ = on_right_side;

    // ── Custom horizontal resize handle (vertical panes only) ─────
    if !horizontal {
        let h_handle_rect = if on_right_side {
            egui::Rect::from_min_size(
                egui::pos2(win_rect.min.x - RESIZE_HANDLE_W, win_rect.min.y),
                egui::vec2(RESIZE_HANDLE_W, win_rect.height()),
            )
        } else {
            egui::Rect::from_min_size(
                egui::pos2(win_rect.max.x, win_rect.min.y),
                egui::vec2(RESIZE_HANDLE_W, win_rect.height()),
            )
        };
        let h_area_id = width_id.with("resize_handle_w");
        egui::Area::new(h_area_id)
            .order(egui::Order::Foreground)
            .fixed_pos(h_handle_rect.min)
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(
                    h_handle_rect.size(),
                    egui::Sense::click_and_drag(),
                );
                let alpha: u8 = if resp.dragged() {
                    160
                } else if resp.hovered() {
                    110
                } else {
                    40
                };
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::same(crate::style::theme().radius_compact),
                    egui::Color32::from_rgba_unmultiplied(
                        accent.r(), accent.g(), accent.b(), alpha,
                    ),
                );
                if resp.hovered() || resp.dragged() {
                    ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                }
                if resp.dragged() {
                    let dx = resp.drag_delta().x;
                    let new_w = if on_right_side {
                        stored_width - dx
                    } else {
                        stored_width + dx
                    };
                    let clamped = new_w.clamp(MIN_PANEL_W, max_allowed_w);
                    ctx.data_mut(|d| d.insert_temp::<f32>(width_id, clamped));
                }
            });
    }

    // ── Custom vertical resize handle (horizontal panes only) ────
    //
    // Symmetric to the h_handle above: vertical (LEFT/RIGHT) panes
    // are width-only resizable, horizontal (TOP/BOTTOM) panes are
    // height-only resizable. Each rail orientation gets exactly one
    // resize axis.
    if horizontal {
        let v_handle_rect = if bottom_anchored {
            egui::Rect::from_min_size(
                egui::pos2(win_rect.min.x, win_rect.min.y - RESIZE_HANDLE_H),
                egui::vec2(win_rect.width(), RESIZE_HANDLE_H),
            )
        } else {
            egui::Rect::from_min_size(
                egui::pos2(win_rect.min.x, win_rect.max.y),
                egui::vec2(win_rect.width(), RESIZE_HANDLE_H),
            )
        };
        let v_area_id = width_id.with("resize_handle_h");
        egui::Area::new(v_area_id)
            .order(egui::Order::Foreground)
            .fixed_pos(v_handle_rect.min)
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(
                    v_handle_rect.size(),
                    egui::Sense::click_and_drag(),
                );
                let alpha: u8 = if resp.dragged() {
                    180
                } else if resp.hovered() {
                    120
                } else {
                    50
                };
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::same(crate::style::theme().radius_compact),
                    egui::Color32::from_rgba_unmultiplied(
                        accent.r(), accent.g(), accent.b(), alpha,
                    ),
                );
                if resp.hovered() || resp.dragged() {
                    ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical);
                }
                if resp.dragged() {
                    let dy = resp.drag_delta().y;
                    let new_h = if bottom_anchored {
                        stored_height - dy
                    } else {
                        stored_height + dy
                    };
                    let clamped = new_h.clamp(MIN_PANEL_H, max_allowed_h);
                    ctx.data_mut(|d| d.insert_temp::<f32>(height_id, clamped));
                }
            });
    }
}
