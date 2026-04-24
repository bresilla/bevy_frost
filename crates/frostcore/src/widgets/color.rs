//! Colour-picker module — label on 70 %, frost-styled colour swatch
//! on 30 %. Clicking the swatch **expands the picker inline** inside
//! the same section body (extending the container downward) instead
//! of opening a floating pop-up.
//!
//! Same layout language as every other widget module: trailing
//! separator, accent-tinted swatch border, expansion state stored in
//! egui memory keyed by label so the UI remembers what was open
//! frame-to-frame.
//!
//! The picker widget itself comes from `egui::color_picker` —
//! [`color_picker_color32`](egui::color_picker::color_picker_color32)
//! — so we don't reinvent the HSV / hue / saturation controls; we
//! just host them in-place rather than in a detached overlay.

use egui;

use super::layout::dual_pane_labelled;
use super::shared::{flush_pending_separator, widget_separator};
use crate::style::{radius, widget_border};

/// Swatch button size — matches the DragValue input width
/// ([`super::drag::INPUT_WIDTH`] = 72 px) and the shared interact
/// row height (20 px). Keeps the right-column cell aligned
/// between colour rows and numeric rows in the same section.
const SWATCH_W: f32 = 72.0;
const SWATCH_H: f32 = 20.0;

/// Labelled sRGB colour swatch with inline expansion.
pub fn color_rgb(
    ui: &mut egui::Ui,
    label: &str,
    rgb: &mut [f32; 3],
    accent: egui::Color32,
) -> egui::Response {
    flush_pending_separator(ui);
    let id = ui.id().with(("frost_color_expand", label));
    let mut open: bool = ui.ctx().data(|d| d.get_temp::<bool>(id).unwrap_or(false));

    let preview = egui::Color32::from_rgb(to_u8(rgb[0]), to_u8(rgb[1]), to_u8(rgb[2]));
    let mut row_resp = dual_pane_labelled(ui, label, |ui| {
        swatch_button(ui, preview, open, accent)
    });

    if row_resp.clicked() {
        open = !open;
        ui.ctx().data_mut(|d| d.insert_temp(id, open));
    }

    if open {
        ui.add_space(4.0);
        // Expand the inline picker to the container's available
        // width. `color_picker_color32` sizes every sub-element from
        // `ui.spacing().slider_width` — the 2D SV square is a N×N
        // square sized off that, and the hue + current-colour
        // previews match its width. Our theme sets `slider_width`
        // to 90 px (tight-row look), which made the picker read as
        // "tiny". Scoping the override to a child `ui.scope` keeps
        // the panel's other sliders at their normal width.
        let mut color32 = preview;
        let changed = picker_scope(ui, |ui| {
            egui::color_picker::color_picker_color32(
                ui,
                &mut color32,
                egui::color_picker::Alpha::Opaque,
            )
        });
        if changed {
            // Alpha is forced opaque by the picker, so reading raw
            // `r()/g()/b()` is safe — they're not premultiplied by
            // anything less than 1.0.
            rgb[0] = color32.r() as f32 / 255.0;
            rgb[1] = color32.g() as f32 / 255.0;
            rgb[2] = color32.b() as f32 / 255.0;
            row_resp.mark_changed();
        }
        ui.add_space(4.0);
    }

    widget_separator(ui);
    row_resp
}

/// Labelled sRGBA colour swatch with inline expansion. Same as
/// [`color_rgb`] but exposes the alpha slider in the expanded
/// picker body.
pub fn color_rgba(
    ui: &mut egui::Ui,
    label: &str,
    rgba: &mut [f32; 4],
    accent: egui::Color32,
) -> egui::Response {
    flush_pending_separator(ui);
    let id = ui.id().with(("frost_color_expand", label));
    let mut open: bool = ui.ctx().data(|d| d.get_temp::<bool>(id).unwrap_or(false));

    // Build the preview colour from the caller's UNMULTIPLIED rgba.
    // `Color32::from_rgba_unmultiplied` premultiplies internally —
    // the same representation egui's picker round-trips through, so
    // passing it in and using `to_srgba_unmultiplied()` on the way
    // out gives us a stable fixed-point (no per-frame drift / jump).
    let preview = egui::Color32::from_rgba_unmultiplied(
        to_u8(rgba[0]),
        to_u8(rgba[1]),
        to_u8(rgba[2]),
        to_u8(rgba[3]),
    );
    let mut row_resp = dual_pane_labelled(ui, label, |ui| {
        swatch_button(ui, preview, open, accent)
    });

    if row_resp.clicked() {
        open = !open;
        ui.ctx().data_mut(|d| d.insert_temp(id, open));
    }

    if open {
        ui.add_space(4.0);
        let mut color32 = preview;
        let changed = picker_scope(ui, |ui| {
            egui::color_picker::color_picker_color32(
                ui,
                &mut color32,
                egui::color_picker::Alpha::OnlyBlend,
            )
        });
        if changed {
            // CRITICAL: read via `to_srgba_unmultiplied` — the raw
            // `r()/g()/b()/a()` accessors return PREMULTIPLIED bytes.
            // Dividing those by 255 and writing back into the user's
            // unmultiplied rgba would reduce each channel by the
            // alpha factor every frame, causing the "jumping" as the
            // colour decayed on each round-trip.
            let [r, g, b, a] = color32.to_srgba_unmultiplied();
            rgba[0] = r as f32 / 255.0;
            rgba[1] = g as f32 / 255.0;
            rgba[2] = b as f32 / 255.0;
            rgba[3] = a as f32 / 255.0;
            row_resp.mark_changed();
        }
        ui.add_space(4.0);
    }

    widget_separator(ui);
    row_resp
}

/// Run a closure inside a child `Ui` whose `slider_width` has been
/// widened to the available row width, so `color_picker_color32`
/// renders at the container's width instead of the theme's compact
/// slider width. Scoping via `ui.scope` confines the override to
/// this call — other sliders in the parent ui keep their normal
/// width.
///
/// Also widens the ui's clip rect by a few px so the 2D picker's
/// circular indicator (drawn at the current sat/value point) can
/// extend past the square's edge without getting sliced by the
/// container's hard-clip when the colour sits at a corner.
fn picker_scope<R>(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let w = ui.available_width();
    ui.scope(|ui| {
        ui.spacing_mut().slider_width = w;
        // Grow the clip rect outward. egui's 2D picker draws its
        // indicator as a `CircleShape` with `radius = rect.width() /
        // 12.0` (see `color_slider_2d` in egui/src/widgets/color_picker.rs),
        // so the expansion scales with the picker size. `w / 10.0 +
        // 4` is a comfortable margin: a bit more than the indicator
        // radius plus the stroke width, so the circle renders fully
        // regardless of where the user drags it. Scoped to this
        // child ui so adjacent widgets keep their clean clip.
        let indicator_margin = (w / 10.0).ceil() + 4.0;
        let clip = ui.clip_rect().expand(indicator_margin);
        ui.set_clip_rect(clip);
        content(ui)
    })
    .inner
}

/// Paint a colour swatch with an accent-tinted border. When the
/// colour is fully opaque the swatch is a flat fill; when the
/// alpha is less than 255, egui's [`show_color_at`] splits the
/// swatch — left half shows the RGB + alpha blended over a
/// checkerboard (so the user can gauge the alpha visually),
/// right half shows the same RGB made fully opaque (so the user
/// can see the "pure" colour). `open` lifts the border to the
/// full accent so the user sees which row is expanded at a
/// glance.
///
/// [`show_color_at`]: egui::color_picker::show_color_at
fn swatch_button(
    ui: &mut egui::Ui,
    color: egui::Color32,
    open: bool,
    accent: egui::Color32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(SWATCH_W, SWATCH_H),
        egui::Sense::click(),
    );
    if ui.is_rect_visible(rect) {
        let border = if open || resp.hovered() {
            accent
        } else {
            widget_border(accent)
        };
        // Inner fill via egui's `show_color_at` — handles the
        // checker + half/half split for transparent colours so RGBA
        // rows read correctly at a glance. Shrunk by 1 px so the
        // border stroke below paints cleanly on top of the inner
        // fill's edge.
        egui::color_picker::show_color_at(ui.painter(), color, rect.shrink(1.0));
        ui.painter().rect_stroke(
            rect,
            egui::CornerRadius::same(radius::COMPACT),
            egui::Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}
