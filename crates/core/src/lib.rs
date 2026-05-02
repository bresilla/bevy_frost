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
pub mod icons;
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
pub use ribbon::{
    draw_assembly, find_item, find_ribbon, RibbonClick, RibbonCluster, RibbonDef, RibbonDrag,
    RibbonEdge, RibbonGlyph, RibbonItem, RibbonMode, RibbonOpen, RibbonPlacement, RibbonRole,
    RibbonWidth,
};
pub use style::{
    apply_theme, set_glass_opacity, AccentColor, GlassOpacity,
};

// `widgets` is the legacy module name for `widget`. Several apps
// still import `bevy_frost::widgets::*` — keep an alias so they
// compile.
pub use widget as widgets;

// Re-export of the bundled `iconflow` crate so consumers can reach
// `iconflow::list(Pack::Fluentui)`, `Pack`, etc. without their own
// dependency on the same version we ship.
pub use iconflow;
