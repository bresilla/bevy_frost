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
    // Periodic chromatic-aberration ghosts (GAME only — gated on
    // `theme.pane_title_chromatic_aberration`). A deterministic
    // per-id timer fires every few seconds, ramping the offset
    // 0→peak→0 over ~280 ms. We paint a red ghost shifted along the
    // reading direction one way and a cyan ghost the other, then
    // the main text on top — the overlap leaves clean coloured
    // fringes only on the leading and trailing letter edges. PRO
    // sets the flag to false, the helper short-circuits to 0.0,
    // and this collapses to the original single-pass paint.
    let aberration = if theme.pane_title_chromatic_aberration {
        style::chromatic_aberration_offset(ui.ctx(), id.with("chrom_aberr"))
    } else {
        0.0
    };
    let chr_red = Color32::from_rgb(220, 60, 70);
    let chr_cyan = Color32::from_rgb(60, 220, 230);

    if is_horizontal_strip {
        let (pos, align) = if centred {
            (rect.center(), egui::Align2::CENTER_CENTER)
        } else if reversed {
            (
                pos2(
                    (rect.max.x - TITLE_INSET).round(),
                    rect.center().y.round(),
                ),
                egui::Align2::RIGHT_CENTER,
            )
        } else {
            (
                pos2(
                    (rect.min.x + TITLE_INSET).round(),
                    rect.center().y.round(),
                ),
                egui::Align2::LEFT_CENTER,
            )
        };
        if aberration > 0.0 {
            // Horizontal strip: text reads along screen-X, so the
            // chromatic split runs along X. Tiny ±1 px cross jitter
            // (Y) on each ghost for a touch of CRT-misregistration
            // grit without smearing the glyph height.
            const CROSS_JITTER: f32 = 1.0;
            ui.painter().text(
                pos2(pos.x - aberration, pos.y - CROSS_JITTER),
                align,
                &displayed,
                font.clone(),
                chr_red,
            );
            ui.painter().text(
                pos2(pos.x + aberration, pos.y + CROSS_JITTER),
                align,
                &displayed,
                font.clone(),
                chr_cyan,
            );
        }
        ui.painter().text(pos, align, displayed, font, text_col);
    } else {
        // Vertical strip: lay out a single galley with placeholder
        // colour so the same `Arc<Galley>` can drive three
        // `TextShape`s tinted differently for the aberration ghosts
        // and the main text. Cheap clone (Arc bump) instead of
        // re-laying out three separate galleys.
        let galley =
            ui.painter().layout_no_wrap(displayed, font, Color32::PLACEHOLDER);
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
        // For rotated text (vertical-strip pane title) the reading
        // direction is screen-Y, so the chromatic split must run
        // along Y to land along the text's length, not across its
        // height. Tiny ±1 px cross jitter (X) on each ghost adds a
        // CRT-misregistration touch without distorting the glyph
        // height after rotation.
        if aberration > 0.0 {
            const CROSS_JITTER: f32 = 1.0;
            let r_pos = pos2(text_pos.x - CROSS_JITTER, text_pos.y - aberration);
            let c_pos = pos2(text_pos.x + CROSS_JITTER, text_pos.y + aberration);
            let mut s_red = egui::epaint::TextShape::new(r_pos, galley.clone(), chr_red);
            s_red.angle = angle;
            ui.painter().add(s_red);
            let mut s_cyan = egui::epaint::TextShape::new(c_pos, galley.clone(), chr_cyan);
            s_cyan.angle = angle;
            ui.painter().add(s_cyan);
        }
        let mut shape = egui::epaint::TextShape::new(text_pos, galley, text_col);
        shape.angle = angle;
        ui.painter().add(shape);
    }

    // ── 4. Blinking pip(s) (GAME only) ──
    if stripes_on {
        const PIP_INSET: f32 = TITLE_INSET;
        // Per-second blink — `ON_FRAC` controls how long the pip
        // stays bright at the start of each cycle. Bumped from 0.08
        // (frostcore's value) so the on-state lingers a touch
        // longer and reads clearly between dims.
        let time = ui.ctx().input(|i| i.time) as f32;
        const ON_FRAC: f32 = 0.16;
        let on = time.fract() < ON_FRAC;
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
