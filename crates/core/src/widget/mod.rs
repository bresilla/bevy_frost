//! Frost-styled widgets that go inside a
//! [`crate::container::Normal`] body.
//!
//! Each widget lives in its own submodule so the surface stays
//! navigable as more land. For now: just [`text_input`].

pub mod text_input;

pub use text_input::text_input;
