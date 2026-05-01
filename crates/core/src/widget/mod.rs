//! Frost-styled widgets that go inside a
//! [`crate::container::Normal`] body — leaf nodes that paint a
//! single primitive (text input, button, slider, …).
//!
//! Inter-pod separators are NOT widgets — they're container-level
//! chrome. See [`crate::container::SeparatorStyle`].

pub mod text_input;

pub use text_input::{text_input, text_input_h};
