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

use egui::{vec2, Color32, Sense, Stroke, Ui};

use crate::style;

/// Vertical strip thickness — the rect [`paint_separator`] reserves
/// in the parent ui. Tight enough that the separator doesn't add
/// noticeable padding between adjacent pods (the visual cue should
/// be the line / dots themselves, not a thick reserved gap).
pub const SEPARATOR_STRIP_H: f32 = 4.0;
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
    /// Hairline + three centred dots + hairline. Future drag
    /// affordance for resizing the pod above; today it's purely
    /// visual.
    LineDots,
}

/// Paint a separator into the parent `ui`. Allocates a strip of
/// height [`SEPARATOR_STRIP_H`] across the available width, then
/// paints into it according to `style`. `accent` drives the colour
/// via the shared `widget_border` recipe.
pub fn paint_separator(ui: &mut Ui, style: SeparatorStyle, accent: Color32) {
    if matches!(style, SeparatorStyle::None) {
        return;
    }
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, SEPARATOR_STRIP_H), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    // Soft ink — the separator is a quiet affordance, not a hard
    // border. Use `border_subtle` at low alpha so it whispers
    // beneath section frames rather than competing with them.
    let base = style::widget_border(accent);
    let ink = Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), 110);
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
