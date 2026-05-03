//! # corekit — modular UI core for `bevy_frost`
//!
//! Successor to `frostcore` (see `PLAN_NEWUI.md` at the repo root).
//! Self-contained: ships its own bundled Iosevka fonts, theme
//! runtime, ribbon strip, and pane system. Nothing here depends on
//! `frostcore`.
//!
//! Naming note: directory is `crates/core/` but the crate identifier
//! is `corekit`. Naming the package `core` would shadow Rust's std
//! `core`, breaking derive macros that expand to `::core::clone::Clone`.
//!
//! ## Modules
//!
//! * [`pane`] — floating pane with anchored positioning, theme-
//!   aware title strip, and GAME / PRO visuals. Layout is plain
//!   egui (no flex).
//! * [`container`] — in-pane content blocks (`Normal`, `Tabbed`).
//!   A container's body accepts a [`pod::Pod`] — never raw
//!   widgets / closures.
//! * [`pod`] — composable content units; the only thing a
//!   container's body accepts. Built-ins so far: `SearchPod`.
//! * [`widget`] — frost-styled widgets (`text_input`, …).
//! * [`style`] — theme + colour + font runtime. `apply_theme` wires
//!   the active `Theme` into egui's `Style`; `set_theme` swaps the
//!   global theme.
//! * [`ribbon`] — edge button strips.
//! * [`icons`] — Fluent UI System Icon glyph painter.

pub mod command_palette;
pub mod container;
pub mod debug;
pub mod extras;
pub mod floating;
pub mod icons;
pub mod layer;
pub mod pane;
pub mod pod;
pub mod ribbon;
pub mod style;
pub mod widget;

// Foundational row-height unit — re-exported at crate root so the
// canonical name is `corekit::UNIT`. Every widget is sized in
// multiples of this. See [`style::UNIT`] for the definition.
pub use style::{BODY_FONT_SIZE, UNIT};

// ─── Top-level convenience re-exports ─────────────────────────────
//
// `bevy_frost::prelude::*` glob-imports `corekit::*`, so anything
// re-exported here surfaces directly under the consumer's prelude
// (`use bevy_frost::prelude::*;` → `RibbonOpen`, `AccentColor`, …
// in scope). These are the names the old `frostcore` crate
// surfaced — keeping them callable via the same paths means apps
// that hadn't fully migrated to nested-module imports still
// compile against corekit without breakage.

pub use command_palette::{command_palette, CommandPaletteState, PaletteItem};
pub use floating::{floating_window_for_item, PaneBuilder};
pub use ribbon::{
    draw_assembly, find_item, find_ribbon, RibbonClick, RibbonCluster, RibbonDef, RibbonDrag,
    RibbonEdge, RibbonGlyph, RibbonItem, RibbonMode, RibbonOpen, RibbonPlacement, RibbonRole,
    RibbonWidth,
};
pub use style::{
    apply_theme, set_glass_opacity, AccentColor, GlassOpacity,
};

// Surface the free widget functions at the crate root so
// `use bevy_frost::prelude::*;` brings every standalone widget
// (`wide_button`, `readout_row`, `chip`, `toggle`, `tree_row`,
// `keybinding_row`, `badge_row`, `context_menu_frost`, …) into
// scope. The TYPE-style names (`Button`, `TreeIconSlot`, …) sit
// here too so trait-shaped widgets compose without a longer path.
pub use widget::{
    badge_row, badge_row_colored, button, button_h, card_button, chip, chip_colored,
    color_rgb, color_rgba, context_menu_frost, drag_value, drag_value_h, dropdown,
    dropdown_control, dropdown_h, hybrid_select_row, hybrid_select_row_h, key_chip,
    keybinding_row, keybinding_row_h, labelled_row, labelled_row_custom_left, pretty_slider,
    progressbar, progressbar_h, readout, readout_h, readout_row, row_separator,
    search_field, select_row, select_row_h, slider, slider_h, sub_caption, text_input,
    text_input_h, toggle, toggle_h, toggle_track_only, tree_row, wide_button, Button,
    FillStyle, HybridSelectResponse, TreeIconKind, TreeIconSlot, TreeRowResponse,
    BADGE_LABEL_COL_W, BADGE_ROW_H, BUTTON_LABEL_FONT, BUTTON_ROW_H, BUTTON_ROW_H_SUBTITLE,
    CARD_BUTTON_ROW_H, CHIP_H, COLOR_SWATCH_H, DROPDOWN_ROW_H, HYBRID_SELECT_ROW_H,
    KEYBINDING_ROW_H, LABEL_COL_WIDTH, READOUT_ROW_H, SELECT_ROW_H, TREE_INDENT, TREE_ROW_H,
};

// `widgets` is the legacy module name for `widget`. Several apps
// still import `bevy_frost::widgets::*` — keep an alias so they
// compile.
pub use widget as widgets;

// Re-export of the bundled `iconflow` crate so consumers can reach
// `iconflow::list(Pack::Fluentui)`, `Pack`, etc. without their own
// dependency on the same version we ship.
pub use iconflow;
