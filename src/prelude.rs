//! The common import line for apps building on top of `bevy_frost`.
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_frost::prelude::*;
//! ```
//!
//! Everything here is re-exported at the crate root too, so you can
//! pick individual symbols if you prefer.

pub use crate::{
    floating::floating_window,
    gizmo_material::GizmoMaterial,
    ribbon::{
        paint_drop_ghost_system, Bar, BarRibbon, RibbonConstraint, RibbonGhostSet, RibbonKind,
        RibbonLayout, RibbonPlugin, Side, SideActive, SideRibbon,
    },
    style::{AccentColor, GlassOpacity, ThemePlugin},
    widgets::{
        axis_drag, card_button, color_rgb, color_rgba, drag_value, dual_pane, dual_pane_labelled,
        group_frame, hybrid_select_row, hybrid_select_row_divided, icon, icon_button,
        keybinding_row, labelled_row, labelled_row_custom_left, pretty_progressbar,
        pretty_progressbar_text, pretty_slider, progressbar_control, readout_row, row_separator,
        section, slider_control, stacked_pane, stacked_pane_labelled, sub_caption, subsection,
        toggle, toggle_control, wide_button, HybridSelectResponse,
    },
    FrostPlugin,
};
