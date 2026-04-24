//! # bevy_frost — reusable glass-themed editor UI kit.
//!
//! Project-agnostic Bevy + egui primitives: design-system tokens,
//! reusable widgets, floating-dock helpers, edge ribbons with live
//! drag + drop, and the transform-gizmo always-on-top material.
//! Use as a base for editor-style tools; nothing inside is tied to
//! a specific application domain.
//!
//! ## Shape
//!
//! * [`style`] — palette, fonts, glass opacity, `apply_theme`.
//! * [`widgets`] — reusable egui widgets built on the tokens.
//! * [`ribbon`] — edge-anchored button strips (static + drag-aware).
//! * [`floating`] — fixed-size panel anchored to a screen corner.
//! * [`gizmo_material`] — always-on-top `StandardMaterial` extension.
//! * [`prelude`] — glob-import module for ergonomic consumer code.
//!
//! ## Getting started
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_frost::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(bevy_egui::EguiPlugin::default())
//!         .add_plugins(FrostPlugin)
//!         .run();
//! }
//! ```
//!
//! ## Plugins
//!
//! * [`FrostPlugin`] — full install (theme + ribbon, glass look,
//!   drag-aware ribbons, ghost preview). Most consumers just add
//!   this.
//! * [`style::ThemePlugin`] — theme pieces only (`AccentColor`,
//!   `GlassOpacity`, `apply_theme`).
//! * [`ribbon::RibbonPlugin`] — ribbon resources + ghost system.
//!   Depends on a theme having been installed.

pub mod floating;
pub mod gizmo_material;
pub mod prelude;
pub mod ribbon;
pub mod snarl;
pub mod style;
pub mod widgets;

// Crate-root re-exports — stable surface so consumers don't have
// to reach into submodules.
pub use floating::{floating_window, floating_window_scoped};
pub use ribbon::{
    draw_assembly, draw_ribbon_buttons, find_item, find_ribbon, floating_window_for_item,
    paint_drop_ghost_system, panel_anchor, panel_anchor_for_item, Bar, BarRibbon, RibbonButton,
    RibbonClick, RibbonCluster, RibbonConstraint, RibbonDef, RibbonDrag, RibbonEdge,
    RibbonGhostSet, RibbonItem, RibbonKind, RibbonLayout, RibbonMode, RibbonOpen,
    RibbonPlacement, RibbonPlugin, RibbonRole, RibbonWidth, Side, SideActive, SideRibbon,
};
pub use style::{AccentColor, GlassOpacity, ThemePlugin};

use bevy::prelude::*;

/// Full frost install — adds [`style::ThemePlugin`] plus
/// [`ribbon::RibbonPlugin`] in one call. Idempotent; safe to drop
/// alongside any other Bevy plugins.
pub struct FrostPlugin;

impl Plugin for FrostPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        if !app.is_plugin_added::<RibbonPlugin>() {
            app.add_plugins(RibbonPlugin);
        }
    }
}
