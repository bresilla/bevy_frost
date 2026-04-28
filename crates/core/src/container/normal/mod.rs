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
    epaint::TextShape, pos2, vec2, Align, Align2, Color32, CornerRadius, FontId, Frame, Id, Layout,
    Rect, Sense, Stroke, Ui, UiBuilder,
};

use super::body::Body;
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
/// Default cross-axis size. Used as the container's locked cross
/// dimension — width for horizontal-title containers, height for
/// vertical-title containers. The MAIN axis stays content-driven
/// (capped via `Body::max_main` for vertical-title to stop a body
/// like `text_input` from growing the pane unboundedly along X).
/// Pane2's locked cross axis matches this constant so the pane and
/// container share the same outer cross dimension.
pub const CONTAINER_DEFAULT_WIDTH: f32 = 280.0;
pub const CONTAINER_DEFAULT_HEIGHT: f32 = 280.0;
/// Outer margin between the container's painted frame and the
/// parent pane's body inset. Small (a few px) just so the container
/// doesn't sit flush against the pane chrome.
const OUTER_MARGIN: i8 = 3;

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
        }
    }

    pub fn show(self, ui: &mut Ui, body: impl FnOnce(&mut Ui)) {
        let title_side = self.anchor.title_side();
        let horizontal_strip = title_side.is_horizontal_strip();
        let title_at_end = title_side.is_at_end();

        let pad = style::section_padding();
        let pad_w = (pad.left as f32) + (pad.right as f32);
        let pad_h = (pad.top as f32) + (pad.bottom as f32);
        // Outer margin is applied by the Frame and reserved on
        // BOTH sides of every axis. Subtract the same amount from
        // the cross-axis target so the inner flex sees the actual
        // content area.
        let outer_w = (OUTER_MARGIN as f32) * 2.0;
        let outer_h = (OUTER_MARGIN as f32) * 2.0;

        // Cross axis = the dim the title strip spans. Locked to
        // `CONTAINER_DEFAULT_*` (clamped to `outer_avail`).
        // Main axis stays content-driven so the container — and the
        // pane wrapping it — collapse to widget size when the body
        // is empty.
        let outer_avail = ui.available_size();
        let cross_inner = if horizontal_strip {
            (CONTAINER_DEFAULT_WIDTH - pad_w - outer_w)
                .min((outer_avail.x - pad_w - outer_w).max(0.0))
                .max(0.0)
        } else {
            (CONTAINER_DEFAULT_HEIGHT - pad_h - outer_h)
                .min((outer_avail.y - pad_h - outer_h).max(0.0))
                .max(0.0)
        };
        // No `max_main` override — Body's set_max_width on the
        // main axis was EXTENDING the child UI's max_rect beyond
        // `full_body_main`, causing `text_input` to render wider
        // than the body slot and the container's `min_rect` to
        // overflow. The child UI built with `max_rect = full_rect`
        // already gives `text_input` the right `available_width`.
        let body_main_max: Option<f32> = None;

        let title_size = if horizontal_strip {
            vec2(cross_inner, TITLE_ZONE_THICKNESS)
        } else {
            vec2(TITLE_ZONE_THICKNESS, cross_inner)
        };

        // Shared body recipe — used by both `Normal` and (later)
        // `Tabbed` so the cross-axis clamp + flex-multipass-safety
        // logic lives in one place.
        let mut body_cfg = Body::new(horizontal_strip, cross_inner);
        if let Some(max) = body_main_max {
            body_cfg = body_cfg.max_main(max);
        }

        let id_salt = ui.id().with("normal_flex");
        let title_text = self.title.clone();
        let anchor = self.anchor;
        let accent = self.accent;

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
        let _ = id_salt; // (still kept around in case the body re-introduces flex later)
        // Body's full main-axis size when fully open. Used as the
        // child UI's `max_rect` extent so widgets ALWAYS render at
        // their natural size; only the clip mask animates.
        let full_body_main = if horizontal_strip {
            (CONTAINER_DEFAULT_HEIGHT
                - TITLE_ZONE_THICKNESS
                - pad_h
                - outer_h
                - TITLE_BODY_GAP_HALF * 2.0)
                .max(0.0)
        } else {
            (CONTAINER_DEFAULT_WIDTH
                - TITLE_ZONE_THICKNESS
                - pad_w
                - outer_w
                - TITLE_BODY_GAP_HALF * 2.0)
                .max(0.0)
        };
        // Body slot size LERPS with `openness` to match Pane2's
        // lerp (both compute openness from the SAME `animate_bool`
        // call, so they animate in lockstep — no anchor drift).
        let body_visible = openness > 0.0;
        let total_gap = TITLE_BODY_GAP_HALF * 2.0 * openness;
        let visible_body_main = openness * full_body_main;

        let frame = self.theme_frame();
        frame.show(ui, |ui| {
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
            let layout = if horizontal_strip {
                Layout::top_down(Align::Min)
            } else {
                Layout::left_to_right(Align::Min)
            };
            let mut layout_ui = ui.new_child(
                UiBuilder::new().max_rect(ui.max_rect()).layout(layout),
            );
            // Zero item_spacing — we control all gaps via
            // `total_gap` (animated) and `allocate_exact_size`.
            // Egui's default spacing (3 px vert / 8 px horiz)
            // would otherwise stack on top of `total_gap` and
            // overflow the pane.
            layout_ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

            let render_title = |ui: &mut Ui| {
                let (rect, resp) = ui.allocate_exact_size(title_size, Sense::click());
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    pane::toggle_body(ui.ctx(), pane_id);
                }
                paint_title(ui, rect, &title_text, anchor, accent, open);
            };

            let render_body = |ui: &mut Ui, body: Box<dyn FnOnce(&mut Ui)>| {
                if !body_visible || full_body_main <= 0.0 {
                    return;
                }
                let cur_min = ui.cursor().min;
                let visible_size = if horizontal_strip {
                    vec2(cross_inner, visible_body_main)
                } else {
                    vec2(visible_body_main, cross_inner)
                };
                let full_size = if horizontal_strip {
                    vec2(cross_inner, full_body_main)
                } else {
                    vec2(full_body_main, cross_inner)
                };
                let visible_rect = Rect::from_min_size(cur_min, visible_size);
                let full_rect = Rect::from_min_size(cur_min, full_size);
                // egui's `CollapsingState` recipe: render at FULL
                // size in a child UI, clip to the VISIBLE portion.
                // Widgets get their natural `available_*` (no
                // shrinking), only the clip mask animates.
                let body_layout = if horizontal_strip {
                    Layout::top_down(Align::Min)
                } else {
                    Layout::left_to_right(Align::Min)
                };
                let mut child = ui.new_child(
                    UiBuilder::new().max_rect(full_rect).layout(body_layout),
                );
                let parent_clip = ui.clip_rect();
                child.set_clip_rect(parent_clip.intersect(visible_rect));
                body_cfg.paint(&mut child, body);
                // Allocate ONLY the visible portion in parent → the
                // container's outer rect lerps with `openness`, in
                // lockstep with Pane2's pre-computed outer size.
                let _ = ui.allocate_rect(visible_rect, Sense::hover());
            };

            let body_box: Box<dyn FnOnce(&mut Ui)> = Box::new(body);
            if title_at_end {
                render_body(&mut layout_ui, body_box);
                if total_gap > 0.0 {
                    layout_ui.add_space(total_gap);
                }
                render_title(&mut layout_ui);
            } else {
                render_title(&mut layout_ui);
                if total_gap > 0.0 {
                    layout_ui.add_space(total_gap);
                }
                render_body(&mut layout_ui, body_box);
            }

            // After laying out, advance the parent ui's cursor
            // past what `layout_ui` used so the Frame measures the
            // animated extent.
            let used = layout_ui.min_rect();
            let _ = ui.allocate_rect(used, Sense::hover());

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
        });
    }

    /// Same recipe as `frostcore::widgets::foldable::section_tracked`'s
    /// outer frame: glass-card fill, accent-tinted border, theme
    /// `radius_md` corners. When the active theme has
    /// `section_show_frame = false` (GAME) we drop the visuals and
    /// keep just the inner padding so body content sits flush.
    fn theme_frame(&self) -> Frame {
        let theme = style::theme();
        let outer = egui::Margin::same(OUTER_MARGIN);
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

/// Paint the title strip into `rect`. Picks horizontal vs rotated
/// text based on `anchor.title_side()` so the container matches the
/// parent pane's title direction.
fn paint_title(
    ui: &mut Ui,
    rect: egui::Rect,
    title: &str,
    anchor: PaneAnchor,
    accent: Color32,
    open: bool,
) {
    let theme = style::theme();
    let title_side = anchor.title_side();
    // GAME-style banner: text uses a contrast colour against the
    // accent. The banner FILL itself is painted outside this fn
    // (in `Normal::show`) so it can extend through the surrounding
    // inner_margin and into half the flex gap — i.e., the padding
    // around the title gets the same accent colour, not the body
    // colour.
    let filled = theme.title_strip_filled;
    let title_col = if filled {
        style::contrast_text_for(accent)
    } else {
        style::section_title_color(accent)
    };
    // Text + galley paints clip to `rect`; the body-facing divider
    // sits ON the rect's edge so it must use the unclipped parent
    // `ui.painter()`, otherwise it gets cut by `painter_at(rect)`.
    let title_painter = ui.painter_at(rect);
    let painter = ui.painter();
    let font = FontId::proportional(13.0);

    match title_side {
        TitleSide::Top | TitleSide::Bottom => {
            // Same alignment recipe as `pane::title::paint_pane_title`:
            // for "reversed" anchors (TS, RS, RE, BE) the first letter
            // sits next to the pane's own button on the rail —
            // RIGHT_CENTER on RightRail anchors, LEFT_CENTER otherwise.
            if anchor.title_reversed() {
                title_painter.text(
                    rect.right_center() - vec2(TITLE_INSET, 0.0),
                    Align2::RIGHT_CENTER,
                    title,
                    font,
                    title_col,
                );
            } else {
                title_painter.text(
                    rect.left_center() + vec2(TITLE_INSET, 0.0),
                    Align2::LEFT_CENTER,
                    title,
                    font,
                    title_col,
                );
            }
            // Hairline divider painted at the MIDPOINT of the flex
            // gap — `TITLE_BODY_GAP_HALF` outside the title rect's
            // body-facing edge — so it gets equal padding on both
            // sides (title-to-divider + divider-to-body). Drawn
            // with the unclipped painter so it isn't culled by
            // `rect`. Skipped in GAME theme since the accent-filled
            // banner already separates title from body.
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
            let galley = title_painter.layout_no_wrap(title.to_string(), font, title_col);
            let g = galley.size();
            let cx = rect.center().x;
            // Matches `pane::title::paint_pane_title`'s convention so
            // the container's rotated text reads in the SAME
            // direction as the parent pane's title:
            //   top_to_bottom = (right_side) XOR (reversed)
            // `title_reversed()` is `true` for TS, RS, RE, BE.
            let on_right_side = title_side == TitleSide::Right;
            let top_to_bottom = on_right_side ^ anchor.title_reversed();
            let (text_pos, angle) = if top_to_bottom {
                (
                    pos2(
                        (cx + g.y * 0.5).round(),
                        (rect.min.y + TITLE_INSET).round(),
                    ),
                    std::f32::consts::FRAC_PI_2,
                )
            } else {
                (
                    pos2(
                        (cx - g.y * 0.5).round(),
                        (rect.max.y - TITLE_INSET).round(),
                    ),
                    -std::f32::consts::FRAC_PI_2,
                )
            };
            let mut shape = TextShape::new(text_pos, galley, title_col);
            shape.angle = angle;
            title_painter.add(shape);
            // Hairline divider painted at the MIDPOINT of the flex
            // gap (see horizontal-strip branch for rationale).
            // Skipped under GAME's filled banner.
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
}
