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

use bevy_egui::egui;

use super::layout::dual_pane_labelled;
use super::shared::widget_separator;
use crate::style::{radius, widget_border};

/// Swatch button size — matches the toggle pill's dimensions so
/// rows line up.
const SWATCH_W: f32 = 38.0;
const SWATCH_H: f32 = 18.0;

/// Labelled sRGB colour swatch with inline expansion.
pub fn color_rgb(
    ui: &mut egui::Ui,
    label: &str,
    rgb: &mut [f32; 3],
    accent: egui::Color32,
) -> egui::Response {
    let id = ui.id().with(("frost_color_expand", label));
    let mut open: bool = ui.ctx().data(|d| d.get_temp::<bool>(id).unwrap_or(false));

    let mut row_resp = dual_pane_labelled(ui, label, |ui| {
        swatch_button(ui, rgb, open, accent)
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
        let mut color32 = egui::Color32::from_rgb(
            to_u8(rgb[0]),
            to_u8(rgb[1]),
            to_u8(rgb[2]),
        );
        let changed = picker_scope(ui, |ui| {
            egui::color_picker::color_picker_color32(
                ui,
                &mut color32,
                egui::color_picker::Alpha::Opaque,
            )
        });
        if changed {
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
    let id = ui.id().with(("frost_color_expand", label));
    let mut open: bool = ui.ctx().data(|d| d.get_temp::<bool>(id).unwrap_or(false));

    let mut row_resp = dual_pane_labelled(ui, label, |ui| {
        let rgb = [rgba[0], rgba[1], rgba[2]];
        swatch_button(ui, &rgb, open, accent)
    });

    if row_resp.clicked() {
        open = !open;
        ui.ctx().data_mut(|d| d.insert_temp(id, open));
    }

    if open {
        ui.add_space(4.0);
        let mut color32 = egui::Color32::from_rgba_unmultiplied(
            to_u8(rgba[0]),
            to_u8(rgba[1]),
            to_u8(rgba[2]),
            to_u8(rgba[3]),
        );
        let changed = picker_scope(ui, |ui| {
            egui::color_picker::color_picker_color32(
                ui,
                &mut color32,
                egui::color_picker::Alpha::OnlyBlend,
            )
        });
        if changed {
            rgba[0] = color32.r() as f32 / 255.0;
            rgba[1] = color32.g() as f32 / 255.0;
            rgba[2] = color32.b() as f32 / 255.0;
            rgba[3] = color32.a() as f32 / 255.0;
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
fn picker_scope<R>(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let w = ui.available_width();
    ui.scope(|ui| {
        ui.spacing_mut().slider_width = w;
        content(ui)
    })
    .inner
}

/// Paint a flat colour swatch with an accent-tinted border. `open`
/// lifts the border to the full accent so the user sees which row
/// is currently expanded at a glance.
fn swatch_button(
    ui: &mut egui::Ui,
    rgb: &[f32; 3],
    open: bool,
    accent: egui::Color32,
) -> egui::Response {
    let fill = egui::Color32::from_rgb(to_u8(rgb[0]), to_u8(rgb[1]), to_u8(rgb[2]));
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
        ui.painter().rect(
            rect,
            egui::CornerRadius::same(radius::COMPACT),
            fill,
            egui::Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}
