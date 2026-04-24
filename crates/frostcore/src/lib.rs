//! # frostcore — framework-agnostic glass UI primitives for egui.
//!
//! Palette + tokens, floating dock windows, side-rail ribbons,
//! reusable widgets (toggles, sliders, inspector rows, colour
//! pickers, trees, dropdowns, node-graph wrapper, code editor).
//! Everything here is plain egui — no Bevy deps unless you flip
//! the `bevy` feature, which only adds `Resource` derives on a
//! few state types so `bevy_frost` can slot them into a Bevy App.
//!
//! Two host crates live on top of this one:
//!
//! * [`bevy_frost`](https://crates.io/crates/bevy_frost) — Bevy
//!   integration: plugins + systems that call the functions here.
//! * [`egui_frost`](https://crates.io/crates/egui_frost) — plain
//!   egui / eframe facade that re-exports `frostcore` directly.
//!
//! Use frostcore when you're driving egui from any other host
//! and want the same widgets + theme without pulling Bevy in.
//!
//! ## Shape
//!
//! * [`style`] — palette, fonts, glass opacity, `apply_theme`.
//! * [`widgets`] — reusable egui widgets built on the tokens.
//! * [`ribbon`] — edge-anchored button strips (static + drag-aware).
//! * [`floating`] — floating dock panels.
//! * [`maximize`] — "expand this widget to full window" wrapper.
//! * [`snarl`] — node-graph widget (egui-snarl) themed to frost.
//! * [`code`] — code-editor widget (egui_code_editor) themed to frost.
//! * [`prelude`] — glob-import module for ergonomic consumer code.

pub mod code;
pub mod command_palette;
pub mod features;
pub mod floating;
pub mod maximize;
pub mod prelude;
pub mod ribbon;
pub mod snarl;
pub mod statusbar;
pub mod style;
pub mod widgets;

// Crate-root re-exports — stable surface so consumers don't have
// to reach into submodules.
pub use command_palette::{command_palette, CommandPaletteState, PaletteItem};
pub use floating::{floating_window, floating_window_scoped, PaneBuilder};
pub use maximize::{is_any_maximized, is_maximized, maximizable, maximize_state_key};
pub use statusbar::statusbar;
pub use ribbon::{
    draw_assembly, draw_ribbon_buttons, find_item, find_ribbon, floating_window_for_item,
    paint_drop_ghost, panel_anchor, panel_anchor_for_item, Bar, BarRibbon, RibbonButton,
    RibbonClick, RibbonCluster, RibbonConstraint, RibbonDef, RibbonDrag, RibbonEdge,
    RibbonItem, RibbonKind, RibbonLayout, RibbonMode, RibbonOpen, RibbonPlacement,
    RibbonRole, RibbonWidth, Side, SideActive, SideRibbon,
};
pub use style::{apply_theme, set_glass_opacity, AccentColor, GlassOpacity};
