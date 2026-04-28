//! `Normal` container — a flex-based, two-part block (title zone +
//! body zone) dropped into a [`crate::pane::Pane2`] body. The
//! container's title sits on the **same side** of the block as the
//! parent pane's title strip, so nested chrome chords with the pane
//! chrome.
//!
//! Layout uses [`crate::flex::Flex`] internally — same multi-pass-
//! safe machinery the pane itself uses. The cross axis (the dim the
//! title strip spans) is locked to the parent pane's locked axis;
//! the main axis is `TITLE_ZONE_THICKNESS + BODY_PAD + BODY_MAIN_SIZE`,
//! capping the pane's growth.
//!
//! ```ignore
//! Normal::new("Properties", anchor, accent).show(ui, |ui| {
//!     ui.label("body content");
//! });
//! ```

use egui::{epaint::TextShape, pos2, vec2, Align2, Color32, CornerRadius, FontId, Frame, Sense, Stroke, Ui};

use super::body::Body;
use crate::flex::{item, Flex, FlexAlign, Size};
use crate::pane::{PaneAnchor, TitleSide};
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
}

impl Normal {
    pub fn new(title: impl Into<String>, anchor: PaneAnchor, accent: Color32) -> Self {
        Self {
            title: title.into(),
            anchor,
            accent,
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
        // Vertical-title containers cap the body's main-axis width
        // (default for X) so a `text_input` (which fills
        // `available_width`) doesn't blow up the container.
        let body_main_max = if horizontal_strip {
            None
        } else {
            Some(
                (CONTAINER_DEFAULT_WIDTH - TITLE_ZONE_THICKNESS - pad_w - outer_w).max(0.0),
            )
        };

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

        let frame = self.theme_frame();
        frame.show(ui, |ui| {
            // Lock the CROSS axis only — main stays content-driven
            // so the pane (which sizes to its body's intrinsic) is
            // small for small content, big for big content.
            let flex = if horizontal_strip {
                Flex::vertical().width(Size::Points(cross_inner))
            } else {
                Flex::horizontal().height(Size::Points(cross_inner))
            };
            // Main-axis gap = 2× `TITLE_BODY_GAP_HALF` so the divider
            // (painted at the gap's MIDPOINT inside `paint_title`)
            // gets equal breathing space on both sides — between the
            // title text and the line, and between the line and the
            // body's first widget.
            let total_gap = TITLE_BODY_GAP_HALF * 2.0;
            let gap = if horizontal_strip {
                vec2(0.0, total_gap)
            } else {
                vec2(total_gap, 0.0)
            };
            flex.gap(gap)
                .align_items(FlexAlign::Stretch)
                .id_salt(id_salt)
                .show(ui, |flex| {
                    let title_paint = move |ui: &mut Ui| {
                        // Allocate EXACT title size — `available_size`
                        // is unsafe inside flex (NOTES.md #1).
                        let (rect, _) = ui.allocate_exact_size(title_size, Sense::hover());
                        paint_title(ui, rect, &title_text, anchor, accent);
                    };

                    if title_at_end {
                        flex.add_ui(body_cfg.flex_item(), move |ui| {
                            body_cfg.paint(ui, body);
                        });
                        flex.add_ui(
                            item().basis(TITLE_ZONE_THICKNESS).min_size(title_size),
                            title_paint,
                        );
                    } else {
                        flex.add_ui(
                            item().basis(TITLE_ZONE_THICKNESS).min_size(title_size),
                            title_paint,
                        );
                        flex.add_ui(body_cfg.flex_item(), move |ui| {
                            body_cfg.paint(ui, body);
                        });
                    }
                });
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
fn paint_title(ui: &mut Ui, rect: egui::Rect, title: &str, anchor: PaneAnchor, accent: Color32) {
    let theme = style::theme();
    let title_col = style::section_title_color(accent);
    let title_side = anchor.title_side();
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
            // `rect`.
            let y = match title_side {
                TitleSide::Top => (rect.bottom() + TITLE_BODY_GAP_HALF).round() + 0.5,
                _ => (rect.top() - TITLE_BODY_GAP_HALF).round() - 0.5,
            };
            let x_range = (rect.left() + DIVIDER_INSET)..=(rect.right() - DIVIDER_INSET);
            painter.hline(x_range, y, Stroke::new(1.0, theme.border_subtle));
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
            let x = match title_side {
                TitleSide::Left => (rect.right() + TITLE_BODY_GAP_HALF).round() + 0.5,
                _ => (rect.left() - TITLE_BODY_GAP_HALF).round() - 0.5,
            };
            let y_range = (rect.top() + DIVIDER_INSET)..=(rect.bottom() - DIVIDER_INSET);
            painter.vline(x, y_range, Stroke::new(1.0, theme.border_subtle));
        }
    }
}
