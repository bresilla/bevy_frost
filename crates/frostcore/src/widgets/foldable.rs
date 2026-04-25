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

use super::shared::flush_pending_separator;

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
    let inner_w = (full_w - OUTER_INSET).max(0.0);
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
    frame.show(ui, |ui| {
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

                    let openness = state.openness(ui.ctx());
                    let chevron_rect = egui::Rect::from_min_size(
                        title_strip_rect.min,
                        egui::vec2(CHEVRON_W, HEADER_H),
                    );
                    paint_chevron(ui, chevron_rect, openness, title_col);

                    // Optional icon between chevron and title.
                    let mut text_x = chevron_rect.right() + 2.0;
                    if let Some(name) = icon {
                        let icon_rect = egui::Rect::from_min_size(
                            egui::pos2(text_x, title_strip_rect.min.y),
                            egui::vec2(ICON_W, HEADER_H),
                        );
                        crate::icons::paint_icon(
                            &ui.painter(),
                            icon_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            name,
                            ICON_W - 4.0,
                            title_col,
                        );
                        text_x = icon_rect.right() + 4.0;
                    }

                    // Render the header text via the `section_caps`
                    // RichText recipe (uppercase + strong) at the
                    // theme-resolved title colour. Wrap the title at
                    // the title-strip width so a long title doesn't
                    // overrun into the actions tail.
                    let title_max_w = (title_strip_rect.max.x - text_x).max(0.0);
                    let title_widget =
                        egui::WidgetText::from(section_caps(title, title_col));
                    let title_galley = title_widget.into_galley(
                        ui,
                        Some(egui::TextWrapMode::Truncate),
                        title_max_w,
                        egui::TextStyle::Body,
                    );
                    let title_pos = egui::pos2(
                        text_x,
                        title_strip_rect.center().y - title_galley.size().y * 0.5,
                    );
                    ui.painter().galley(title_pos, title_galley, title_col);

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
                        let indent = crate::style::theme().section_body_indent;
                        if indent > 0.0 {
                            ui.horizontal(|ui| {
                                ui.add_space(indent);
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 0.0;
                                    body(ui);
                                });
                            });
                        } else {
                            body(ui);
                        }
                    });
                },
            );
        });

    let outer_bottom = ui.cursor().min.y;
    let outer_rect = egui::Rect::from_min_max(
        outer_top,
        egui::pos2(outer_top.x + full_w, outer_bottom),
    );
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
