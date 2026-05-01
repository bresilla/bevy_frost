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

pub mod container;
pub mod debug;
pub mod icons;
pub mod pane;
pub mod pod;
pub mod ribbon;
pub mod style;
pub mod widget;

// Re-export of the bundled `iconflow` crate so consumers can reach
// `iconflow::list(Pack::Fluentui)`, `Pack`, etc. without their own
// dependency on the same version we ship.
pub use iconflow;
