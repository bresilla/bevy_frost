//! # frost_graph
//!
//! Standalone node-graph crate for egui. Vendored fork of
//! [`egui-snarl`](https://crates.io/crates/egui-snarl) plus a
//! sharp-zoom [`node_view`] helper that renders the graph into a
//! secondary [`egui::Context`] backed by a wgpu texture.
//!
//! The crate is theme-neutral: it ships [`default_snarl_style`] as
//! a sensible egui-default starting point, and lets the caller
//! configure everything else. Frost-tinted styling lives in the
//! `frostui` crate behind the optional `graph` feature, which
//! depends on this crate and wires the embed / maximise affordance
//! on top.
//!
//! Use it standalone:
//!
//! ```ignore
//! use frost_graph::{Snarl, SnarlWidget, SnarlViewer, default_snarl_style};
//!
//! let style = default_snarl_style();
//! SnarlWidget::new()
//!     .id_salt("my_graph")
//!     .style(style)
//!     .min_size(egui::vec2(320.0, 260.0))
//!     .show(&mut state.graph, &mut state.viewer, ui);
//! ```

mod snarl;
pub mod node_view;

pub use snarl::{
    ui::{
        AnyPins, BackgroundPattern, Dots, Grid, Hex, NodeHalo, NodeLayout, PinInfo, PinPlacement,
        PinShape, SnarlPin, SnarlState, SnarlStyle, SnarlViewer, SnarlWidget, WireColorMode,
    },
    InPin, InPinId, Node, NodeId, OutPin, OutPinId, Snarl,
};

pub use node_view::{NodeViewBackend, NodeViewState, show, show_with_anchor};

/// A [`SnarlStyle`] with library defaults — no frost theming, just
/// `SnarlStyle::new()`. Use this for a vanilla node graph that
/// inherits whatever style the parent `egui::Context` carries.
#[must_use]
pub fn default_snarl_style() -> SnarlStyle {
    SnarlStyle::new()
}
