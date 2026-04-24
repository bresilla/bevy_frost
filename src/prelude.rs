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
    floating::{floating_window, floating_window_scoped, PaneBuilder},
    gizmo_material::GizmoMaterial,
    ribbon::{
        draw_assembly, draw_ribbon_buttons, find_item, find_ribbon, floating_window_for_item,
        paint_drop_ghost_system, panel_anchor, panel_anchor_for_item, Bar, BarRibbon,
        RibbonButton, RibbonClick, RibbonCluster, RibbonConstraint, RibbonDef, RibbonDrag,
        RibbonEdge, RibbonGhostSet, RibbonItem, RibbonKind, RibbonLayout, RibbonMode,
        RibbonOpen, RibbonPlacement, RibbonPlugin, RibbonRole, RibbonWidth, Side, SideActive,
        SideRibbon,
    },
    style::{AccentColor, GlassOpacity, ThemePlugin},
    widgets::{
        axis_drag, card_button, color_rgb, color_rgba, drag_value, dropdown, dropdown_control,
        dual_pane, dual_pane_labelled, group_frame, hybrid_select_row, hybrid_select_row_divided,
        icon, icon_button, keybinding_row, labelled_row, labelled_row_custom_left,
        pretty_progressbar, pretty_progressbar_text, pretty_slider, progressbar_control,
        readout_row, row_separator, row_separator_resize, section, slider_control, stacked_pane,
        stacked_pane_labelled, sub_caption, subsection, toggle, toggle_control, tree_row,
        wide_button,
        HybridSelectResponse, TreeIconKind, TreeIconSlot, TreeRowResponse, HYBRID_SELECT_ROW_H,
        TREE_ROW_H,
    },
    FrostPlugin,
};
