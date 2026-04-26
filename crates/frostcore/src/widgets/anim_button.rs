//! Hover-fill animated buttons — twelve fill styles translated from
//! a popular CSS hover-button collection, all drawn inside the
//! frost kit's standard [`wide_button`](super::wide_button) shell.
//!
//! At rest each button renders **identically** to `wide_button`: the
//! same row height (24 px), the same theme-driven corner radius and
//! border width, the same `paint_accent_bg` rest-tint fill recipe,
//! the same label font / size / contrast colour. The only behaviour
//! that differs is the **hover transition** — instead of nudging
//! the fill alpha, we paint a darkened-accent shape that slides /
//! grows / converges into the button's interior over 0.5 s.
//!
//! Single entry point: [`animated_button(ui, label, accent, style)`].

use egui;

use crate::style::{glass_alpha_card, surface_lift_target, widget_border};
use super::shared::{
    flush_pending_separator, lerp_color, paint_click_pulse, press_depress_amount,
    widget_separator,
};

/// Picks which of the twelve hover-fill animations to run.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FillStyle {
    /// Solid rect slides in from left edge.
    SlideLeft,
    /// Right-leaning parallelogram slides in from left.
    Parallelogram,
    /// Two parallelograms — one from each side — meet in the centre.
    ParallelogramMeet,
    /// Two opposite-leaning parallelograms forming a bowtie.
    Bowtie,
    /// Two triangle-edged narrow trapezoids meeting from sides.
    BandsMeet,
    /// Four quadrant squares converging diagonally to centre.
    CornerSquares,
    /// Two large triangles growing from opposite corners.
    DiagonalTriangles,
    /// Single circle expanding from centre to fill the button.
    CircleGrow,
    /// Four vertical bars rising from the bottom edge (equalizer).
    Equalizer,
    /// Top + bottom halves slide vertically to meet at the middle.
    HorizontalSlide,
    /// Four horizontal bars top/bottom with a 0.4 s phase delay.
    HorizontalSlideDelayed,
    /// Four vertical bars left/right with a 0.4 s phase delay.
    VerticalSlideDelayed,
    /// Two circles enter from opposite ends and cross through middle.
    CrissCross,
}

/// Animated hover-fill button. Returns the egui [`Response`] so
/// callers can react to clicks. Visually a drop-in replacement for
/// [`wide_button`](super::wide_button); only the hover transition
/// differs.
pub fn animated_button(
    ui: &mut egui::Ui,
    label: &str,
    accent: egui::Color32,
    style: FillStyle,
) -> egui::Response {
    flush_pending_separator(ui);
    const ROW_H: f32 = 24.0;
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, ROW_H), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        widget_separator(ui);
        return resp;
    }

    let th = crate::style::theme();
    let radius = egui::CornerRadius::same(th.radius_widget);
    let pressed = resp.is_pointer_button_down_on();

    // Hover progress for the CSS animation. 0.25 s — half the
    // source CSS `0.5s ease-in-out` so the fill snaps in before
    // the user can pull off. Press contributes too so the fill
    // settles fully while held, mirroring `wide_button`'s tint
    // collapsing onto `tint_press`.
    let hover_t = ui.ctx().animate_bool_with_time(
        resp.id.with("frost_anim_btn_hover"),
        resp.hovered() || pressed,
        0.25,
    );

    // Press depress — shared helper. Paints into the depressed
    // rect just like `wide_button`, so a held animated button
    // physically shrinks the same 2 px each side.
    let depress_px = press_depress_amount(ui.ctx(), resp.id, pressed, 2.0);
    let painted_rect = rect.shrink(depress_px);

    // ── Rest-state fill — identical to `paint_accent_bg` at rest ──
    //
    // Same `lerp(base, surface_lift_target(body_accent), tint_rest)`
    // recipe `wide_button` uses, painted at `glass_alpha_card`. The
    // CSS animation paints DARKER accent on top, so the user sees
    // the regular button → animated dark fill → final dark state
    // transition rather than a separate widget identity.
    let body_acc = crate::style::body_accent(accent);
    let base = if th.section_show_frame {
        crate::style::section_fill(accent)
    } else {
        crate::style::pane_fill(accent)
    };
    let target = surface_lift_target(body_acc);
    let rest_solid = lerp_color(base, target, th.button_tint_rest);
    let rest_bg = egui::Color32::from_rgba_unmultiplied(
        rest_solid.r(),
        rest_solid.g(),
        rest_solid.b(),
        glass_alpha_card(),
    );

    // The hover-fill colour matches `wide_button`'s pressed tint
    // (`button_tint_press`) so a fully-animated-in animated button
    // and a held wide_button settle on the same colour.
    let hover_solid = lerp_color(base, target, th.button_tint_press);
    let fill_color = egui::Color32::from_rgba_unmultiplied(
        hover_solid.r(),
        hover_solid.g(),
        hover_solid.b(),
        glass_alpha_card(),
    );

    // ── Paint stack ──
    //
    // TWO painters: `inner` clipped to the depressed button rect so
    // the hover-fill polygons can extend their slants / circles
    // beyond the rect mathematically without bleeding visually
    // outside the button; `outer` clipped to a wider region so the
    // click-pulse stroke (which paints OUTSIDE the rect by up to
    // 8 px) isn't cut off.
    let inner = ui.painter_at(painted_rect);
    let outer = ui.painter_at(rect.expand(10.0));
    // 1. Rest fill across the depressed button (always present).
    inner.rect_filled(painted_rect, radius, rest_bg);
    // 2. CSS-style animated hover overlay — clipped to the button
    //    bounds via `inner` so slant edges, oversized circles, and
    //    off-screen-then-slide-in shapes stay neatly inside.
    paint_fill(&inner, painted_rect, hover_t, fill_color, style);
    // 3. Border — `widget_border` at rest, full accent on hover,
    //    same as `wide_button`.
    let border_col = lerp_color(widget_border(accent), accent, hover_t);
    inner.rect_stroke(
        painted_rect,
        radius,
        egui::Stroke::new(th.border_width, border_col),
        egui::StrokeKind::Inside,
    );
    // 4. Label — pick the contrast colour against whichever bg is
    //    visible at the centre right now (rest_bg at t=0,
    //    fill_color at t=1).
    let centre_bg = lerp_color(rest_bg, fill_color, hover_t);
    inner.text(
        painted_rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(12.0),
        crate::style::contrast_text_for(centre_bg),
    );
    // 5. Click pulse — concentric ring discharge, shared with
    //    `wide_button` / `card_button`. Uses the WIDER painter so
    //    the +8 px expansion stroke renders fully.
    paint_click_pulse(ui.ctx(), &outer, &resp, painted_rect, accent, radius);

    widget_separator(ui);
    resp
}

fn paint_fill(
    p: &egui::Painter,
    rect: egui::Rect,
    t: f32,
    color: egui::Color32,
    style: FillStyle,
) {
    use FillStyle::*;
    match style {
        SlideLeft => fill_slide_left(p, rect, t, color),
        Parallelogram => fill_parallelogram(p, rect, t, color),
        ParallelogramMeet => fill_parallelogram_meet(p, rect, t, color),
        Bowtie => fill_bowtie(p, rect, t, color),
        BandsMeet => fill_bands_meet(p, rect, t, color),
        CornerSquares => fill_corner_squares(p, rect, t, color),
        DiagonalTriangles => fill_diagonal_triangles(p, rect, t, color),
        CircleGrow => fill_circle_grow(p, rect, t, color),
        Equalizer => fill_equalizer(p, rect, t, color),
        HorizontalSlide => fill_horizontal_slide(p, rect, t, color),
        HorizontalSlideDelayed => fill_horizontal_slide_delayed(p, rect, t, color),
        VerticalSlideDelayed => fill_vertical_slide_delayed(p, rect, t, color),
        CrissCross => fill_criss_cross(p, rect, t, color),
    }
}

// Slant in logical points used by the parallelogram variants.
// Scaled relative to the 24-px row height so the slope angle stays
// consistent across themes.
const SLANT: f32 = 12.0;

// ─── Variant 0 — SlideLeft ──────────────────────────────────────────

fn fill_slide_left(p: &egui::Painter, rect: egui::Rect, t: f32, c: egui::Color32) {
    let w = rect.width();
    let dx = -w * (1.0 - t);
    let r = rect.translate(egui::vec2(dx, 0.0));
    p.rect_filled(r, egui::CornerRadius::ZERO, c);
}

// ─── Variant 1 — Parallelogram ──────────────────────────────────────

fn fill_parallelogram(p: &egui::Painter, rect: egui::Rect, t: f32, c: egui::Color32) {
    let total_w = rect.width() + SLANT;
    let dx = -total_w * (1.0 - t);
    let poly = vec![
        egui::pos2(rect.min.x + dx,                rect.min.y),
        egui::pos2(rect.min.x + dx + total_w,      rect.min.y),
        egui::pos2(rect.min.x + dx + total_w - SLANT, rect.max.y),
        egui::pos2(rect.min.x + dx - SLANT,        rect.max.y),
    ];
    p.add(egui::Shape::convex_polygon(poly, c, egui::Stroke::NONE));
}

// ─── Variant 1-2 — ParallelogramMeet ────────────────────────────────

fn fill_parallelogram_meet(p: &egui::Painter, rect: egui::Rect, t: f32, c: egui::Color32) {
    let half_w = rect.width() * 0.5 + SLANT * 0.5;
    let dx_left = -half_w * (1.0 - t);
    let dx_right = half_w * (1.0 - t);
    let l = vec![
        egui::pos2(rect.min.x + dx_left,                  rect.min.y),
        egui::pos2(rect.min.x + dx_left + half_w,         rect.min.y),
        egui::pos2(rect.min.x + dx_left + half_w - SLANT, rect.max.y),
        egui::pos2(rect.min.x + dx_left - SLANT,          rect.max.y),
    ];
    let r = vec![
        egui::pos2(rect.max.x + dx_right - half_w,        rect.min.y),
        egui::pos2(rect.max.x + dx_right,                 rect.min.y),
        egui::pos2(rect.max.x + dx_right + SLANT,         rect.max.y),
        egui::pos2(rect.max.x + dx_right - half_w + SLANT, rect.max.y),
    ];
    p.add(egui::Shape::convex_polygon(l, c, egui::Stroke::NONE));
    p.add(egui::Shape::convex_polygon(r, c, egui::Stroke::NONE));
}

// ─── Variant 2 — Bowtie ─────────────────────────────────────────────

fn fill_bowtie(p: &egui::Painter, rect: egui::Rect, t: f32, c: egui::Color32) {
    let total_w = rect.width() * 0.51;
    let dx_l = -total_w * (1.0 - t);
    let dx_r = total_w * (1.0 - t);
    let l = vec![
        egui::pos2(rect.min.x + dx_l,                   rect.min.y),
        egui::pos2(rect.min.x + dx_l + total_w,         rect.min.y),
        egui::pos2(rect.min.x + dx_l + total_w - SLANT, rect.max.y),
        egui::pos2(rect.min.x + dx_l,                   rect.max.y),
    ];
    let r = vec![
        egui::pos2(rect.max.x + dx_r - total_w + SLANT, rect.min.y),
        egui::pos2(rect.max.x + dx_r,                   rect.min.y),
        egui::pos2(rect.max.x + dx_r,                   rect.max.y),
        egui::pos2(rect.max.x + dx_r - total_w,         rect.max.y),
    ];
    p.add(egui::Shape::convex_polygon(l, c, egui::Stroke::NONE));
    p.add(egui::Shape::convex_polygon(r, c, egui::Stroke::NONE));
}

// ─── Variant 3 — BandsMeet ──────────────────────────────────────────

fn fill_bands_meet(p: &egui::Painter, rect: egui::Rect, t: f32, c: egui::Color32) {
    let half_h = rect.height() * 0.5;
    let band_w = rect.width() * 0.7;
    // Slide travel = `band_w + half_h + 2` so the chevron's tip
    // (which protrudes `half_h` past its rectangular body) sits a
    // safe 2 px outside the button at t=0. Without the extra
    // `half_h`, the tip would peek into the rest-state button as
    // a faint arrow.
    let slide = band_w + half_h + 2.0;
    let dx_l = -slide * (1.0 - t);
    let dx_r = slide * (1.0 - t);
    let mid_y = rect.min.y + half_h;
    let l = vec![
        egui::pos2(rect.min.x + dx_l,                   rect.min.y),
        egui::pos2(rect.min.x + dx_l + band_w,          rect.min.y),
        egui::pos2(rect.min.x + dx_l + band_w + half_h, mid_y),
        egui::pos2(rect.min.x + dx_l + band_w,          rect.max.y),
        egui::pos2(rect.min.x + dx_l,                   rect.max.y),
    ];
    let r = vec![
        egui::pos2(rect.max.x + dx_r - band_w,          rect.min.y),
        egui::pos2(rect.max.x + dx_r,                   rect.min.y),
        egui::pos2(rect.max.x + dx_r,                   rect.max.y),
        egui::pos2(rect.max.x + dx_r - band_w,          rect.max.y),
        egui::pos2(rect.max.x + dx_r - band_w - half_h, mid_y),
    ];
    p.add(egui::Shape::convex_polygon(l, c, egui::Stroke::NONE));
    p.add(egui::Shape::convex_polygon(r, c, egui::Stroke::NONE));
}

// ─── Variant 4 — CornerSquares ──────────────────────────────────────

fn fill_corner_squares(p: &egui::Painter, rect: egui::Rect, t: f32, c: egui::Color32) {
    let qw = rect.width() * 0.5;
    let qh = rect.height() * 0.5;
    let dx = qw * (1.0 - t);
    let dy = qh * (1.0 - t);
    let cx = rect.center().x;
    let cy = rect.center().y;
    let q = |x_min: f32, y_min: f32| {
        egui::Rect::from_min_size(egui::pos2(x_min, y_min), egui::vec2(qw, qh))
    };
    p.rect_filled(q(rect.min.x - dx, rect.min.y - dy), egui::CornerRadius::ZERO, c);
    p.rect_filled(q(cx + dx,         rect.min.y - dy), egui::CornerRadius::ZERO, c);
    p.rect_filled(q(rect.min.x - dx, cy + dy),         egui::CornerRadius::ZERO, c);
    p.rect_filled(q(cx + dx,         cy + dy),         egui::CornerRadius::ZERO, c);
}

// ─── Variant 5 — DiagonalTriangles ──────────────────────────────────

fn fill_diagonal_triangles(p: &egui::Painter, rect: egui::Rect, t: f32, c: egui::Color32) {
    let bw = rect.width() * 1.05 * t;
    let bh = rect.height() * t;
    let bl = vec![
        egui::pos2(rect.min.x,        rect.max.y),
        egui::pos2(rect.min.x + bw,   rect.max.y),
        egui::pos2(rect.min.x,        rect.max.y - bh),
    ];
    let tr = vec![
        egui::pos2(rect.max.x,        rect.min.y),
        egui::pos2(rect.max.x - bw,   rect.min.y),
        egui::pos2(rect.max.x,        rect.min.y + bh),
    ];
    p.add(egui::Shape::convex_polygon(bl, c, egui::Stroke::NONE));
    p.add(egui::Shape::convex_polygon(tr, c, egui::Stroke::NONE));
}

// ─── Variant 6 — CircleGrow ─────────────────────────────────────────

fn fill_circle_grow(p: &egui::Painter, rect: egui::Rect, t: f32, c: egui::Color32) {
    let r_max = (rect.width().powi(2) + rect.height().powi(2)).sqrt() * 0.5;
    let r = r_max * t;
    p.circle_filled(rect.center(), r, c);
}

// ─── Variant 7 — Equalizer ──────────────────────────────────────────

fn fill_equalizer(p: &egui::Painter, rect: egui::Rect, t: f32, c: egui::Color32) {
    let bar_w = rect.width() * 0.25;
    let bar_h = rect.height() * t;
    for i in 0..4 {
        let x = rect.min.x + (i as f32) * bar_w;
        let bar = egui::Rect::from_min_size(
            egui::pos2(x, rect.max.y - bar_h),
            egui::vec2(bar_w, bar_h),
        );
        p.rect_filled(bar, egui::CornerRadius::ZERO, c);
    }
}

// ─── Variant 8 — HorizontalSlide ────────────────────────────────────

fn fill_horizontal_slide(p: &egui::Painter, rect: egui::Rect, t: f32, c: egui::Color32) {
    let h = rect.height();
    let dy = h * (1.0 - t);
    let top = rect.translate(egui::vec2(0.0, -dy));
    let top_clipped = egui::Rect::from_min_max(
        top.min,
        egui::pos2(top.max.x, top.max.y - h * 0.5),
    );
    p.rect_filled(top_clipped, egui::CornerRadius::ZERO, c);
    let bot = rect.translate(egui::vec2(0.0, dy));
    let bot_clipped = egui::Rect::from_min_max(
        egui::pos2(bot.min.x, bot.min.y + h * 0.5),
        bot.max,
    );
    p.rect_filled(bot_clipped, egui::CornerRadius::ZERO, c);
}

// ─── Variant 9 — HorizontalSlideDelayed ─────────────────────────────

fn fill_horizontal_slide_delayed(
    p: &egui::Painter,
    rect: egui::Rect,
    t: f32,
    c: egui::Color32,
) {
    let half_h = rect.height() * 0.5;
    let phase_a = (t * 2.0).clamp(0.0, 1.0);
    let phase_b = ((t - 0.5) * 2.0).clamp(0.0, 1.0);
    let a_h = half_h * phase_a;
    let top_a = egui::Rect::from_min_size(
        rect.min,
        egui::vec2(rect.width(), a_h),
    );
    let bot_a = egui::Rect::from_min_size(
        egui::pos2(rect.min.x, rect.max.y - a_h),
        egui::vec2(rect.width(), a_h),
    );
    p.rect_filled(top_a, egui::CornerRadius::ZERO, c);
    p.rect_filled(bot_a, egui::CornerRadius::ZERO, c);
    if phase_b > 0.0 {
        let b_h = half_h * phase_b;
        let top_b = egui::Rect::from_min_size(
            egui::pos2(rect.min.x, rect.min.y + half_h - b_h),
            egui::vec2(rect.width(), b_h),
        );
        let bot_b = egui::Rect::from_min_size(
            egui::pos2(rect.min.x, rect.min.y + half_h),
            egui::vec2(rect.width(), b_h),
        );
        p.rect_filled(top_b, egui::CornerRadius::ZERO, c);
        p.rect_filled(bot_b, egui::CornerRadius::ZERO, c);
    }
}

// ─── Variant 10 — VerticalSlideDelayed ──────────────────────────────

fn fill_vertical_slide_delayed(
    p: &egui::Painter,
    rect: egui::Rect,
    t: f32,
    c: egui::Color32,
) {
    let half_w = rect.width() * 0.5;
    let phase_a = (t * 2.0).clamp(0.0, 1.0);
    let phase_b = ((t - 0.5) * 2.0).clamp(0.0, 1.0);
    let a_w = half_w * phase_a;
    let l_a = egui::Rect::from_min_size(rect.min, egui::vec2(a_w, rect.height()));
    let r_a = egui::Rect::from_min_size(
        egui::pos2(rect.max.x - a_w, rect.min.y),
        egui::vec2(a_w, rect.height()),
    );
    p.rect_filled(l_a, egui::CornerRadius::ZERO, c);
    p.rect_filled(r_a, egui::CornerRadius::ZERO, c);
    if phase_b > 0.0 {
        let b_w = half_w * phase_b;
        let l_b = egui::Rect::from_min_size(
            egui::pos2(rect.min.x + half_w - b_w, rect.min.y),
            egui::vec2(b_w, rect.height()),
        );
        let r_b = egui::Rect::from_min_size(
            egui::pos2(rect.min.x + half_w, rect.min.y),
            egui::vec2(b_w, rect.height()),
        );
        p.rect_filled(l_b, egui::CornerRadius::ZERO, c);
        p.rect_filled(r_b, egui::CornerRadius::ZERO, c);
    }
}

// ─── Variant 11 — CrissCross ────────────────────────────────────────

fn fill_criss_cross(p: &egui::Painter, rect: egui::Rect, t: f32, c: egui::Color32) {
    let cx = rect.center().x;
    let cy = rect.center().y;
    let max_r = rect.width() * 0.85;
    let dot_r = 6.0;
    let r;
    let off_x;
    if t < 0.5 {
        let p1 = t * 2.0;
        r = dot_r;
        off_x = egui::lerp(rect.width() * 0.55..=0.0, p1);
    } else {
        let p2 = (t - 0.5) * 2.0;
        r = egui::lerp(dot_r..=max_r, p2);
        off_x = 0.0;
    }
    p.circle_filled(egui::pos2(cx - off_x, cy), r, c);
    p.circle_filled(egui::pos2(cx + off_x, cy), r, c);
}
