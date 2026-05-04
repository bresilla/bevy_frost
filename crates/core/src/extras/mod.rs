//! Optional host widgets — frost-themed wrappers around vendored
//! `egui-snarl` (node graphs) and `egui_code_editor` (code editor)
//! crates. The vendored sources live under `extras::snarl` and
//! `extras::code_editor` (zero external dependencies); the public
//! [`graph`] / [`code`] modules expose `frost_snarl` /
//! `frost_code_editor` host functions that draw the canvas with
//! the active corekit theme.
//!
//! These were lifted out of the legacy `frostcore::features/` tree
//! and parked in `bevy_frost` rather than `corekit` because they're
//! domain-specific (one app might want a graph, another might not),
//! and corekit stays focused on the panel / pod / widget primitives.

pub(crate) mod code_editor;
pub mod maximize;
pub(crate) mod snarl;
pub mod code;
pub mod graph;
pub mod node_view;
