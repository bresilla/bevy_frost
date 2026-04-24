//! Glob-import for apps building on top of `bevy_frost`.
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_frost::prelude::*;
//! ```
//!
//! Brings in everything `frostcore` exports (widgets, ribbons,
//! floating panels, node-graph wrapper, code editor, theme
//! primitives) plus the Bevy-specific additions from this crate —
//! `FrostPlugin`, `ThemePlugin`, `RibbonPlugin`, `RibbonGhostSet`,
//! and `GizmoMaterial`.

pub use frostcore::prelude::*;

pub use crate::{
    gizmo_material::GizmoMaterial, FrostPlugin, RibbonGhostSet, RibbonPlugin, ThemePlugin,
};
