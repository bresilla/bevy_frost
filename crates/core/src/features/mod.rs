//! Vendored third-party crates pinned at a specific revision so we
//! can edit them in place. See `ACKNOWLEDGEMENTS.md` at the crate
//! root for licenses + upstream attribution (MIT).
//!
//! ## What's here
//!
//! * [`flex`] — flexbox-style layout container, originally
//!   `egui_flex` 0.6.0.
//!   Upstream: <https://github.com/lucasmerlin/hello_egui/tree/main/crates/egui_flex>.
//!
//! Future additions land here too — keep them isolated under
//! `features/` so the namespace doesn't pollute the rest of `core`.

pub mod flex;
