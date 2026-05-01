//! Frost-styled button — fills the available row width, height
//! [`crate::style::UNIT`] (1U). Accent-tinted glass background that
//! brightens on hover / press.
//!
//! Theme behaviour matches the rest of the kit:
//!
//! * **PRO** — pressed buttons fill with `button_tint_press`-lerped
//!   accent over the panel surface.
//! * **GAME** (`button_full_accent_on_press = true`) — pressed
//!   buttons fill solid with the body accent.
//! * Tint fractions (rest / hover / press) all come from the theme,
//!   so a third profile dialed via the `Theme` literal gets matching
//!   button feedback for free.
//!
//! Compared to the older `frostcore::widgets::button::wide_button`,
//! the corekit version drops the press-depress animation and the
//! click-pulse for now — they can come back as a polish pass.

use crate::style::{
    body_accent, contrast_text_for, glass_alpha_card, pane_fill, section_fill,
    section_show_frame, surface_lift_target, theme, widget_border,
};

/// Default button row height (matches
/// `frostcore::widgets::button::wide_button`'s `ROW_H = 24`). A
/// touch taller than 1U so the button reads as chunky/primary —
/// the rest of the kit's widgets are 1U; buttons stand out.
pub const BUTTON_ROW_H: f32 = 24.0;
/// Centred label font size (matches `frostcore`'s wide_button).
pub const BUTTON_LABEL_FONT: f32 = 12.0;

/// Render a button at the default [`BUTTON_ROW_H`] height.
/// `label` is the centre-aligned caption. Returns the egui
/// `Response` so callers can check `clicked()`, `hovered()`, etc.
pub fn button(ui: &mut egui::Ui, label: &str, accent: egui::Color32) -> egui::Response {
    button_h(ui, label, accent, BUTTON_ROW_H)
}

/// Variable-height variant — used by resizable pods. `height` is
/// the button's flow-axis size. Cross axis fills available width.
pub fn button_h(
    ui: &mut egui::Ui,
    label: &str,
    accent: egui::Color32,
    height: f32,
) -> egui::Response {
    let w = ui.available_width();
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(w, height), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let bg = paint_accent_bg(ui, rect, accent, &resp);
    let label_col = contrast_text_for(bg);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(BUTTON_LABEL_FONT),
        label_col,
    );
    resp
}

/// Paint the button's accent-tinted glass background. Picks fill
/// + border + corner radius from the active theme so PRO and GAME
/// look right without per-callsite tweaking.
fn paint_accent_bg(
    ui: &egui::Ui,
    rect: egui::Rect,
    accent: egui::Color32,
    resp: &egui::Response,
) -> egui::Color32 {
    let th = theme();
    let pressed = resp.is_pointer_button_down_on();
    let body_acc = body_accent(accent);
    let bg = if pressed && th.button_full_accent_on_press {
        body_acc
    } else {
        let tint = if pressed {
            th.button_tint_press
        } else if resp.hovered() {
            th.button_tint_hover
        } else {
            th.button_tint_rest
        };
        let base = if section_show_frame() {
            section_fill(accent)
        } else {
            pane_fill(accent)
        };
        let target = surface_lift_target(body_acc);
        let solid = lerp_col(base, target, tint);
        egui::Color32::from_rgba_unmultiplied(
            solid.r(),
            solid.g(),
            solid.b(),
            glass_alpha_card(),
        )
    };
    let border_col = if resp.hovered() {
        accent
    } else {
        widget_border(accent)
    };
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(th.radius_widget),
        bg,
        egui::Stroke::new(th.border_width, border_col),
        egui::epaint::StrokeKind::Inside,
    );
    bg
}

/// Linear interpolation in straight-RGB space. Used by the button
/// fill to lerp between the panel base and the accent target;
/// matches `frostcore::widgets::shared::lerp_color`.
fn lerp_col(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let blend = |x: u8, y: u8| ((x as f32) * (1.0 - t) + (y as f32) * t).round() as u8;
    egui::Color32::from_rgb(blend(a.r(), b.r()), blend(a.g(), b.g()), blend(a.b(), b.b()))
}

// ─── Card button ───────────────────────────────────────────────────

/// Card button row height. Matches `frostcore`'s `card_button`
/// `ROW_H = 32` — chunky enough to fit the two text rows + glyph.
pub const CARD_BUTTON_ROW_H: f32 = 32.0;
/// Glyph font size in card buttons.
const CARD_GLYPH_FONT: f32 = 14.0;
/// Primary `name` line font size.
const CARD_NAME_FONT: f32 = 12.0;
/// Secondary `subtitle` line font size.
const CARD_SUBTITLE_FONT: f32 = 10.0;

/// Full-width preset card — accent glyph on the left, primary
/// `name` + small `subtitle` stacked on the right. Reads like UE5's
/// "Create" entries; matches `frostcore::widgets::button::card_button`.
pub fn card_button(
    ui: &mut egui::Ui,
    glyph: &str,
    name: &str,
    subtitle: &str,
    accent: egui::Color32,
) -> egui::Response {
    const EDGE_PAD: f32 = 8.0;
    const GLYPH_COL: f32 = 14.0;
    const GLYPH_GAP: f32 = 8.0;
    let w = ui.available_width();
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(w, CARD_BUTTON_ROW_H), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let bg = paint_accent_bg(ui, rect, accent, &resp);
    let primary = contrast_text_for(bg);
    let secondary = {
        let f = 0.4_f32;
        let bl = |a: u8, b: u8| ((a as f32) * (1.0 - f) + (b as f32) * f).round() as u8;
        egui::Color32::from_rgb(
            bl(primary.r(), bg.r()),
            bl(primary.g(), bg.g()),
            bl(primary.b(), bg.b()),
        )
    };
    let painter = ui.painter_at(rect);
    // Glyph pinned to the left.
    painter.text(
        egui::pos2(rect.min.x + EDGE_PAD + GLYPH_COL * 0.5, rect.center().y),
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::FontId::proportional(CARD_GLYPH_FONT),
        accent,
    );
    let text_left = rect.min.x + EDGE_PAD + GLYPH_COL + GLYPH_GAP;
    let text_right = rect.max.x - (EDGE_PAD + GLYPH_COL + GLYPH_GAP);
    let max_w = (text_right - text_left).max(0.0);
    let name_galley = elided_galley(
        ui,
        name,
        egui::FontId::proportional(CARD_NAME_FONT),
        primary,
        max_w,
    );
    let sub_galley = elided_galley(
        ui,
        subtitle,
        egui::FontId::proportional(CARD_SUBTITLE_FONT),
        secondary,
        max_w,
    );
    let cy = rect.center().y;
    painter.galley(
        egui::pos2(text_left, cy - 6.0 - name_galley.size().y * 0.5),
        name_galley,
        primary,
    );
    painter.galley(
        egui::pos2(text_left, cy + 7.0 - sub_galley.size().y * 0.5),
        sub_galley,
        secondary,
    );
    resp
}

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
