//! Shared **body** layout for [`super::Normal`] and [`super::tabbed`]
//! containers.
//!
//! Both container shapes share the same body recipe:
//!
//! 1. The body is a flex item whose **cross axis** is locked to the
//!    parent pane's locked dim (passed in as `cross_inner`).
//! 2. Its **main axis** is content-driven (no fixed `basis`) so the
//!    body shrinks to fit empty content and grows with widgets.
//! 3. Inside the body closure, `set_max_*` clamps the inner UI's
//!    cross axis BEFORE the user's widgets run — keeps
//!    `ui.available_*` stable across flex's intrinsic-measurement
//!    pass and final-layout pass (NOTES.md #1; same trick
//!    `Pane2::lay_out_flex` uses).
//! 4. An optional `max_main` clamp lets the container cap the body
//!    along the main axis (e.g. `Normal`'s `CONTAINER_MAX_WIDTH`
//!    rule for vertical-strip panes).

use egui::{vec2, Align2, Ui, Vec2};

use crate::flex::{item, FlexInstance, FlexItem};

/// Body layout config — the bundle of decisions a container needs
/// to add a content body to its inner flex.
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

    /// `min_size` to set on the flex item — locks cross only,
    /// leaves main free for content-driven growth.
    pub fn min_size(&self) -> Vec2 {
        if self.horizontal_strip {
            vec2(self.cross_inner, 0.0)
        } else {
            vec2(0.0, self.cross_inner)
        }
    }

    /// Build the [`FlexItem`] to pass to `flex.add_ui(item, …)`.
    /// Sets `min_size` on the cross axis and pins the inner content
    /// rect to `LEFT_TOP` so widgets stack from the slot's start.
    /// Without this, egui_flex defaults `align_self_content` to
    /// `CENTER_CENTER` — it positions the inner ui's `min` at the
    /// slot's CENTRE before growing the rect to full size, so a
    /// short body (one widget) ends up vertically centred inside
    /// the slot instead of starting at the top.
    pub fn flex_item(&self) -> FlexItem<'static> {
        item()
            .min_size(self.min_size())
            .align_self_content(Align2::LEFT_TOP)
    }

    /// Inside the flex closure, clamps the inner UI's cross axis
    /// (and optionally the main axis) BEFORE running `body`.
    /// Keeps `ui.available_*` stable across flex passes.
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

    /// One-shot helper: build the flex item, register it with the
    /// flex instance, and run the body closure with the cross-axis
    /// clamp applied. Returns the inner result.
    pub fn add_to_flex<'a, R>(
        self,
        flex: &mut FlexInstance<'a>,
        body: impl FnOnce(&mut Ui) -> R + 'static,
    ) -> R
    where
        R: 'static,
    {
        let cfg = self;
        flex.add_ui(cfg.flex_item(), move |ui| cfg.paint(ui, body))
            .inner
    }
}
