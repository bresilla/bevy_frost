//! Shared **body** layout helper for [`super::Normal`] and (later)
//! `super::tabbed`. Just a thin wrapper around the cross-axis +
//! optional main-axis clamp so child widgets see a stable
//! `ui.available_*` regardless of the surrounding layout.
//!
//! ## What it does
//!
//! Inside [`Body::paint`]:
//!
//! 1. Clamps the inner ui's CROSS axis to the parent pane's locked
//!    dim (passed in as `cross_inner`). This guarantees a widget
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
    /// the X axis (Top/Bottom title) — body's cross axis is X.
    /// `false` when the strip runs along Y (Left/Right title) —
    /// body's cross axis is Y.
    pub horizontal_strip: bool,
    /// Pane's locked cross-axis size in pixels. Width when
    /// `horizontal_strip`, height otherwise.
    pub cross_inner: f32,
    /// Optional cap on the body's main-axis size — used by
    /// vertical-strip containers to keep total width within the
    /// caller's `CONTAINER_MAX_WIDTH`.
    pub max_main: Option<f32>,
}

impl Body {
    pub fn new(horizontal_strip: bool, cross_inner: f32) -> Self {
        Self {
            horizontal_strip,
            cross_inner,
            max_main: None,
        }
    }

    /// Cap the body's main axis (the dim perpendicular to the
    /// title strip).
    pub fn max_main(mut self, max: f32) -> Self {
        self.max_main = Some(max);
        self
    }

    /// Apply the cross-axis (and optional main-axis) clamp to
    /// `ui`, then run `body`. Keeps `ui.available_*` stable for
    /// child widgets that allocate using those values.
    pub fn paint<R>(&self, ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> R {
        if self.horizontal_strip {
            ui.set_max_width(self.cross_inner);
        } else {
            ui.set_max_height(self.cross_inner);
            if let Some(m) = self.max_main {
                ui.set_max_width(m);
            }
        }
        body(ui)
    }
}
