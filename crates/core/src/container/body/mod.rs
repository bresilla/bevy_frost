//! Shared **body** layout helper for [`super::Normal`] and (later)
//! `super::tabbed`. Just a thin wrapper around the span-axis +
//! optional flow-axis clamp so child widgets see a stable
//! `ui.available_*` regardless of the surrounding layout.
//!
//! ## What it does
//!
//! Inside [`Body::paint`]:
//!
//! 1. Clamps the inner ui's CROSS axis to the parent pane's locked
//!    dim (passed in as `span_inner`). This guarantees a widget
//!    that calls `ui.available_width()` / `available_height()` (e.g.
//!    `text_input`) sees the same value across the layout's
//!    measurement passes.
//! 2. Optionally clamps the MAIN axis too, when the caller wants
//!    to bound the body's content size (e.g. `Normal`'s
//!    `CONTAINER_DEFAULT_*` recipe).
//! 3. Runs the user's body closure.

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

    /// Apply the span-axis (and optional flow-axis) clamp to `ui`,
    /// then run `body` inside a forced `top_down` layout so user
    /// content always stacks vertically — regardless of which rail
    /// the parent pane lives on. Without the override, vertical-
    /// strip panes (LEFT / RIGHT rails) inherit a `left_to_right`
    /// layout from the pane and any sequence of widgets the caller
    /// adds (e.g. a column of text inputs) ends up rendered
    /// side-by-side instead of stacked.
    pub fn paint<R>(&self, ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> R {
        if self.horizontal_strip {
            ui.set_max_width(self.span_inner);
        } else {
            ui.set_max_height(self.span_inner);
            if let Some(m) = self.max_flow {
                ui.set_max_width(m);
            }
        }
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), body)
            .inner
    }
}
