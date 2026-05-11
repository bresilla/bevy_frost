//! Glob-import for apps building on top of `bevy_frost`.
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_frost::prelude::*;
//! ```
//!
//! Brings in everything `corekit` exports (panes, ribbons,
//! containers, pods, widgets, theme primitives) plus the
//! Bevy-specific additions from this crate — `FrostPlugin`,
//! `ThemePlugin`, `RibbonPlugin`, `RibbonGhostSet`, and
//! `GizmoMaterial`.

pub use frost_core::*;

pub use crate::{
    gizmo_material::GizmoMaterial,
    node_view_backend::{
        BevyNodeViewBackend, NodeViewCopy, NodeViewPlugin, NodeViewSlots, PendingNodeViewCopies,
    },
    EguiInputAbsorbPlugin, FrostPlugin, RibbonGhostSet, RibbonPlugin, ThemePlugin,
};
