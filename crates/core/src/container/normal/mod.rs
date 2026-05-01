//! `Normal` container — a flex-based, two-part block (title zone +
//! body zone) dropped into a [`crate::pane::Pane2`] body. The
//! container's title sits on the **same side** of the block as the
//! parent pane's title strip, so nested chrome chords with the pane
//! chrome.
//!
//! Layout is plain egui (no flex). Title strip is allocated with
//! `allocate_exact_size`; the body is rendered into a child UI
//! whose `max_rect` is the FULL body extent and whose `clip_rect`
//! lerps with `ctx.animate_bool(...)` — same recipe egui's
//! `CollapsingState::show_body_unindented` uses.
//!
//! ```ignore
//! Normal::new("Properties", anchor, accent).show(ui, |ui| {
//!     ui.label("body content");
//! });
//! ```

use egui::{
    epaint::TextShape, pos2, vec2, Align, Color32, CornerRadius, FontId, Frame, Id, Layout, Rect,
    Sense, Stroke, Ui, UiBuilder,
};

use super::body::Body;
use crate::icons::Icon;
use crate::pane::{self, PaneAnchor, TitleSide};
use crate::style;

/// Title-bar thickness (perpendicular to the strip's long axis).
pub const TITLE_ZONE_THICKNESS: f32 = 22.0;
/// Inset between strip edge and the title text's reading-start.
const TITLE_INSET: f32 = 6.0;
/// Inset on each end of the title/body divider line so it stops
/// short of the container's frame corners.
const DIVIDER_INSET: f32 = 6.0;
/// Padding on EACH side of the divider — `TITLE_BODY_GAP_HALF` of
/// breathing space between the title text and the divider line, and
/// the same on the body side. Total flex gap = 2 × this constant.
const TITLE_BODY_GAP_HALF: f32 = 4.0;
/// Padding between title strip and body (currently rendered as gap=0
/// so the hairline divider reads cleanly; kept as a knob for later
/// tuning).
const _BODY_PAD: f32 = 6.0;
/// Default span-axis size. Used as the container's locked cross
/// dimension — width for horizontal-title containers, height for
/// vertical-title containers. The MAIN axis stays content-driven
/// (capped via `Body::max_flow` for vertical-title to stop a body
/// like `text_input` from growing the pane unboundedly along X).
/// Pane2's locked span axis matches this constant so the pane and
/// container share the same outer cross dimension.
pub const CONTAINER_DEFAULT_WIDTH: f32 = 280.0;
pub const CONTAINER_DEFAULT_HEIGHT: f32 = 280.0;
/// Default lower bound on a container's WIDTH. Bumped 30 % above
/// the old `frostcore::floating::MIN_PANEL_W` (= 220) so containers
/// don't open at a cramped slim width — vertical-strip panes
/// stack containers side-by-side, so a too-small default leaves
/// each one barely wider than its title strip until the user
/// drags. Used when a caller doesn't override via
/// [`Normal::min_width`]. The parent pane's resize handles consult
/// the maximum of its containers' min widths (span axis) or the
/// sum (flow axis) and refuse to shrink below it.
pub const CONTAINER_DEFAULT_MIN_WIDTH: f32 = 286.0;
// Container outer margins now come from the active theme:
//   `theme.section_outer_margin_main`  — flow-axis (between stacked
//      containers and between first container ↔ pane title strip).
//   `theme.section_outer_margin_span` — span-axis (between the
//      container's painted edge and the pane's left/right or
//      top/bottom chrome). PRO ≈ 3/3, GAME ≈ 9/1.

/// A labelled, single-body container. Build with [`Normal::new`],
/// then [`Normal::show`] each frame. The `anchor` is forwarded to
/// pick the title side; pass the same anchor the parent
/// [`crate::pane::Pane2`] uses. The `accent` drives the frame fill,
/// border, and (in PRO theme) title text colour.
pub struct Normal {
    title: String,
    anchor: PaneAnchor,
    accent: Color32,
    /// Parent pane's id. Used to look up / toggle the shared
    /// `body_open` state and the animation's `openness`, so
    /// `Pane2` and the container animate in lockstep.
    pane_id: Id,
    /// Optional title icon. Either a Fluent name or raw SVG markup.
    /// In PRO theme (`section_icon_at_end = false`) the icon is
    /// inlined into the title `LayoutJob` at the reading-start. In
    /// GAME theme (`section_icon_at_end = true`) it floats at the
    /// strip's far end and grows when the body unfolds — matching
    /// `frostcore::widgets::foldable::section_tracked`.
    icon: Option<Icon<'static>>,
    /// Optional override for the body slot's flow-axis size. Default
    /// derives from `CONTAINER_DEFAULT_HEIGHT/WIDTH` minus chrome,
    /// which is right for one-container-per-pane layouts. When you
    /// stack multiple containers in a single pane, divide the pane's
    /// available main extent and pass each container its share via
    /// this builder so they don't all claim the full pane.
    body_flow: Option<f32>,
    /// Minimum WIDTH this container will accept. The parent pane's
    /// user-resize handles consult the registered minimums (one per
    /// container painted this frame) and stop shrinking once the
    /// pane reaches that bound. Defaults to
    /// [`CONTAINER_DEFAULT_MIN_WIDTH`] when not set.
    min_width: Option<f32>,
}

impl Normal {
    pub fn new(
        title: impl Into<String>,
        anchor: PaneAnchor,
        accent: Color32,
        pane_id: impl Into<Id>,
    ) -> Self {
        Self {
            title: title.into(),
            anchor,
            accent,
            pane_id: pane_id.into(),
            icon: None,
            body_flow: None,
            min_width: None,
        }
    }

    /// Set the container's minimum WIDTH. The parent pane's resize
    /// handles refuse to shrink the pane below the largest min
    /// width registered by its containers (or the sum, when the
    /// containers stack along the pane's flow axis). Defaults to
    /// [`CONTAINER_DEFAULT_MIN_WIDTH`] (220 px) when unset.
    pub fn min_width(mut self, w: f32) -> Self {
        self.min_width = Some(w.max(0.0));
        self
    }

    /// Attach a title icon. Accepts a Fluent icon name (e.g.
    /// `"settings"`) or raw SVG markup — `Icon::from(&str)` picks
    /// the right variant from the leading characters.
    pub fn icon(mut self, icon: impl Into<Icon<'static>>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Override the body slot's flow-axis size. Use when stacking
    /// multiple containers in a pane so each gets a slice of the
    /// available main extent instead of all claiming the full pane.
    pub fn body_flow(mut self, main: f32) -> Self {
        self.body_flow = Some(main.max(0.0));
        self
    }

    /// Render the container with one or more pods stacked in the
    /// body slot. Containers only accept [`crate::pod::Pod`]s — raw
    /// widgets / closures are intentionally not supported. Returns
    /// one [`crate::pod::PodResponse`] per pod, in declaration
    /// order. Pass via `vec![pod1, pod2, ...]` or any
    /// `IntoIterator<Item = Pod>`.
    pub fn show(
        self,
        ui: &mut Ui,
        pods: impl IntoIterator<Item = crate::pod::Pod>,
    ) -> Vec<crate::pod::PodResponse> {
        // Push a per-container `id` salt so every widget the
        // container creates — `Frame::show`'s anonymous content_ui,
        // the body's `ScrollArea`, the pods' `text_input`s, etc. —
        // gets an id chain rooted at THIS container's `pane_id`.
        // Without this, multiple containers in the same parent
        // body_ui inherit `body_ui.id().with("child")` (egui's
        // default fallback in `Ui::new_child`), which is identical
        // across containers, and any widget inside would trip
        // egui's "id reused" check_for_id_clash on every frame.
        let pane_id = self.pane_id;
        ui.push_id(pane_id, |ui| self.show_inner(ui, pods)).inner
    }

    fn show_inner(
        self,
        ui: &mut Ui,
        pods: impl IntoIterator<Item = crate::pod::Pod>,
    ) -> Vec<crate::pod::PodResponse> {
        // Padding INSIDE each pod (between the pod's painted Frame
        // and its widgets). Bigger than the now-halved
        // section_padding so most of the breathing room around pod
        // content lives in the pod chrome, not the container chrome.
        const POD_PAD_X: i8 = 8;
        const POD_PAD_Y: i8 = 6;
        let pods: Vec<crate::pod::Pod> = pods.into_iter().collect();
        let pods_total = pods.len();
        let pods_accent = self.accent;
        let mut out: Vec<crate::pod::PodResponse> = Vec::with_capacity(pods_total);
        self.show_with_body(ui, |body_ui| {
            for (i, pod) in pods.into_iter().enumerate() {
                // Capture metadata BEFORE the pod is consumed by `show`.
                let pod_id = pod.id();
                let pod_is_resizable = pod.is_resizable();
                let pod_widget_count = pod.widget_count();
                let separator_after = if i + 1 < pods_total {
                    pod.separator_style()
                } else {
                    // Last pod — never paint a separator after,
                    // regardless of its `separator_style()`.
                    crate::container::SeparatorStyle::None
                };
                let frame_resp = Frame::new()
                    .inner_margin(egui::Margin::symmetric(POD_PAD_X, POD_PAD_Y))
                    .show(body_ui, |inner_ui| {
                        out.push(pod.show(inner_ui));
                    });
                crate::debug::tag(
                    body_ui,
                    frame_resp.response.rect,
                    format!("Pod[{:?}]", pod_id),
                );
                if separator_after != crate::container::SeparatorStyle::None {
                    let sep_rect_before = body_ui.cursor();
                    let resizable_handle = pod_is_resizable
                        && separator_after == crate::container::SeparatorStyle::LineDots;
                    if resizable_handle {
                        // Interactive variant: drag delta updates
                        // the pod's persisted per-widget height,
                        // divided by widget_count so the cursor
                        // tracks the pod's bottom edge (each
                        // widget grows by delta/N).
                        let resp = crate::container::paint_separator_resize(
                            body_ui,
                            separator_after,
                            // Inter-pod separators are always
                            // horizontal — the body forces a
                            // top_down layout, so pods stack
                            // vertically inside the container
                            // regardless of the parent pane's
                            // orientation.
                            crate::container::SeparatorOrient::Horizontal,
                            pod_id,
                            pods_accent,
                        );
                        if resp.dragged() && pod_widget_count > 0 {
                            let key = crate::pod::Pod::widget_height_key(pod_id);
                            let cur = body_ui
                                .ctx()
                                .data_mut(|d| d.get_persisted::<f32>(key))
                                .unwrap_or(crate::style::UNIT);
                            let delta_per_widget =
                                resp.drag_delta().y / pod_widget_count as f32;
                            let new = (cur + delta_per_widget)
                                .clamp(crate::pod::POD_MIN_WIDGET_H, crate::pod::POD_MAX_WIDGET_H);
                            body_ui.ctx().data_mut(|d| d.insert_persisted(key, new));
                        }
                    } else {
                        crate::container::paint_separator(
                            body_ui,
                            separator_after,
                            crate::container::SeparatorOrient::Horizontal,
                        );
                    }
                    let sep_rect_after = body_ui.cursor();
                    // Tag the separator strip for the F10 inspector
                    // so the user can see which boundary owns
                    // which style. Use the cursor delta since the
                    // separator paint functions don't return rects.
                    let strip_rect = egui::Rect::from_min_max(
                        sep_rect_before.min,
                        egui::pos2(sep_rect_before.max.x, sep_rect_after.min.y),
                    );
                    crate::debug::tag(
                        body_ui,
                        strip_rect,
                        format!("separator[{:?}]", separator_after),
                    );
                }
            }
        });
        out
    }

    fn show_with_body(self, ui: &mut Ui, body: impl FnOnce(&mut Ui)) {
        // Register this container's MIN WIDTH with the parent pane
        // so the pane's resize handles can refuse to shrink the
        // pane below the union of its containers' bounds. Keyed
        // on the active pane id (`pane::active_pane_key`) which
        // `Pane2::show` writes at the top of every frame, then
        // clears the accumulator before running the body callback.
        // First-frame fallback: if no active pane is set yet,
        // register against the container's own pane_id so the
        // entry isn't lost.
        let parent_pane_id: Id = ui
            .ctx()
            .data(|d| d.get_temp(pane::active_pane_key()))
            .unwrap_or(self.pane_id);
        let min_w = self.min_width.unwrap_or(CONTAINER_DEFAULT_MIN_WIDTH);
        ui.ctx().data_mut(|d| {
            let key = parent_pane_id.with("frost_pane_container_min_widths");
            let mut acc: Vec<f32> = d.get_temp(key).unwrap_or_default();
            acc.push(min_w);
            d.insert_temp(key, acc);
        });

        let title_side = self.anchor.title_side();
        let horizontal_strip = title_side.is_horizontal_strip();

        let theme_now = style::theme();

        // Register this container's MINIMUM flow-axis chrome with the
        // parent pane so horizontal-strip pane resize handles can
        // refuse to shrink past where containers would start
        // overlapping. egui's `available_rect_before_wrap` collapses
        // to zero-height once the layout cursor overshoots
        // `max_rect`; subsequent Frame allocations still draw
        // their content + margins, which extends below the cursor by
        // the bottom-side chrome (inner_margin + stroke +
        // outer_margin) and visually overlaps the previous container.
        // Floor = sum of (TITLE_ZONE_THICKNESS + title-body gap +
        //                 inner_margin both sides + stroke ×2 +
        //                 outer_margin both sides) for every container,
        // computed at the current `openness` so the floor naturally
        // shrinks to title-only when all containers are folded.
        let openness_for_min = pane::body_openness(ui.ctx(), self.pane_id);
        let pad_for_min = style::section_padding();
        let pad_flow_for_min = if horizontal_strip {
            (pad_for_min.top as f32) + (pad_for_min.bottom as f32)
        } else {
            (pad_for_min.left as f32) + (pad_for_min.right as f32)
        };
        let outer_flow_for_min = (theme_now.section_outer_margin_flow_title as f32)
            + (theme_now.section_outer_margin_flow_body as f32);
        let stroke_for_min = if style::section_show_frame() {
            theme_now.border_width * 2.0
        } else {
            0.0
        };
        let min_flow = TITLE_ZONE_THICKNESS
            + TITLE_BODY_GAP_HALF * 2.0 * openness_for_min
            + pad_flow_for_min
            + outer_flow_for_min
            + stroke_for_min;
        ui.ctx().data_mut(|d| {
            let key = parent_pane_id.with("frost_pane_container_min_flows");
            let mut acc: Vec<f32> = d.get_temp(key).unwrap_or_default();
            acc.push(min_flow);
            d.insert_temp(key, acc);
        });
        let pad = style::section_padding();
        let pad_w = (pad.left as f32) + (pad.right as f32);
        let pad_h = (pad.top as f32) + (pad.bottom as f32);
        // Frame chrome that sits OUTSIDE the span_inner slot:
        //   `pad_*` — Frame's `inner_margin` (theme `section_padding`).
        //   `outer_*` — Frame's `outer_margin`, per-axis from theme.
        //   `stroke_*` — border drawn on either side (PRO=1, GAME=0).
        // Subtract them so the Frame's resulting outer rect fits
        // inside `outer_avail` exactly — no 2-px overflow into the
        // pane's stroke or shadow when the theme has a visible border.
        // Total outer-margin on each axis. Cross-axis is symmetric
        // (`2 × cross`); flow-axis sums the per-side title-facing
        // and body-facing margins.
        let flow_outer_total = (theme_now.section_outer_margin_flow_title as f32)
            + (theme_now.section_outer_margin_flow_body as f32);
        let span_outer_total = (theme_now.section_outer_margin_span as f32) * 2.0;
        // X axis = cross when horizontal-strip, main when vertical-strip.
        let outer_w = if horizontal_strip {
            span_outer_total
        } else {
            flow_outer_total
        };
        // Y axis = main when horizontal-strip, cross when vertical-strip.
        let outer_h = if horizontal_strip {
            flow_outer_total
        } else {
            span_outer_total
        };
        let stroke_w = if style::section_show_frame() {
            theme_now.border_width * 2.0
        } else {
            0.0
        };

        // Cross axis = the dim the title strip spans. Track the
        // PARENT's available cross extent so the container grows
        // along with the (user-resized) pane instead of staying
        // capped at `CONTAINER_DEFAULT_*`. Subtract the Frame
        // chrome on each side so the inner content slot fits
        // inside the painted Frame.
        let outer_avail = ui.available_size();
        let span_inner = if horizontal_strip {
            (outer_avail.x - pad_w - outer_w - stroke_w).max(0.0)
        } else {
            (outer_avail.y - pad_h - outer_h - stroke_w).max(0.0)
        };

        let title_size = if horizontal_strip {
            vec2(span_inner, TITLE_ZONE_THICKNESS)
        } else {
            vec2(TITLE_ZONE_THICKNESS, span_inner)
        };

        // Shared body recipe — applies the span-axis clamp so child
        // widgets see a stable `ui.available_*` regardless of the
        // surrounding layout's measurement passes.
        let body_cfg = Body::new(horizontal_strip, span_inner);

        let title_text = self.title.clone();
        let anchor = self.anchor;
        let accent = self.accent;
        let icon = self.icon;

        let banner_filled = style::theme().title_strip_filled;

        // Open state + animation are stored on the parent pane's
        // id (NOT `ui.id()`) so `Pane2::show` and `Normal::show`
        // both compute the SAME `openness` from the same
        // `animate_bool` call within a frame. That synchronises the
        // pane's outer size and the container's body slot — no
        // anchor lag, no per-frame edge drift.
        let pane_id = self.pane_id;
        let open: bool = ui.ctx().data_mut(|d| {
            *d.get_persisted_mut_or_insert_with(pane_id.with("body_open"), || true)
        });
        let openness = pane::body_openness(ui.ctx(), pane_id);
        // Body's full flow-axis size when fully open. Used as the
        // child UI's `max_rect` extent so widgets ALWAYS render at
        // their natural size; only the clip mask animates.
        //
        // Resolution order (first non-`None` wins):
        //   1. `Normal::body_flow` builder override — explicit caller
        //      control, e.g. for tests or fixed-height containers.
        //   2. `crate::container::container_flow(self.pane_id)` —
        //      the per-container persisted flow size, written by
        //      the inter-container drag-resize handle.
        //   3. The `CONTAINER_DEFAULT_*` fallback computed from
        //      title strip + chrome.
        let full_body_flow = self.body_flow.unwrap_or_else(|| {
            // Persisted per-container flow takes precedence over
            // the static fallback. Returns
            // `CONTAINER_DEFAULT_FLOW` clamped on first read.
            crate::container::container_flow(ui.ctx(), pane_id)
        });
        // Publish this container's cid to the parent pane so
        // `Pane2::show` can sum each container's LIVE persisted
        // flow when it auto-sizes (`PaneResize::flow` off).
        pane::publish_container_cid(ui.ctx(), parent_pane_id, pane_id);
        // Body slot size LERPS with `openness` to match Pane2's
        // lerp (both compute openness from the SAME `animate_bool`
        // call, so they animate in lockstep — no anchor drift).
        let body_visible = openness > 0.0;
        let total_gap = TITLE_BODY_GAP_HALF * 2.0 * openness;
        let visible_body_flow = openness * full_body_flow;

        // ── Per-section staggered fade-in (verbatim port of
        //    `frostcore::PaneBuilder::section_with`) ──
        //
        // Look up the parent Pane2's id via the global "active
        // pane" pointer (Normal's own `pane_id` field is the
        // container's body-open id, NOT Pane2's id, so we can't
        // use it for the stagger lookup). Pane2::show populates
        // `frost_pane_open_elapsed` and resets
        // `frost_pane_section_idx` to 0 on every frame; we
        // post-increment to claim THIS container's index.
        const STAGGER_BASE: f32 = 0.18;
        const FADE_BASE: f32 = 0.45;
        let stagger_opacity: f32 = {
            let theme_now = style::theme();
            let scale = theme_now.pane_fade_scale.max(0.01);
            let stagger = STAGGER_BASE * scale;
            let fade = FADE_BASE * scale;
            ui.ctx().data_mut(|d| {
                let pane2_id: Id = d
                    .get_temp::<Id>(pane::active_pane_key())
                    .unwrap_or(pane_id);
                let elapsed: f32 = d
                    .get_temp(pane2_id.with("frost_pane_open_elapsed"))
                    .unwrap_or(99.0);
                let idx_key = pane2_id.with("frost_pane_section_idx");
                let idx: u32 = d.get_temp(idx_key).unwrap_or(0);
                d.insert_temp(idx_key, idx + 1);
                let start = (idx as f32) * stagger;
                let raw = ((elapsed - start) / fade).clamp(0.0, 1.0);
                raw * raw * (3.0 - 2.0 * raw) // smoothstep
            })
        };
        let prev_opacity = ui.opacity();
        if stagger_opacity < 1.0 {
            ui.multiply_opacity(stagger_opacity);
        }

        // Drag-lift: if this container IS the one being dragged,
        // bail out entirely — no layout slot, no paint. The other
        // containers below collapse upward to fill the gap, and
        // the floating preview painted by `Pane2`'s finalize
        // shows what's being held. Matches frostcore's
        // `section_with` early-return.
        let active = pane::active_drag(ui.ctx());
        let is_dragging_self = active
            .and_then(|(_, s)| s.item)
            .map(|id| id == pane_id)
            .unwrap_or(false);
        if is_dragging_self {
            ui.set_opacity(prev_opacity);
            return;
        }

        // Inline ghost gap: if the cursor's target slot equals
        // THIS container's position in the non-dragged sequence,
        // allocate + paint a ghost rect of the dragged size
        // BEFORE rendering. Pushes this container (and the rest)
        // along the stack axis so the drop slot is visible.
        if let Some((parent_pane_id, drag_state)) = active {
            if let (Some(dragged_id), Some(cursor)) =
                (drag_state.item, drag_state.cursor)
            {
                let snap = pane::snapshot(ui.ctx(), parent_pane_id);
                let horizontal_stack = !title_side.is_horizontal_strip();
                let cursor_axis =
                    if horizontal_stack { cursor.x } else { cursor.y };
                let target_idx = pane::compute_target(
                    &snap,
                    dragged_id,
                    cursor_axis,
                    horizontal_stack,
                );
                let cur_idx =
                    pane::current_cache(ui.ctx(), parent_pane_id).len();
                if cur_idx == target_idx {
                    if let Some(size) = pane::dragged_size(&snap, dragged_id) {
                        pane::paint_ghost_gap_inline(
                            ui,
                            size,
                            accent,
                            horizontal_stack,
                        );
                    }
                }
            }
        }

        let frame = self.theme_frame();
        let frame_response = frame.show(ui, |ui| {
            // GAME-style banner placeholder, set AFTER the layout is
            // measured so we know where the title strip ended up.
            let banner_idx = if banner_filled {
                Some(ui.painter().add(egui::Shape::Noop))
            } else {
                None
            };

            // ── Manual layout (no flex) ──
            // egui's `CollapsingState` recipe: title is allocated at
            // its exact size, the body is rendered at FULL size into
            // a clipped child UI, and only the VISIBLE portion is
            // allocated to the parent ui (`force_set_min_rect` /
            // `allocate_rect`). So:
            //   • body's content widgets keep their natural
            //     `available_*` width — no per-frame text_input
            //     shrinking,
            //   • the parent's min_rect lerps smoothly with
            //     `openness`, which animates the container chrome
            //     and the parent pane's `fixed_pos` together,
            //   • no flex item state changes, no `request_discard`
            //     storm, no PERF WARNING overlay.
            // Inherit the parent's layout direction directly into
            // the Frame's content_ui — DON'T create a child with a
            // forced `top_down`. Frame computes its outer rect from
            // `content_ui.min_rect()`, so the inner allocations
            // determine where the Frame lands inside the pane body.
            // Forcing `top_down` made the container always appear
            // at the TOP of available area (since cursor starts at
            // max_rect.min for top_down), which in a `bottom_up`
            // pane parent left every container at the FAR edge from
            // the rail instead of stacking against the title strip.
            // Inheriting the parent layout makes:
            //   • TopDown    → first allocation at top  (TopRail).
            //   • BottomUp   → first allocation at bottom (BottomRail).
            //   • LeftToRight→ first allocation at left  (LeftRail).
            //   • RightToLeft→ first allocation at right (RightRail).
            // Always render TITLE first then BODY: layout direction
            // does the visual placement work, no `if title_at_end`
            // swap needed at this level.
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

            let render_title = |ui: &mut Ui| {
                // Title strip is also the drag handle: `click_and_drag`
                // sense reports both — `clicked()` toggles the body
                // open state, `drag_started()` lifts this container
                // for reorder via the parent pane's drag machine.
                let (rect, resp) =
                    ui.allocate_exact_size(title_size, Sense::click_and_drag());
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    pane::toggle_body(ui.ctx(), pane_id);
                }
                if resp.drag_started() {
                    if let Some(active_pane_id) = ui
                        .ctx()
                        .data(|d| d.get_temp::<Id>(pane::active_pane_key()))
                    {
                        pane::set_drag(
                            ui.ctx(),
                            active_pane_id,
                            pane::DragState {
                                item: Some(pane_id),
                                cursor: ui.ctx().pointer_interact_pos(),
                            },
                        );
                    }
                }
                paint_title(ui, rect, &title_text, anchor, accent, open, openness, icon, pane_id);
            };

            let render_body = |ui: &mut Ui, body: Box<dyn FnOnce(&mut Ui)>| {
                if !body_visible || full_body_flow <= 0.0 {
                    return;
                }
                let visible_size = if horizontal_strip {
                    vec2(span_inner, visible_body_flow)
                } else {
                    vec2(visible_body_flow, span_inner)
                };
                let full_size = if horizontal_strip {
                    vec2(span_inner, full_body_flow)
                } else {
                    vec2(full_body_flow, span_inner)
                };
                // `allocate_space` respects the parent's layout
                // direction, so `visible_rect` lands at the correct
                // edge (bottom for BottomUp, right for RightToLeft).
                let (_, visible_rect) = ui.allocate_space(visible_size);
                // `full_rect` extends the visible slot to the full
                // body size in the layout direction (so body
                // widgets render at natural size and only the clip
                // mask animates). For reversed layouts we anchor
                // `full_rect`'s FAR edge to `visible_rect`'s far
                // edge — the body grows AWAY from the title strip
                // direction.
                let full_rect = match ui.layout().main_dir() {
                    egui::Direction::BottomUp => Rect::from_min_size(
                        egui::pos2(
                            visible_rect.min.x,
                            visible_rect.max.y - full_size.y,
                        ),
                        full_size,
                    ),
                    egui::Direction::RightToLeft => Rect::from_min_size(
                        egui::pos2(
                            visible_rect.max.x - full_size.x,
                            visible_rect.min.y,
                        ),
                        full_size,
                    ),
                    _ => Rect::from_min_size(visible_rect.min, full_size),
                };
                // Body's child layout matches parent direction so
                // body widgets anchor against the title strip side
                // (BottomRail body → widgets stack from bottom up).
                let body_layout = match ui.layout().main_dir() {
                    egui::Direction::TopDown    => Layout::top_down(Align::Min),
                    egui::Direction::BottomUp   => Layout::bottom_up(Align::Min),
                    egui::Direction::LeftToRight=> Layout::left_to_right(Align::Min),
                    egui::Direction::RightToLeft=> Layout::right_to_left(Align::Min),
                };
                let mut child = ui.new_child(
                    UiBuilder::new().max_rect(full_rect).layout(body_layout),
                );
                let parent_clip = ui.clip_rect();
                child.set_clip_rect(parent_clip.intersect(visible_rect));
                // Inner top-pad on the title-facing edge of the
                // body (theme-driven). Allocated FIRST in the body
                // layout so the cursor advances past it before the
                // user's body callback runs — pushes the first
                // widget away from the title strip without changing
                // the title's own thickness or the inter-container
                // gap. PRO = 0 (no-op); GAME ≈ 8.
                let body_top_pad = style::theme().section_body_inner_top_pad;
                if body_top_pad > 0.0 {
                    child.add_space(body_top_pad);
                }
                body_cfg.paint(&mut child, body);
            };

            // ALWAYS title FIRST, body SECOND. Layout direction
            // (inherited from pane parent) handles which edge the
            // title lands at.
            render_title(ui);
            if total_gap > 0.0 {
                ui.add_space(total_gap);
            }
            let body_box: Box<dyn FnOnce(&mut Ui)> = Box::new(body);
            render_body(ui, body_box);

            // After flex is laid out, paint the GAME banner into
            // the deferred shape index. Banner extends from the
            // frame's painted edge (= ui.min_rect() expanded by
            // section_padding) through the title strip and into
            // half the flex gap. Equivalent to `foldable.rs`'s
            // banner trick — the painted accent zone covers the
            // title slot AND the inner_margin around it.
            if let Some(idx) = banner_idx {
                let used = ui.min_rect();
                let pad = style::section_padding();
                let painted_l = used.left() - pad.left as f32;
                let painted_r = used.right() + pad.right as f32;
                let painted_t = used.top() - pad.top as f32;
                let painted_b = used.bottom() + pad.bottom as f32;
                // When collapsed, banner covers the entire painted
                // frame (no body, no gap to extend into). Otherwise
                // the banner stops at the gap midpoint.
                let banner = if !open {
                    egui::Rect::from_min_max(
                        egui::pos2(painted_l, painted_t),
                        egui::pos2(painted_r, painted_b),
                    )
                } else {
                    match title_side {
                        TitleSide::Top => egui::Rect::from_min_max(
                            egui::pos2(painted_l, painted_t),
                            egui::pos2(
                                painted_r,
                                used.top() + TITLE_ZONE_THICKNESS + TITLE_BODY_GAP_HALF,
                            ),
                        ),
                        TitleSide::Bottom => egui::Rect::from_min_max(
                            egui::pos2(
                                painted_l,
                                used.bottom() - TITLE_ZONE_THICKNESS - TITLE_BODY_GAP_HALF,
                            ),
                            egui::pos2(painted_r, painted_b),
                        ),
                        TitleSide::Left => egui::Rect::from_min_max(
                            egui::pos2(painted_l, painted_t),
                            egui::pos2(
                                used.left() + TITLE_ZONE_THICKNESS + TITLE_BODY_GAP_HALF,
                                painted_b,
                            ),
                        ),
                        TitleSide::Right => egui::Rect::from_min_max(
                            egui::pos2(
                                used.right() - TITLE_ZONE_THICKNESS - TITLE_BODY_GAP_HALF,
                                painted_t,
                            ),
                            egui::pos2(painted_r, painted_b),
                        ),
                    }
                };
                let p = ui.painter().clone().with_clip_rect(banner.expand(2.0));
                p.set(
                    idx,
                    egui::Shape::rect_filled(banner, egui::CornerRadius::ZERO, accent),
                );
            }

            // Corner ticks (GAME): L-shaped marks at each corner of
            // the container's outer rect, with a slow breathing
            // pulse. PRO has `section_corner_ticks = 0` so this is
            // a no-op there.
            let used_outer = {
                let pad = style::section_padding();
                let r = ui.min_rect();
                egui::Rect::from_min_max(
                    egui::pos2(r.left() - pad.left as f32, r.top() - pad.top as f32),
                    egui::pos2(r.right() + pad.right as f32, r.bottom() + pad.bottom as f32),
                )
            };
            paint_corner_ticks(ui, used_outer, accent, title_side, openness, pane_id);
        });
        // Restore the parent ui's opacity so subsequent containers
        // in the same body callback start from a clean baseline.
        ui.set_opacity(prev_opacity);

        // Publish the rendered Frame's outer rect to the parent
        // pane's per-frame cache. `Pane2`'s finalize builds next
        // frame's snapshot from this (with the dragged
        // container's prev rect carried forward).
        if let Some((active_pane_id, _)) = active {
            pane::push_rect(
                ui.ctx(),
                active_pane_id,
                pane_id,
                frame_response.response.rect,
            );
        }
        // Custom debug inspector — outline the container's full
        // painted Frame rect with a `Normal[<title>]` label.
        crate::debug::tag(
            ui,
            frame_response.response.rect,
            format!("Normal[{}]", title_text),
        );
    }

    /// Same recipe as `frostcore::widgets::foldable::section_tracked`'s
    /// outer frame: glass-card fill, accent-tinted border, theme
    /// `radius_md` corners. When the active theme has
    /// `section_show_frame = false` (GAME) we drop the visuals and
    /// keep just the inner padding so body content sits flush.
    /// `outer_margin` is per-side from the theme:
    ///   • flow-axis title-FACING side — sets the gap between the
    ///     pane title strip and the FIRST container.
    ///   • flow-axis body-FACING side — combines with the next
    ///     container's title-side margin to produce the
    ///     inter-container gap.
    ///   • span-axis sides — breathing space against the pane's
    ///     left/right (or top/bottom for vertical-strip) chrome.
    fn theme_frame(&self) -> Frame {
        let theme = style::theme();
        let title_side = self.anchor.title_side();
        let main_title = theme.section_outer_margin_flow_title;
        let main_body = theme.section_outer_margin_flow_body;
        let cross = theme.section_outer_margin_span;
        // Each title side puts the title-facing margin on a
        // different edge of the container's outer rect; the
        // body-facing margin lives on the OPPOSITE edge. Cross-axis
        // (the two sides parallel to the title strip) always uses
        // `cross`.
        let outer = match title_side {
            TitleSide::Top => egui::Margin {
                left: cross, right: cross,
                top: main_title, bottom: main_body,
            },
            TitleSide::Bottom => egui::Margin {
                left: cross, right: cross,
                top: main_body, bottom: main_title,
            },
            TitleSide::Left => egui::Margin {
                top: cross, bottom: cross,
                left: main_title, right: main_body,
            },
            TitleSide::Right => egui::Margin {
                top: cross, bottom: cross,
                left: main_body, right: main_title,
            },
        };
        if style::section_show_frame() {
            Frame::new()
                .fill(style::glass_fill(
                    style::section_fill(self.accent),
                    self.accent,
                    style::glass_alpha_card(),
                ))
                .corner_radius(CornerRadius::same(theme.radius_md))
                .stroke(Stroke::new(theme.border_width, style::widget_border(self.accent)))
                .inner_margin(style::section_padding())
                .outer_margin(outer)
        } else {
            Frame::new()
                .inner_margin(style::section_padding())
                .outer_margin(outer)
        }
    }
}

/// Paint the title strip into `rect`. Theme-aware:
/// * Title size, letter-spacing, font family, brackets, chevron all
///   from `theme()`.
/// * UPPERCASE always.
/// * `[ TITLE ]` brackets when `theme.section_title_brackets` —
///   layout space is reserved even when invisible so the title text
///   doesn't shift between collapsed / open.
/// * Chevron prefix when `theme.show_section_chevron` (PRO).
/// * Hairline divider on the body-facing edge in PRO; banner cover
///   in GAME (painted by caller).
fn paint_title(
    ui: &mut Ui,
    rect: egui::Rect,
    title: &str,
    anchor: PaneAnchor,
    accent: Color32,
    open: bool,
    openness: f32,
    icon: Option<Icon<'_>>,
    pane_id: Id,
) {
    let theme = style::theme();
    let title_side = anchor.title_side();
    let filled = theme.title_strip_filled;
    let title_col = if filled {
        style::contrast_text_for(accent)
    } else {
        style::section_title_color(accent)
    };

    let title_painter = ui.painter_at(rect);
    let painter = ui.painter();

    let title_font = FontId::new(theme.section_title_size, style::title_font_family());
    let bracket_visible = theme.section_title_brackets && !open;
    let any_brackets = theme.section_title_brackets;
    let title_uc = title.to_uppercase();
    // Inline icon dispatch:
    //   PRO (`section_icon_at_end = false`): icon glyph is prepended
    //     to the title `LayoutJob` so it tracks the same scramble /
    //     glitch / rotation pipeline as the title.
    //   GAME (`section_icon_at_end = true`): icon floats at the
    //     strip's far end after the title text is painted, with a
    //     smoothstep-eased size lerp keyed off `openness` (small when
    //     folded → large overflowing the strip when open).
    // SVG icons can't be inlined into a `LayoutJob`, so they fall
    // through to the floating-paint path even in PRO.
    let inline_icon = !theme.section_icon_at_end;
    // GAME theme: scramble-decode the title each time the container
    // reappears (matching the old `section_tracked` recipe), AND
    // every time the user folds / unfolds the container. The
    // scramble id is salted with two values:
    //   • `appearance_session(...)` — bumps when the title widget
    //     was missing for a frame and reappeared (e.g. pane closed
    //     and reopened).
    //   • `pane::fold_version(pane_id)` — bumps inside `toggle_body`
    //     on every fold / unfold click.
    // Either one changing produces a fresh `scramble_id`, which
    // makes `scramble_text` see no stored prev for this id and
    // restart the decode cycle from t = 0.
    let displayed = if theme.scramble_titles {
        let session_id = ui.id().with(("frost_normal_title_session", title));
        let session = style::appearance_session(ui.ctx(), session_id);
        let fold_ver = pane::fold_version(ui.ctx(), pane_id);
        let scramble_id = session_id.with(session).with(fold_ver);
        let active = ui.opacity() >= 0.95;
        let scrambled = style::scramble_text(ui.ctx(), scramble_id, &title_uc, active);
        // Post-stabilisation glitch: every ~5 s a random letter
        // momentarily becomes a scramble symbol and reverts.
        style::glitch_text(ui.ctx(), session_id.with("glitch"), &scrambled)
    } else {
        title_uc
    };

    let default_format = egui::TextFormat {
        font_id: title_font.clone(),
        color: title_col,
        extra_letter_spacing: theme.section_title_letter_spacing,
        ..Default::default()
    };
    let bracket_format = egui::TextFormat {
        color: if bracket_visible {
            title_col
        } else {
            Color32::TRANSPARENT
        },
        ..default_format.clone()
    };

    let mut job = egui::text::LayoutJob::default();
    // Optional theme prefix (PRO only — drops when bracket framing
    // is on so `▸ [ … ]` doesn't read as cluttered).
    if let (Some(prefix), false) = (theme.section_title_prefix, any_brackets) {
        job.append(prefix, 0.0, default_format.clone());
        job.append(" ", 0.0, default_format.clone());
    }
    if any_brackets {
        job.append("[ ", 0.0, bracket_format.clone());
    }
    // Resolve the inline-icon glyph + family ONCE, then decide
    // whether it appears before or after the title text. The chevron
    // paints separately at the strip's reading-start, so the icon
    // wants to sit BETWEEN the chevron and the title text:
    //   • horizontal non-reversed (TM, BM, BS-as-Left? actually just
    //     anchors with `title_reversed = false`): chevron on LEFT,
    //     LayoutJob renders LTR, so `icon, title` is correct.
    //   • horizontal reversed (RS = RightRail Start → Top, RE =
    //     RightRail End → Bottom): chevron on RIGHT, LayoutJob still
    //     renders LTR, so `title, icon` puts icon adjacent to the
    //     chevron. Without this swap the icon ended up on the FAR
    //     left of the strip — opposite the chevron — and the user's
    //     chevron→icon→title reading order broke.
    //   • vertical strips: TextShape rotation places LayoutJob's
    //     first character closest to the chevron regardless of
    //     direction (CW for top_to_bottom, CCW otherwise), so
    //     `icon, title` is always correct.
    let icon_after_title = title_side.is_horizontal_strip() && anchor.title_reversed();
    let inline_glyph: Option<(String, egui::FontFamily)> = if inline_icon {
        match icon {
            Some(Icon::Name(name)) => crate::icons::icon(name)
                .map(|(g, family)| (g.to_string(), family)),
            _ => None,
        }
    } else {
        None
    };
    // Inline-icon glyph is rendered 20 % larger than the title text
    // — Fluent glyphs are designed at a square optical size and
    // visually feel small next to a same-pt UPPERCASE caption, so a
    // small bump pulls the icon weight up to match the title.
    let inline_icon_size = theme.section_title_size * 1.2;
    // Px gap between the icon and the title — applied via egui's
    // `leading_space` on the next segment, which produces a clean
    // horizontal gap independent of the chosen separator character.
    const ICON_TITLE_GAP: f32 = 6.0;
    let icon_format_for = |family: egui::FontFamily| egui::TextFormat {
        font_id: FontId::new(inline_icon_size, family),
        color: title_col,
        ..Default::default()
    };
    if !icon_after_title {
        if let Some((glyph, family)) = &inline_glyph {
            job.append(glyph, 0.0, icon_format_for(family.clone()));
        }
    }
    let title_lead = if !icon_after_title && inline_glyph.is_some() {
        ICON_TITLE_GAP
    } else {
        0.0
    };
    job.append(&displayed, title_lead, default_format.clone());
    if icon_after_title {
        if let Some((glyph, family)) = &inline_glyph {
            job.append(glyph, ICON_TITLE_GAP, icon_format_for(family.clone()));
        }
    }
    if any_brackets {
        job.append(" ]", 0.0, bracket_format);
    }
    let galley = title_painter.layout_job(job);
    let g_size = galley.size();

    match title_side {
        TitleSide::Top | TitleSide::Bottom => {
            // Optional chevron painted ahead of the title text.
            let mut text_inset = TITLE_INSET;
            if theme.show_section_chevron {
                const CHEVRON_W: f32 = 14.0;
                let chevron_x = if anchor.title_reversed() {
                    rect.right() - TITLE_INSET - CHEVRON_W * 0.5
                } else {
                    rect.left() + TITLE_INSET + CHEVRON_W * 0.5
                };
                paint_chevron_h(
                    &title_painter,
                    egui::pos2(chevron_x, rect.center().y),
                    title_side,
                    if open { 1.0 } else { 0.0 },
                    title_col,
                );
                text_inset = TITLE_INSET + CHEVRON_W + 2.0;
            }

            let text_pos = if anchor.title_reversed() {
                pos2(rect.right() - text_inset - g_size.x, rect.center().y - g_size.y * 0.5)
            } else {
                pos2(rect.left() + text_inset, rect.center().y - g_size.y * 0.5)
            };
            title_painter.galley(text_pos, galley, title_col);

            // Body-facing divider — PRO only, when expanded.
            if !filled && open {
                let y = match title_side {
                    TitleSide::Top => (rect.bottom() + TITLE_BODY_GAP_HALF).round() + 0.5,
                    _ => (rect.top() - TITLE_BODY_GAP_HALF).round() - 0.5,
                };
                let x_range = (rect.left() + DIVIDER_INSET)..=(rect.right() - DIVIDER_INSET);
                painter.hline(x_range, y, Stroke::new(1.0, theme.border_subtle));
            }
        }
        TitleSide::Left | TitleSide::Right => {
            let cx = rect.center().x;
            let on_right_side = title_side == TitleSide::Right;
            let top_to_bottom = on_right_side ^ anchor.title_reversed();

            // Optional chevron at the reading-start of the title.
            let mut text_inset = TITLE_INSET;
            if theme.show_section_chevron {
                const CHEVRON_W: f32 = 14.0;
                let chevron_y = if top_to_bottom {
                    rect.top() + TITLE_INSET + CHEVRON_W * 0.5
                } else {
                    rect.bottom() - TITLE_INSET - CHEVRON_W * 0.5
                };
                paint_chevron_h(
                    &title_painter,
                    egui::pos2(cx, chevron_y),
                    title_side,
                    if open { 1.0 } else { 0.0 },
                    title_col,
                );
                text_inset = TITLE_INSET + CHEVRON_W + 2.0;
            }

            let (text_pos, angle) = if top_to_bottom {
                (
                    pos2(
                        (cx + g_size.y * 0.5).round(),
                        (rect.min.y + text_inset).round(),
                    ),
                    std::f32::consts::FRAC_PI_2,
                )
            } else {
                (
                    pos2(
                        (cx - g_size.y * 0.5).round(),
                        (rect.max.y - text_inset).round(),
                    ),
                    -std::f32::consts::FRAC_PI_2,
                )
            };
            let mut shape = TextShape::new(text_pos, galley, title_col);
            shape.angle = angle;
            title_painter.add(shape);

            if !filled && open {
                let x = match title_side {
                    TitleSide::Left => (rect.right() + TITLE_BODY_GAP_HALF).round() + 0.5,
                    _ => (rect.left() - TITLE_BODY_GAP_HALF).round() - 0.5,
                };
                let y_range = (rect.top() + DIVIDER_INSET)..=(rect.bottom() - DIVIDER_INSET);
                painter.vline(x, y_range, Stroke::new(1.0, theme.border_subtle));
            }
        }
    }

    // Floating icon (GAME mode) — paints AFTER the title text so it
    // rides on top of the banner. Same recipe as
    // `frostcore::widgets::foldable::section_tracked`'s right-edge
    // icon: small when folded so it tucks inside the collapsed
    // banner, big when open so it overflows the strip and reads as a
    // floating ornament. The growth is `smoothstep`-eased so it pops
    // through `cubic-bezier(0.42, 0, 0.58, 1)` rather than linear.
    if !inline_icon {
        if let Some(icon_src) = icon {
            paint_floating_icon(ui, rect, anchor, title_col, openness, icon_src);
        }
    }
}

/// Paint a "floating" icon on the title strip — small when folded,
/// big when open. Mirrors `frostcore::widgets::foldable`'s
/// right-edge icon: the icon overflows the strip's body-facing edge
/// when fully open, framed by clipping +8 px around the painted
/// rect. Vertical strips paint the icon centred (no rotation —
/// Fluent glyphs read fine in either orientation, and rotating
/// would require a `TextShape` round-trip just for a decoration).
///
/// Painted on `Order::Foreground` so the icon sits ABOVE the ribbon
/// buttons (`Order::Middle`) and the pane chrome (`Order::Background`).
fn paint_floating_icon(
    ui: &mut Ui,
    strip_rect: egui::Rect,
    anchor: PaneAnchor,
    title_col: Color32,
    openness: f32,
    icon_src: Icon<'_>,
) {
    let theme = style::theme();
    let base_size = theme.section_icon_size.max(0.0);
    if base_size <= 0.0 {
        return;
    }
    // Constants pulled from `frostcore::widgets::foldable`'s tuned
    // values — keep them in sync if either gets re-tuned.
    let folded_size = base_size * 0.85;
    let unfolded_size = base_size * 2.9106;
    const UNFOLDED_OFFSET: f32 = 29.294;
    let folded_offset = folded_size * 0.5;
    let t = smoothstep(openness);
    let size = egui::lerp(folded_size..=unfolded_size, t);
    let offset = egui::lerp(folded_offset..=UNFOLDED_OFFSET, t);

    let title_side = anchor.title_side();
    let reversed = anchor.title_reversed();
    let (icon_pos, icon_align, icon_rect) = match title_side {
        TitleSide::Top | TitleSide::Bottom => {
            // Icon overflows UP for top-side / DOWN for bottom-side
            // so it grows AWAY from the body. `cy` anchors the
            // icon's top (or bottom for bottom-anchored) at
            // `center.y ∓ offset`.
            let cy = if title_side == TitleSide::Top {
                (strip_rect.center().y - offset).round()
            } else {
                // Bottom strip: a small openness-scaled downward
                // bias so the icon doesn't read as "pushed up" into
                // the body above. `BIAS` is the extra pixels of
                // downward shift at full open; folded keeps the
                // existing centred position.
                const BIAS: f32 = 6.0;
                (strip_rect.center().y + offset + BIAS * t).round()
            };
            let on_far_end_left = reversed;
            if on_far_end_left {
                let pos = pos2((strip_rect.min.x + 6.0).round(), cy);
                let rect = if title_side == TitleSide::Top {
                    Rect::from_min_size(pos, vec2(size, size))
                } else {
                    Rect::from_min_size(pos2(pos.x, pos.y - size), vec2(size, size))
                };
                let align = if title_side == TitleSide::Top {
                    egui::Align2::LEFT_TOP
                } else {
                    egui::Align2::LEFT_BOTTOM
                };
                (pos, align, rect)
            } else {
                let pos = pos2((strip_rect.max.x - 6.0).round(), cy);
                let rect = if title_side == TitleSide::Top {
                    Rect::from_min_size(pos2(pos.x - size, pos.y), vec2(size, size))
                } else {
                    Rect::from_min_size(pos2(pos.x - size, pos.y - size), vec2(size, size))
                };
                let align = if title_side == TitleSide::Top {
                    egui::Align2::RIGHT_TOP
                } else {
                    egui::Align2::RIGHT_BOTTOM
                };
                (pos, align, rect)
            }
        }
        TitleSide::Left | TitleSide::Right => {
            // Vertical strip: icon overflows AWAY from the body
            // (LEFT for Left-anchored, RIGHT for Right-anchored).
            let cx = if title_side == TitleSide::Left {
                (strip_rect.center().x - offset).round()
            } else {
                (strip_rect.center().x + offset).round()
            };
            let on_right_side = title_side == TitleSide::Right;
            let top_to_bottom = on_right_side ^ reversed;
            if top_to_bottom {
                let pos = pos2(cx, (strip_rect.max.y - 6.0).round());
                let rect = if title_side == TitleSide::Left {
                    Rect::from_min_size(pos2(pos.x, pos.y - size), vec2(size, size))
                } else {
                    Rect::from_min_size(pos2(pos.x - size, pos.y - size), vec2(size, size))
                };
                let align = if title_side == TitleSide::Left {
                    egui::Align2::LEFT_BOTTOM
                } else {
                    egui::Align2::RIGHT_BOTTOM
                };
                (pos, align, rect)
            } else {
                let pos = pos2(cx, (strip_rect.min.y + 6.0).round());
                let rect = if title_side == TitleSide::Left {
                    Rect::from_min_size(pos, vec2(size, size))
                } else {
                    Rect::from_min_size(pos2(pos.x - size, pos.y), vec2(size, size))
                };
                let align = if title_side == TitleSide::Left {
                    egui::Align2::LEFT_TOP
                } else {
                    egui::Align2::RIGHT_TOP
                };
                (pos, align, rect)
            }
        }
    };

    let _ = icon_rect;
    // Render on `Order::Foreground` so the icon sits above ribbon
    // buttons (`Order::Middle`) and the pane chrome
    // (`Order::Background`). Foreground-layer painters do NOT
    // inherit the parent ui's opacity, so during the stagger fade
    // the icon would otherwise pop in at full alpha while the
    // container chrome was still fading. Mirror the parent's
    // opacity onto this layer's painter so the icon fades with
    // its container.
    let layer_id = egui::LayerId::new(
        egui::Order::Foreground,
        ui.id().with("frost_floating_icon_layer"),
    );
    let parent_opacity = ui.opacity();
    match icon_src {
        Icon::Name(name) => {
            let mut p = ui.ctx().layer_painter(layer_id);
            p.set_opacity(parent_opacity);
            crate::icons::paint_icon(&p, icon_pos, icon_align, name, size, title_col);
        }
        Icon::Svg(_) => {
            let mut child = ui.new_child(
                UiBuilder::new()
                    .layer_id(layer_id)
                    .max_rect(icon_rect)
                    .layout(Layout::default()),
            );
            child.set_opacity(parent_opacity);
            crate::icons::paint_section_icon(
                &mut child,
                icon_pos,
                icon_align,
                icon_src,
                size,
                title_col,
            );
        }
    }
}

/// Polynomial smoothstep, `t * t * (3 - 2t)`. Approximates
/// `cubic-bezier(0.42, 0, 0.58, 1)` for a gentle ease-in-ease-out —
/// matches the same helper in `frostcore::widgets::foldable`.
#[inline]
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// "Ease-out-elastic" — exponentially damped sine that overshoots
/// past 1.0 once before settling. `t = 0 → 0`, `t = 1 → 1` exactly
/// (both endpoints early-return). Tuned subtle: a fast decay
/// (`exp(-5.0 t)`) plus an `AMP = 0.45` scale on the deviation
/// keeps the overshoot small (~5 %) and the undershoot barely
/// perceptible — a hint of bounce, not a wobble.
#[inline]
fn ease_out_elastic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t == 0.0 {
        return 0.0;
    }
    if t == 1.0 {
        return 1.0;
    }
    const AMP: f32 = 0.45;
    let c = std::f32::consts::TAU / 3.0;
    -(AMP * (-5.0 * t).exp() * ((t * 3.5 - 0.75) * c).sin()) + 1.0
}

/// Paint a chevron at `center` rotated to match the title side and
/// `openness` 0..=1. Glyph reads `›` (closed) → `⌄` (open) for a
/// Top title; mirrored / rotated for the other three sides.
fn paint_chevron_h(
    painter: &egui::Painter,
    center: egui::Pos2,
    title_side: TitleSide,
    openness: f32,
    tint: Color32,
) {
    const GLYPH_W: f32 = 8.0;
    const GLYPH_H: f32 = 5.0;
    let hw = GLYPH_W * 0.5;
    let hh = GLYPH_H * 0.5;
    // Base shape `⌄`: arms at top corners, apex at bottom centre.
    let raw = [
        egui::vec2(-hw, -hh),
        egui::vec2(0.0, hh),
        egui::vec2(hw, -hh),
    ];
    use std::f32::consts::TAU;
    // Closed → open angle ranges per side:
    //   Top:    -90° → 0°   (›  → ⌄)
    //   Bottom: -90° → 180° (›  → ^)
    //   Left:    0°  → -90° (⌄  → ›)
    //   Right:   0°  →  90° (⌄  → ‹)
    let (closed, open) = match title_side {
        TitleSide::Top => (-TAU / 4.0, 0.0),
        TitleSide::Bottom => (-TAU / 4.0, TAU / 2.0),
        TitleSide::Left => (0.0, -TAU / 4.0),
        TitleSide::Right => (0.0, TAU / 4.0),
    };
    let rot = egui::emath::Rot2::from_angle(egui::lerp(closed..=open, openness));
    let pts: Vec<egui::Pos2> = raw
        .iter()
        .map(|v| {
            let r = rot * *v;
            egui::pos2(center.x + r.x, center.y + r.y)
        })
        .collect();
    painter.add(egui::Shape::line(pts, Stroke::new(1.6, tint)));
}

/// Paint L-shaped corner ticks around `outer_rect`. Gated on
/// `theme.section_corner_ticks > 0` (GAME enables, PRO disables —
/// PRO ships `0.0` so this whole function returns early there).
/// Title-side corners use the contrast colour (white-on-banner);
/// body-side corners use a breathing-accent so they pulse against
/// the panel surface.
///
/// `openness` drives a **corner-bracket snap** on container open:
///
/// * Brackets start `START_OFFSET` px outside the rest position
///   when the container is collapsed.
/// * `snap_t = (openness × SNAP_RATIO).clamp(0, 1)` reaches `1` when
///   openness ≈ `1 / SNAP_RATIO`, so the snap completes BEFORE the
///   body finishes opening — chrome lands first, body fills in.
/// * `ease_out_back` produces a small overshoot past rest before
///   settling, plus a fade-in driven by the same `snap_t` so a
///   collapsed container doesn't have ticks "floating" outside it.
// Per-container stable id passed in as `container_id`. `ui.id()`
// inside the function is the Frame's content_ui id which collapses
// to `parent.with("child")` — the SAME id for every sibling Frame
// in the same parent — so we can't key per-container snap state on
// it. The caller passes the Normal's own `pane_id` (= the
// container's `cid`, unique per stack slot) and we key state under
// that.
fn paint_corner_ticks(
    ui: &mut Ui,
    outer_rect: egui::Rect,
    accent: Color32,
    title_side: TitleSide,
    openness: f32,
    container_id: Id,
) {
    let theme = style::theme();
    let tick_len = theme.section_corner_ticks;
    if tick_len <= 0.0 {
        return;
    }
    let rest_inset = theme.section_corner_ticks_inset;
    // Snap-in animation parameters. The snap clock starts only
    // when `ui.opacity() >= 0.95` — i.e. AFTER the per-section
    // staggered fade-in has essentially finished — so the user
    // actually sees the brackets fly in instead of having the
    // animation play out invisibly under the fade. Same gating
    // pattern the cipher uses.
    const APPEAR_DUR: f32 = 1.0;
    // Brackets fly in from this many pixels OUTSIDE the rest position.
    // Reduced 10 → 7 so a fully-collapsed container's corner ticks
    // sit 3 px closer to the frame edge (the user complained the old
    // value left them visibly floating outside the container).
    const START_OFFSET: f32 = 7.0;
    // Gate at `1.0 - ε` (not `0.95`) so the snap starts only AFTER
    // this container's stagger fade has fully completed, not 5 %
    // before the end. `stagger_opacity` reaches exactly `1.0` at
    // the end of the fade (smoothstep at `t = 1.0` is `1.0`), and
    // `multiply_opacity` is skipped when `stagger_opacity == 1.0`
    // → `ui.opacity()` jumps to exactly `1.0` — `0.999` is just
    // a float-tolerance cushion against rounding.
    const OPACITY_GATE: f32 = 0.999;
    /// Extra delay between the fade completing and the snap
    /// starting — the brackets sit motionless at their start
    /// position for this long after the container becomes fully
    /// opaque, then fly in. Gives the eye a beat to register
    /// "container has arrived" before the next motion starts.
    const DELAY_AFTER_FADE: f64 = 0.25;
    let snap_id = container_id.with("frost_corner_snap");
    let prev_active_id = snap_id.with("prev_active");
    let prev_body_open_id = snap_id.with("prev_body_open");
    let first_seen_id = snap_id.with("first_seen");
    let now = ui.ctx().input(|i| i.time);
    let opacity_active = ui.opacity() >= OPACITY_GATE;
    let body_open_now: bool = ui.ctx().data_mut(|d| {
        d.get_persisted::<bool>(container_id.with("body_open"))
            .unwrap_or(true)
    });
    // `first_seen` is the start-of-snap timestamp. It's set on
    // either of two events and otherwise left alone, so idle paints
    // never replay the animation:
    //   1. Opacity transitions INACTIVE → ACTIVE — i.e. the per-
    //      section staggered fade-in finishes after a real
    //      reappearance (pane just opened or toggled). Fires for
    //      ALL containers in the pane.
    //   2. THIS container's `body_open` flips false → true — the
    //      user unfolded the section. Fires for the single
    //      affected container only; folding (true → false) doesn't
    //      re-fire (the container is going away, the brackets just
    //      track its shrinking edge).
    let first_seen: Option<f64> = ui.ctx().data_mut(|d| {
        let prev_active = d.get_temp::<bool>(prev_active_id).unwrap_or(false);
        d.insert_temp(prev_active_id, opacity_active);
        let became_inactive = prev_active && !opacity_active;

        let prev_body_open = d
            .get_temp::<bool>(prev_body_open_id)
            .unwrap_or(body_open_now);
        d.insert_temp(prev_body_open_id, body_open_now);
        let just_unfolded = !prev_body_open && body_open_now;

        if became_inactive || just_unfolded {
            // Either the whole pane started fading out (will fade
            // back in shortly), or the user just unfolded this
            // section. Drop the recorded `first_seen` so the next
            // active frame re-arms the snap.
            d.remove::<f64>(first_seen_id);
        }
        let existing = d.get_temp::<f64>(first_seen_id);
        match (existing, opacity_active) {
            (Some(t), _) => Some(t),
            (None, true) => {
                // Bias `first_seen` into the future by
                // `DELAY_AFTER_FADE` so `appear = now - first_seen`
                // stays negative (clamped to 0) for that delay,
                // pinning brackets at the start position. The
                // snap then kicks off naturally once `now` catches
                // up with the biased first_seen.
                let biased = now + DELAY_AFTER_FADE;
                d.insert_temp(first_seen_id, biased);
                Some(biased)
            }
            (None, false) => None,
        }
    });
    let appear = match first_seen {
        Some(t) => (((now - t) as f32) / APPEAR_DUR).clamp(0.0, 1.0),
        None => 0.0,
    };
    if appear < 1.0 {
        ui.ctx().request_repaint();
    }
    // Snap progress is driven by `appear` ALONE — re-arming events
    // (pane launch, single-container unfold) drop `first_seen`,
    // which restarts `appear` at 0 and lets `ease_out_elastic`
    // bounce the brackets in over `APPEAR_DUR`. We deliberately do
    // NOT factor `openness` in here: the previous version used
    // `appear.min(openness_t)`, which made the brackets fly OUT
    // during a fold (openness 1 → 0 dragged the easing curve
    // backward through its overshoot region) and re-fly-in during
    // an unfold. With elastic that produced a visible vertical
    // shift on folded panes — the bracket landed past rest, then
    // the outer_rect shrank around it so it appeared offset DOWN
    // from where it should be. By using `appear` alone, the
    // brackets sit exactly at `rest_inset` whenever no snap is
    // playing — folded or open, the placement is identical
    // relative to the (animated) outer_rect.
    let snap_t = appear;
    let snap = ease_out_elastic(snap_t);
    let extra = egui::lerp(-START_OFFSET..=0.0, snap);
    // Resting inset lerps with `openness`: when fully open, brackets
    // sit `rest_inset` px INSIDE the painted outer_rect (theme
    // value, gives breathing room from the frame stroke). When
    // fully folded, they slide out to `FOLDED_INSET` — slightly
    // OUTSIDE the painted edge — so the title strip reads as a
    // self-contained mark with the brackets clinging to its
    // border, not nested inside a small box. `extra` (the snap-in
    // offset) is added on top, so the elastic bounce still plays
    // around whatever resting inset the current fold state picks.
    const FOLDED_INSET: f32 = -1.0;
    let resting = egui::lerp(FOLDED_INSET..=rest_inset, openness);
    let inset = resting + extra;
    let r = outer_rect.shrink(inset);

    // Snap the L-bracket corner positions for a 2-px stroke. The
    // line is drawn centred on `(snap_low|snap_high)(edge)`; with
    // `width = 2.0` the stroke straddles ±1 px around the centre.
    // Using `+ 0.5 / - 0.5` (the right offsets for a 1-px line)
    // pushed half the stroke OUTSIDE the rect on the min edges
    // (left / top) while keeping max edges flush — the visible 1-px
    // overflow on left + bottom. `+ 1.0 / - 1.0` centres the stroke
    // 1 px inside the rounded edge so the full 2-px bar sits inside
    // the rect on every side.
    let snap_low = |v: f32| v.round() + 1.0;
    let snap_high = |v: f32| v.round() - 1.0;
    let lx = snap_low(r.min.x);
    let ty = snap_low(r.min.y);
    let rx = snap_high(r.max.x);
    let by = snap_high(r.max.y);
    let len = tick_len;

    let contrast_col = style::contrast_text_for(accent);
    // Body-side corner ticks paint in the EXACT accent the caller
    // passed — not the brightness-adjusted `high_contrast_accent`
    // variant. The user picks an accent and expects to see THAT
    // colour; the brightness lift was producing a tick that read
    // off-hue from every other accent surface.
    //
    // Brackets sit at full opacity at rest. There's no breathing
    // pulse — a slow alpha sine on the body-side accent reads as
    // unwanted motion in the user's peripheral vision and forces a
    // 30-fps repaint loop just to drive the fade. Snap-in still
    // animates on first appearance / fold-unfold; once that
    // settles, the brackets are static.
    let bracket_accent = accent;
    let accent_col = Color32::from_rgba_unmultiplied(
        bracket_accent.r(),
        bracket_accent.g(),
        bracket_accent.b(),
        255,
    );
    let contrast_col = Color32::from_rgba_unmultiplied(
        contrast_col.r(),
        contrast_col.g(),
        contrast_col.b(),
        255,
    );
    // Body-side bracket colour LERPS from contrast (folded) to
    // accent (unfolded). Folded → all four corners paint in the
    // contrast colour (the "other" colour against the accent panel).
    // As the body unfolds, the body-side pair fades to the accent.
    // Title-side ticks stay contrast throughout (they sit on the
    // accent banner regardless of fold state, so contrast is the
    // only readable choice there).
    let lerp_u8 = |a: u8, b: u8, t: f32| {
        ((a as f32) * (1.0 - t) + (b as f32) * t).round() as u8
    };
    let body_side_col = Color32::from_rgba_unmultiplied(
        lerp_u8(contrast_col.r(), accent_col.r(), openness),
        lerp_u8(contrast_col.g(), accent_col.g(), openness),
        lerp_u8(contrast_col.b(), accent_col.b(), openness),
        lerp_u8(contrast_col.a(), accent_col.a(), openness),
    );
    // Pick which corners are "title-side" vs "body-side" per anchor.
    let (tl, tr, bl, br) = match title_side {
        TitleSide::Top => (contrast_col, contrast_col, body_side_col, body_side_col),
        TitleSide::Bottom => (body_side_col, body_side_col, contrast_col, contrast_col),
        TitleSide::Left => (contrast_col, body_side_col, contrast_col, body_side_col),
        TitleSide::Right => (body_side_col, contrast_col, body_side_col, contrast_col),
    };
    // Doubled-thickness stroke (was 1.0) so the corner ticks read
    // as bold marks rather than hairlines — easier to spot and
    // gives the GAME chrome more visual weight.
    let stroke = |c: Color32| Stroke::new(2.0, c);

    let shapes: [egui::Shape; 8] = [
        // ┌ top-left
        egui::Shape::line_segment([egui::pos2(lx, ty), egui::pos2(lx + len, ty)], stroke(tl)),
        egui::Shape::line_segment([egui::pos2(lx, ty), egui::pos2(lx, ty + len)], stroke(tl)),
        // ┐ top-right
        egui::Shape::line_segment([egui::pos2(rx - len, ty), egui::pos2(rx, ty)], stroke(tr)),
        egui::Shape::line_segment([egui::pos2(rx, ty), egui::pos2(rx, ty + len)], stroke(tr)),
        // └ bottom-left
        egui::Shape::line_segment([egui::pos2(lx, by - len), egui::pos2(lx, by)], stroke(bl)),
        egui::Shape::line_segment([egui::pos2(lx, by), egui::pos2(lx + len, by)], stroke(bl)),
        // ┘ bottom-right
        egui::Shape::line_segment([egui::pos2(rx - len, by), egui::pos2(rx, by)], stroke(br)),
        egui::Shape::line_segment([egui::pos2(rx, by - len), egui::pos2(rx, by)], stroke(br)),
    ];
    ui.painter().extend(shapes);
}

