//! Shared **body** layout helper for [`super::Normal`] and (later)
//! `super::tabbed`.
//!
//! Inside [`Body::paint`]:
//!
//! 1. Snapshot the parent ui's available rect and pre-allocate that
//!    exact size via `allocate_ui_with_layout` — gives a fixed-size
//!    slot for the inner [`egui::ScrollArea`] to fill.
//! 2. Clamp the inner ui's CROSS axis to `span_inner` so widgets
//!    that call `ui.available_width()` / `available_height()` see a
//!    stable value across the layout's measurement passes.
//! 3. Wrap the user's body closure in a `ScrollArea` whose scroll
//!    axis matches the container's flow axis (vertical for
//!    horizontal-strip containers, horizontal for vertical-strip).

use egui::Ui;

#[derive(Copy, Clone, Debug)]
pub struct Body {
    /// `true` when the parent container's title strip runs along
    /// the X axis (Top/Bottom title) — body's span axis is X.
    /// `false` when the strip runs along Y (Left/Right title) —
    /// body's span axis is Y.
    pub horizontal_strip: bool,
    /// Pane's locked span-axis size in pixels. Width when
    /// `horizontal_strip`, height otherwise.
    pub span_inner: f32,
    /// Optional cap on the body's flow-axis size — used by
    /// vertical-strip containers to keep total width within the
    /// caller's `CONTAINER_MAX_WIDTH`.
    pub max_flow: Option<f32>,
}

impl Body {
    pub fn new(horizontal_strip: bool, span_inner: f32) -> Self {
        Self {
            horizontal_strip,
            span_inner,
            max_flow: None,
        }
    }

    /// Cap the body's flow axis (the dim perpendicular to the
    /// title strip).
    pub fn max_flow(mut self, max: f32) -> Self {
        self.max_flow = Some(max);
        self
    }

    /// Pre-allocate a fixed-size rect for the body slot, clamp the
    /// cross axis, and wrap `body` in a [`egui::ScrollArea`] whose
    /// scroll axis matches the container's flow axis (vertical for
    /// horizontal-strip containers, horizontal for vertical-strip).
    ///
    /// Two non-obvious settings make scrolling work for the small
    /// body slots a stack of pods produces:
    ///
    /// * `allocate_ui_with_layout(slot_size, …)` instead of letting
    ///   the ScrollArea derive its own size from
    ///   `available_rect_before_wrap`. The latter inside a Frame
    ///   whose content_ui's `max_rect` extends past the
    ///   openness-clipped visible area returns an inflated value,
    ///   and `max_offset = content_size - viewport_size` ends up
    ///   wrong.
    /// * `min_scrolled_height(0.0)` (or `_width`) to disable
    ///   ScrollArea's default `min_scrolled_size = 64`. With the
    ///   default in place, when the actual slot is smaller than 64
    ///   the ScrollArea inflates `inner_size` to 64; the visible
    ///   viewport stays clipped to the real slot but the scroll
    ///   max_offset is computed against the inflated 64, so the
    ///   user can scroll content past the bottom of the visible
    ///   area and the last pod ends up half-cut below.
    pub fn paint<R>(&self, ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> R {
        let slot_size = ui.available_rect_before_wrap().size();
        let scroll_id = if self.horizontal_strip {
            "frost_body_scroll_v"
        } else {
            "frost_body_scroll_h"
        };
        let span_inner = self.span_inner;
        let horizontal_strip = self.horizontal_strip;
        let max_flow = self.max_flow;
        ui.allocate_ui_with_layout(
            slot_size,
            egui::Layout::top_down(egui::Align::Min),
            move |ui| {
                if horizontal_strip {
                    ui.set_max_width(span_inner);
                    egui::ScrollArea::vertical()
                        .id_salt(scroll_id)
                        .auto_shrink([false, false])
                        .min_scrolled_height(0.0)
                        .show(ui, body)
                        .inner
                } else {
                    ui.set_max_height(span_inner);
                    if let Some(m) = max_flow {
                        ui.set_max_width(m);
                    }
                    egui::ScrollArea::horizontal()
                        .id_salt(scroll_id)
                        .auto_shrink([false, false])
                        .min_scrolled_width(0.0)
                        .show(ui, body)
                        .inner
                }
            },
        )
        .inner
    }
}
