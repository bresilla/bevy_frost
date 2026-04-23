//! # Widgets — one file per concept.
//!
//! Each submodule is a focused unit you can customise or fork
//! without sifting through an unrelated pile of code:
//!
//! * [`foldable`]  — the **foldable container** (what you see
//!                   stacked inside panels — accent header + body,
//!                   click the chevron to collapse). Exported as
//!                   [`section`]. Future siblings:
//!                   `unfoldable` (same frame, no collapse header).
//! * [`group`]     — tight rounded frame for clusters of controls
//!                   inside a foldable container (e.g. radio groups,
//!                   button + hint pairs). Exported as
//!                   [`group_frame`].
//! * [`row`]       — labelled row primitives (`labelled_row`,
//!                   `readout_row`, axis readouts, …).
//! * [`layout`]    — proportional-pane arrangements
//!                   (`dual_pane`, `dual_pane_labelled`, …).
//! * [`caption`]   — small-text bits (sub-captions, key-chip rows).
//! * [`button`]    — primary-action (`wide_button`) + card-style
//!                   (`card_button`) buttons.
//! * [`toggle`]    — iOS-style pill on/off switch.
//! * [`slider`]    — custom pretty slider.
//! * [`progressbar`] — read-only sibling of the slider.
//! * [`icon`]      — inline icon glyph (placeholder you'll grow).
//!
//! Each widget is **stateless** — pass plain values in, get an
//! `egui::Response` back, react in the caller. No hidden resources,
//! no plugin install; add [`crate::ThemePlugin`] so they pick up
//! the accent colour and call it a day.

pub mod button;
pub mod caption;
pub mod color;
pub mod drag;
pub mod foldable;
pub mod group;
pub mod icon;
pub mod hybrid_select;
pub mod layout;
pub mod progressbar;
pub mod row;
mod shared;
pub mod slider;
pub mod subsection;
pub mod toggle;

pub use button::{card_button, wide_button};
pub use caption::{keybinding_row, sub_caption};
pub use color::{color_rgb, color_rgba};
pub use drag::{axis_drag, drag_value};

/// Public alias for the widget trailing divider. Use it after a
/// bespoke inline row (e.g. a cluster of small buttons rendered
/// with `ui.horizontal(...)`) so the visual cadence of "every row
/// is a module with a separator" stays unbroken.
pub fn row_separator(ui: &mut bevy_egui::egui::Ui) {
    shared::widget_separator(ui);
}
pub use foldable::section;
pub use group::group_frame;
pub use icon::{icon, icon_button, ICON_BODY_SIZE};
pub use hybrid_select::{
    hybrid_select_row, hybrid_select_row_divided, HybridSelectResponse, HYBRID_SELECT_ROW_H,
};
pub use layout::{
    dual_pane, dual_pane_labelled, stacked_pane, stacked_pane_labelled, DUAL_PANE_LEFT_FRACTION,
};
pub use progressbar::{
    pretty_progressbar, pretty_progressbar_text, progressbar_control,
};
pub use row::{
    axis_readout_row, labelled_row, labelled_row_custom_left, readout_row, LABEL_COL_WIDTH,
};
pub use slider::{pretty_slider, slider_control};
pub use subsection::{subsection, SUBSECTION_BODY_INDENT};
pub use toggle::{toggle, toggle_control};
