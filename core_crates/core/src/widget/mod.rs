//! Frost-styled widgets that go inside a
//! [`crate::container::Normal`] body — leaf nodes that paint a
//! single primitive (text input, button, slider, …).
//!
//! Inter-pod separators are NOT widgets — they're container-level
//! chrome. See [`crate::container::SeparatorStyle`].

pub mod badge;
pub mod button;
pub mod chip;
pub mod color;
pub mod context_menu;
pub mod drag_value;
pub mod dropdown;
pub mod foldable;
pub mod keybinding;
pub mod legacy;
pub mod progressbar;
pub mod readout;
pub mod select;
pub mod slider;
pub mod text_input;
pub mod toggle;
pub mod tree;

pub use button::{
    button, button_h, card_button, Button, FillStyle, BUTTON_LABEL_FONT, BUTTON_ROW_H,
    BUTTON_ROW_H_SUBTITLE, CARD_BUTTON_ROW_H,
};
pub use badge::{
    badge_row, badge_row_colored, BADGE_LABEL_COL_W, BADGE_ROW_H,
};
pub use chip::{chip, chip_colored, CHIP_H};
pub use context_menu::context_menu_frost;
pub use foldable::section;
pub use color::{color_rgb, color_rgba, COLOR_SWATCH_H};
pub use drag_value::{axis_drag, axis_drag_h, drag_value, drag_value_h};
pub use dropdown::{dropdown, dropdown_h, DROPDOWN_ROW_H};
pub use keybinding::{keybinding_row, keybinding_row_h, KEYBINDING_ROW_H};
pub use legacy::{
    dropdown_control, key_chip, labelled_row, labelled_row_custom_left, pretty_slider,
    readout_row, row_separator, search_field, sub_caption, wide_button, LABEL_COL_WIDTH,
};
pub use progressbar::{progressbar, progressbar_h};
pub use readout::{readout, readout_h, READOUT_ROW_H};
pub use select::{
    hybrid_select_row, hybrid_select_row_h, select_row, select_row_h, HybridSelectResponse,
    HYBRID_SELECT_ROW_H, SELECT_ROW_H,
};
pub use slider::{slider, slider_h};
pub use text_input::{text_input, text_input_h};
pub use toggle::{toggle, toggle_h, toggle_track_only};
pub use tree::{
    tree_row, TreeBody, TreeIconKind, TreeIconSlot, TreeRowResponse, TREE_INDENT, TREE_ROW_H,
};
