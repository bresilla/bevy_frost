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
    let _ = section_tracked(
        ui,
        id_salt,
        title,
        accent,
        default_open,
        None,
        0,
        |_| {},
        body,
    );
}

/// Width allocated for one header-action chip plus the spacing
/// between chips. Chips are `HEADER_ACTION_SIZE` wide, separated by
/// `HEADER_ACTION_GAP`. `header_actions_width` resolves a chip count
/// to a tail width; `header_action_size` is exported for callers
/// that paint custom action chips so they match the reserved cell.
pub const HEADER_ACTION_SIZE: f32 = 18.0;
pub const HEADER_ACTION_GAP: f32 = 2.0;

/// Tail width reserved for `count` action chips, including a small
/// trailing gutter so the rightmost chip doesn't kiss the section's
/// inner border.
pub fn header_actions_width(count: u8) -> f32 {
    if count == 0 {
        0.0
    } else {
        let n = count as f32;
        n * HEADER_ACTION_SIZE + (n - 1.0).max(0.0) * HEADER_ACTION_GAP + 6.0
    }
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
    action_count: u8,
    actions: impl FnOnce(&mut egui::Ui),
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
                    let actions_w = header_actions_width(action_count);
                    let (full_rect, _) = ui.allocate_exact_size(
                        egui::vec2(header_w, HEADER_H),
                        egui::Sense::hover(),
                    );

                    let title_strip_rect = egui::Rect::from_min_max(
                        full_rect.min,
                        egui::pos2(full_rect.max.x - actions_w, full_rect.max.y),
                    );
                    let actions_rect = egui::Rect::from_min_max(
                        egui::pos2(full_rect.max.x - actions_w, full_rect.min.y),
                        full_rect.max,
                    );

                    let resp = ui.interact(
                        title_strip_rect,
                        ui.id().with(("frost_section_title_strip", id_salt)),
                        egui::Sense::click_and_drag(),
                    );

                    // Theme-resolved title colour: PRO → accent;
                    // GAME → contrast-against-panel (so titles read
                    // dark against a bright accent panel).
                    let title_col = crate::style::section_title_color(accent);

                    let theme_now = crate::style::theme();

                    // Chevron — themes that opt out (GAME) skip both
                    // the paint AND its reserved horizontal slot, so
                    // the title shifts left and reclaims the space.
                    let mut text_x = title_strip_rect.min.x;
                    if theme_now.show_section_chevron {
                        let openness = state.openness(ui.ctx());
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
                    let title_size_pt = 12.0 * 1.15;
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

                    // Icon — embedded in the same galley using the
                    // Fluent font family so it sits inline with the
                    // bracketed title text, no separator dot.
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
                        let rule_x_end = title_strip_rect.max.x - 4.0;
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

                    // Header actions tail. Right-to-left layout so the
                    // closure can call `actions_button(...)` repeatedly
                    // and have each chip stack from the right edge
                    // inward. A no-op closure with action_count = 0
                    // collapses the tail to zero width.
                    if action_count > 0 {
                        let mut action_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(actions_rect)
                                .layout(egui::Layout::right_to_left(egui::Align::Center)),
                        );
                        action_ui.spacing_mut().item_spacing =
                            egui::vec2(HEADER_ACTION_GAP, 0.0);
                        actions(&mut action_ui);
                    }

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
        let col = egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 220);
        let stroke = egui::Stroke::new(1.0, col);
        let painter = ui.painter();
        // Top-left  ┌
        painter.line_segment(
            [egui::pos2(lx, ty), egui::pos2(lx + len, ty)],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(lx, ty), egui::pos2(lx, ty + len)],
            stroke,
        );
        // Top-right ┐
        painter.line_segment(
            [egui::pos2(rx - len, ty), egui::pos2(rx, ty)],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(rx, ty), egui::pos2(rx, ty + len)],
            stroke,
        );
        // Bottom-left └
        painter.line_segment(
            [egui::pos2(lx, by - len), egui::pos2(lx, by)],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(lx, by), egui::pos2(lx + len, by)],
            stroke,
        );
        // Bottom-right ┘
        painter.line_segment(
            [egui::pos2(rx - len, by), egui::pos2(rx, by)],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(rx, by - len), egui::pos2(rx, by)],
            stroke,
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
