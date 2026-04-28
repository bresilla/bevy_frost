//! In-pane container widgets — visual blocks the caller drops into a
//! [`crate::pane::Pane2`] body to organise content. Each container
//! defines its own title zone + content zone and grows the pane along
//! the body axis (since `Pane2` is content-driven; see
//! `pane::Pane2::lay_out_flex`).
//!
//! Variants:
//!
//! * [`normal`] — single title bar above a single body. The default
//!   building block; matches frostcore's `Section`.
//! * [`tabbed`] — multiple labelled bodies behind a tab strip
//!   (placeholder; not yet implemented).

pub mod body;
pub mod normal;
pub mod tabbed;

pub use body::Body;
pub use normal::Normal;
