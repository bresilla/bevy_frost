//! In-pane container widgets — visual blocks the caller drops into a
//! [`crate::pane::Pane2`] body to organise content. A container holds
//! [`crate::pod::Pod`]s and (between consecutive pods) separators —
//! see [`SeparatorStyle`].
//!
//! Variants:
//!
//! * [`normal`] — single title bar above a single body. The default
//!   building block; matches frostcore's `Section`.
//! * [`tabbed`] — multiple labelled bodies behind a tab strip
//!   (placeholder; not yet implemented).

pub mod body;
pub mod normal;
pub mod separator;
pub mod tabbed;

pub use body::Body;
pub use normal::Normal;
pub use separator::{paint_separator, paint_separator_resize, SeparatorStyle};
