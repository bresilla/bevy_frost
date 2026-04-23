//! Small helpers shared across widgets. Kept `pub(super)` so they
//! don't leak into the public surface — each widget module picks
//! what it needs.

use bevy_egui::egui;

use crate::style::BORDER_SUBTLE;

/// Thin horizontal rule painted across the current row. Widget
/// modules append one of these after their body so every row has a
/// subtle trailing divider — looks like "between rows" visually;
/// on the last row of a container the division is a hair against
/// the inner bottom padding, which reads as expected rather than
/// as an extra border.
///
/// Also re-exported publicly as [`super::row_separator`] so callers
/// who assemble bespoke inline row layouts (e.g. a tight cluster of
/// small buttons) can paint the matching divider themselves.
pub(super) fn widget_separator(ui: &mut egui::Ui) {
    // Hairline breathing room — 1 px above, 1 px line, 1 px below.
    // Tight on purpose: at panel density, 3 px top + bottom made the
    // rows feel airy rather than tabular.
    ui.add_space(1.0);
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 1.0), egui::Sense::hover());
    let color = egui::Color32::from_rgba_unmultiplied(
        BORDER_SUBTLE.r(),
        BORDER_SUBTLE.g(),
        BORDER_SUBTLE.b(),
        96,
    );
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        egui::Stroke::new(1.0, color),
    );
    ui.add_space(1.0);
}

/// Linear colour interpolation across RGBA channels. `t` is clamped
/// to `0.0..=1.0`.
pub(super) fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| ((x as f32) * (1.0 - t) + (y as f32) * t).round() as u8;
    egui::Color32::from_rgba_premultiplied(
        mix(a.r(), b.r()),
        mix(a.g(), b.g()),
        mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}

/// Paint a track + accent-filled portion with a centred value
/// readout. Shared by [`super::slider`] (which layers interaction
/// on top) and [`super::progressbar`] (which doesn't).
///
/// The text is painted twice — once per side of the fill edge —
/// each time clipped to that side's rect. Callers pass two colours
/// so the readout reads cleanly whichever colour it lands on.
pub(super) fn paint_value_bar(
    ui: &egui::Ui,
    rect: egui::Rect,
    fill_fraction: f32,
    text: &str,
    font: egui::FontId,
    accent: egui::Color32,
    track_text_color: egui::Color32,
    fill_text_color: egui::Color32,
    corner_radius: u8,
) {
    let painter = ui.painter_at(rect);
    let fraction = fill_fraction.clamp(0.0, 1.0);
    let fill_w = rect.width() * fraction;

    // Unfilled track (full width).
    painter.rect_filled(
        rect,
        egui::CornerRadius::same(corner_radius),
        ui.visuals().extreme_bg_color,
    );

    // Accent fill pinned to the left.
    if fill_w > 0.5 {
        let fill_rect = egui::Rect::from_min_size(rect.min, egui::vec2(fill_w, rect.height()));
        painter.rect_filled(
            fill_rect,
            egui::CornerRadius::same(corner_radius),
            accent,
        );
    }

    // Text — two passes, each clipped so the colour switches
    // cleanly at the fill edge. `painter_at` restricts the
    // sub-painter's clip rect, so draws outside the sub-rect are
    // hidden.
    let center = rect.center();

    if fraction < 1.0 {
        let track_sub = egui::Rect::from_min_max(
            egui::pos2(rect.min.x + fill_w, rect.min.y),
            rect.max,
        );
        ui.painter_at(track_sub).text(
            center,
            egui::Align2::CENTER_CENTER,
            text,
            font.clone(),
            track_text_color,
        );
    }
    if fraction > 0.0 {
        let fill_sub = egui::Rect::from_min_size(rect.min, egui::vec2(fill_w, rect.height()));
        ui.painter_at(fill_sub).text(
            center,
            egui::Align2::CENTER_CENTER,
            text,
            font,
            fill_text_color,
        );
    }
}
