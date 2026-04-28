//! Flexbox-style layout container — wrapper around the vendored
//! [`crate::features::flex`] (originally [`egui_flex`](https://github.com/lucasmerlin/hello_egui/tree/main/crates/egui_flex)).
//!
//! Use `Flex::horizontal()` / `Flex::vertical()` to lay out a row or
//! column whose children grow / shrink to fit the available space.
//! Pass each child through [`item()`] to control its grow / shrink /
//! basis / alignment, just like the CSS `flex` shorthand.

pub use crate::features::flex::{
    item, Flex, FlexAlign, FlexAlignContent, FlexContainerResponse, FlexContainerUi,
    FlexDirection, FlexInstance, FlexItem, FlexJustify, FlexWidget, Size,
};
