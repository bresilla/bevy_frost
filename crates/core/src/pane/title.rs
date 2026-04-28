//! Title-strip painter for [`super::Pane2`]. Theme-aware (PRO solid
//! accent vs GAME caution stripes), with text alignment + blinking
//! pip placement driven by the pane's [`PaneAnchor`].

use egui::{pos2, Color32, Id, Rect};

use super::anchor::{PaneAnchor, TitleSide};
use crate::style;

/// Paint the title strip background + text inside `rect`. Five
/// pieces:
///
/// 1. Background: theme-driven panel fill (PRO) or animated
///    caution stripes (GAME) restricted to the title rect.
/// 2. Title text: scramble-decoded when `scramble_titles` is on,
///    aligned per anchor (centred for Middle zones, reversed for
///    TS / RS / RE / BE so the first letter sits next to the
///    pane's own button).
/// 3. Blinking pip(s) — single pip opposite the text on corner
///    anchors; two pips (one each end) for Middle anchors.
/// 4. Divider hairline on the body-facing edge of the strip
///    (`pane_show_title_divider`).
pub(crate) fn paint_pane_title(
    ui: &mut egui::Ui,
    rect: Rect,
    id: Id,
    title: &str,
    anchor: PaneAnchor,
    accent: Color32,
) {
    const TITLE_INSET: f32 = 8.0;
    const PIP_SIZE: f32 = 6.0;
    let title_size = 15.0 * 1.15;
    let theme = style::theme();
    let stripes_on = theme.pane_title_stripes;

    // ── 1. Background ──
    if !theme.pane_fill_visible && !stripes_on {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(theme.radius_lg),
            style::pane_fill(accent),
        );
    }
    if stripes_on {
        style::paint_caution_stripes(ui.painter(), rect, accent);
    }

    // ── 2. Title text colour + content ──
    let text_col = if stripes_on {
        if theme.is_light { Color32::BLACK } else { Color32::WHITE }
    } else {
        style::section_title_color(accent)
    };
    let font = egui::FontId::new(title_size, style::title_font_family());
    let title_uc = title.to_uppercase();
    let displayed = if theme.scramble_titles {
        let session_id = id.with("pane2_title_session");
        let session = style::appearance_session(ui.ctx(), session_id);
        let scramble_id = session_id.with(session);
        let scrambled = style::scramble_text(ui.ctx(), scramble_id, &title_uc, true);
        // Same periodic single-letter glitch the container title
        // uses — keeps the pane title alive after its decode cycle.
        style::glitch_text(ui.ctx(), session_id.with("glitch"), &scrambled)
    } else {
        title_uc
    };

    let title_side = anchor.title_side();
    let is_horizontal_strip = title_side.is_horizontal_strip();
    let reversed = anchor.title_reversed();
    let centred = anchor.is_middle();

    // ── 3. Title text paint ──
    if is_horizontal_strip {
        if centred {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                displayed,
                font,
                text_col,
            );
        } else if reversed {
            let pos = pos2(
                (rect.max.x - TITLE_INSET).round(),
                rect.center().y.round(),
            );
            ui.painter().text(
                pos,
                egui::Align2::RIGHT_CENTER,
                displayed,
                font,
                text_col,
            );
        } else {
            let pos = pos2(
                (rect.min.x + TITLE_INSET).round(),
                rect.center().y.round(),
            );
            ui.painter().text(
                pos,
                egui::Align2::LEFT_CENTER,
                displayed,
                font,
                text_col,
            );
        }
    } else {
        let galley = ui.painter().layout_no_wrap(displayed, font, text_col);
        let g = galley.size();
        let cx = rect.center().x;
        let on_right_side = title_side == TitleSide::Right;
        let top_to_bottom = on_right_side ^ reversed;
        let (text_pos, angle) = if centred {
            if top_to_bottom {
                (
                    pos2(
                        (cx + g.y * 0.5).round(),
                        (rect.center().y - g.x * 0.5).round(),
                    ),
                    std::f32::consts::FRAC_PI_2,
                )
            } else {
                (
                    pos2(
                        (cx - g.y * 0.5).round(),
                        (rect.center().y + g.x * 0.5).round(),
                    ),
                    -std::f32::consts::FRAC_PI_2,
                )
            }
        } else if top_to_bottom {
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
        let mut shape = egui::epaint::TextShape::new(text_pos, galley, text_col);
        shape.angle = angle;
        ui.painter().add(shape);
    }

    // ── 4. Blinking pip(s) (GAME only) ──
    if stripes_on {
        const PIP_INSET: f32 = TITLE_INSET;
        let time = ui.ctx().input(|i| i.time) as f32;
        let on = time.fract() < 0.08;
        let alpha = if on { 255 } else { 76 };
        let pip_color = Color32::from_rgba_unmultiplied(
            text_col.r(),
            text_col.g(),
            text_col.b(),
            alpha,
        );
        let paint_pip = |r: Rect| {
            ui.painter()
                .rect_filled(r, egui::CornerRadius::ZERO, pip_color);
        };

        if is_horizontal_strip {
            let cy = (rect.center().y - PIP_SIZE * 0.5).round();
            let right_x = (rect.max.x - PIP_INSET - PIP_SIZE).round();
            let left_x = (rect.min.x + PIP_INSET).round();
            if centred {
                paint_pip(Rect::from_min_size(
                    pos2(left_x, cy),
                    egui::vec2(PIP_SIZE, PIP_SIZE),
                ));
                paint_pip(Rect::from_min_size(
                    pos2(right_x, cy),
                    egui::vec2(PIP_SIZE, PIP_SIZE),
                ));
            } else if reversed {
                paint_pip(Rect::from_min_size(
                    pos2(left_x, cy),
                    egui::vec2(PIP_SIZE, PIP_SIZE),
                ));
            } else {
                paint_pip(Rect::from_min_size(
                    pos2(right_x, cy),
                    egui::vec2(PIP_SIZE, PIP_SIZE),
                ));
            }
        } else {
            let cx = (rect.center().x - PIP_SIZE * 0.5).round();
            let top_y = (rect.min.y + PIP_INSET).round();
            let bottom_y = (rect.max.y - PIP_INSET - PIP_SIZE).round();
            let on_right_side = title_side == TitleSide::Right;
            let top_to_bottom = on_right_side ^ reversed;
            if centred {
                paint_pip(Rect::from_min_size(
                    pos2(cx, top_y),
                    egui::vec2(PIP_SIZE, PIP_SIZE),
                ));
                paint_pip(Rect::from_min_size(
                    pos2(cx, bottom_y),
                    egui::vec2(PIP_SIZE, PIP_SIZE),
                ));
            } else if top_to_bottom {
                paint_pip(Rect::from_min_size(
                    pos2(cx, bottom_y),
                    egui::vec2(PIP_SIZE, PIP_SIZE),
                ));
            } else {
                paint_pip(Rect::from_min_size(
                    pos2(cx, top_y),
                    egui::vec2(PIP_SIZE, PIP_SIZE),
                ));
            }
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(33));
    }

    // ── 5. Divider hairline on the body-facing edge ──
    if theme.pane_show_title_divider {
        let stroke =
            egui::Stroke::new(theme.border_width, style::widget_border(accent));
        match title_side {
            TitleSide::Top => {
                ui.painter().hline(
                    rect.min.x..=rect.max.x,
                    rect.max.y - 0.5,
                    stroke,
                );
            }
            TitleSide::Bottom => {
                ui.painter().hline(
                    rect.min.x..=rect.max.x,
                    rect.min.y + 0.5,
                    stroke,
                );
            }
            TitleSide::Left => {
                ui.painter().vline(
                    rect.max.x - 0.5,
                    rect.min.y..=rect.max.y,
                    stroke,
                );
            }
            TitleSide::Right => {
                ui.painter().vline(
                    rect.min.x + 0.5,
                    rect.min.y..=rect.max.y,
                    stroke,
                );
            }
        }
    }
}
