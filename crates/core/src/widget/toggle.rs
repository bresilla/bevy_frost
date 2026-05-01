//! Frost-styled binary on/off toggle in a labelled row layout.
//! Label on the left, pill track + sliding knob on the right —
//! row total height = 1U ([`crate::style::UNIT`]) by default.
//!
//! Mirrors `frostcore::widgets::toggle::toggle` (the row-with-label
//! variant — `frostcore`'s naked `toggle_control` corresponds to
//! [`toggle_track_only`] here).

use crate::style::{
    body_accent, on_panel, on_track, theme, track_fill, widget_border,
    BODY_FONT_SIZE,
};

/// Default toggle row height. Matches
/// `frostcore::widgets::toggle::H = 18` so the corekit toggle
/// lines up with frostcore at the same scale.
pub const TOGGLE_ROW_H: f32 = 18.0;
/// Track width — matches `frostcore::widgets::toggle::W = 38`.
pub const TOGGLE_TRACK_W: f32 = 38.0;

/// Default labelled toggle row.
pub fn toggle(
    ui: &mut egui::Ui,
    label: &str,
    on: &mut bool,
    accent: egui::Color32,
) -> egui::Response {
    toggle_h(ui, label, on, accent, TOGGLE_ROW_H)
}

/// Variable-height variant. Track aspect (~2:1 of height) and
/// label font scale with `height` so the row reads consistently
/// regardless of pod resize.
pub fn toggle_h(
    ui: &mut egui::Ui,
    label: &str,
    on: &mut bool,
    accent: egui::Color32,
    height: f32,
) -> egui::Response {
    /// Gap between label text and the track.
    const LABEL_TRACK_GAP: f32 = 6.0;

    let total_w = ui.available_width();
    let (row_rect, _) = ui.allocate_exact_size(
        egui::vec2(total_w, height),
        egui::Sense::hover(),
    );
    // Track keeps the original 38×18 frostcore proportions when
    // the row is at default height; scales linearly otherwise.
    let scale = height / TOGGLE_ROW_H;
    let track_w = (TOGGLE_TRACK_W * scale).round();
    let track_rect = egui::Rect::from_min_size(
        egui::pos2(row_rect.right() - track_w, row_rect.top()),
        egui::vec2(track_w, height),
    );
    let id = ui.id().with(("frost_toggle", label));
    let mut resp = ui
        .interact(track_rect, id, egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, label)
    });
    if !ui.is_rect_visible(row_rect) {
        return resp;
    }

    // Label, vertically centred, font scaled to height.
    if !label.is_empty() {
        let label_font = egui::FontId::proportional((BODY_FONT_SIZE * scale).round());
        ui.painter().text(
            egui::pos2(row_rect.left(), row_rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            label_font,
            on_panel(),
        );
        // Bound the label to stop short of the track + gap (egui's
        // `text` doesn't auto-clip; we just trust the caller's
        // labels are short enough at the typical 1U row width.
        // For longer labels, render via `add(Label::new(label).truncate())`
        // — kept simple here for v1.
        let _ = LABEL_TRACK_GAP;
    }
    paint_track(ui, track_rect, *on, resp.id, accent);
    resp
}

/// Standalone track + knob with no label, no row. For custom
/// compositions (e.g. an inline status row that already has its
/// own label rendering).
pub fn toggle_track_only(
    ui: &mut egui::Ui,
    on: &mut bool,
    accent: egui::Color32,
) -> egui::Response {
    let height = TOGGLE_ROW_H;
    let track_w = TOGGLE_TRACK_W;
    let (rect, mut resp) =
        ui.allocate_exact_size(egui::vec2(track_w, height), egui::Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if ui.is_rect_visible(rect) {
        paint_track(ui, rect, *on, resp.id, accent);
    }
    resp
}

fn paint_track(
    ui: &egui::Ui,
    rect: egui::Rect,
    on: bool,
    id: egui::Id,
    accent: egui::Color32,
) {
    /// Padding between the track edge and the knob.
    const KNOB_PAD: f32 = 2.0;
    /// Track tint at ON — small enough to stay under the
    /// irradiation-illusion threshold.
    const TRACK_ACCENT_HINT: f32 = 0.22;

    let th = theme();
    let how_on = ui.ctx().animate_bool_responsive(id, on);
    let painter = ui.painter_at(rect);
    let body_acc = body_accent(accent);
    let track_bg = lerp_col(track_fill(accent), body_acc, how_on * TRACK_ACCENT_HINT);
    let corner = egui::CornerRadius::same(th.radius_compact);
    painter.rect(
        rect,
        corner,
        track_bg,
        egui::Stroke::new(th.border_width, widget_border(accent)),
        egui::epaint::StrokeKind::Inside,
    );
    let knob_size = (rect.height() - KNOB_PAD * 2.0).max(1.0);
    let x_min = rect.left() + KNOB_PAD;
    let x_max = rect.right() - KNOB_PAD - knob_size;
    let knob_x = egui::lerp(x_min..=x_max, how_on);
    let knob_rect = egui::Rect::from_min_size(
        egui::pos2(knob_x, rect.top() + KNOB_PAD),
        egui::vec2(knob_size, knob_size),
    );
    let knob_color = lerp_col(on_track(), body_acc, how_on);
    painter.rect(
        knob_rect,
        corner,
        knob_color,
        egui::Stroke::new(th.border_width, widget_border(accent)),
        egui::epaint::StrokeKind::Inside,
    );
}

fn lerp_col(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let blend = |x: u8, y: u8| ((x as f32) * (1.0 - t) + (y as f32) * t).round() as u8;
    egui::Color32::from_rgb(blend(a.r(), b.r()), blend(a.g(), b.g()), blend(a.b(), b.b()))
}
