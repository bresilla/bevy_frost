//! # corekit — modular UI core for `bevy_frost`
//!
//! Successor to `frostcore` (see `PLAN_NEWUI.md` at the repo root).
//! Self-contained: ships its own bundled Iosevka fonts, vendored
//! `egui_flex`, theme runtime, ribbon strip, and flex-based pane
//! system. Nothing here depends on `frostcore`.
//!
//! Naming note: directory is `crates/core/` but the crate identifier
//! is `corekit`. Naming the package `core` would shadow Rust's std
//! `core`, breaking derive macros that expand to `::core::clone::Clone`.
//!
//! ## Modules
//!
//! * [`pane`] — flex-based floating pane (the new pane system).
//!   Phase 1 of the migration; corner-aware title placement,
//!   per-anchor positioning, theme-aware GAME / PRO visuals.
//! * [`style`] — theme + colour + font runtime. `apply_theme` wires
//!   the active `Theme` into egui's `Style`; `set_theme` swaps the
//!   global theme.
//! * [`ribbon`] — edge button strips. Currently a verbatim copy of
//!   `frostcore::ribbon`; will get split / pruned as we iterate.
//! * [`icons`] — Fluent UI System Icon glyph painter.
//! * [`flex`] — wrapper over the vendored `egui_flex` (see
//!   [`features::flex`]).
//! * [`features`] — vendored third-party crates kept here so we can
//!   edit them in place. Each sub-module documents its upstream +
//!   license.

pub mod features;

pub mod flex;
pub mod icons;
pub mod pane;
pub mod ribbon;
pub mod style;

// Re-export of the bundled `iconflow` crate so consumers can reach
// `iconflow::list(Pack::Fluentui)`, `Pack`, etc. without their own
// dependency on the same version we ship.
pub use iconflow;
