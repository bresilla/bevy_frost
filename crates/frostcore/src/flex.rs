//! Flexbox-style layout container — frost-themed wrapper around the
//! vendored [`crate::features::flex`] (originally
//! [`egui_flex`](https://github.com/lucasmerlin/hello_egui/tree/main/crates/egui_flex)).
//!
//! Use `Flex::horizontal()` / `Flex::vertical()` to lay out a row or
//! column whose children grow / shrink to fit the available space.
//! Pass each child through [`item()`] to control its grow / shrink /
//! basis / alignment, just like the CSS `flex` shorthand.
//!
//! ```ignore
//! use frostcore::flex::{Flex, item};
//!
//! Flex::horizontal().show(ui, |flex| {
//!     flex.add(item().grow(1.0), egui::Button::new("a"));
//!     flex.add(item().grow(2.0), egui::Button::new("b"));
//! });
//! ```
//!
//! See the vendored module for the full type list — `FlexDirection`,
//! `FlexJustify`, `FlexAlign`, `FlexAlignContent`, `Size`, etc.

pub use crate::features::flex::{
    item, Flex, FlexAlign, FlexAlignContent, FlexContainerResponse, FlexContainerUi,
    FlexDirection, FlexInstance, FlexItem, FlexJustify, FlexWidget, Size,
};
