//! Buttons — wide primary and card (glyph + title + subtitle)
//! variants. Both share a single accent-tinted glass background
//! recipe so they read as one family.

use egui;

use crate::style::{
    glass_alpha_card, radius, widget_border, BG_2_RAISED, TEXT_PRIMARY, TEXT_SECONDARY,
};

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
        paint_accent_bg(ui, rect, accent, &resp);
        ui.painter_at(rect).text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(12.0),
            TEXT_PRIMARY,
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

    paint_accent_bg(ui, rect, accent, &resp);

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

    let name_galley = elided_galley(ui, name, egui::FontId::proportional(12.0), TEXT_PRIMARY, max_w);
    let subtitle_galley = elided_galley(
        ui,
        subtitle,
        egui::FontId::proportional(10.0),
        TEXT_SECONDARY,
        max_w,
    );

    let center_y = rect.center().y;
    let name_pos = egui::pos2(text_left, center_y - 6.0 - name_galley.size().y * 0.5);
    let subtitle_pos = egui::pos2(text_left, center_y + 7.0 - subtitle_galley.size().y * 0.5);
    painter.galley(name_pos, name_galley, TEXT_PRIMARY);
    painter.galley(subtitle_pos, subtitle_galley, TEXT_SECONDARY);

    widget_separator(ui);
    resp
}

// ─── Shared paint helpers ───────────────────────────────────────────

/// Paint a rounded-rect button background with a tiny accent tint.
/// Shared by `card_button` + `wide_button` so they read as one family.
fn paint_accent_bg(
    ui: &egui::Ui,
    rect: egui::Rect,
    accent: egui::Color32,
    resp: &egui::Response,
) {
    let tint = if resp.is_pointer_button_down_on() {
        0.30
    } else if resp.hovered() {
        0.16
    } else {
        0.08
    };
    // Preserve the glass alpha so buttons blend into the panel the
    // same way card frames do. Unmultiplied so low alphas read as
    // "mostly scene + tiny surface tint" rather than "gray block
    // with partial transparency added on top".
    let solid = lerp_color(BG_2_RAISED, accent, tint);
    let bg = egui::Color32::from_rgba_unmultiplied(
        solid.r(),
        solid.g(),
        solid.b(),
        glass_alpha_card(),
    );
    let border_col = if resp.hovered() { accent } else { widget_border(accent) };
    ui.painter_at(rect).rect(
        rect,
        egui::CornerRadius::same(radius::WIDGET),
        bg,
        egui::Stroke::new(1.0, border_col),
        egui::StrokeKind::Inside,
    );
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
