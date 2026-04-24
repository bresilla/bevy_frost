//! Shared layout constants + the single paint helper every ribbon
//! button renders through. Internal to the `ribbon` module; the
//! static ribbons and the drag-aware layout both route here so the
//! pixel-level look stays identical whichever path the caller took.

use egui;

use crate::style::{
    glass_alpha_card, glass_alpha_window, glass_fill, BG_1_PANEL, BG_2_RAISED, BORDER_SUBTLE,
    TEXT_PRIMARY, TEXT_SECONDARY,
};

// ─── Layout constants ───────────────────────────────────────────────

/// Edge length of each square ribbon button (VS Code / Fleet size).
pub const SIDE_BTN_SIZE: f32 = 34.0;
/// Gap between adjacent ribbon buttons.
pub const SIDE_BTN_GAP: f32 = 4.0;
/// Distance from the screen edge to the near edge of each button.
pub const EDGE_GAP: f32 = 8.0;

// ─── Paint ──────────────────────────────────────────────────────────

/// Shared background / border recipe for every ribbon button. Same
/// glass look as the main panels — [`BG_1_PANEL`] idle, lifts to
/// [`BG_2_RAISED`] on hover, 25 % accent blend + accent stroke when
/// active.
pub(crate) fn paint_ribbon_button(
    painter: &egui::Painter,
    rect: egui::Rect,
    accent: egui::Color32,
    is_active: bool,
    hovered: bool,
) {
    let bg = if is_active {
        let blend = |a: u8, b: u8| ((a as f32) * 0.75 + (b as f32) * 0.25).round() as u8;
        let tinted = egui::Color32::from_rgb(
            blend(BG_2_RAISED.r(), accent.r()),
            blend(BG_2_RAISED.g(), accent.g()),
            blend(BG_2_RAISED.b(), accent.b()),
        );
        glass_fill(tinted, accent, glass_alpha_window())
    } else if hovered {
        glass_fill(BG_2_RAISED, accent, glass_alpha_window())
    } else {
        glass_fill(BG_1_PANEL, accent, glass_alpha_window())
    };
    let stroke = if is_active { accent } else { BORDER_SUBTLE };
    // ONE `rect` call so the stroke and fill share the same rounded-
    // corner tessellation. `StrokeKind::Inside` keeps the border
    // flush with the edge.
    painter.rect(
        rect,
        egui::CornerRadius::same(6),
        bg,
        egui::Stroke::new(1.0, stroke),
        egui::StrokeKind::Inside,
    );
    let _ = glass_alpha_card();
}

/// Paint a single static ribbon button at `anchor + offset` and
/// return its `Response`. Shared by [`super::static_ribbon`]; the
/// drag-aware `RibbonLayout` constructs its areas by hand so it can
/// set `Order::Tooltip` while a button is lifted.
pub(crate) fn ribbon_button_area(
    id: &'static str,
    ctx: &egui::Context,
    anchor: egui::Align2,
    offset: egui::Vec2,
    glyph: &str,
    tooltip: &str,
    is_active: bool,
    accent: egui::Color32,
    on_click: impl FnOnce(),
) {
    egui::Area::new(egui::Id::new(id))
        .anchor(anchor, offset)
        .interactable(true)
        .show(ctx, |ui| {
            let (rect, resp) = ui.allocate_exact_size(
                egui::vec2(SIDE_BTN_SIZE, SIDE_BTN_SIZE),
                egui::Sense::click(),
            );

            paint_ribbon_button(ui.painter(), rect, accent, is_active, resp.hovered());
            let fg = if is_active { TEXT_PRIMARY } else { TEXT_SECONDARY };
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                glyph,
                egui::FontId::new(14.0, egui::FontFamily::Monospace),
                fg,
            );

            if resp.on_hover_text(tooltip).clicked() {
                on_click();
            }
        });
}
