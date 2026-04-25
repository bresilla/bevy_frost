//! Frost-styled single-line search field.
//!
//! Shape:
//!
//! ```text
//!   🔍  query text…                    ✕
//!   └── leading glyph       trailing clear button
//! ```
//!
//! A thin wrapper around `egui::TextEdit::singleline` with:
//!
//! * A leading magnifier glyph painted inside the field (caller
//!   supplies `placeholder` text shown when the buffer is empty).
//! * A trailing `✕` glyph that clears the buffer when clicked.
//! * Accent-tinted border using the same [`widget_border`] recipe
//!   every other frost input wears.
//! * Height = 20 px, so it sits flush with a pane title row or a
//!   section header.
//!
//! Returns the `TextEdit`'s `Response`; call `.changed()` to react
//! to each keystroke (the clear button also marks the response as
//! changed when clicked).

use egui;

use crate::style::widget_border;

use super::shared::flush_pending_separator;

/// Total field height. Matches the shared `interact_size.y`
/// (20 px) so the search field aligns with every other frost
/// row-height control (toggle, slider, dropdown).
const SEARCH_H: f32 = 20.0;
/// Width of the leading / trailing glyph columns.
const GLYPH_W: f32 = 18.0;
/// Padding between the text and the glyph columns.
const TEXT_PAD: f32 = 4.0;

/// Render a search field bound to `text`. Pass `placeholder` to
/// show a dim hint when the buffer is empty. Returns the
/// `egui::Response` from the inner `TextEdit` — inspect
/// `.changed()` to debounce / run the search. When the user
/// clicks the trailing `✕`, the buffer is cleared and the
/// response is marked changed.
pub fn search_field(
    ui: &mut egui::Ui,
    text: &mut String,
    placeholder: &str,
    accent: egui::Color32,
) -> egui::Response {
    flush_pending_separator(ui);
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(w, SEARCH_H),
        egui::Sense::hover(),
    );

    // Background + border — single accent-tinted glass surface,
    // same recipe a dropdown trigger / DragValue input would use.
    if ui.is_rect_visible(rect) {
        ui.painter().rect(
            rect,
            egui::CornerRadius::same(crate::style::theme().radius_widget),
            crate::style::track_fill(accent),
            egui::Stroke::new(crate::style::theme().border_width, widget_border(accent)),
            egui::StrokeKind::Inside,
        );
    }

    // Leading magnifier — filled Fluent UI search icon, falls back
    // to a plain emoji glyph if the font lookup ever fails.
    let mid_y = rect.center().y;
    let glyph_pos = egui::pos2(rect.min.x + GLYPH_W * 0.5, mid_y);
    let glyph_color = crate::style::on_track_dim();
    crate::icons::paint_icon(
        ui.painter(),
        glyph_pos,
        egui::Align2::CENTER_CENTER,
        "search",
        13.0,
        glyph_color,
    );

    // Trailing clear (✕) hit-rect + glyph. Only visible /
    // clickable when the buffer is non-empty, so an empty field
    // doesn't show a dead button.
    let clear_rect = egui::Rect::from_min_size(
        egui::pos2(rect.max.x - GLYPH_W, rect.min.y),
        egui::vec2(GLYPH_W, SEARCH_H),
    );
    let mut cleared = false;
    if !text.is_empty() {
        let clear_resp = ui
            .interact(
                clear_rect,
                ui.id().with("frost_search_clear"),
                egui::Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let color = if clear_resp.hovered() {
            accent
        } else {
            crate::style::on_track_dim()
        };
        crate::icons::paint_icon(
            ui.painter(),
            clear_rect.center(),
            egui::Align2::CENTER_CENTER,
            "dismiss",
            13.0,
            color,
        );
        if clear_resp.clicked() {
            text.clear();
            cleared = true;
        }
    }

    // Inner text-edit rect — carved out of the full rect minus
    // the two glyph columns and their padding.
    let text_rect = egui::Rect::from_min_max(
        egui::pos2(rect.min.x + GLYPH_W + TEXT_PAD, rect.min.y),
        egui::pos2(rect.max.x - GLYPH_W - TEXT_PAD, rect.max.y),
    );
    // Position the TextEdit at `text_rect` using a child Ui —
    // saves having to wrap the whole field in a horizontal
    // layout.
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(text_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let mut resp = child.add(
        egui::TextEdit::singleline(text)
            .desired_width(text_rect.width())
            .frame(false)
            .background_color(egui::Color32::TRANSPARENT)
            .hint_text(placeholder),
    );
    if cleared {
        // Surfaces the programmatic clear to callers that only
        // look at `.changed()` to drive their filter.
        resp.mark_changed();
    }
    resp
}
