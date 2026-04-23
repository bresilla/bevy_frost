//! Icon glyph painted as an egui `Label`.
//!
//! Intentionally minimal today — a named slot for per-project icon
//! widgets (SVG, icon-font lookup, cached textures, …) to grow
//! into. Defaults to a single-glyph monospace label sized to match
//! ribbon / section-header typography.

use bevy_egui::egui;

/// Baseline point size for an inline icon glyph next to body copy.
/// Matches the ribbon button glyph size so icons feel "the same
/// weight" wherever they appear.
pub const ICON_BODY_SIZE: f32 = 14.0;

/// Paint `glyph` as a coloured monospace character. Returns the
/// `Response` so callers can compose further (hover text, click
/// sensing) with `.on_hover_text(...)` etc.
pub fn icon(ui: &mut egui::Ui, glyph: &str, color: egui::Color32) -> egui::Response {
    ui.add(
        egui::Label::new(
            egui::RichText::new(glyph)
                .monospace()
                .size(ICON_BODY_SIZE)
                .color(color),
        )
        .sense(egui::Sense::hover()),
    )
}

/// Clickable variant — same paint, but emits a click response.
pub fn icon_button(ui: &mut egui::Ui, glyph: &str, color: egui::Color32) -> egui::Response {
    ui.add(
        egui::Label::new(
            egui::RichText::new(glyph)
                .monospace()
                .size(ICON_BODY_SIZE)
                .color(color),
        )
        .sense(egui::Sense::click()),
    )
}
