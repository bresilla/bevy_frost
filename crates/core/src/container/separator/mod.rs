//! Frost-styled inter-pod separator. Two variants:
//!
//! * [`SeparatorStyle::Line`] — thin hairline rule across the
//!   container's body width. Pure visual cue, no interaction.
//! * [`SeparatorStyle::LineDots`] — short rule on each side of three
//!   centred dots. Same visual rhythm as the line variant; the
//!   three dots are a placeholder for a future drag handle that will
//!   resize the pod above (see `frostcore::widgets::splitter`).
//!
//! Both variants share the same total strip thickness so swapping
//! variants on a pod doesn't shift its neighbours' positions. The
//! ink colour matches the rest of the frost border family
//! (`widget_border(accent)` softened to `border_subtle` alpha) so
//! the separator reads as part of the same "border" language as
//! every section frame and widget outline.

use std::hash::Hash;

use egui::{vec2, Color32, Response, Sense, Stroke, Ui};

use crate::style;

/// Alpha applied to [`style::outline_base`] when painting the
/// separator. Low enough to whisper beneath section frames
/// rather than compete with them — the line should hint at "this
/// is where one pod ends and the next begins" without drawing the
/// eye away from the actual content.
const SEPARATOR_ALPHA: u8 = 90;

/// Vertical strip thickness — the rect EVERY separator reserves in
/// the parent ui, both [`paint_separator`] (non-interactive) and
/// [`paint_separator_resize`] (the drag-handle variant). Same value
/// for both so swapping one variant for the other doesn't shift
/// neighbouring pod positions, and tight enough that adjacent pods
/// sit close together — the visual cue is the line / dots
/// themselves, not a thick reserved gap.
pub const SEPARATOR_STRIP_H: f32 = 2.0;
/// Centre-to-centre spacing between the three dots in
/// [`SeparatorStyle::LineDots`].
const DOT_SPACING: f32 = 5.0;
/// Dot radius. Tuned so a three-dot diameter (`2 * DOT_R`) reads
/// proportionate to the 1-px stroke width of the flanking rules.
const DOT_R: f32 = 0.9;
/// Stroke width for the line / flanking rules. Same width every
/// other frost surface uses for its outline so the separator reads
/// as part of the same border family.
const RULE_W: f32 = 1.0;
/// Inset from the edge of the parent ui where the line / flanking
/// rules begin. Keeps the separator from butting up against the
/// container's chrome.
const EDGE_INSET: f32 = 2.0;
/// Gap between the dot cluster and the start of each flanking rule
/// in the `LineDots` variant — keeps the dots and the rules from
/// touching.
const GRIP_HALF_GAP: f32 = 3.0;

/// Visual style for an inter-pod separator. The container reads
/// each pod's chosen style and paints it BETWEEN that pod and the
/// next (not after the last pod). See module docs.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum SeparatorStyle {
    /// No separator — pods sit flush with just the container's
    /// inter-pod gap between them.
    None,
    /// Plain thin hairline across the full width.
    #[default]
    Line,
    /// Hairline + three centred dots + hairline. When the pod above
    /// is [`crate::pod::Pod::resizable`], this becomes the drag
    /// handle that grows / shrinks the pod (paint via
    /// [`paint_separator_resize`]); otherwise it's purely visual.
    LineDots,
}

/// Paint a separator into the parent `ui`. Allocates a strip of
/// height [`SEPARATOR_STRIP_H`] across the available width, then
/// paints into it according to `style`.
///
/// Colour comes from [`style::outline_base`], which auto-flips
/// per theme luma — white-tinted on dark themes, black-tinted on
/// light themes — so the separator stays a subtle whisper of the
/// "opposite" colour against whichever surface the theme paints.
/// Without that flip, a single `widget_border(accent)` recipe
/// would render near-black on a Light theme and read as a heavy
/// hard rule.
pub fn paint_separator(ui: &mut Ui, style: SeparatorStyle) {
    if matches!(style, SeparatorStyle::None) {
        return;
    }
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, SEPARATOR_STRIP_H), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    paint_into(ui, rect, style, default_ink());
}

/// Interactive variant: same exact strip allocation as
/// [`paint_separator`] (same [`SEPARATOR_STRIP_H`]) but with
/// `Sense::drag` so the user can grab it. Returns the drag
/// `Response` — the pod above is expected to grow / shrink in
/// response to `response.drag_delta()`; it's the caller's job to
/// clamp + persist the new size.
///
/// On hover or drag, the line / dots paint in `accent` (so the
/// affordance lights up clearly when the user reaches for it);
/// otherwise it stays in the same theme-flipped subtle ink as
/// [`paint_separator`].
pub fn paint_separator_resize(
    ui: &mut Ui,
    style: SeparatorStyle,
    id_salt: impl Hash,
    accent: Color32,
) -> Response {
    let w = ui.available_width();
    // Reserve with `Sense::hover` so `allocate_exact_size`'s
    // auto-id doesn't claim our interaction id; the explicit
    // `interact` call below owns the drag id under the caller-
    // supplied salt.
    let (rect, _) = ui.allocate_exact_size(vec2(w, SEPARATOR_STRIP_H), Sense::hover());
    let id = ui.id().with(("frost_separator_resize", id_salt));
    let resp = ui
        .interact(rect, id, Sense::drag())
        .on_hover_cursor(egui::CursorIcon::ResizeVertical);
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let bright = resp.hovered() || resp.dragged();
    let ink = if bright { accent } else { default_ink() };
    paint_into(ui, rect, style, ink);
    resp
}

/// Theme-flipped ink shared by [`paint_separator`] and the rest
/// state of [`paint_separator_resize`].
fn default_ink() -> Color32 {
    let base = style::outline_base();
    Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), SEPARATOR_ALPHA)
}

fn paint_into(ui: &Ui, rect: egui::Rect, style: SeparatorStyle, ink: Color32) {
    let stroke = Stroke::new(RULE_W, ink);
    let mid_y = rect.center().y;
    match style {
        SeparatorStyle::None => {}
        SeparatorStyle::Line => {
            ui.painter().hline(
                (rect.left() + EDGE_INSET)..=(rect.right() - EDGE_INSET),
                mid_y,
                stroke,
            );
        }
        SeparatorStyle::LineDots => {
            let mid_x = rect.center().x;
            for dx in [-DOT_SPACING, 0.0, DOT_SPACING] {
                ui.painter()
                    .circle_filled(egui::pos2(mid_x + dx, mid_y), DOT_R, ink);
            }
            let half = DOT_SPACING + DOT_R + GRIP_HALF_GAP;
            ui.painter().hline(
                (rect.left() + EDGE_INSET)..=(mid_x - half),
                mid_y,
                stroke,
            );
            ui.painter().hline(
                (mid_x + half)..=(rect.right() - EDGE_INSET),
                mid_y,
                stroke,
            );
        }
    }
}
