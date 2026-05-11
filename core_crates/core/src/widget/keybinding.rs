//! Key-chip + action-label row, used in "Controls" / "Keys" help
//! sections. Renders a small monospace key chip on the left and the
//! action description on the right; the action truncates with `…`
//! when the row is too narrow.
//!
//! Sits at the same brightness tier as the search field / dropdown
//! trigger (`track_fill`), with text colour picked by `on_track`
//! so the chip stays readable across theme + accent combos.

use crate::style::{
    on_section_dim, on_track, theme, track_fill,
};

/// Canonical key-row height. One U so a `Pod::with_keybindings`
/// row matches the rhythm of every other 1U widget.
pub const KEYBINDING_ROW_H: f32 = crate::style::UNIT;

/// Inner horizontal padding in the key chip.
const KEY_CHIP_PAD_X: f32 = 5.0;
/// Vertical padding in the key chip.
const KEY_CHIP_PAD_Y: f32 = 1.0;
/// Gap between key chip and action label.
const KEY_TO_ACTION_GAP: f32 = 8.0;

/// Render a single keybinding row: `[keys]  action description`.
pub fn keybinding_row(ui: &mut egui::Ui, keys: &str, action: &str) -> egui::Response {
    keybinding_row_h(ui, keys, action, KEYBINDING_ROW_H)
}

/// Variable-height variant — caller fixes the row height (used by
/// `Pod::with_keybindings` so all rows in a list share the same
/// metric).
pub fn keybinding_row_h(
    ui: &mut egui::Ui,
    keys: &str,
    action: &str,
    height: f32,
) -> egui::Response {
    let avail_w = ui.available_width();
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(avail_w, height), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let painter = ui.painter_at(rect);
    let accent = ui.visuals().selection.stroke.color;
    let mid_y = rect.center().y;

    // ── Key chip ──
    let key_font = egui::FontId::monospace(11.0);
    let key_galley = {
        let mut job = egui::text::LayoutJob::single_section(
            keys.to_string(),
            egui::TextFormat::simple(key_font, on_track()),
        );
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        painter.layout_job(job)
    };
    let key_text_w = key_galley.size().x.ceil();
    let key_text_h = key_galley.size().y.ceil();
    let chip_w = key_text_w + KEY_CHIP_PAD_X * 2.0;
    let chip_h = key_text_h + KEY_CHIP_PAD_Y * 2.0;
    let chip_rect = egui::Rect::from_min_size(
        egui::pos2(rect.min.x, mid_y - chip_h * 0.5),
        egui::vec2(chip_w, chip_h),
    );
    painter.rect_filled(
        chip_rect,
        egui::CornerRadius::same(theme().radius_widget),
        track_fill(accent),
    );
    painter.galley(
        egui::pos2(chip_rect.min.x + KEY_CHIP_PAD_X, mid_y - key_text_h * 0.5),
        key_galley,
        on_track(),
    );

    // ── Action label (truncating) ──
    let action_x = chip_rect.max.x + KEY_TO_ACTION_GAP;
    let action_max_w = (rect.max.x - action_x).max(0.0);
    if action_max_w > 0.0 {
        let action_font = egui::FontId::proportional(11.0);
        let mut job = egui::text::LayoutJob::single_section(
            action.to_string(),
            egui::TextFormat::simple(action_font, on_section_dim()),
        );
        job.wrap.max_rows = 1;
        job.wrap.max_width = action_max_w;
        job.wrap.break_anywhere = true;
        let action_galley = painter.layout_job(job);
        painter.galley(
            egui::pos2(action_x, mid_y - action_galley.size().y * 0.5),
            action_galley,
            on_section_dim(),
        );
    }
    resp
}
