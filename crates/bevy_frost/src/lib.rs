//! # bevy_frost — Bevy integration for the frost UI kit.
//!
//! All UI primitives (widgets, ribbons, floating panels, node-graph
//! wrapper, code editor, theme) live in the framework-agnostic
//! [`frostcore`] crate. This crate adds:
//!
//! * [`FrostPlugin`] — one-line install that registers frostcore's
//!   state types as Bevy `Resource`s and runs the theme + ghost
//!   systems every frame.
//! * [`ThemePlugin`] / [`RibbonPlugin`] — granular alternatives if
//!   you want just one piece.
//! * [`GizmoMaterial`] — always-on-top transform-gizmo material
//!   extension (Bevy-specific).
//!
//! Consumers using `use bevy_frost::prelude::*;` keep the same API
//! they had before the workspace split — this crate re-exports
//! everything from `frostcore` verbatim and adds the plugins on top.
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

pub mod gizmo_material;
pub mod prelude;

// Re-export all of frostcore under `bevy_frost::*` so existing
// consumers don't notice the workspace split.
pub use frostcore::*;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};

// ─── Theme ──────────────────────────────────────────────────────────

/// Registers [`frostcore::AccentColor`] + [`frostcore::GlassOpacity`]
/// as Bevy resources and runs [`frostcore::apply_theme`] every frame.
pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<frostcore::AccentColor>()
            .init_resource::<frostcore::GlassOpacity>()
            .add_systems(PreUpdate, sync_glass_opacity_system)
            .add_systems(EguiPrimaryContextPass, apply_theme_system);
    }
}

fn sync_glass_opacity_system(opacity: Res<frostcore::GlassOpacity>) {
    frostcore::set_glass_opacity(opacity.0);
}

fn apply_theme_system(
    mut contexts: EguiContexts,
    accent: Res<frostcore::AccentColor>,
    opacity: Res<frostcore::GlassOpacity>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    frostcore::apply_theme(ctx, *accent, *opacity);
}

// ─── Ribbons ────────────────────────────────────────────────────────

/// SystemSet the ribbon ghost paint lives in. Downstream plugins
/// can pin their own ribbon-painting panels `.before(RibbonGhostSet)`
/// to keep the ghost on top of their ribbons.
#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct RibbonGhostSet;

/// Registers the ribbon `Resource`s and the ghost drop-preview
/// system. [`FrostPlugin`] installs this transitively.
pub struct RibbonPlugin;

impl Plugin for RibbonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<frostcore::RibbonLayout>()
            .init_resource::<frostcore::SideActive>()
            .init_resource::<frostcore::RibbonOpen>()
            .init_resource::<frostcore::RibbonWidth>()
            .init_resource::<frostcore::RibbonPlacement>()
            .init_resource::<frostcore::RibbonDrag>()
            .configure_sets(
                EguiPrimaryContextPass,
                RibbonGhostSet.after(apply_theme_system),
            )
            .add_systems(
                EguiPrimaryContextPass,
                paint_drop_ghost_system.in_set(RibbonGhostSet),
            );
    }
}

fn paint_drop_ghost_system(
    mut contexts: EguiContexts,
    layout: Res<frostcore::RibbonLayout>,
    accent: Res<frostcore::AccentColor>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    frostcore::paint_drop_ghost(ctx, &*layout, *accent);
}

// ─── Combined install ──────────────────────────────────────────────

/// Full frost install — `ThemePlugin` + `RibbonPlugin`. Idempotent;
/// safe to add alongside any other Bevy plugins.
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
