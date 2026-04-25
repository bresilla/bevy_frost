//! Buttons — wide primary and card (glyph + title + subtitle)
//! variants. Both share a single accent-tinted glass background
//! recipe so they read as one family.

use egui;

use crate::style::{glass_alpha_card, widget_border};

use super::shared::{flush_pending_separator, lerp_color, widget_separator};

// ─── Wide button ────────────────────────────────────────────────────

/// A chunky primary-looking button that fills the available row
/// width. Carries a subtle accent tint at rest (≈ 8 % of accent
/// over the raised panel colour), brightens on hover / press, and
/// paints an accent border on hover — so the user's eye can tell
/// it's interactive at a glance without the button screaming for
/// attention.
pub fn wide_button(ui: &mut egui::Ui, label: &str, accent: egui::Color32) -> egui::Response {
    flush_pending_separator(ui);
    const ROW_H: f32 = 24.0;
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, ROW_H), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let bg_col = paint_accent_bg(ui, rect, accent, &resp);
        // Pick a contrasting label colour for whichever fill the
        // button ended up with (accent under a GAME press, glass
        // tint otherwise).
        ui.painter_at(rect).text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(12.0),
            crate::style::contrast_text_for(bg_col),
        );
    }
    widget_separator(ui);
    resp
}

// ─── Card button ────────────────────────────────────────────────────

/// Full-width preset card — accent glyph on the left, primary
/// `name` + small `subtitle` stacked on the right. Reads like UE5's
/// "Create" entries.
pub fn card_button(
    ui: &mut egui::Ui,
    glyph: &str,
    name: &str,
    subtitle: &str,
    accent: egui::Color32,
) -> egui::Response {
    flush_pending_separator(ui);
    // Reserving the SAME blank space on the right as the glyph
    // column consumes on the left keeps the text optically centred
    // and leaves a clean "runway" for the ellipsis when either line
    // is too long — rather than text bleeding into the rounded
    // corner.
    const ROW_H: f32 = 32.0;
    const EDGE_PAD: f32 = 8.0; // from card edge to content
    const GLYPH_COL: f32 = 14.0; // glyph bbox from start of content
    const GLYPH_GAP: f32 = 8.0; // glyph-to-text gap
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, ROW_H), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        widget_separator(ui);
        return resp;
    }

    let bg_col = paint_accent_bg(ui, rect, accent, &resp);
    let primary_col = crate::style::contrast_text_for(bg_col);
    // Subtitle = primary blended 40 % toward bg → secondary
    // hierarchy that always contrasts.
    let secondary_col = {
        let f = 0.4_f32;
        let lerp = |a: u8, b: u8| ((a as f32) * (1.0 - f) + (b as f32) * f).round() as u8;
        egui::Color32::from_rgb(
            lerp(primary_col.r(), bg_col.r()),
            lerp(primary_col.g(), bg_col.g()),
            lerp(primary_col.b(), bg_col.b()),
        )
    };

    let painter = ui.painter_at(rect);

    // Glyph pinned to the left.
    let glyph_x = rect.min.x + EDGE_PAD + GLYPH_COL * 0.5;
    painter.text(
        egui::pos2(glyph_x, rect.center().y),
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::FontId::proportional(14.0),
        accent,
    );

    // Text column starts past the glyph, ends at a mirrored gutter
    // on the right.
    let text_left = rect.min.x + EDGE_PAD + GLYPH_COL + GLYPH_GAP;
    let text_right = rect.max.x - (EDGE_PAD + GLYPH_COL + GLYPH_GAP);
    let max_w = (text_right - text_left).max(0.0);

    let name_galley = elided_galley(ui, name, egui::FontId::proportional(12.0), primary_col, max_w);
    let subtitle_galley = elided_galley(
        ui,
        subtitle,
        egui::FontId::proportional(10.0),
        secondary_col,
        max_w,
    );

    let center_y = rect.center().y;
    let name_pos = egui::pos2(text_left, center_y - 6.0 - name_galley.size().y * 0.5);
    let subtitle_pos = egui::pos2(text_left, center_y + 7.0 - subtitle_galley.size().y * 0.5);
    painter.galley(name_pos, name_galley, primary_col);
    painter.galley(subtitle_pos, subtitle_galley, secondary_col);

    widget_separator(ui);
    resp
}

// ─── Shared paint helpers ───────────────────────────────────────────

/// Paint a rounded-rect button background with a tiny accent tint.
/// Shared by `card_button` + `wide_button` so they read as one
/// family.
///
/// Press visual depends on the active theme:
/// - PRO (`button_full_accent_on_press = false`) — pressed buttons
///   show a `button_tint_press` accent lerp over the panel colour.
/// - GAME (`button_full_accent_on_press = true`) — pressed buttons
///   fill solid with `accent`, no halftone.
///
/// All three tint fractions (rest / hover / press) come from the
/// active theme so a custom profile can dial them.
fn paint_accent_bg(
    ui: &egui::Ui,
    rect: egui::Rect,
    accent: egui::Color32,
    resp: &egui::Response,
) -> egui::Color32 {
    let th = crate::style::theme();
    let pressed = resp.is_pointer_button_down_on();

    let bg = if pressed && th.button_full_accent_on_press {
        accent
    } else {
        let tint = if pressed {
            th.button_tint_press
        } else if resp.hovered() {
            th.button_tint_hover
        } else {
            th.button_tint_rest
        };
        // Base = whichever surface the button is sitting on
        // (section card if framed, pane otherwise) so the tint sits
        // on the right colour family in both PRO and GAME.
        let base = if th.section_show_frame {
            crate::style::section_fill(accent)
        } else {
            crate::style::pane_fill(accent)
        };
        let solid = lerp_color(base, accent, tint);
        egui::Color32::from_rgba_unmultiplied(
            solid.r(),
            solid.g(),
            solid.b(),
            glass_alpha_card(),
        )
    };
    let border_col = if resp.hovered() { accent } else { widget_border(accent) };
    ui.painter_at(rect).rect(
        rect,
        egui::CornerRadius::same(th.radius_widget),
        bg,
        egui::Stroke::new(th.border_width, border_col),
        egui::StrokeKind::Inside,
    );
    bg
}

/// Lay out `text` onto a single row, truncated with `…` when it
/// would otherwise exceed `max_w`. Matches egui's own
/// `Label::truncate()` wrap settings.
fn elided_galley(
    ui: &egui::Ui,
    text: &str,
    font: egui::FontId,
    color: egui::Color32,
    max_w: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::single_section(
        text.to_string(),
        egui::TextFormat::simple(font, color),
    );
    job.wrap.max_width = max_w;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.halign = egui::Align::LEFT;
    ui.painter().layout_job(job)
}
