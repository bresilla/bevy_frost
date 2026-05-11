//! Frost-themed wrappers around the standalone host-widget crates.
//!
//! The vendored node-graph and code-editor implementations live in
//! their own crates ([`frost_graph`], [`frost_code`]) so they can
//! be used independently of `frost_core`. The wrappers in this
//! module bundle each crate's widget with the [`embed`](crate::embed)
//! maximise / restore mechanism and the active frost theme — call
//! them when you want "the graph in a frost-styled pod, with the
//! fullscreen affordance" rather than the bare standalone widget.
//!
//! Feature gates:
//!
//! * `graph` (default-on) — enables [`graph`] and pulls in
//!   [`frost_graph`].
//! * `code` (default-on) — enables [`code`] and pulls in
//!   [`frost_code`].
//!
//! Legacy paths `extras::maximize` (now [`crate::embed`]),
//! `extras::node_view` (now [`frost_graph::node_view`]),
//! `extras::snarl` / `extras::code_editor` (now the contents of
//! the two standalone crates) are kept as re-exports under
//! `#[doc(hidden)]` so existing imports keep compiling.

#[cfg(feature = "graph")]
pub mod graph;
#[cfg(feature = "code")]
pub mod code;

/// Legacy alias — `extras::maximize` was promoted to the
/// crate-root [`embed`](crate::embed) module.
#[doc(hidden)]
pub use crate::embed as maximize;

/// Legacy alias — `extras::node_view` lives in
/// [`frost_graph::node_view`] now that the graph is a standalone
/// crate.
#[cfg(feature = "graph")]
#[doc(hidden)]
pub use frost_graph::node_view;
