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

use egui::{epaint::TextShape, pos2, vec2, Align2, Color32, CornerRadius, FontId, Frame, Sense, Stroke, Ui, Vec2};

use crate::flex::{item, Flex, FlexAlign, Size};
use crate::pane::{PaneAnchor, TitleSide};
use crate::style;

/// Title-bar thickness (perpendicular to the strip's long axis).
pub const TITLE_ZONE_THICKNESS: f32 = 22.0;
/// Inset between strip edge and the title text's reading-start.
const TITLE_INSET: f32 = 6.0;
/// Padding between title strip and body (currently rendered as gap=0
/// so the hairline divider reads cleanly; kept as a knob for later
/// tuning).
const _BODY_PAD: f32 = 6.0;
/// Default body main-axis size — the dimension perpendicular to the
/// title strip. Caps the pane's growth so an empty container doesn't
/// inflate to fill the screen.
pub const BODY_MAIN_SIZE: f32 = 180.0;
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

        // Cross axis = the dim the title strip spans. The parent pane
        // locked this axis (Pane2's `set_max_width` / `set_max_height`
        // in `body_paint`) so reading available_size here is stable
        // across flex passes.
        let outer_avail = ui.available_size();
        let cross_inner = if horizontal_strip {
            (outer_avail.x - pad_w - outer_w).max(0.0)
        } else {
            (outer_avail.y - pad_h - outer_h).max(0.0)
        };

        let title_size = if horizontal_strip {
            vec2(cross_inner, TITLE_ZONE_THICKNESS)
        } else {
            vec2(TITLE_ZONE_THICKNESS, cross_inner)
        };
        let body_size = if horizontal_strip {
            vec2(cross_inner, BODY_MAIN_SIZE)
        } else {
            vec2(BODY_MAIN_SIZE, cross_inner)
        };

        let id_salt = ui.id().with("normal_flex");
        let title_text = self.title.clone();
        let anchor = self.anchor;
        let accent = self.accent;

        let frame = self.theme_frame();
        frame.show(ui, |ui| {
            let flex = if horizontal_strip {
                Flex::vertical().width(Size::Points(cross_inner))
            } else {
                Flex::horizontal().height(Size::Points(cross_inner))
            };
            flex.gap(Vec2::ZERO)
                .align_items(FlexAlign::Stretch)
                .id_salt(id_salt)
                .show(ui, |flex| {
                    let title_paint = move |ui: &mut Ui| {
                        // Allocate EXACT title size — `available_size`
                        // is unsafe inside flex (NOTES.md #1).
                        let (rect, _) = ui.allocate_exact_size(title_size, Sense::hover());
                        paint_title(ui, rect, &title_text, anchor, accent);
                    };
                    let body_paint = move |ui: &mut Ui| {
                        // Same recipe — pre-computed body_size.
                        let (_, _) = ui.allocate_exact_size(body_size, Sense::hover());
                        body(ui);
                    };

                    if title_at_end {
                        flex.add_ui(
                            item().basis(BODY_MAIN_SIZE).min_size(body_size),
                            body_paint,
                        );
                        flex.add_ui(
                            item().basis(TITLE_ZONE_THICKNESS).min_size(title_size),
                            title_paint,
                        );
                    } else {
                        flex.add_ui(
                            item().basis(TITLE_ZONE_THICKNESS).min_size(title_size),
                            title_paint,
                        );
                        flex.add_ui(
                            item().basis(BODY_MAIN_SIZE).min_size(body_size),
                            body_paint,
                        );
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
    let painter = ui.painter_at(rect);
    let font = FontId::proportional(13.0);

    match title_side {
        TitleSide::Top | TitleSide::Bottom => {
            // Same alignment recipe as `pane::title::paint_pane_title`:
            // for "reversed" anchors (TS, RS, RE, BE) the first letter
            // sits next to the pane's own button on the rail —
            // RIGHT_CENTER on RightRail anchors, LEFT_CENTER otherwise.
            if anchor.title_reversed() {
                painter.text(
                    rect.right_center() - vec2(TITLE_INSET, 0.0),
                    Align2::RIGHT_CENTER,
                    title,
                    font,
                    title_col,
                );
            } else {
                painter.text(
                    rect.left_center() + vec2(TITLE_INSET, 0.0),
                    Align2::LEFT_CENTER,
                    title,
                    font,
                    title_col,
                );
            }
            // Hairline divider on the side facing the body.
            let y = match title_side {
                TitleSide::Top => rect.bottom().round() + 0.5,
                _ => rect.top().round() - 0.5,
            };
            painter.hline(rect.x_range(), y, Stroke::new(1.0, theme.border_subtle));
        }
        TitleSide::Left | TitleSide::Right => {
            let galley = painter.layout_no_wrap(title.to_string(), font, title_col);
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
            painter.add(shape);
            let x = match title_side {
                TitleSide::Left => rect.right().round() + 0.5,
                _ => rect.left().round() - 0.5,
            };
            painter.vline(x, rect.y_range(), Stroke::new(1.0, theme.border_subtle));
        }
    }
}
