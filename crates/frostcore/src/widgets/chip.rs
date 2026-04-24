//! Compact pill / tag for inline status labels.
//!
//! Use these for per-row feature flags (`anim`, `var`, `inst`, …),
//! category labels in info panels, or any "small chunk of text
//! that belongs next to another thing". They sit inline — call
//! from inside `ui.horizontal(|ui| { ... })` to chain them.
//!
//! Visual:
//!
//! ```text
//!   Planet  [anim] [inst]      <- chips after a label
//! ```
//!
//! Two variants ship:
//!
//! * [`chip`] — faint accent-tinted fill + accent-tinted border.
//!   The default, neutral "there's a property here" chip.
//! * [`chip_colored`] — caller supplies a fill colour (e.g.
//!   [`SUCCESS`](crate::style::SUCCESS) /
//!   [`WARNING`](crate::style::WARNING) /
//!   [`DANGER`](crate::style::DANGER)) for chips that categorise
//!   (status, severity). Border still uses `widget_border(accent)`
//!   so the family remains coherent.

use crate::style::{font, glass_alpha_group, glass_fill, radius, widget_border, BG_3_HOVER, TEXT_PRIMARY};

/// Total chip height, in px. Matches the tree-row rhythm so chips
/// sit visually aligned with tree-row labels.
const CHIP_H: f32 = 16.0;
/// Horizontal padding inside the chip, between the border and the
/// text.
const CHIP_PAD_X: f32 = 6.0;

/// Compact pill with a faint tinted fill + accent-tinted border.
/// Text is at [`font::CAPTION`] size. Returns the `Response` so
/// callers can react to clicks / hover (e.g. tooltips).
pub fn chip(ui: &mut egui::Ui, label: &str, accent: egui::Color32) -> egui::Response {
    let fill = glass_fill(BG_3_HOVER, accent, glass_alpha_group());
    chip_colored(ui, label, fill, accent)
}

/// Chip with an explicit fill colour. Useful for categorisation —
/// red for errors, green for OK, etc. Callers supply a
/// pre-computed [`egui::Color32`]; pass any alpha you like (the
/// border stroke uses the standard frost recipe on top).
pub fn chip_colored(
    ui: &mut egui::Ui,
    label: &str,
    fill: egui::Color32,
    accent: egui::Color32,
) -> egui::Response {
    // Lay the text out first so we can size the chip to it. Single
    // row, no wrap.
    let font = egui::FontId::proportional(font::CAPTION);
    let galley = {
        let mut job = egui::text::LayoutJob::single_section(
            label.to_string(),
            egui::TextFormat::simple(font, TEXT_PRIMARY),
        );
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        ui.painter().layout_job(job)
    };
    let text_w = galley.size().x.ceil();
    let size = egui::vec2(text_w + CHIP_PAD_X * 2.0, CHIP_H);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        ui.painter().rect(
            rect,
            egui::CornerRadius::same(radius::WIDGET),
            fill,
            egui::Stroke::new(1.0, widget_border(accent)),
            egui::StrokeKind::Inside,
        );
        ui.painter().galley(
            egui::pos2(
                rect.min.x + CHIP_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            TEXT_PRIMARY,
        );
    }
    resp
}
