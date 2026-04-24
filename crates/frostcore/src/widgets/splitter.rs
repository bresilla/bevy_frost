//! Resize-capable variant of [`super::row_separator`].
//!
//! Same vertical rhythm and colour as the plain trailing divider,
//! but its centre carries a three-dot grip that the user can drag
//! vertically to resize whichever scroll area / container sits
//! directly above it. Call the plain [`super::row_separator`]
//! everywhere you'd normally end a module; swap in
//! [`row_separator_resize`] when the thing above is height-owning
//! state the user should be able to adjust.
//!
//! One widget, two modes:
//!
//! * **Usual separator** — `row_separator(ui)` (thin rule, no
//!   interaction).
//! * **Resizer separator** — `row_separator_resize(ui, id, &mut h,
//!   min, max)` (three dots, no rule, vertical drag).
//!
//! ```text
//!   ┌────────── SCENE ──────────────┐
//!   │  tree rows…                    │ ← ScrollArea::max_height(*h)
//!   └────────────────────────────────┘
//!         • • •        ← row_separator_resize — drag me up/down
//! ```
//!
//! Colour matches the plain separator (`BORDER_SUBTLE` + alpha)
//! at rest; hover / drag lift it toward `TEXT_SECONDARY` so the
//! grip obviously wakes up. The hit-rect is taller than the dots
//! themselves so the drag target is comfortable even though the
//! visual footprint is minimal.

use std::hash::Hash;

use egui;

use crate::style::BORDER_SUBTLE;

/// Strip height — the invisible hit-rect the handle interacts on.
/// Tall enough for an easy drag target while the painted dots stay
/// thin visually.
const STRIP_H: f32 = 8.0;
/// Centre-to-centre spacing of the three grip dots.
const DOT_SPACING: f32 = 5.0;
/// Dot radius, in px. Tuned so a three-dot diameter (`2 * DOT_R`)
/// matches the 1 px stroke width visually — larger and the dots
/// look like a heavier "border" than the flanking rules.
const DOT_R: f32 = 0.9;
/// Flanking-rule stroke width. Matches [`widget_border`]'s 1.0 px
/// stroke used by every frost surface (section frames, toggles,
/// inputs), so the grip belongs to one border family.
const RULE_W: f32 = 1.0;

/// Resize-capable separator. Paints three small dots centred in a
/// pair of accent-tinted rules; adds the vertical drag delta to
/// `*size`, clamped to `[min, max]`. Returns the drag `Response` so
/// callers can tell whether the user is currently interacting with
/// the handle.
///
/// `max` is caller-supplied so it can be **dynamic**: pass the
/// content's current natural height (e.g.
/// `ScrollAreaOutput::content_size.y`) to prevent the user from
/// dragging past the point where the scroll would start generating
/// empty space. A fixed pixel cap works too when there's no
/// measurable content.
///
/// `accent` is used for the flanking rule colour via the shared
/// [`widget_border`] recipe, so the grip reads as part of the same
/// border family as every other frost surface. Hover / drag brighten
/// the dots + rules toward `TEXT_SECONDARY`.
///
/// To make the container above actually honour the size, wrap its
/// scroll in `ui.allocate_ui_with_layout(vec2(w, *size), …)` — the
/// scroll's intrinsic `max_height` is clamped to the parent ui's
/// available height, which in a foldable section body is nominally
/// zero. Pre-allocating a sized rect bypasses that cap.
pub fn row_separator_resize(
    ui: &mut egui::Ui,
    id_salt: impl Hash,
    size: &mut f32,
    min: f32,
    max: f32,
    accent: egui::Color32,
) -> egui::Response {
    let w = ui.available_width();
    // Reserve the strip with `hover` sense so allocate_exact_size
    // doesn't claim an interaction id; the explicit interact below
    // owns the click/drag id so the caller's id_salt is the stable
    // anchor (no id collision with `allocate_exact_size`'s auto-id
    // counter).
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(w, STRIP_H),
        egui::Sense::hover(),
    );
    let id = ui.id().with(("frost_row_sep_resize", id_salt));
    let resp = ui.interact(rect, id, egui::Sense::drag());

    if resp.dragged() {
        *size += resp.drag_delta().y;
    }
    // Always clamp — not just on drag — so a caller-supplied default
    // that exceeds the live `max` (e.g. "show 8 rows" with only 6 rows
    // of content) snaps to `max` on the very next frame instead of
    // waiting for the user to nudge the handle. Without this, first
    // paint would render the scroll at the default size with empty
    // space below the content end.
    *size = size.clamp(min, max);
    let resp = resp.on_hover_cursor(egui::CursorIcon::ResizeVertical);

    if ui.is_rect_visible(rect) {
        let bright = resp.hovered() || resp.dragged();
        // Rest: intentionally SOFTER than a section border — the
        // grip is a "quiet" affordance that should recede into the
        // panel until the user reaches for it (alpha 80 over
        // `BORDER_SUBTLE`). Hover / drag lift it to the accent
        // colour so it clearly wakes up *in the accent family*,
        // matching the scrollbar / toggle / button hover language
        // (not a neutral white, which reads as disconnected).
        let soft = egui::Color32::from_rgba_unmultiplied(
            BORDER_SUBTLE.r(),
            BORDER_SUBTLE.g(),
            BORDER_SUBTLE.b(),
            80,
        );
        let ink = if bright { accent } else { soft };
        let mid = rect.center();
        // Three centre dots.
        for dx in [-DOT_SPACING, 0.0, DOT_SPACING] {
            ui.painter()
                .circle_filled(egui::pos2(mid.x + dx, mid.y), DOT_R, ink);
        }
        // Side rules — use `hline` with the same stroke recipe
        // (`Stroke::new(RULE_W, widget_border(accent))`) as every
        // frost surface border, so the flanking lines are visually
        // indistinguishable from a foldable section's edge. Painted
        // with a small gap around the dots so the two don't kiss.
        let grip_half = DOT_SPACING + DOT_R + 3.0;
        let stroke = egui::Stroke::new(RULE_W, ink);
        ui.painter().hline(
            (rect.left() + 2.0)..=(mid.x - grip_half),
            mid.y,
            stroke,
        );
        ui.painter().hline(
            (mid.x + grip_half)..=(rect.right() - 2.0),
            mid.y,
            stroke,
        );
    }
    resp
}
