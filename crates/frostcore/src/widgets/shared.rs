//! Small helpers shared across widgets. Kept `pub(super)` so they
//! don't leak into the public surface — each widget module picks
//! what it needs.

use egui;

use crate::style::BORDER_SUBTLE;

/// Key under which the "pending trailing separator" flag lives in
/// egui temp data, scoped to the current `ui`'s id. Stored value is
/// the `cumulative_pass_nr` at which the mark was set, so a stale
/// mark left over from a previous frame (e.g. the last widget
/// disappeared between frames) is ignored on the current frame.
fn pending_separator_key(ui: &egui::Ui) -> egui::Id {
    ui.id().with("frost_pending_separator")
}

/// Trailing divider marker appended by every widget module. Does
/// NOT paint immediately — it only records that *this frame* the
/// current `ui` has a pending trailing separator. The paint is
/// performed lazily by [`flush_pending_separator`] at the START of
/// whichever widget comes next.
///
/// Consequence: if nothing follows (the mark is the last thing in
/// its container), the mark simply decays without ever being
/// painted — so a container's last row auto-hides its trailing
/// divider without the caller needing to know or annotate it.
///
/// Also re-exported publicly as [`super::row_separator`] so callers
/// who assemble bespoke inline row layouts can request the matching
/// divider with the same smart behaviour.
pub(super) fn widget_separator(ui: &mut egui::Ui) {
    let pass = ui.ctx().cumulative_pass_nr();
    let key = pending_separator_key(ui);
    ui.ctx().data_mut(|d| d.insert_temp::<u64>(key, pass));
}

/// Paint the deferred trailing separator — if any — that the prior
/// widget marked on this same frame. Call this at the very start of
/// every widget body. Idempotent; cheap no-op when no mark is
/// pending or the mark is stale. Clears the mark after handling so
/// subsequent calls on the same frame (e.g. during a re-run inside
/// the same pass) don't double-paint.
pub(super) fn flush_pending_separator(ui: &mut egui::Ui) {
    let key = pending_separator_key(ui);
    let current = ui.ctx().cumulative_pass_nr();
    let stored: Option<u64> = ui.ctx().data(|d| d.get_temp::<u64>(key));
    if stored.is_some() {
        ui.ctx().data_mut(|d| d.remove::<u64>(key));
    }
    if stored == Some(current) {
        paint_hairline(ui);
    }
}

/// Discard any pending trailing separator WITHOUT painting it. Used
/// by the resize-grip separator so a widget stack above + grip below
/// doesn't stack a hairline on top of the grip — the grip itself IS
/// the visual separator at that boundary.
pub(super) fn clear_pending_separator(ui: &mut egui::Ui) {
    let key = pending_separator_key(ui);
    ui.ctx().data_mut(|d| d.remove::<u64>(key));
}

/// Unconditional hairline — 1 px above, 1 px line, 1 px below. Tight
/// cadence matches the tabular density of frost panels; used only
/// by [`flush_pending_separator`].
fn paint_hairline(ui: &mut egui::Ui) {
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
