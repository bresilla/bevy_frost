//! Foldable container — the main building block of every panel.
//!
//! A collapsible card with an accent-coloured UPPERCASE header:
//!
//!   [HEADER]                (accent UPPERCASE, chevron on the left)
//!   body goes here
//!
//! Uses egui's `CollapsingHeader` under the hood so the ▶ / ▼
//! chevron still animates. The frame pins to the panel's full
//! available width and hard-clips its body so unconstrained child
//! widgets can't push the card wider than the panel.
//!
//! When `unfoldable` siblings land (a container with the same frame
//! but no collapse header), they'll share this file's sizing
//! constants.

use egui;

use crate::style::{
    glass_alpha_card, glass_fill, section_caps, thin_divider, widget_border,
};

use super::shared::{begin_row_zebra, commit_row_zebra, flush_pending_separator};

/// Horizontal inner padding inside the container, in px.
pub const PAD_X: i8 = 4;
/// Vertical inner padding inside the container, in px.
pub const PAD_Y: i8 = 3;
/// Pixels reserved inside the panel for the card's horizontal
/// padding + stroke. Must match `2 * PAD_X + stroke_width`.
pub const OUTER_INSET: f32 = (PAD_X as f32) * 2.0 + 2.0;

pub fn section(
    ui: &mut egui::Ui,
    id_salt: &str,
    title: &str,
    accent: egui::Color32,
    default_open: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    let _ = section_tracked(ui, id_salt, title, accent, default_open, None, body);
}

/// Kept (deprecated) so any downstream code that referenced these
/// constants still compiles. Action-button slots in section
/// headers were removed — the floating overlay (`maximizable`'s
/// own chip) is the canonical home for "lift this widget" controls
/// now.
#[deprecated(note = "section header actions slot was removed")]
pub const HEADER_ACTION_SIZE: f32 = 18.0;
#[deprecated(note = "section header actions slot was removed")]
pub const HEADER_ACTION_GAP: f32 = 2.0;
#[deprecated(note = "section header actions slot was removed")]
pub fn header_actions_width(_count: u8) -> f32 {
    0.0
}

/// What [`section_tracked`] reports back to the pane: the egui id
/// under which the section's `CollapsingState` lives, the full outer
/// rect (frame included), and the header's drag-aware response (a
/// `click_and_drag` Sense — short clicks toggle the chevron, drags
/// drive the pane's reorder gesture).
pub(crate) struct SectionTrack {
    pub state_id: egui::Id,
    pub outer_rect: egui::Rect,
    pub header_response: egui::Response,
}

/// Same visual recipe as [`section`] but with a custom-painted
/// header (chevron + UPPERCASE title) backed by a single
/// `click_and_drag` interaction zone. That's what lets the pane
/// host both fold-toggle (short click) and drag-reorder (sustained
/// motion past egui's drag threshold) on the same header strip
/// without the two senses fighting each other — the long-standing
/// "I can't click to fold once drag is wired up" problem.
///
/// Returns the section's outer rect, the underlying
/// `CollapsingState` id (so the pane's auto-fold pass can force the
/// section closed from outside), and the header's response. Body
/// rendering and frame styling are unchanged from `section`.
pub(crate) fn section_tracked(
    ui: &mut egui::Ui,
    id_salt: &str,
    title: &str,
    accent: egui::Color32,
    default_open: bool,
    icon: Option<&str>,
    body: impl FnOnce(&mut egui::Ui),
) -> SectionTrack {
    flush_pending_separator(ui);
    let full_w = ui.available_width();
    // Theme-driven outer inset — must match the frame's actual
    // horizontal footprint or successive sections render at slightly
    // different visible widths (frame overflows by a few px and
    // egui clips it inconsistently). Footprint = inner_margin x2 +
    // border_width x2.
    let theme_outer_inset = (crate::style::theme().section_pad_x as f32) * 2.0
        + crate::style::theme().border_width * 2.0;
    let inner_w = (full_w - theme_outer_inset).max(0.0);
    let outer_top = ui.cursor().min;

    // Use a frost-managed state id so the pane can read / write the
    // open flag from outside without having to mirror egui's internal
    // `make_persistent_id` chain.
    let state_id = ui.make_persistent_id(("frost_section", id_salt));
    let mut state =
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), state_id, default_open);

    let mut captured_header_response: Option<egui::Response> = None;
    // Banner placeholder: reserved as a `Shape::Noop` inside the
    // frame body (so it lands at the right shape index — above the
    // frame fill, below all header / body widgets) and SET after
    // `frame.show` returns, once `outer_rect` and the fold openness
    // are known. This is what lets the banner stretch all the way
    // down to cover the *whole* section when it's folded — at
    // shape-allocation time, the height isn't known yet.
    let mut banner_idx: Option<egui::layers::ShapeIdx> = None;
    let mut captured_openness: f32 = 1.0;

    // Theme-driven section frame: PRO paints the glass card (fill +
    // border + corners + padding); GAME bypasses the frame entirely
    // (`section_show_frame = false`) so body content sits flush on
    // the pane background. Inner padding, fill, stroke, corner —
    // all read from the active theme.
    let frame = if crate::style::section_show_frame() {
        egui::Frame::new()
            .fill(glass_fill(crate::style::section_fill(accent), accent, glass_alpha_card()))
            .corner_radius(egui::CornerRadius::same(crate::style::theme().radius_md))
            .stroke(egui::Stroke::new(crate::style::theme().border_width, widget_border(accent)))
            .inner_margin(crate::style::section_padding())
    } else {
        // No frame at all — body content paints directly on the
        // pane background.
        egui::Frame::new().inner_margin(crate::style::section_padding())
    };
    let frame_inner = frame.show(ui, |ui| {
            // Reserve a placeholder for the title banner. Set after
            // `frame.show` returns so we can compute the height
            // based on the section's final outer rect + openness.
            if crate::style::theme().title_strip_filled {
                banner_idx = Some(ui.painter().add(egui::Shape::Noop));
            }
            ui.allocate_ui_with_layout(
                egui::vec2(inner_w, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(inner_w);
                    let clip = ui.clip_rect().intersect(egui::Rect::from_min_size(
                        ui.min_rect().min,
                        egui::vec2(inner_w, f32::INFINITY),
                    ));
                    ui.set_clip_rect(clip);

                    // Custom header layout:
                    //   [chevron] [optional icon] TITLE  …  [actions]
                    //
                    // The chevron+icon+title strip on the LEFT carries
                    // a single `click_and_drag` sense so short clicks
                    // toggle the section and drags drive reorder; the
                    // actions tail on the RIGHT is a child Ui with its
                    // own click handling so action-button clicks don't
                    // bubble up as section-toggle clicks. The two
                    // rects don't overlap, which keeps egui's hit
                    // priority predictable.
                    const HEADER_H: f32 = 22.0;
                    const CHEVRON_W: f32 = 16.0;
                    const ICON_W: f32 = 18.0;
                    let header_w = ui.available_width();
                    let (full_rect, _) = ui.allocate_exact_size(
                        egui::vec2(header_w, HEADER_H),
                        egui::Sense::hover(),
                    );

                    // No actions tail any more — the title strip
                    // claims the full header width.
                    let title_strip_rect = full_rect;

                    let resp = ui.interact(
                        title_strip_rect,
                        ui.id().with(("frost_section_title_strip", id_salt)),
                        egui::Sense::click_and_drag(),
                    );

                    // Theme-resolved title colour. When the title
                    // strip is filled with accent, override to the
                    // contrast colour against the accent so the
                    // bracketed title reads as dark text on a bright
                    // banner (the user explicitly asked for "accent
                    // background, dark colour, bold").
                    let title_col = if crate::style::theme().title_strip_filled {
                        crate::style::contrast_text_for(accent)
                    } else {
                        crate::style::section_title_color(accent)
                    };

                    let theme_now = crate::style::theme();

                    // Capture fold state for the deferred banner +
                    // bottom-corner-flip logic. `state.show_body_*`
                    // consumes `state` later so we can't read it
                    // afterwards.
                    let openness = state.openness(ui.ctx());
                    captured_openness = openness;

                    // Chevron — themes that opt out (GAME) skip both
                    // the paint AND its reserved horizontal slot, so
                    // the title shifts left and reclaims the space.
                    let mut text_x = title_strip_rect.min.x;
                    if theme_now.show_section_chevron {
                        let chevron_rect = egui::Rect::from_min_size(
                            title_strip_rect.min,
                            egui::vec2(CHEVRON_W, HEADER_H),
                        );
                        paint_chevron(ui, chevron_rect, openness, title_col);
                        text_x = chevron_rect.right() + 2.0;
                    }

                    // Build a mixed-font galley for the header so the
                    // optional Fluent icon sits *inside* the bracket
                    // pair next to the title — `[ ICON TITLE ]` — with
                    // no separator dot between icon and title. The
                    // icon needs the Fluent UI font family; the
                    // brackets and title use the default proportional
                    // font with the same caps + size + letter-spacing
                    // recipe `section_caps` would have produced.
                    //
                    // PRO (no brackets, with icon) renders the icon as
                    // a separate paint before the title text — same
                    // visual as before, just routed through the same
                    // composition path.
                    let title_size_pt = theme_now.section_title_size;
                    let default_font =
                        egui::FontId::new(title_size_pt, egui::FontFamily::Proportional);
                    let default_format = egui::TextFormat {
                        font_id: default_font.clone(),
                        color: title_col,
                        extra_letter_spacing: theme_now.section_title_letter_spacing,
                        ..Default::default()
                    };

                    let title_max_w = (title_strip_rect.max.x - text_x).max(0.0);
                    let mut job = egui::text::LayoutJob::default();
                    job.wrap.max_width = title_max_w;
                    job.wrap.max_rows = 1;
                    job.wrap.break_anywhere = true;

                    // Optional prefix glyph (only when brackets are
                    // OFF — when brackets are ON the prefix is dropped
                    // because the bracket pair is the visual anchor
                    // and chaining `▸ [ … ]` reads as cluttered).
                    if let (Some(prefix), false) =
                        (theme_now.section_title_prefix, theme_now.section_title_brackets)
                    {
                        job.append(prefix, 0.0, default_format.clone());
                        job.append(" ", 0.0, default_format.clone());
                    }

                    if theme_now.section_title_brackets {
                        job.append("[ ", 0.0, default_format.clone());
                    }

                    // Icon — embedded inline with the title text
                    // UNLESS the active theme requested it floats at
                    // the right edge instead. In that mode we skip
                    // the inline glyph entirely and paint it as a
                    // bigger separate icon after the galley renders.
                    if !theme_now.section_icon_at_end {
                        if let Some(name) = icon {
                            if let Some((glyph, family)) = crate::icons::icon(name) {
                                let icon_format = egui::TextFormat {
                                    font_id: egui::FontId::new(title_size_pt, family),
                                    color: title_col,
                                    ..Default::default()
                                };
                                job.append(&glyph.to_string(), 0.0, icon_format);
                                job.append(" ", 0.0, default_format.clone());
                            }
                        }
                    }

                    job.append(&title.to_uppercase(), 0.0, default_format.clone());

                    if theme_now.section_title_brackets {
                        job.append(" ]", 0.0, default_format.clone());
                    }

                    let title_galley = ui.painter().layout_job(job);
                    let title_pos = egui::pos2(
                        text_x,
                        title_strip_rect.center().y - title_galley.size().y * 0.5,
                    );
                    let title_size = title_galley.size();
                    ui.painter().galley(title_pos, title_galley, title_col);

                    // Floating right-edge icon. When the active theme
                    // sets `section_icon_at_end`, the inline icon
                    // section was skipped above; we now paint the
                    // icon as a bigger glyph at the right of the
                    // title strip. Reads as a "floating accent
                    // ornament" because the size (theme-controlled
                    // `section_icon_size`) is well above the title
                    // text size, and it's positioned with right-edge
                    // anchoring so it always hugs the strip's right
                    // side regardless of title length.
                    if theme_now.section_icon_at_end {
                        if let Some(name) = icon {
                            let size = theme_now.section_icon_size.max(0.0);
                            if size > 0.0 {
                                let icon_pos = egui::pos2(
                                    title_strip_rect.max.x - 6.0,
                                    title_strip_rect.center().y,
                                );
                                crate::icons::paint_icon(
                                    &ui.painter(),
                                    icon_pos,
                                    egui::Align2::RIGHT_CENTER,
                                    name,
                                    size,
                                    title_col,
                                );
                            }
                        }
                    }

                    // Trailing horizontal rule (DOOM Eternal /
                    // Helldivers / EVE pattern): from just past the
                    // title text to the actions tail's left edge. Uses
                    // the same dash recipe as row separators so the
                    // rule reads as part of the same machine-drawn
                    // family. Theme-gated via
                    // `section_title_trailing_rule`.
                    if crate::style::theme().section_title_trailing_rule {
                        let rule_y = title_strip_rect.center().y;
                        let rule_x_start = title_pos.x + title_size.x + 8.0;
                        // Stop the dashed rule before the right-edge
                        // floating icon so they don't visually
                        // collide. With no floating icon, the rule
                        // runs all the way to the strip edge as
                        // before.
                        let icon_reserve = if theme_now.section_icon_at_end && icon.is_some() {
                            theme_now.section_icon_size + 10.0
                        } else {
                            0.0
                        };
                        let rule_x_end = title_strip_rect.max.x - 4.0 - icon_reserve;
                        if rule_x_end > rule_x_start + 4.0 {
                            let alpha = crate::style::theme().row_separator_alpha.max(50);
                            let base = crate::style::theme().border_subtle;
                            let line_col = egui::Color32::from_rgba_unmultiplied(
                                base.r(),
                                base.g(),
                                base.b(),
                                alpha,
                            );
                            let stroke = egui::Stroke::new(1.0, line_col);
                            let p1 = egui::pos2(rule_x_start, rule_y);
                            let p2 = egui::pos2(rule_x_end, rule_y);
                            if let Some((on, off)) =
                                crate::style::theme().row_separator_dash
                            {
                                crate::style::paint_dashed_line(
                                    &ui.painter(),
                                    p1,
                                    p2,
                                    on,
                                    off,
                                    stroke,
                                );
                            } else {
                                ui.painter().line_segment([p1, p2], stroke);
                            }
                        }
                    }

                    if resp.clicked() {
                        state.toggle(ui);
                    }

                    captured_header_response = Some(resp);

                    state.show_body_unindented(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        if crate::style::section_show_title_divider() {
                            thin_divider(ui);
                            ui.add_space(6.0);
                        }
                        // Body indent — creates the "title at edge,
                        // body nested" hierarchy. The horizontal +
                        // vertical wrap forces a fixed-width
                        // left spacer in front of every body widget
                        // without disturbing each widget's own
                        // dual_pane / labelled_row layout.
                        //
                        // Row-zebra brackets — `begin_row_zebra` resets
                        // the per-section row counter so every section
                        // alternates from row 0; `commit_row_zebra`
                        // closes off the LAST row's deferred fill (the
                        // normal `flush_pending_separator`-driven
                        // resolve path only ever closes the *previous*
                        // row, so without an end-of-body flush the
                        // bottom row of every section would be left
                        // unstriped).
                        let indent = crate::style::theme().section_body_indent;
                        if indent > 0.0 {
                            ui.horizontal(|ui| {
                                ui.add_space(indent);
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 0.0;
                                    // Begin AFTER the indent wrap so the
                                    // owner_ui captured matches the ui
                                    // each direct row widget will be
                                    // called on.
                                    begin_row_zebra(ui);
                                    body(ui);
                                    commit_row_zebra(ui, accent);
                                });
                            });
                        } else {
                            begin_row_zebra(ui);
                            body(ui);
                            commit_row_zebra(ui, accent);
                        }
                    });
                },
            );
        });

    // Outer rect = the frame's *actual painted rect*, not the
    // post-frame cursor. After `frame.show`, `ui.cursor()` already
    // includes egui's `item_spacing.y` (~4–6 px), so anchoring the
    // bottom corners to the cursor placed them several pixels BELOW
    // the visible edge of the section card. The frame's own
    // `InnerResponse.response.rect` is exactly the painted area.
    let outer_rect = frame_inner.response.rect;
    let _ = (theme_outer_inset, inner_w); // referenced only inside body now

    // Now that we know `outer_rect`, set the banner placeholder. The
    // height interpolates between *full section* (folded) and
    // *title strip only* (open), driven by `captured_openness` —
    // when the section is folded the banner extends all the way to
    // the bottom of the card so the whole collapsed section is
    // accent-coloured; when fully open it covers only the top
    // strip + top-padding. Mid-animation, the banner shrinks
    // smoothly with the body's growth.
    let title_strip_h = (crate::style::theme().section_pad_y as f32) + 22.0;
    let mut banner_max_y = outer_rect.min.y + title_strip_h;
    if let Some(idx) = banner_idx {
        let target_h = egui::lerp(outer_rect.height()..=title_strip_h, captured_openness);
        let banner_rect = egui::Rect::from_min_size(
            outer_rect.min,
            egui::vec2(outer_rect.width(), target_h),
        );
        banner_max_y = banner_rect.max.y;
        let p = ui.painter().clone().with_clip_rect(banner_rect.expand(2.0));
        p.set(
            idx,
            egui::Shape::rect_filled(banner_rect, egui::CornerRadius::ZERO, accent),
        );
    }

    // Section bottom rule — dashed accent line along the bottom
    // edge after body finishes. Gives every section a "closes here"
    // boundary even though the theme paints no border. Y is snapped
    // so the line lands fully inside the section card (`max.y -
    // inset - 0.5` keeps the 1 px stroke crisp and inside the rect).
    if crate::style::theme().section_bottom_rule {
        let alpha = crate::style::theme().row_separator_alpha.max(50);
        let col = egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), alpha);
        let stroke = egui::Stroke::new(1.0, col);
        let inset = crate::style::theme().section_corner_ticks_inset;
        let y = (outer_rect.max.y - inset).round() - 0.5;
        let p1 = egui::pos2(outer_rect.min.x + inset, y);
        let p2 = egui::pos2(outer_rect.max.x - inset, y);
        if let Some((on, off)) = crate::style::theme().row_separator_dash {
            crate::style::paint_dashed_line(ui.painter(), p1, p2, on, off, stroke);
        } else {
            ui.painter().line_segment([p1, p2], stroke);
        }
    }

    // L-bracket corner ticks — every game-UI agent flagged this as
    // the highest-ROI HUD signal: four 1 px strokes per corner of
    // the section's outer rect, sized via `section_corner_ticks`
    // (PRO `0.0` → no-op). Tinted in accent at α 200 so they read as
    // accent-coloured ticks, not random strokes. The corners are
    // optionally inset by `section_corner_ticks_inset` px so they
    // sit *inside* the section rect — pairs with the inter-section
    // gap so each module reads as a self-contained bracketed card.
    let tick_len = crate::style::theme().section_corner_ticks;
    if tick_len > 0.0 {
        let inset = crate::style::theme().section_corner_ticks_inset;
        let r = if inset > 0.0 {
            outer_rect.shrink(inset)
        } else {
            outer_rect
        };
        // Pixel-snap. For a 1 px stroke to render crisp the line
        // *centre* must sit on half-integer coords. Crucially, left /
        // top edges snap with `+0.5` (line covers pixels at the
        // edge, just inside the rect) while right / bottom edges
        // snap with `-0.5` (also just inside) — without this, the
        // right & bottom strokes land OUTSIDE the rect by half a
        // pixel, which is exactly the "going outside few pixels"
        // bleed the user reported.
        let snap_low = |v: f32| v.round() + 0.5;
        let snap_high = |v: f32| v.round() - 0.5;
        let lx = snap_low(r.min.x);
        let ty = snap_low(r.min.y);
        let rx = snap_high(r.max.x);
        let by = snap_high(r.max.y);
        let len = tick_len;
        // Bottom corners flip to the contrast colour when the banner
        // has grown down to cover them (folded / mid-animation).
        // When the section is fully open the banner only covers the
        // top, so the bottom corners sit on the dark card and stay
        // accent-coloured — exactly what the user described.
        let accent_col =
            egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 220);
        let contrast_col = crate::style::contrast_text_for(accent);
        let top_col = if crate::style::theme().title_strip_filled {
            contrast_col
        } else {
            accent_col
        };
        // The bottom corners' Y is `by`; if the banner extends past
        // that Y, the corners are sitting on the banner.
        let bot_col = if crate::style::theme().title_strip_filled
            && banner_max_y >= by - 0.5
        {
            contrast_col
        } else {
            accent_col
        };
        let top_stroke = egui::Stroke::new(1.0, top_col);
        let bot_stroke = egui::Stroke::new(1.0, bot_col);
        let painter = ui.painter();
        // Top-left  ┌
        painter.line_segment(
            [egui::pos2(lx, ty), egui::pos2(lx + len, ty)],
            top_stroke,
        );
        painter.line_segment(
            [egui::pos2(lx, ty), egui::pos2(lx, ty + len)],
            top_stroke,
        );
        // Top-right ┐
        painter.line_segment(
            [egui::pos2(rx - len, ty), egui::pos2(rx, ty)],
            top_stroke,
        );
        painter.line_segment(
            [egui::pos2(rx, ty), egui::pos2(rx, ty + len)],
            top_stroke,
        );
        // Bottom-left └
        painter.line_segment(
            [egui::pos2(lx, by - len), egui::pos2(lx, by)],
            bot_stroke,
        );
        painter.line_segment(
            [egui::pos2(lx, by), egui::pos2(lx + len, by)],
            bot_stroke,
        );
        // Bottom-right ┘
        painter.line_segment(
            [egui::pos2(rx - len, by), egui::pos2(rx, by)],
            bot_stroke,
        );
        painter.line_segment(
            [egui::pos2(rx, by - len), egui::pos2(rx, by)],
            bot_stroke,
        );
    }

    SectionTrack {
        state_id,
        outer_rect,
        header_response: captured_header_response.expect("header always allocated"),
    }
}

/// Thin stroked chevron (`›` rotating to `⌄`) at `rect`, picking up
/// the size from the cell and the tint from the header. Replaces
/// the filled triangle that used to live here — modern UIs (Linear,
/// VS Code, Raycast) all use a stroked V because the negative space
/// inside the chevron makes "fold state" easier to read at a glance
/// than a solid fill.
///
/// Geometry: a 3-point polyline arranged as `⌄` when `openness = 1`
/// and rotated -90° (pointing right, `›`) when `openness = 0`. Egui
/// rounds the polyline join automatically when the stroke is thick
/// enough relative to the segment length, which gives the apex a
/// soft "bullet" tip rather than a sharp pixel-y corner.
fn paint_chevron(ui: &mut egui::Ui, rect: egui::Rect, openness: f32, tint: egui::Color32) {
    // Glyph bounds — slightly smaller than the cell so the chevron
    // reads as a typographic mark rather than a hit-area icon.
    const GLYPH_W: f32 = 8.0;
    const GLYPH_H: f32 = 5.0;
    let cx = rect.center().x;
    let cy = rect.center().y;

    // `⌄` open shape, centred at origin: arms at the top corners,
    // apex at bottom centre. We treat half-extents so the rotation
    // pivot stays at (0, 0).
    let hw = GLYPH_W * 0.5;
    let hh = GLYPH_H * 0.5;
    let raw = [
        egui::vec2(-hw, -hh), // top-left arm tip
        egui::vec2(0.0, hh),  // apex
        egui::vec2(hw, -hh),  // top-right arm tip
    ];

    // Rotate from -90° (closed → `›`) up to 0° (open → `⌄`).
    use std::f32::consts::TAU;
    let rot = egui::emath::Rot2::from_angle(egui::lerp(-TAU / 4.0..=0.0, openness));
    let pts: Vec<egui::Pos2> = raw
        .iter()
        .map(|v| {
            let r = rot * *v;
            egui::pos2(cx + r.x, cy + r.y)
        })
        .collect();

    // Stroke width 1.6 reads cleanly at 1× DPI without going chunky
    // on a 2× display. Egui's polyline tessellator rounds the join
    // at the apex when the segments are short relative to the
    // stroke — the chevron's GLYPH_W vs 1.6 ratio lands right in
    // that sweet spot.
    ui.painter()
        .add(egui::Shape::line(pts, egui::Stroke::new(1.6, tint)));
}
